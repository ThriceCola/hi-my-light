use std::sync::mpsc::Receiver;
use std::time::Duration;

use gpui::App;

use crate::workspace;

enum TrayCmd {
    ShowHome,
    Devices,
    Quit,
}

pub struct TrayHost {
    cmds: Receiver<TrayCmd>,
}

impl gpui::Global for TrayHost {}

pub fn install(cx: &mut App) {
    // Linux：tray-icon 走 AppIndicator，必须 gtk::init + GTK 主循环，
    // 和 GPUI 窗口循环冲突，所以只用 StatusNotifier（D-Bus）。
    // Windows：独立消息线程 + Shell_NotifyIcon，不碰 GTK / ksni。
    match build_tray() {
        Some(host) => {
            cx.set_global(host);
            cx.spawn(async move |cx| {
                loop {
                    let wait = cx.update(|cx| {
                        if cx
                            .try_global::<crate::workspace::Workspace>()
                            .and_then(|ws| ws.window.as_ref())
                            .is_some()
                        {
                            Duration::from_millis(80)
                        } else {
                            Duration::from_millis(400)
                        }
                    });
                    cx.background_executor().timer(wait).await;
                    cx.update(poll);
                }
            })
            .detach();
        }
        None => {
            eprintln!("托盘不可用：关掉窗口后进程仍会保留，可用 Alt+F4 退出");
        }
    }
}

fn poll(cx: &mut App) {
    let mut cmds = Vec::new();
    if let Some(host) = cx.try_global::<TrayHost>() {
        while let Ok(cmd) = host.cmds.try_recv() {
            cmds.push(cmd);
        }
    }
    for cmd in cmds {
        match cmd {
            TrayCmd::ShowHome => workspace::activate_home(cx),
            TrayCmd::Devices => workspace::open_devices(cx),
            TrayCmd::Quit => cx.quit(),
        }
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn build_tray() -> Option<TrayHost> {
    None
}

#[cfg(windows)]
fn build_tray() -> Option<TrayHost> {
    win::spawn()
}

#[cfg(target_os = "linux")]
fn build_tray() -> Option<TrayHost> {
    let (tx, rx) = std::sync::mpsc::channel();
    let tray = SniTray {
        pixels: halo_rgba32(),
        tx,
    };
    std::thread::Builder::new()
        .name("hi-my-light-sni".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(err) => {
                    eprintln!("托盘 runtime 失败: {err}");
                    return;
                }
            };
            rt.block_on(async move {
                match ksni::TrayMethods::spawn(tray).await {
                    Ok(_handle) => std::future::pending::<()>().await,
                    Err(err) => eprintln!("StatusNotifier 托盘失败: {err}"),
                }
            });
        })
        .ok()?;

    Some(TrayHost { cmds: rx })
}

#[cfg(target_os = "linux")]
struct SniTray {
    pixels: Vec<u8>,
    tx: std::sync::mpsc::Sender<TrayCmd>,
}

#[cfg(target_os = "linux")]
impl ksni::Tray for SniTray {
    fn id(&self) -> String {
        "hi-my-light".into()
    }

    fn title(&self) -> String {
        "hi-my-light".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: 32,
            height: 32,
            data: rgba_to_argb(&self.pixels),
        }]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            icon_name: String::new(),
            icon_pixmap: Vec::new(),
            title: "hi-my-light".into(),
            description: String::new(),
        }
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.tx.send(TrayCmd::ShowHome);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::*;
        vec![
            StandardItem {
                label: "显示主页".into(),
                activate: Box::new(|this: &mut SniTray| {
                    let _ = this.tx.send(TrayCmd::ShowHome);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "设备".into(),
                activate: Box::new(|this: &mut SniTray| {
                    let _ = this.tx.send(TrayCmd::Devices);
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: "退出".into(),
                activate: Box::new(|this: &mut SniTray| {
                    let _ = this.tx.send(TrayCmd::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

#[cfg(any(target_os = "linux", windows))]
fn halo_rgba32() -> Vec<u8> {
    crate::icon::halo_rgba(32).into_raw()
}

#[cfg(target_os = "linux")]
fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect()
}

#[cfg(windows)]
mod win {
    #![allow(unsafe_op_in_unsafe_fn)]

    use super::{TrayCmd, TrayHost, halo_rgba32};
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicIsize, AtomicU32, Ordering};
    use std::sync::mpsc::Sender;

    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
    use windows_sys::Win32::Graphics::Gdi::{
        BI_BITFIELDS, BITMAPINFO, BITMAPV5HEADER, CreateBitmap, CreateDIBSection, DIB_RGB_COLORS,
        DeleteObject, GetDC, ReleaseDC,
    };
    use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
    use windows_sys::Win32::UI::Shell::{
        NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        AppendMenuW, CreateIconIndirect, CreatePopupMenu, CreateWindowExW, DefWindowProcW,
        DestroyIcon, DestroyMenu, DispatchMessageW, GetCursorPos, GetMessageW, HICON, ICONINFO,
        IMAGE_ICON, LR_DEFAULTSIZE, LR_SHARED, LoadImageW, MF_SEPARATOR, MF_STRING, MSG,
        RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_DESTROY, WM_LBUTTONUP,
        WM_RBUTTONUP, WM_USER, WNDCLASSW, WS_EX_TOOLWINDOW, WS_POPUP,
    };

    const WM_TRAYICON: u32 = WM_USER + 1;
    const ID_SHOW: usize = 1001;
    const ID_DEVICES: usize = 1002;
    const ID_QUIT: usize = 1003;
    const TRAY_ID: u32 = 1;

    static TX: Mutex<Option<Sender<TrayCmd>>> = Mutex::new(None);
    static ICON_ATOM: AtomicIsize = AtomicIsize::new(0);
    static ICON_OWNED: AtomicU32 = AtomicU32::new(0);
    static TASKBAR_CREATED: AtomicU32 = AtomicU32::new(0);

    pub fn spawn() -> Option<TrayHost> {
        let (tx, rx) = std::sync::mpsc::channel();
        *TX.lock().expect("tray tx") = Some(tx);
        std::thread::Builder::new()
            .name("hi-my-light-tray".into())
            .spawn(|| unsafe { message_loop() })
            .ok()?;
        Some(TrayHost { cmds: rx })
    }

    unsafe fn message_loop() {
        let taskbar = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        TASKBAR_CREATED.store(taskbar, Ordering::SeqCst);

        let class_name = wide("HMLTrayHost");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(wndproc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: std::ptr::null_mut(),
            hIcon: std::ptr::null_mut(),
            hCursor: std::ptr::null_mut(),
            hbrBackground: std::ptr::null_mut(),
            lpszMenuName: std::ptr::null(),
            lpszClassName: class_name.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            eprintln!("托盘窗口类注册失败");
            return;
        }

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW,
            class_name.as_ptr(),
            class_name.as_ptr(),
            WS_POPUP,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null(),
        );
        if hwnd.is_null() {
            eprintln!("托盘窗口创建失败");
            return;
        }
        let (icon, owned) = match load_exe_icon() {
            Some(icon) => (icon, false),
            None => match hicon_from_rgba(&halo_rgba32(), 32) {
                Some(icon) => (icon, true),
                None => {
                    eprintln!("托盘图标创建失败");
                    return;
                }
            },
        };
        ICON_ATOM.store(icon as isize, Ordering::SeqCst);
        ICON_OWNED.store(u32::from(owned), Ordering::SeqCst);

        if !add_icon(hwnd, icon) {
            eprintln!("托盘图标添加失败");
            if owned {
                DestroyIcon(icon);
            }
            return;
        }

        let mut msg = std::mem::zeroed::<MSG>();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        let taskbar = TASKBAR_CREATED.load(Ordering::SeqCst);
        if taskbar != 0 && msg == taskbar {
            let icon = ICON_ATOM.load(Ordering::SeqCst) as HICON;
            if !icon.is_null() {
                let _ = add_icon(hwnd, icon);
            }
            return 0;
        }
        if msg == WM_TRAYICON {
            let event = lparam as u32;
            if event == WM_LBUTTONUP {
                send(TrayCmd::ShowHome);
            } else if event == WM_RBUTTONUP {
                show_menu(hwnd);
            }
            return 0;
        }
        if msg == WM_DESTROY {
            delete_icon(hwnd);
            let icon = ICON_ATOM.swap(0, Ordering::SeqCst) as HICON;
            if !icon.is_null() && ICON_OWNED.swap(0, Ordering::SeqCst) != 0 {
                DestroyIcon(icon);
            }
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    fn send(cmd: TrayCmd) {
        if let Ok(guard) = TX.lock() {
            if let Some(tx) = guard.as_ref() {
                let _ = tx.send(cmd);
            }
        }
    }

    unsafe fn add_icon(hwnd: HWND, icon: HICON) -> bool {
        let nid = notify_data(hwnd, icon);
        Shell_NotifyIconW(NIM_ADD, &nid) != 0
    }

    unsafe fn delete_icon(hwnd: HWND) {
        let icon = ICON_ATOM.load(Ordering::SeqCst) as HICON;
        let nid = notify_data(hwnd, icon);
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }

    fn notify_data(
        hwnd: HWND,
        icon: HICON,
    ) -> NOTIFYICONDATAW {
        let mut nid = unsafe { std::mem::zeroed::<NOTIFYICONDATAW>() };
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = TRAY_ID;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = icon;
        let tip = wide("hi-my-light");
        let copy = tip.len().min(nid.szTip.len());
        nid.szTip[..copy].copy_from_slice(&tip[..copy]);
        nid
    }

    unsafe fn show_menu(hwnd: HWND) {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, ID_SHOW, wide("显示主页").as_ptr());
        AppendMenuW(menu, MF_STRING, ID_DEVICES, wide("设备").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
        AppendMenuW(menu, MF_STRING, ID_QUIT, wide("退出").as_ptr());

        let mut pt = POINT { x: 0, y: 0 };
        GetCursorPos(&mut pt);
        SetForegroundWindow(hwnd);
        let cmd = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            pt.x,
            pt.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        DestroyMenu(menu);

        match cmd as usize {
            ID_SHOW => send(TrayCmd::ShowHome),
            ID_DEVICES => send(TrayCmd::Devices),
            ID_QUIT => send(TrayCmd::Quit),
            _ => {}
        }
    }

    unsafe fn load_exe_icon() -> Option<HICON> {
        let module = GetModuleHandleW(std::ptr::null());
        if module.is_null() {
            return None;
        }
        let handle = LoadImageW(
            module,
            1 as _,
            IMAGE_ICON,
            0,
            0,
            LR_DEFAULTSIZE | LR_SHARED,
        );
        if handle.is_null() {
            None
        } else {
            Some(handle)
        }
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    unsafe fn hicon_from_rgba(rgba: &[u8], size: i32) -> Option<HICON> {
        let mut bgra = Vec::with_capacity(rgba.len());
        for pixel in rgba.chunks_exact(4) {
            bgra.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }

        let hdc = GetDC(std::ptr::null_mut());
        if hdc.is_null() {
            return None;
        }

        let mut header = std::mem::zeroed::<BITMAPV5HEADER>();
        header.bV5Size = std::mem::size_of::<BITMAPV5HEADER>() as u32;
        header.bV5Width = size;
        header.bV5Height = -size;
        header.bV5Planes = 1;
        header.bV5BitCount = 32;
        header.bV5Compression = BI_BITFIELDS;
        header.bV5RedMask = 0x00FF_0000;
        header.bV5GreenMask = 0x0000_FF00;
        header.bV5BlueMask = 0x0000_00FF;
        header.bV5AlphaMask = 0xFF00_0000;

        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(
            hdc,
            &header as *const BITMAPV5HEADER as *const BITMAPINFO,
            DIB_RGB_COLORS,
            &mut bits,
            std::ptr::null_mut(),
            0,
        );
        ReleaseDC(std::ptr::null_mut(), hdc);
        if color.is_null() || bits.is_null() {
            return None;
        }
        std::ptr::copy_nonoverlapping(bgra.as_ptr(), bits.cast(), bgra.len());

        let mask = CreateBitmap(size, size, 1, 1, std::ptr::null());
        let info = ICONINFO {
            fIcon: 1,
            xHotspot: 0,
            yHotspot: 0,
            hbmMask: mask,
            hbmColor: color,
        };
        let icon = CreateIconIndirect(&info);
        if !color.is_null() {
            DeleteObject(color);
        }
        if !mask.is_null() {
            DeleteObject(mask);
        }
        if icon.is_null() { None } else { Some(icon) }
    }
}

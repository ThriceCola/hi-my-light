use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::bridge::BleCmd;
use crate::session::Session;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static SENT_CMD: AtomicBool = AtomicBool::new(false);
static WANT_INHIBIT: AtomicBool = AtomicBool::new(false);
static GOT_TERM: AtomicBool = AtomicBool::new(false);

#[derive(Clone)]
struct Snapshot {
    cmd: Sender<BleCmd>,
    addr: Option<String>,
    off_on_shutdown: bool,
    lamp_on: bool,
}

static SNAP: Mutex<Option<Snapshot>> = Mutex::new(None);

pub fn bind(cmd: Sender<BleCmd>) {
    let mut guard = SNAP.lock().expect("shutdown snapshot");
    match guard.as_mut() {
        Some(snap) => snap.cmd = cmd,
        None => {
            *guard = Some(Snapshot {
                cmd,
                addr: None,
                off_on_shutdown: true,
                lamp_on: true,
            });
        }
    }
}

pub fn update_snapshot(addr: Option<String>, off_on_shutdown: bool, lamp_on: bool) {
    if let Some(snap) = SNAP.lock().expect("shutdown snapshot").as_mut() {
        snap.addr = addr;
        snap.off_on_shutdown = off_on_shutdown;
        snap.lamp_on = lamp_on;
    }
    WANT_INHIBIT.store(off_on_shutdown, Ordering::SeqCst);
}

pub fn request() {
    REQUESTED.store(true, Ordering::SeqCst);
    let snap = SNAP.lock().expect("shutdown snapshot").clone();
    let Some(snap) = snap else {
        log_line("收到关机信号，但还没绑定设备状态");
        return;
    };
    if !(snap.off_on_shutdown && snap.lamp_on) {
        log_line("收到关机信号，设置里关灯已关或灯本来就是灭的");
        return;
    }
    if SENT_CMD.swap(true, Ordering::SeqCst) {
        return;
    }
    let mut session = Session::load();
    session.restore_after_shutdown = true;
    session.save();
    let _ = snap
        .cmd
        .send(BleCmd::ShutdownOff(snap.addr.clone()));
    match snap.addr {
        Some(addr) => {
            log_line(&format!("开始独立关灯 {addr}"));
            let result = run_turn_off(&addr);
            log_line(&format!("独立关灯结果: {result}"));
        }
        None => log_line("没有记住的设备地址，无法关灯"),
    }
}

fn run_turn_off(addr: &str) -> String {
    let work = async {
        match tokio::time::timeout(
            Duration::from_secs(4),
            hi_my_light::shutdown_turn_off(addr),
        )
        .await
        {
            Ok(Ok(())) => "ok".into(),
            Ok(Err(err)) => format!("err:{err}"),
            Err(_) => "timeout".into(),
        }
    };
    match tokio::runtime::Handle::try_current() {
        Ok(_) => std::thread::scope(|scope| {
            scope
                .spawn(|| {
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("shutdown runtime")
                        .block_on(work)
                })
                .join()
                .unwrap_or_else(|_| "thread-panic".into())
        }),
        Err(_) => tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map(|rt| rt.block_on(work))
            .unwrap_or_else(|err| format!("runtime:{err}")),
    }
}

pub fn take() -> bool {
    REQUESTED.swap(false, Ordering::SeqCst)
}

pub fn already_fired() -> bool {
    SENT_CMD.load(Ordering::SeqCst)
}

pub fn log_path() -> std::path::PathBuf {
    Session::path()
        .parent()
        .map(|dir| dir.join("shutdown.log"))
        .unwrap_or_else(|| std::path::PathBuf::from("shutdown.log"))
}

pub fn log_line(message: &str) {
    use std::io::Write;
    let path = log_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(file, "{} {message}", now_stamp());
    }
    eprintln!("hml-shutdown: {message}");
}

fn now_stamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:03}", now.as_secs(), now.subsec_millis())
}

pub fn install() {
    let session = Session::load();
    WANT_INHIBIT.store(session.off_on_shutdown, Ordering::SeqCst);
    let _ = ctrlc::set_handler(|| {
        log_line("ctrlc 回调");
        request();
    });
    #[cfg(unix)]
    install_term_handler();
    #[cfg(unix)]
    install_term_poller();
    #[cfg(target_os = "linux")]
    install_logind();
    #[cfg(windows)]
    install_win_session();
}

#[cfg(unix)]
fn install_term_handler() {
    unsafe {
        libc::signal(libc::SIGTERM, term_handler as *const () as libc::sighandler_t);
        libc::signal(libc::SIGINT, term_handler as *const () as libc::sighandler_t);
    }
}

#[cfg(unix)]
extern "C" fn term_handler(_: libc::c_int) {
    GOT_TERM.store(true, Ordering::SeqCst);
}

#[cfg(unix)]
fn install_term_poller() {
    std::thread::Builder::new()
        .name("hml-sigterm".into())
        .spawn(|| loop {
            install_term_handler();
            if GOT_TERM.swap(false, Ordering::SeqCst) {
                log_line("收到 SIGTERM/SIGINT");
                request();
                std::process::exit(0);
            }
            std::thread::sleep(Duration::from_millis(40));
        })
        .ok();
}

#[cfg(target_os = "linux")]
fn install_logind() {
    std::thread::Builder::new()
        .name("hml-logind".into())
        .spawn(|| {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                log_line("logind runtime 创建失败");
                return;
            };
            rt.block_on(watch_logind());
        })
        .ok();
}

#[cfg(target_os = "linux")]
#[zbus::proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Login1Manager {
    fn inhibit(
        &self,
        what: &str,
        who: &str,
        why: &str,
        mode: &str,
    ) -> zbus::Result<zbus::zvariant::OwnedFd>;

    #[zbus(property)]
    fn preparing_for_shutdown(&self) -> zbus::Result<bool>;

    #[zbus(signal)]
    fn prepare_for_shutdown(&self, start: bool) -> zbus::Result<()>;

    #[zbus(signal)]
    fn prepare_for_shutdown_with_metadata(
        &self,
        start: bool,
        metadata: std::collections::HashMap<String, zbus::zvariant::OwnedValue>,
    ) -> zbus::Result<()>;
}

#[cfg(target_os = "linux")]
async fn watch_logind() {
    use futures_lite::StreamExt;

    let Ok(conn) = zbus::Connection::system().await else {
        log_line("连不上 system bus，关灯只能靠 SIGTERM");
        return;
    };
    let Ok(proxy) = Login1ManagerProxy::new(&conn).await else {
        log_line("拿不到 logind Manager");
        return;
    };
    let mut signals = match proxy.receive_prepare_for_shutdown().await {
        Ok(stream) => stream,
        Err(err) => {
            log_line(&format!("订阅 PrepareForShutdown 失败: {err}"));
            return;
        }
    };
    let mut signals_meta = proxy.receive_prepare_for_shutdown_with_metadata().await.ok();

    let mut delay_fd = take_inhibit(&proxy).await;
    if delay_fd.is_some() {
        log_line("已申请 shutdown delay inhibit");
    } else {
        log_line("delay inhibit 未申请到（设置可能是关的，或 logind 拒绝）");
    }

    loop {
        tokio::select! {
            signal = signals.next() => {
                let Some(signal) = signal else {
                    log_line("PrepareForShutdown 流结束");
                    break;
                };
                if signal.args().map(|args| args.start).unwrap_or(false) {
                    log_line("收到 PrepareForShutdown");
                    request();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    delay_fd = None;
                }
            }
            signal = async {
                match signals_meta.as_mut() {
                    Some(stream) => stream.next().await,
                    None => std::future::pending().await,
                }
            } => {
                if let Some(signal) = signal {
                    if signal.args().map(|args| args.start).unwrap_or(false) {
                        log_line("收到 PrepareForShutdownWithMetadata");
                        request();
                        tokio::time::sleep(Duration::from_millis(500)).await;
                        delay_fd = None;
                    }
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(400)) => {
                install_term_handler();
                if GOT_TERM.swap(false, Ordering::SeqCst) {
                    log_line("logind 线程收到 SIGTERM/SIGINT");
                    request();
                    std::process::exit(0);
                }
                if proxy.preparing_for_shutdown().await.ok() == Some(true) {
                    log_line("轮询到 PreparingForShutdown=true");
                    request();
                    tokio::time::sleep(Duration::from_millis(500)).await;
                    delay_fd = None;
                }
                let want = WANT_INHIBIT.load(Ordering::SeqCst);
                if want && delay_fd.is_none() && !SENT_CMD.load(Ordering::SeqCst) {
                    delay_fd = take_inhibit(&proxy).await;
                } else if !want {
                    delay_fd = None;
                }
            }
        }
    }
}

#[cfg(target_os = "linux")]
async fn take_inhibit(proxy: &Login1ManagerProxy<'_>) -> Option<zbus::zvariant::OwnedFd> {
    if !WANT_INHIBIT.load(Ordering::SeqCst) {
        return None;
    }
    match proxy
        .inhibit("shutdown", "hi-my-light", "系统关机时关灯", "delay")
        .await
    {
        Ok(fd) => Some(fd),
        Err(err) => {
            log_line(&format!("Inhibit 失败: {err}"));
            None
        }
    }
}

#[cfg(windows)]
fn install_win_session() {
    std::thread::Builder::new()
        .name("hml-win-shutdown".into())
        .spawn(|| unsafe { win_message_loop() })
        .ok();
}

#[cfg(windows)]
unsafe fn win_message_loop() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, HWND_MESSAGE, MSG,
        RegisterClassW, TranslateMessage, WINDOW_EX_STYLE, WINDOW_STYLE, WM_ENDSESSION,
        WM_QUERYENDSESSION, WNDCLASSW,
    };

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_QUERYENDSESSION {
            request();
            return 1;
        }
        if msg == WM_ENDSESSION {
            request();
            return 1;
        }
        unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
    }

    let class_name: Vec<u16> = "HMLShutdownWatch\0".encode_utf16().collect();
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
        return;
    }
    let hwnd = CreateWindowExW(
        WINDOW_EX_STYLE::default(),
        class_name.as_ptr(),
        class_name.as_ptr(),
        WINDOW_STYLE::default(),
        0,
        0,
        0,
        0,
        HWND_MESSAGE,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        std::ptr::null(),
    );
    if hwnd.is_null() {
        return;
    }
    let mut msg = std::mem::zeroed::<MSG>();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

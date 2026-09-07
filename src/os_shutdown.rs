use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::bridge::BleCmd;
use crate::session::Session;
use hi_my_light::shutdown_once;

static REQUESTED: AtomicBool = AtomicBool::new(false);
static WANT_INHIBIT: AtomicBool = AtomicBool::new(false);
static OFF_ONCE: shutdown_once::ShutdownOnce = shutdown_once::ShutdownOnce::new();

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
    OFF_ONCE.with_lock(|| {
        REQUESTED.store(true, Ordering::SeqCst);
        if OFF_ONCE.already_started() {
            return;
        }
        let snap = SNAP.lock().expect("shutdown snapshot").clone();
        let Some(snap) = snap else {
            log_line("收到关机信号，但还没绑定设备状态");
            return;
        };
        if !shutdown_once::should_turn_off(snap.off_on_shutdown) {
            log_line("收到关机信号，设置里关灯已关");
            OFF_ONCE.mark_started();
            return;
        }
        OFF_ONCE.mark_started();
        let mut session = Session::load();
        session.restore_after_shutdown = true;
        session.save();
        let addr = shutdown_once::turn_off_addr(
            snap.addr.as_deref(),
            session.last_addr.as_deref(),
        );
        let _ = snap.cmd.send(BleCmd::ShutdownOff(addr.clone()));
        match addr {
            Some(addr) => {
                log_line(&format!(
                    "开始独立关灯 {addr} ui_lamp_on={}",
                    snap.lamp_on
                ));
                let result = run_turn_off(&addr);
                log_line(&format!("独立关灯结果: {result}"));
            }
            None => log_line("没有记住的设备地址，无法关灯"),
        }
    });
}

fn run_turn_off(addr: &str) -> String {
    let work = async {
        match tokio::time::timeout(
            Duration::from_secs(8),
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
    OFF_ONCE.already_started()
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
    #[cfg(unix)]
    block_shutdown_signals();
    #[cfg(unix)]
    install_sigwait();
    #[cfg(target_os = "linux")]
    install_logind();
    #[cfg(windows)]
    install_win_session();
}

#[cfg(unix)]
fn block_shutdown_signals() {
    unsafe {
        let mut set = std::mem::zeroed();
        libc::sigemptyset(&mut set);
        libc::sigaddset(&mut set, libc::SIGTERM);
        libc::sigaddset(&mut set, libc::SIGINT);
        libc::pthread_sigmask(libc::SIG_BLOCK, &set, std::ptr::null_mut());
    }
}

/// 屏蔽 SIGTERM 后用 sigwait 收，避免 GPUI/GTK 长时间跑着把 handler 抢走。
#[cfg(unix)]
fn install_sigwait() {
    std::thread::Builder::new()
        .name("hml-sigwait".into())
        .spawn(|| unsafe {
            let mut set = std::mem::zeroed();
            libc::sigemptyset(&mut set);
            libc::sigaddset(&mut set, libc::SIGTERM);
            libc::sigaddset(&mut set, libc::SIGINT);
            loop {
                let mut sig = 0;
                if libc::sigwait(&set, &mut sig) != 0 {
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
                log_line("收到 SIGTERM/SIGINT");
                request();
                std::process::exit(0);
            }
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
            rt.block_on(watch_logind_forever());
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
async fn watch_logind_forever() {
    let mut failures = 0u32;
    loop {
        match watch_logind_once().await {
            WatchEnd::TurnedOff => return,
            WatchEnd::Disconnected(reason) => {
                if shutdown_once::logind_should_reconnect(true) {
                    log_line(&format!("logind 断开，准备重连: {reason}"));
                }
                failures = failures.saturating_add(1);
                let wait = Duration::from_millis(100 * u64::from(failures.min(10)));
                tokio::time::sleep(wait).await;
            }
        }
    }
}

#[cfg(target_os = "linux")]
enum WatchEnd {
    TurnedOff,
    Disconnected(String),
}

#[cfg(target_os = "linux")]
async fn watch_logind_once() -> WatchEnd {
    use futures_lite::StreamExt;

    let Ok(conn) = zbus::Connection::system().await else {
        return WatchEnd::Disconnected("连不上 system bus".into());
    };
    let Ok(proxy) = Login1ManagerProxy::new(&conn).await else {
        return WatchEnd::Disconnected("拿不到 logind Manager".into());
    };
    let mut signals = match proxy.receive_prepare_for_shutdown().await {
        Ok(stream) => stream,
        Err(err) => {
            return WatchEnd::Disconnected(format!("订阅 PrepareForShutdown 失败: {err}"));
        }
    };
    let mut signals_meta = proxy.receive_prepare_for_shutdown_with_metadata().await.ok();

    let mut delay_fd = take_inhibit(&proxy).await;
    if delay_fd.is_some() {
        log_line("已申请 shutdown block inhibit");
    } else {
        log_line("block inhibit 未申请到（设置可能是关的，或 logind 拒绝）");
    }

    loop {
        tokio::select! {
            signal = signals.next() => {
                let Some(signal) = signal else {
                    return WatchEnd::Disconnected("PrepareForShutdown 流结束".into());
                };
                if signal.args().map(|args| args.start).unwrap_or(false) {
                    log_line("收到 PrepareForShutdown");
                    request();
                    delay_fd.take();
                    return WatchEnd::TurnedOff;
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
                        delay_fd.take();
                        return WatchEnd::TurnedOff;
                    }
                } else {
                    signals_meta = None;
                }
            }
            _ = tokio::time::sleep(Duration::from_millis(800)) => {
                if proxy.preparing_for_shutdown().await.ok() == Some(true) {
                    log_line("轮询到 PreparingForShutdown=true");
                    request();
                    delay_fd.take();
                    return WatchEnd::TurnedOff;
                }
                let want = WANT_INHIBIT.load(Ordering::SeqCst);
                if want && delay_fd.is_none() && !OFF_ONCE.already_started() {
                    delay_fd = take_inhibit(&proxy).await;
                    if delay_fd.is_some() {
                        log_line("已重新申请 shutdown block inhibit");
                    }
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
        .inhibit("shutdown", "hi-my-light", "系统关机时关灯", "block")
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
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn win_message_loop() {
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::System::Shutdown::{
        ShutdownBlockReasonCreate, ShutdownBlockReasonDestroy,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, MSG, RegisterClassW,
        TranslateMessage, WM_ENDSESSION, WM_QUERYENDSESSION, WNDCLASSW, WS_EX_TOOLWINDOW,
        WS_POPUP,
    };

    unsafe extern "system" fn wndproc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        if msg == WM_QUERYENDSESSION || msg == WM_ENDSESSION {
            let reason: Vec<u16> = "正在关灯\0".encode_utf16().collect();
            let _ = ShutdownBlockReasonCreate(hwnd, reason.as_ptr());
            request();
            let _ = ShutdownBlockReasonDestroy(hwnd);
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
        log_line("注册关机窗口类失败");
        return;
    }
    // 必须是顶层窗口：HWND_MESSAGE 收不到 WM_QUERYENDSESSION 广播。
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
        log_line("创建关机监听窗口失败");
        return;
    }
    log_line("已监听系统关机");
    let mut msg = std::mem::zeroed::<MSG>();
    while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }
}

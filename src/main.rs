//! hi-my-light

#![cfg_attr(windows, windows_subsystem = "windows")]

mod bridge;
mod chrome;
mod desktop;
mod devices;
mod home;
mod icon;
mod lamp;
mod os_shutdown;
mod service;
mod session;
mod settings;
mod shell;
mod theme;
mod tray;
mod workspace;

use gpui::{App, AppContext as _, KeyBinding, QuitMode, actions};
use gpui_platform::application;

use service::LampService;
use workspace::Workspace;

actions!(hi_my_light, [Quit, OpenDevices, ShowHome, OpenSettings]);

fn main() {
    #[cfg(windows)]
    attach_cli_console();

    #[cfg(windows)]
    set_app_user_model_id();

    if std::env::args().any(|arg| arg == "--install") {
        match desktop::install_user() {
            Ok(exec) => {
                println!("{}", desktop::install_done_status());
                println!("可执行文件: {}", exec.display());
                println!("快捷方式: {}", desktop::menu_entry_path().display());
            }
            Err(err) => {
                eprintln!("安装失败: {err}");
                std::process::exit(1);
            }
        }
        return;
    }

    if std::env::args().any(|arg| arg == "--simulate-shutdown") {
        let session = session::Session::load();
        let Some(addr) = session.last_addr.clone().filter(|addr| !addr.is_empty()) else {
            eprintln!("没有记住的设备，无法模拟关灯");
            std::process::exit(2);
        };
        println!("模拟关机关灯: {addr}");
        os_shutdown::log_line(&format!("simulate-shutdown {addr}"));
        let result = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(async {
                tokio::time::timeout(
                    std::time::Duration::from_secs(8),
                    hi_my_light::shutdown_turn_off(&addr),
                )
                .await
            });
        match result {
            Ok(Ok(())) => {
                let mut session = session;
                session.restore_after_shutdown = true;
                session.save();
                os_shutdown::log_line("simulate-shutdown ok");
                println!("关灯指令已发出");
            }
            Ok(Err(err)) => {
                os_shutdown::log_line(&format!("simulate-shutdown err:{err}"));
                eprintln!("关灯失败: {err}");
                std::process::exit(1);
            }
            Err(_) => {
                os_shutdown::log_line("simulate-shutdown timeout");
                eprintln!("关灯超时");
                std::process::exit(1);
            }
        }
        return;
    }

    os_shutdown::install();
    application().run(|cx: &mut App| {
        cx.set_quit_mode(QuitMode::Explicit);
        cx.bind_keys([
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-q", Quit, None),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("alt-f4", Quit, None),
            #[cfg(target_os = "macos")]
            KeyBinding::new("cmd-d", OpenDevices, None),
            #[cfg(not(target_os = "macos"))]
            KeyBinding::new("ctrl-d", OpenDevices, None),
            KeyBinding::new("ctrl-1", ShowHome, None),
            KeyBinding::new("ctrl-comma", OpenSettings, None),
        ]);
        cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
        cx.on_action(|_: &OpenDevices, cx: &mut App| workspace::show_devices(cx));
        cx.on_action(|_: &ShowHome, cx: &mut App| workspace::show_home(cx));
        cx.on_action(|_: &OpenSettings, cx: &mut App| workspace::show_settings(cx));

        cx.on_app_quit(|cx| {
            if let Some(ws) = cx.try_global::<Workspace>() {
                ws.service.read(cx).persist();
            }
            async {}
        })
        .detach();

        // 关光所有窗口也不退出：BLE + 托盘跟着 App 活着。
        cx.on_window_closed(|cx, id| {
            workspace::forget_window(cx, id);
        })
        .detach();

        let service = cx.new(LampService::new);
        let hidden = desktop::start_hidden();
        cx.set_global(Workspace {
            service: service.clone(),
            window: None,
            shell: None,
        });
        if !hidden {
            let page = workspace::initial_page(&service.read(cx));
            let handle = workspace::open_shell(cx, service, page);
            cx.global_mut::<Workspace>().window = Some(handle);
        }
        tray::install(cx);
        if !hidden {
            cx.activate(true);
        }
    });
}

#[cfg(windows)]
fn attach_cli_console() {
    let cli = std::env::args().any(|arg| {
        matches!(
            arg.as_str(),
            "--install" | "--simulate-shutdown" | "--help" | "-h"
        )
    });
    unsafe {
        use windows_sys::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole, GetConsoleWindow,
        };
        use windows_sys::Win32::UI::WindowsAndMessaging::{SW_HIDE, ShowWindow};

        if cli {
            AttachConsole(ATTACH_PARENT_PROCESS);
            return;
        }
        let hwnd = GetConsoleWindow();
        if !hwnd.is_null() {
            ShowWindow(hwnd, SW_HIDE);
            FreeConsole();
        }
    }
}

#[cfg(windows)]
fn set_app_user_model_id() {
    let id: Vec<u16> = "ThriceCola.HiMyLight\0".encode_utf16().collect();
    unsafe {
        let _ = windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(id.as_ptr());
    }
}

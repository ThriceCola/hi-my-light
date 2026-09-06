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
    // Linux 上 tray-icon 走 AppIndicator，必须 gtk::init + GTK 主循环，
    // 和 GPUI 的窗口循环冲突。这里只用 StatusNotifier（D-Bus）。
    match build_sni() {
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

#[cfg(not(target_os = "linux"))]
fn build_sni() -> Option<TrayHost> {
    None
}

#[cfg(target_os = "linux")]
fn build_sni() -> Option<TrayHost> {
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

#[cfg(target_os = "linux")]
fn halo_rgba32() -> Vec<u8> {
    crate::icon::halo_rgba(32).into_raw()
}

#[cfg(target_os = "linux")]
fn rgba_to_argb(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [p[3], p[0], p[1], p[2]])
        .collect()
}

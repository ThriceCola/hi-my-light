use gpui::{App, AppContext as _, Entity, Global, Window, WindowHandle, WindowId};

use crate::chrome;
use crate::service::LampService;
use crate::session::ClosePreference;
use crate::shell::ShellView;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Devices,
    Settings,
}

pub struct Workspace {
    pub service: Entity<LampService>,
    pub window: Option<WindowHandle<ShellView>>,
    pub shell: Option<Entity<ShellView>>,
}

impl Global for Workspace {}

pub fn initial_page(service: &LampService) -> Page {
    if service.has_known_device() || service.connecting {
        Page::Home
    } else {
        Page::Devices
    }
}

pub fn open_shell(cx: &mut App, service: Entity<LampService>, page: Page) -> WindowHandle<ShellView> {
    service.update(cx, |service, cx| {
        service.wake();
        cx.notify();
    });
    cx.open_window(chrome::window_options(cx), |window, cx| {
        window.set_window_title("HML");
        window.activate_window();
        cx.new(|cx| ShellView::new(service, page, window, cx))
    })
    .expect("无法打开窗口")
}

pub fn show_home(cx: &mut App) {
    show_page(cx, Page::Home);
}

pub fn show_devices(cx: &mut App) {
    show_page(cx, Page::Devices);
}

pub fn show_settings(cx: &mut App) {
    show_page(cx, Page::Settings);
}

pub fn activate_home(cx: &mut App) {
    show_home(cx);
}

pub fn open_devices(cx: &mut App) {
    show_devices(cx);
}

fn show_page(cx: &mut App, page: Page) {
    if let Some(service) = cx.try_global::<Workspace>().map(|ws| ws.service.clone()) {
        service.update(cx, |service, cx| {
            service.wake();
            cx.notify();
        });
    }

    if let Some(shell) = cx.try_global::<Workspace>().and_then(|ws| ws.shell.clone()) {
        shell.update(cx, |shell, cx| shell.show_page(page, cx));
        return;
    }

    if let Some(handle) = cx.try_global::<Workspace>().and_then(|ws| ws.window.clone()) {
        cx.defer(move |cx| {
            let _ = handle.update(cx, |shell, window, cx| {
                shell.show_page(page, cx);
                window.activate_window();
            });
        });
        return;
    }

    let Some(service) = cx.try_global::<Workspace>().map(|ws| ws.service.clone()) else {
        return;
    };
    let handle = open_shell(cx, service, page);
    cx.global_mut::<Workspace>().window = Some(handle);
}

pub fn park_to_background(cx: &mut App) {
    if let Some(service) = cx.try_global::<Workspace>().map(|ws| ws.service.clone()) {
        service.update(cx, |service, cx| {
            service.park();
            cx.notify();
        });
    }
}

pub fn apply_close_choice(cx: &mut App, window: &mut Window, preference: ClosePreference) {
    match preference {
        ClosePreference::Ask => {}
        ClosePreference::Background => {
            park_to_background(cx);
            window.remove_window();
        }
        ClosePreference::Quit => {
            if let Some(service) = cx.try_global::<Workspace>().map(|ws| ws.service.clone()) {
                service.read(cx).persist();
            }
            cx.quit();
        }
    }
}

pub fn forget_window(cx: &mut App, id: WindowId) {
    let Some(ws) = cx.try_global::<Workspace>() else {
        return;
    };
    let gone = ws
        .window
        .as_ref()
        .is_some_and(|handle| handle.window_id() == id);
    if gone {
        if let Some(service) = cx.try_global::<Workspace>().map(|ws| ws.service.clone()) {
            service.update(cx, |service, cx| {
                if !service.parked {
                    service.park();
                    cx.notify();
                }
            });
        }
        let ws = cx.global_mut::<Workspace>();
        ws.window = None;
        ws.shell = None;
    }
}

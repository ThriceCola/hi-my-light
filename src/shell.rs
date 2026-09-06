use gpui::{
    Context, Entity, InteractiveElement, IntoElement, ParentElement, Render, StatefulInteractiveElement,
    Styled, Subscription, Window, div, prelude::*, px, rgb,
};

use crate::chrome::{brand, frame, titlebar, TitleDrag};
use crate::devices::DevicesView;
use crate::home::HomeView;
use crate::service::LampService;
use crate::session::ClosePreference;
use crate::settings::SettingsView;
use crate::theme::{AMBER, INK, LINE, PANEL, STONE, check_box, chip_btn, status_pill, tone};
use crate::workspace::{self, Page};

pub struct ShellView {
    service: Entity<LampService>,
    page: Page,
    home: Entity<HomeView>,
    devices: Entity<DevicesView>,
    settings: Entity<SettingsView>,
    title_drag: bool,
    close_prompt: bool,
    remember_close: bool,
    _observe: Subscription,
    _appearance: Subscription,
}

impl ShellView {
    pub fn new(
        service: Entity<LampService>,
        page: Page,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let home = cx.new(|cx| HomeView::new(service.clone(), cx));
        let devices = cx.new(|cx| DevicesView::new(service.clone(), cx));
        let settings = cx.new(|cx| SettingsView::new(service.clone(), cx));
        if page == Page::Devices {
            devices.update(cx, |view, cx| view.enter(cx));
        }
        let observe = cx.observe(&service, |_, _, cx| cx.notify());
        let appearance = cx.observe_window_appearance(window, |_, window, _| {
            window.refresh();
        });
        window.on_window_should_close(cx, |window, cx| os_should_close(window, cx));
        let entity = cx.entity();
        if cx.try_global::<crate::workspace::Workspace>().is_some() {
            cx.global_mut::<crate::workspace::Workspace>().shell = Some(entity);
        } else {
            cx.defer(move |cx| {
                if cx.try_global::<crate::workspace::Workspace>().is_some() {
                    cx.global_mut::<crate::workspace::Workspace>().shell = Some(entity);
                }
            });
        }
        Self {
            service,
            page,
            home,
            devices,
            settings,
            title_drag: false,
            close_prompt: false,
            remember_close: true,
            _observe: observe,
            _appearance: appearance,
        }
    }

    pub fn show_page(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        if page == Page::Devices {
            self.devices.update(cx, |view, cx| view.enter(cx));
        }
        cx.notify();
    }

    pub fn show_close_prompt(&mut self, cx: &mut Context<Self>) {
        self.close_prompt = true;
        cx.notify();
    }

    fn choose_close(&mut self, preference: ClosePreference, window: &mut Window, cx: &mut Context<Self>) {
        if self.remember_close {
            self.service.update(cx, |service, cx| {
                service.set_close_preference(preference);
                cx.notify();
            });
        }
        self.close_prompt = false;
        workspace::apply_close_choice(cx, window, preference);
        cx.notify();
    }
}

impl TitleDrag for ShellView {
    fn title_dragging(&self) -> bool {
        self.title_drag
    }

    fn set_title_drag(&mut self, dragging: bool) {
        self.title_drag = dragging;
    }

    fn on_close_click(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.service.read(cx).close_preference {
            ClosePreference::Ask => self.show_close_prompt(cx),
            other => workspace::apply_close_choice(cx, window, other),
        }
    }
}

fn os_should_close(_window: &mut Window, cx: &mut gpui::App) -> bool {
    let Some(service) = cx.try_global::<workspace::Workspace>().map(|ws| ws.service.clone()) else {
        return true;
    };
    match service.read(cx).close_preference {
        ClosePreference::Ask => {
            if let Some(handle) = cx.try_global::<workspace::Workspace>().and_then(|ws| ws.window.clone())
            {
                let _ = handle.update(cx, |shell, _, cx| shell.show_close_prompt(cx));
            }
            false
        }
        ClosePreference::Background => {
            workspace::park_to_background(cx);
            true
        }
        ClosePreference::Quit => {
            service.read(cx).persist();
            cx.quit();
            false
        }
    }
}

impl Render for ShellView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        window.set_window_title("HML");
        let (connected, scanning) = {
            let snap = self.service.read(cx);
            (snap.connected(), snap.scanning)
        };
        let page = self.page;
        let prompt = self.close_prompt;
        let remember = self.remember_close;

        let controls = window.window_controls();
        div()
            .size_full()
            .relative()
            .child(frame(
                window,
                titlebar(
                    controls,
                    brand(),
                    div()
                        .flex()
                        .gap_2()
                        .items_center()
                        .child(status_pill(connected, scanning))
                        .child(nav_chip(
                            "nav-home",
                            "主页",
                            page == Page::Home,
                            cx.listener(|this, _, _, cx| this.show_page(Page::Home, cx)),
                        ))
                        .child(nav_chip(
                            "nav-devices",
                            "设备",
                            page == Page::Devices,
                            cx.listener(|this, _, _, cx| this.show_page(Page::Devices, cx)),
                        ))
                        .child(nav_chip(
                            "nav-settings",
                            "设置",
                            page == Page::Settings,
                            cx.listener(|this, _, _, cx| this.show_page(Page::Settings, cx)),
                        )),
                    cx,
                ),
                div().flex_1().min_h(px(0.)).child(match page {
                    Page::Home => self.home.clone().into_any_element(),
                    Page::Devices => self.devices.clone().into_any_element(),
                    Page::Settings => self.settings.clone().into_any_element(),
                }),
            ))
            .when(prompt, |root| {
                root.child(close_dialog(
                    remember,
                    cx.listener(|this, _, _, cx| {
                        this.remember_close = !this.remember_close;
                        cx.notify();
                    }),
                    cx.listener(|this, _, window, cx| {
                        this.choose_close(ClosePreference::Background, window, cx);
                    }),
                    cx.listener(|this, _, window, cx| {
                        this.choose_close(ClosePreference::Quit, window, cx);
                    }),
                    cx.listener(|this, _, _, cx| {
                        this.close_prompt = false;
                        cx.notify();
                    }),
                ))
            })
    }
}

fn close_dialog(
    remember: bool,
    toggle_remember: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_background: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_quit: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_cancel: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id("close-veil")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(tone(INK, 0.72))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(
            div()
                .w(px(360.))
                .px_4()
                .py_4()
                .rounded_lg()
                .border_1()
                .border_color(rgb(LINE))
                .bg(rgb(PANEL))
                .flex()
                .flex_col()
                .gap_3()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(AMBER))
                        .child("关闭窗口"),
                )
                .child(
                    div()
                        .id("remember-close")
                        .flex()
                        .gap_2()
                        .items_center()
                        .cursor_pointer()
                        .on_click(toggle_remember)
                        .child(check_box(remember))
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(STONE))
                                .child("记住选择"),
                        ),
                )
                .child(
                    div()
                        .flex()
                        .gap_2()
                        .justify_end()
                        .child(chip_btn("close-cancel", "取消", false, on_cancel))
                        .child(chip_btn("close-quit", "关闭软件", false, on_quit))
                        .child(chip_btn("close-bg", "留在后台", false, on_background)),
                ),
        )
}

fn nav_chip(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_1p5()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .border_1()
        .border_color(if active { rgb(AMBER) } else { rgb(LINE) })
        .bg(if active { rgb(0x2A2216) } else { rgb(PANEL) })
        .text_color(if active { rgb(AMBER) } else { rgb(STONE) })
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x221E18)))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(label)
}

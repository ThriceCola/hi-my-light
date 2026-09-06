use gpui::{
    Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, rgb,
};

use crate::desktop;
use crate::service::LampService;
use crate::session::ClosePreference;
use crate::theme::{AMBER, LINE, PANEL, STONE, check_box, section_label};

pub struct SettingsView {
    service: Entity<LampService>,
    menu_installed: bool,
    _observe: Subscription,
}

impl SettingsView {
    pub fn new(service: Entity<LampService>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&service, |_, _, cx| cx.notify());
        Self {
            service,
            menu_installed: desktop::is_menu_installed(),
            _observe: observe,
        }
    }
}

impl Render for SettingsView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let preference = self.service.read(cx).close_preference;
        let off_on_shutdown = self.service.read(cx).off_on_shutdown;
        let autostart = self.service.read(cx).autostart;

        div()
            .flex()
            .flex_col()
            .size_full()
            .px_4()
            .py_4()
            .gap_4()
            .child(
                div()
                    .text_xs()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(AMBER))
                    .child("设置"),
            )
            .child(section_label("关闭窗口时"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(pref_row(
                        "pref-ask",
                        "每次询问",
                        preference == ClosePreference::Ask,
                        cx.listener(|this, _, _, cx| {
                            this.service.update(cx, |service, cx| {
                                service.set_close_preference(ClosePreference::Ask);
                                cx.notify();
                            });
                        }),
                    ))
                    .child(pref_row(
                        "pref-bg",
                        "留在后台",
                        preference == ClosePreference::Background,
                        cx.listener(|this, _, _, cx| {
                            this.service.update(cx, |service, cx| {
                                service.set_close_preference(ClosePreference::Background);
                                cx.notify();
                            });
                        }),
                    ))
                    .child(pref_row(
                        "pref-quit",
                        "关闭软件",
                        preference == ClosePreference::Quit,
                        cx.listener(|this, _, _, cx| {
                            this.service.update(cx, |service, cx| {
                                service.set_close_preference(ClosePreference::Quit);
                                cx.notify();
                            });
                        }),
                    )),
            )
            .child(section_label("电源"))
            .child(pref_row(
                "pref-shutdown-off",
                "系统关机时关灯",
                off_on_shutdown,
                cx.listener(|this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_off_on_shutdown(!service.off_on_shutdown);
                        cx.notify();
                    });
                }),
            ))
            .child(section_label("桌面"))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(pref_row(
                        "pref-autostart",
                        "开机自启动（仅托盘）",
                        autostart,
                        cx.listener(|this, _, _, cx| {
                            this.service.update(cx, |service, cx| {
                                service.set_autostart(!service.autostart);
                                cx.notify();
                            });
                            this.menu_installed = desktop::is_menu_installed();
                        }),
                    ))
                    .child(install_row(
                        self.menu_installed,
                        cx.listener(|this, _, _, cx| {
                            match desktop::install_user() {
                                Ok(_) => {
                                    this.menu_installed = true;
                                    this.service.update(cx, |service, cx| {
                                        if service.autostart {
                                            let _ = desktop::sync_autostart(true);
                                        }
                                        service.status = desktop::install_done_status().into();
                                        cx.notify();
                                    });
                                }
                                Err(err) => {
                                    this.service.update(cx, |service, cx| {
                                        service.status = format!("安装失败: {err}").into();
                                        cx.notify();
                                    });
                                }
                            }
                            cx.notify();
                        }),
                    )),
            )
    }
}

fn install_row(
    installed: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let title = desktop::install_title(installed);
    div()
        .id("pref-install-menu")
        .flex()
        .gap_3()
        .items_center()
        .px_3()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(rgb(LINE))
        .bg(rgb(PANEL))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x221E18)))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(0xE8E0D4))
                        .child(title),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(STONE))
                        .child(desktop::install_hint(installed)),
                ),
        )
}

fn pref_row(
    id: &'static str,
    title: &'static str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .gap_3()
        .items_center()
        .px_3()
        .py_2()
        .rounded_lg()
        .border_1()
        .border_color(if active { rgb(AMBER) } else { rgb(LINE) })
        .bg(if active { rgb(0x2A2216) } else { rgb(PANEL) })
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x221E18)))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(check_box(active))
        .child(
            div()
                .text_sm()
                .text_color(if active { rgb(AMBER) } else { rgb(0xE8E0D4) })
                .child(title),
        )
}

use gpui::{
    Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px, rgb,
};

use crate::desktop;
use crate::service::LampService;
use crate::session::ClosePreference;
use crate::theme::{HOVER, INK, LINE, PAPER, STONE, check_box, ghost_btn, meta};

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
            .size_full()
            .child(
                div()
                    .w(px(168.))
                    .h_full()
                    .px_4()
                    .py_7()
                    .border_r_1()
                    .border_color(rgb(LINE))
                    .flex()
                    .flex_col()
                    .gap_3()
                    .child(nav_item("DEVICE", true))
                    .child(nav_item("CLOSE", false)),
            )
            .child(
                div()
                    .id("settings-page")
                    .flex_1()
                    .h_full()
                    .min_w(px(0.))
                    .px_10()
                    .py_9()
                    .overflow_y_scroll()
                    .flex()
                    .flex_col()
                    .child(meta("MANUAL 01"))
                    .child(
                        div()
                            .mt_1()
                            .mb_6()
                            .text_size(px(42.))
                            .font_weight(FontWeight::BOLD)
                            .line_height(px(44.))
                            .text_color(rgb(PAPER))
                            .child("Device"),
                    )
                    .child(section_row(
                        "关闭窗口",
                        preference_label(preference),
                    ))
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
                    ))
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
                    ))
            )
    }
}

fn preference_label(preference: ClosePreference) -> &'static str {
    match preference {
        ClosePreference::Ask => "Ask",
        ClosePreference::Background => "Tray",
        ClosePreference::Quit => "Quit",
    }
}

fn nav_item(label: &'static str, current: bool) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if current { rgb(PAPER) } else { rgb(STONE) })
        .child(label)
}

fn section_row(title: &'static str, value: &'static str) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .py_3()
        .border_t_1()
        .border_color(rgb(LINE))
        .child(div().text_sm().text_color(rgb(PAPER)).child(title))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(STONE))
                .child(value),
        )
}

fn install_row(
    installed: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    let title = desktop::install_title(installed);
    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .py_3()
        .border_t_1()
        .border_color(rgb(LINE))
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .child(div().text_sm().text_color(rgb(PAPER)).child(title))
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(STONE))
                        .child(desktop::install_hint(installed)),
                ),
        )
        .child(ghost_btn("pref-install-menu", "INSTALL", false, on_click))
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
        .w_full()
        .items_center()
        .justify_between()
        .py_3()
        .border_t_1()
        .border_color(rgb(LINE))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(HOVER)))
        .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(
            div()
                .flex()
                .gap_3()
                .items_center()
                .child(check_box(active))
                .child(
                    div()
                        .text_sm()
                        .text_color(if active { rgb(PAPER) } else { rgb(STONE) })
                        .child(title),
                ),
        )
        .child(
            div()
                .px_3()
                .py_1()
                .border_1()
                .border_color(rgb(LINE))
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if active { rgb(INK) } else { rgb(STONE) })
                .bg(if active { rgb(PAPER) } else { rgb(0x000000) })
                .child(if active { "ON" } else { "OFF" }),
        )
}

use gpui::{
    Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, px, rgb,
};

use crate::desktop;
use crate::service::LampService;
use crate::session::ClosePreference;
use crate::theme::{HOVER, LINE, PAPER, STONE, check_box, ghost_btn};

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
            .id("settings-page")
            .size_full()
            .px_10()
            .py_9()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .child(
                div()
                    .mb_6()
                    .text_size(px(32.))
                    .font_weight(FontWeight::BOLD)
                    .line_height(px(36.))
                    .text_color(rgb(PAPER))
                    .child("设置"),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_8()
                    .child(section(
                        "关闭窗口",
                        div()
                            .flex()
                            .flex_col()
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
                    ))
                    .child(section(
                        "灯光",
                        pref_row(
                            "pref-shutdown-off",
                            "系统关机时关灯",
                            off_on_shutdown,
                            cx.listener(|this, _, _, cx| {
                                this.service.update(cx, |service, cx| {
                                    service.set_off_on_shutdown(!service.off_on_shutdown);
                                    cx.notify();
                                });
                            }),
                        ),
                    ))
                    .child(section(
                        "本机",
                        div()
                            .flex()
                            .flex_col()
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
                                                service.status =
                                                    desktop::install_done_status().into();
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
                    )),
            )
    }
}

fn section(title: &'static str, body: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(STONE))
                .child(title),
        )
        .child(body)
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
        .child(ghost_btn("pref-install-menu", "安装", false, on_click))
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
}

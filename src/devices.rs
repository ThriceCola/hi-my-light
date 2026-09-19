use gpui::{
    Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*, px, rgb,
    uniform_list,
};

use crate::bridge::DeviceRow;
use crate::service::LampService;
use crate::theme::{HOVER, INK, LINE, PAPER, STONE, chip_btn, status_pill};

const ROW_H: f32 = 76.0;

pub struct DevicesView {
    service: Entity<LampService>,
    _observe: Subscription,
}

impl DevicesView {
    pub fn new(service: Entity<LampService>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&service, |_, _, cx| cx.notify());
        Self {
            service,
            _observe: observe,
        }
    }

    pub fn enter(&mut self, cx: &mut Context<Self>) {
        self.service.update(cx, |service, cx| {
            if !service.scanning && !service.connecting {
                service.scan();
                cx.notify();
            }
        });
    }
}

impl Render for DevicesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (count, scanning, ready, connected, status) = {
            let snap = self.service.read(cx);
            (
                snap.devices.len(),
                snap.scanning,
                snap.ready,
                snap.connected(),
                snap.status.clone(),
            )
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.render_header(connected, scanning, ready, cx))
            .child(
                div()
                    .id("device-scroll")
                    .flex_1()
                    .w_full()
                    .min_h(px(0.))
                    .px_4()
                    .pb_3()
                    .child(if count == 0 {
                        div()
                            .size_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_sm()
                            .text_color(rgb(STONE))
                            .child(if scanning { "扫描中" } else { "未发现设备" })
                            .into_any_element()
                    } else {
                        uniform_list(
                            "device-rows",
                            count,
                            cx.processor(|this, range: std::ops::Range<usize>, _, cx| {
                                let devices = this.service.read(cx).devices.clone();
                                let connected_addr =
                                    this.service.read(cx).connected_addr.clone();
                                let mut items = Vec::new();
                                for ix in range {
                                    if let Some(device) = devices.get(ix) {
                                        items.push(device_row(
                                            device,
                                            connected_addr.as_deref(),
                                            cx,
                                        ));
                                    }
                                }
                                items
                            }),
                        )
                        .h_full()
                        .into_any_element()
                    }),
            )
            .child(
                div()
                    .px_4()
                    .py_2()
                    .border_t_1()
                    .border_color(rgb(LINE))
                    .text_xs()
                    .text_color(rgb(STONE))
                    .child(status),
            )
    }
}

impl DevicesView {
    fn render_header(
        &self,
        connected: bool,
        scanning: bool,
        ready: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w_full()
            .px_10()
            .pt_9()
            .pb_4()
            .gap_4()
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_end()
                    .justify_between()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(rgb(STONE))
                                    .child("MANUAL 01"),
                            )
                            .child(
                                div()
                                    .text_size(px(42.))
                                    .font_weight(FontWeight::BOLD)
                                    .line_height(px(44.))
                                    .text_color(rgb(PAPER))
                                    .child("Device"),
                            ),
                    )
                    .child(status_pill(connected, scanning)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(chip_btn(
                        "scan",
                        if scanning { "SCAN…" } else { "SCAN" },
                        !ready || scanning,
                        cx.listener(|this, _, _, cx| {
                            this.service.update(cx, |service, cx| {
                                service.scan();
                                cx.notify();
                            });
                        }),
                    ))
                    .when(connected, |row| {
                        row.child(chip_btn(
                            "disc",
                            "DISCONNECT",
                            false,
                            cx.listener(|this, _, _, cx| {
                                this.service.update(cx, |service, cx| {
                                    service.disconnect();
                                    cx.notify();
                                });
                            }),
                        ))
                    }),
            )
    }
}

fn device_row(
    device: &DeviceRow,
    connected_addr: Option<&str>,
    cx: &mut Context<DevicesView>,
) -> gpui::AnyElement {
    let addr = device.addr.clone();
    let preferred = device.is_preferred();
    let selected = connected_addr == Some(addr.as_str());
    let rssi = device
        .rssi
        .map(|v| format!("{v} dBm"))
        .unwrap_or_else(|| "—".into());

    div()
        .id(gpui::SharedString::from(format!("dev-{addr}")))
        .h(px(ROW_H))
        .w_full()
        .px_1()
        .py_1()
        .child(
            div()
                .id(gpui::SharedString::from(format!("dev-body-{addr}")))
                .flex()
                .size_full()
                .px_3()
                .justify_between()
                .items_center()
                .cursor_pointer()
                .border_t_1()
                .border_color(rgb(LINE))
                .bg(if preferred || selected {
                    rgb(PAPER)
                } else {
                    rgb(0x000000)
                })
                .hover(|s| {
                    if preferred || selected {
                        s
                    } else {
                        s.bg(rgb(HOVER))
                    }
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    let addr = addr.clone();
                    this.service.update(cx, |service, cx| {
                        service.connect(addr);
                        cx.notify();
                    });
                    crate::workspace::show_home(cx);
                }))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .gap_1()
                        .child(
                            div()
                                .flex()
                                .gap_2()
                                .items_center()
                                .child(div().text_sm().text_color(if preferred || selected { rgb(INK) } else { rgb(PAPER) }).child(device.name.clone()))
                                .when(preferred, |row| {
                                    row.child(
                                        div()
                                            .px_1()
                                            .bg(rgb(INK))
                                            .text_color(rgb(PAPER))
                                            .text_xs()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .child("PRIMARY"),
                                    )
                                }),
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(rgb(STONE))
                                .child(device.addr.clone()),
                        ),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if selected || preferred {
                            rgb(INK)
                        } else {
                            rgb(STONE)
                        })
                        .child(if selected {
                            "CONNECTED".to_string()
                        } else {
                            rssi
                        }),
                ),
        )
        .into_any_element()
}

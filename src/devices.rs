use gpui::{
    Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Subscription, Window, div, prelude::*, px, rgb,
    uniform_list,
};

use crate::bridge::DeviceRow;
use crate::service::LampService;
use crate::theme::{AMBER, LINE, STONE, chip_btn, status_pill};

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
            .px_4()
            .pt_4()
            .pb_3()
            .gap_3()
            .border_b_1()
            .border_color(rgb(LINE))
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(AMBER))
                            .child("设备"),
                    )
                    .child(status_pill(connected, scanning)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(chip_btn(
                        "scan",
                        if scanning { "扫描…" } else { "扫描" },
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
                            "断开",
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
                .rounded_lg()
                .justify_between()
                .items_center()
                .cursor_pointer()
                .border_1()
                .border_color(if preferred {
                    rgb(AMBER)
                } else if selected {
                    rgb(0x5A4A30)
                } else {
                    rgb(LINE)
                })
                .bg(if preferred {
                    rgb(0x2A2216)
                } else if selected {
                    rgb(0x1C1914)
                } else {
                    rgb(0x181612)
                })
                .when(preferred, |row| {
                    row.shadow(vec![gpui::BoxShadow {
                        color: gpui::hsla(0.11, 0.48, 0.42, 0.55),
                        offset: gpui::point(px(0.), px(6.)),
                        blur_radius: px(22.),
                        spread_radius: px(2.),
                        inset: false,
                    }])
                })
                .hover(|s| s.bg(rgb(0x221E18)))
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
                                .child(div().text_sm().child(device.name.clone()))
                                .when(preferred, |row| {
                                    row.child(
                                        div()
                                            .px_1()
                                            .rounded_sm()
                                            .bg(rgb(AMBER))
                                            .text_color(rgb(0x1A140C))
                                            .text_xs()
                                            .child("首选"),
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
                            rgb(AMBER)
                        } else {
                            rgb(STONE)
                        })
                        .child(if selected {
                            "已连接".to_string()
                        } else {
                            rssi
                        }),
                ),
        )
        .into_any_element()
}

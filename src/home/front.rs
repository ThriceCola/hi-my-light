use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, px, rgb,
};

use crate::lamp::SCENES;
use crate::theme::{AMBER, AMBER_SOFT, LINE, PANEL, STONE, glow_from_kelvin};

use super::drag::Track;
use super::slider::{self, Fill};
use super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let (on, level, kelvin) = {
        let snap = this.service.read(cx);
        (
            snap.front.on,
            snap.front.level.percent(),
            snap.front.cct.kelvin(),
        )
    };

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .child(card().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(power_row("前置主灯", on, cx.listener(|this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.toggle_front();
                        cx.notify();
                    });
                })))
                .child(slider::row(
                    "亮度",
                    format!("{:.0}%", level),
                    Track::FrontLevel,
                    level / 100.0,
                    Fill::Solid(glow_from_kelvin(kelvin)),
                    cx,
                )),
        ))
        .child(card().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(slider::row(
                    "色温",
                    format!("{kelvin}K"),
                    Track::FrontCct,
                    (kelvin as f32 - 3000.0) / 3000.0,
                    Fill::Cct,
                    cx,
                ))
                .child(cct_presets(kelvin, cx)),
        ))
        .child(scenes(on, level, kelvin, cx))
}

fn cct_presets(kelvin: u16, cx: &mut Context<HomeView>) -> impl IntoElement {
    const PRESETS: [(u16, &'static str); 5] = [
        (3000, "暖"),
        (4000, "柔"),
        (4500, "中"),
        (5000, "亮"),
        (6000, "冷"),
    ];
    div()
        .flex()
        .w_full()
        .gap_1()
        .children(PRESETS.into_iter().map(|(k, label)| {
            let active = kelvin.abs_diff(k) < 80;
            div()
                .id(gpui::SharedString::from(format!("cct-{k}")))
                .flex_1()
                .h(px(40.))
                .px_1()
                .py_1()
                .rounded_md()
                .cursor_pointer()
                .flex()
                .flex_col()
                .justify_center()
                .items_center()
                .gap_0p5()
                .border_1()
                .border_color(if active { rgb(AMBER) } else { rgb(LINE) })
                .bg(if active { rgb(0x241C12) } else { rgb(0x181510) })
                .hover(|s| s.bg(rgb(0x221E18)))
                .child(
                    div()
                        .text_xs()
                        .text_color(if active { rgb(AMBER) } else { rgb(STONE) })
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(if active { rgb(AMBER_SOFT) } else { rgb(0x5A564C) })
                        .child(format!("{k}")),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_cct(k as f32);
                        service.flush_now();
                        cx.notify();
                    });
                }))
        }))
}

fn scenes(on: bool, level: f32, kelvin: u16, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .gap_2()
        .children(SCENES.iter().map(|scene| {
            let scene = *scene;
            let active = on
                && (scene.level.percent() - level).abs() < 1.5
                && scene.cct.kelvin().abs_diff(kelvin) < 80;
            div()
                .id(scene.id)
                .flex_1()
                .h(px(40.))
                .px_2()
                .rounded_md()
                .cursor_pointer()
                .flex()
                .justify_center()
                .items_center()
                .border_1()
                .border_color(if active { rgb(AMBER) } else { rgb(LINE) })
                .bg(if active { rgb(0x241C12) } else { rgb(PANEL) })
                .hover(|s| s.bg(rgb(0x221E18)))
                .text_sm()
                .text_color(if active { rgb(AMBER_SOFT) } else { rgb(STONE) })
                .child(scene.name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_scene(scene);
                        cx.notify();
                    });
                }))
        }))
}

pub fn card() -> gpui::Div {
    div()
        .w_full()
        .px_3()
        .py_3()
        .rounded_lg()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(LINE))
}

pub fn power_row(
    title: &'static str,
    on: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_sm()
                .text_color(rgb(0xE8E0D4))
                .child(title),
        )
        .child(
            div()
                .id(format!("power-{title}"))
                .w(px(48.))
                .h(px(26.))
                .rounded_full()
                .bg(if on { rgb(AMBER) } else { rgb(0x3A3630) })
                .cursor_pointer()
                .px_0p5()
                .flex()
                .items_center()
                .when(on, |d| d.justify_end())
                .when(!on, |d| d.justify_start())
                .child(
                    div()
                        .size(px(20.))
                        .rounded_full()
                        .bg(if on { rgb(0x1A140C) } else { rgb(0xC8C0B4) }),
                )
                .on_click(on_click),
        )
}

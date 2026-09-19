use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px, rgb};

use crate::theme::{LINE, STONE, glow_from_kelvin};

use super::super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let (on, level, kelvin) = {
        let snap = this.service.read(cx);
        (
            snap.front.on,
            snap.front.level.percent(),
            snap.front.cct.kelvin(),
        )
    };
    let glow = if on {
        glow_from_kelvin(kelvin)
    } else {
        LINE
    };

    div()
        .id("kelvin-readout")
        .flex_none()
        .flex()
        .w_full()
        .items_end()
        .justify_between()
        .py_4()
        .opacity(if on { 1.0 } else { 0.32 })
        .child(column("亮度", format!("{:.0}", level), glow, false))
        .child(column("色温", format!("{kelvin}"), glow, true))
}

fn column(label: &'static str, value: String, color: u32, right: bool) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .when(right, |d| d.items_end())
        .when(!right, |d| d.items_start())
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(STONE))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_size(px(96.))
                .font_weight(gpui::FontWeight::BOLD)
                .line_height(px(82.))
                .text_color(rgb(color))
                .child(value),
        )
}

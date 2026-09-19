use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, rgb};

use crate::theme::{STONE, step_btn};

use super::super::drag::Track;
use super::super::slider::{self, Fill};
use super::super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let kelvin = this.service.read(cx).front.cct.kelvin();

    div()
        .id("cct-block")
        .flex_none()
        .flex()
        .flex_col()
        .w_full()
        .pt_2()
        .child(slider::track_bar(
            Track::FrontCct,
            (kelvin as f32 - 3000.0) / 3000.0,
            Fill::Cct,
            cx,
        ))
        .child(
            div()
                .mt_2()
                .flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(step_btn(
                    "cct-dec",
                    "−",
                    cx.listener(move |this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.set_cct(kelvin as f32 - 100.0);
                            service.flush_now();
                            cx.notify();
                        });
                    }),
                ))
                .child(
                    div()
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(STONE))
                        .child("3000 — 6000"),
                )
                .child(step_btn(
                    "cct-inc",
                    "+",
                    cx.listener(move |this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.set_cct(kelvin as f32 + 100.0);
                            service.flush_now();
                            cx.notify();
                        });
                    }),
                )),
        )
}

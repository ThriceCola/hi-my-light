use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use crate::theme::{LINE, glow_from_kelvin, step_btn};

use super::super::drag::Track;
use super::super::slider::{self, Fill};
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
        .id("brightness-block")
        .flex_none()
        .flex()
        .w_full()
        .items_center()
        .gap_3()
        .pt_4()
        .pb_4()
        .child(
            div().flex_1().min_w(px(0.)).child(slider::track_bar(
                Track::FrontLevel,
                level / 100.0,
                Fill::Solid(glow),
                cx,
            )),
        )
        .child(
            div()
                .flex()
                .gap_1p5()
                .child(step_btn(
                    "bri-dec",
                    "−",
                    cx.listener(move |this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.set_front_level(level - 5.0);
                            service.flush_now();
                            cx.notify();
                        });
                    }),
                ))
                .child(step_btn(
                    "bri-inc",
                    "+",
                    cx.listener(move |this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.set_front_level(level + 5.0);
                            service.flush_now();
                            cx.notify();
                        });
                    }),
                )),
        )
}

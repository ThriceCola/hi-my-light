use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, IntoElement, ParentElement, Styled, canvas, div, prelude::*,
    px, rgb,
};

use crate::chrome::connecting_overlay;
use crate::lamp::RearLook;
use crate::theme::{LINE, PAPER};

use super::preview::{self, LampPreview};
use super::HomeView;

pub fn render_stage(
    this: &HomeView,
    connecting: bool,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let snap = this.service.read(cx);
    let preview = LampPreview {
        aspect: this.aspect,
        front_on: snap.front.on,
        front_level: snap.front.level.percent(),
        front_kelvin: snap.front.cct.kelvin(),
        rear_on: snap.rear.on,
        rear_level: snap.rear.level.percent(),
        rear: snap.rear.look,
        phase: 0.0,
    };
    let front_caption = if preview.front_on {
        format!("前置  {:.0}%  {}K", preview.front_level, preview.front_kelvin)
    } else {
        "前置  关".into()
    };
    let rear_caption = if !preview.rear_on {
        "后置  关".into()
    } else {
        match preview.rear {
            RearLook::Solid(color) => format!("后置  {:.0}%  #{:06x}", preview.rear_level, color.packed()),
            RearLook::Play(effect) => format!("后置  {:.0}%  {}", preview.rear_level, effect.name()),
        }
    };
    let playing = preview.rear_on && matches!(preview.rear, RearLook::Play(_));
    let period = preview::period_ms(preview.rear, snap.rear.speed.percent());
    let aspect = preview.aspect;
    let front_on = preview.front_on;
    let front_level = preview.front_level;
    let front_kelvin = preview.front_kelvin;
    let rear_on = preview.rear_on;
    let rear_level = preview.rear_level;
    let rear = preview.rear;

    div()
        .id("stage")
        .relative()
        .w_full()
        .h(px(236.))
        .px_4()
        .pt_3()
        .pb_1()
        .child(
            div()
                .id("halo-canvas")
                .relative()
                .size_full()
                .rounded_xl()
                .border_1()
                .border_color(rgb(LINE))
                .overflow_hidden()
                .child(
                    div()
                        .id("halo-paint")
                        .size_full()
                        .with_animation(
                            ("lamp-preview", period),
                            Animation::new(Duration::from_millis(period)).repeat(),
                            move |this, delta| {
                                this.child(
                                    canvas(
                                        |_, _, _| {},
                                        move |bounds, _, window, _| {
                                            preview::paint(
                                                bounds,
                                                LampPreview {
                                                    aspect,
                                                    front_on,
                                                    front_level,
                                                    front_kelvin,
                                                    rear_on,
                                                    rear_level,
                                                    rear,
                                                    phase: if playing { delta } else { 0.0 },
                                                },
                                                window,
                                            );
                                        },
                                    )
                                    .size_full(),
                                )
                            },
                        ),
                )
                .when(connecting, |stage| stage.child(connecting_overlay())),
        )
        .child(
            div()
                .absolute()
                .bottom_3()
                .left_6()
                .right_6()
                .flex()
                .justify_between()
                .child(caption(front_caption))
                .child(caption(rear_caption)),
        )
}

fn caption(text: String) -> impl IntoElement {
    div()
        .px_3()
        .py_1()
        .rounded_md()
        .flex()
        .items_center()
        .bg(gpui::hsla(0.08, 0.12, 0.06, 0.62))
        .text_xs()
        .text_color(rgb(PAPER))
        .child(text)
}

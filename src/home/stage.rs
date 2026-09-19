use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Context, IntoElement, ParentElement, Styled, canvas, div, prelude::*,
    px,
};

use crate::chrome::connecting_overlay;
use crate::lamp::RearLook;

use super::preview::{self, LampPreview};
use super::HomeView;

const STAGE_H: f32 = 176.0;

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
        audio_loud: snap.audio_loud,
        audio_rms: snap.audio_rms,
    };
    let playing = preview.rear_on && matches!(preview.rear, RearLook::Play(_) | RearLook::Audio);
    let period = preview::period_ms(preview.rear, snap.rear.speed.percent());
    let anim_key = match preview.rear {
        RearLook::Solid(_) => 0u32,
        RearLook::Audio => 1,
        RearLook::Play(effect) => 2 + u32::from(effect.byte()),
    };
    let aspect = preview.aspect;
    let front_on = preview.front_on;
    let front_level = preview.front_level;
    let front_kelvin = preview.front_kelvin;
    let rear_on = preview.rear_on;
    let rear_level = preview.rear_level;
    let rear = preview.rear;
    let audio_loud = preview.audio_loud;
    let audio_rms = preview.audio_rms;

    div()
        .id("stage")
        .relative()
        .flex_none()
        .w_full()
        .h(px(STAGE_H))
        .px_6()
        .pt_4()
        .child(
            div()
                .id("halo-canvas")
                .relative()
                .size_full()
                .overflow_hidden()
                .child(
                    div()
                        .id("halo-paint")
                        .size_full()
                        .with_animation(
                            ("lamp-preview", anim_key),
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
                                                    audio_loud,
                                                    audio_rms,
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
}

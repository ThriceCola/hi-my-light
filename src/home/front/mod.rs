mod brightness;
mod cct;
mod kelvin;
mod scenes;

use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*};

use crate::theme::power_switch;

use super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let on = this.service.read(cx).front.on;

    div()
        .id("front-page")
        .flex()
        .flex_col()
        .w_full()
        .child(kelvin::render(this, cx))
        .child(cct::render(this, cx))
        .child(brightness::render(this, cx))
        .child(scenes::render(this, cx))
        .child(
            div()
                .flex_none()
                .flex()
                .w_full()
                .items_center()
                .pt_4()
                .child(power_switch(
                    "power-front",
                    on,
                    cx.listener(|this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.toggle_front();
                            cx.notify();
                        });
                    }),
                )),
        )
}

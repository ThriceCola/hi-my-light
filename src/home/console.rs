use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*, px};

use super::{front, rear, Face, HomeView};

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .id("lamp-console")
        .flex_1()
        .w_full()
        .min_h(px(0.))
        .px_4()
        .pt_3()
        .pb_3()
        .overflow_y_scroll()
        .child(match this.face {
            Face::Front => front::render(this, cx).into_any_element(),
            Face::Rear => rear::render(this, cx).into_any_element(),
        })
}

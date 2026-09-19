use gpui::{Context, IntoElement, ParentElement, Styled, div, prelude::*};

use super::stage::render_stage;
use super::widgets;
use super::{front, rear, tabs, Face, HomeView};

pub fn render(this: &HomeView, connecting: bool, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .id("home-page")
        .flex()
        .flex_col()
        .w_full()
        .child(render_stage(this, connecting, cx))
        .child(
            div()
                .w_full()
                .px_6()
                .pt_3()
                .pb_4()
                .child(
                    widgets::plaque()
                        .child(tabs::render(this, cx))
                        .child(widgets::body().child(match this.face {
                            Face::Front => front::render(this, cx).into_any_element(),
                            Face::Rear => rear::render(this, cx).into_any_element(),
                        })),
                ),
        )
}

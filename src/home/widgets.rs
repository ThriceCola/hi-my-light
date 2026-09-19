use gpui::{Div, InteractiveElement, Styled, div, rgb};

use crate::theme::{LINE, PANEL};

pub fn plaque() -> gpui::Stateful<Div> {
    div()
        .id("plaque")
        .flex()
        .flex_col()
        .w_full()
        .bg(rgb(PANEL))
        .border_1()
        .border_color(rgb(LINE))
}

pub fn body() -> gpui::Stateful<Div> {
    div()
        .id("plaque-body")
        .flex()
        .flex_col()
        .w_full()
        .px_6()
        .pt_4()
        .pb_4()
}

use gpui::{
    Div, InteractiveElement, IntoElement, ParentElement, Styled, Window, div, rgb,
};

use crate::theme::{LINE, PANEL, STONE, power_switch};

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

pub fn card() -> Div {
    div().w_full()
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
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(STONE))
                .child(title),
        )
        .child(power_switch(format!("power-{title}"), on, on_click))
}

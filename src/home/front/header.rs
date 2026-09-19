use gpui::{Context, IntoElement, ParentElement, Styled, div};

use super::super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let (connected, scanning) = {
        let snap = this.service.read(cx);
        (snap.connected(), snap.scanning)
    };
    let byline = if connected {
        "L2 · BLE"
    } else if scanning {
        "L2 · SCAN"
    } else {
        "L2 · IDLE"
    };

    div()
        .flex_none()
        .flex()
        .w_full()
        .items_end()
        .justify_between()
        .pb_3()
        .child(crate::theme::meta(byline))
        .child(crate::theme::status_pill(connected, scanning))
}

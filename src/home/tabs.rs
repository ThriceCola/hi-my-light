use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    Window, div, prelude::*, px, rgb,
};

use crate::theme::{HOVER, INK, LINE, PAPER, STONE};

use super::{Face, HomeView, RearPane};
use crate::lamp::RearLook;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .flex_none()
        .border_b_1()
        .border_color(rgb(LINE))
        .child(tab(
            "face-front",
            "正面",
            this.face == Face::Front,
            false,
            cx.listener(|this, _, _, cx| {
                this.face = Face::Front;
                this.hsv_open = false;
                cx.notify();
            }),
        ))
        .child(tab(
            "face-rear",
            "背面",
            this.face == Face::Rear,
            true,
            cx.listener(|this, _, _, cx| {
                this.face = Face::Rear;
                let rear = this.service.read(cx).rear;
                this.rear_pane = match rear.look {
                    RearLook::Play(effect) => {
                        this.effect_group = effect.group();
                        RearPane::Motion
                    }
                    RearLook::Solid(color) => {
                        this.sv_hue = color.hue();
                        RearPane::Solid
                    }
                };
                cx.notify();
            }),
        ))
}

pub(crate) fn tab(
    id: &'static str,
    label: &'static str,
    active: bool,
    last: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .h(px(40.))
        .flex()
        .justify_center()
        .items_center()
        .cursor_pointer()
        .when(!last, |d| d.border_r_1().border_color(rgb(LINE)))
        .when(active, |d| d.bg(rgb(PAPER)))
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(if active { rgb(INK) } else { rgb(STONE) })
        .hover(|s| {
            if active {
                s
            } else {
                s.bg(rgb(HOVER)).text_color(rgb(PAPER))
            }
        })
        .on_click(on_click)
        .child(label)
}

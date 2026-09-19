use gpui::{
    Context, InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
    div, prelude::*, px, rgb,
};

use crate::lamp::SCENES;
use crate::theme::{HOVER, INK, LINE, PAPER, STONE};

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
    let last = SCENES.len().saturating_sub(1);

    div()
        .id("scenes")
        .flex_none()
        .flex()
        .w_full()
        .border_t_1()
        .border_b_1()
        .border_color(rgb(LINE))
        .children(SCENES.iter().enumerate().map(|(i, scene)| {
            let scene = *scene;
            let active = on
                && (scene.level.percent() - level).abs() < 1.5
                && scene.cct.kelvin().abs_diff(kelvin) < 80;
            div()
                .id(scene.id)
                .flex_1()
                .h(px(48.))
                .cursor_pointer()
                .flex()
                .justify_center()
                .items_center()
                .when(i != last, |d| d.border_r_1().border_color(rgb(LINE)))
                .when(active, |d| d.bg(rgb(PAPER)))
                .hover(|s| {
                    if active {
                        s
                    } else {
                        s.bg(rgb(HOVER))
                    }
                })
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(if active { rgb(INK) } else { rgb(STONE) })
                .child(scene.name)
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_scene(scene);
                        cx.notify();
                    });
                }))
        }))
}

mod console;
mod drag;
mod front;
mod preview;
mod rear;
mod slider;
mod stage;

use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
    prelude::*, px, rgb,
};

use hi_my_light::EffectGroup;

use crate::lamp::RearLook;
use crate::service::LampService;
use crate::theme::{AMBER, LINE, STONE, display_aspect};

use drag::Track;
use stage::render_stage;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Front,
    Rear,
}

pub struct HomeView {
    service: Entity<LampService>,
    aspect: f32,
    face: Face,
    effect_group: EffectGroup,
    dragging: Option<Track>,
    tracks: slider::Tracks,
    sv_hue: f32,
    hsv_open: bool,
    _observe: Subscription,
}

impl HomeView {
    pub fn new(service: Entity<LampService>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&service, |_, _, cx| cx.notify());
        let (effect_group, sv_hue) = match service.read(cx).rear.look {
            RearLook::Play(effect) => (effect.group(), 0.0),
            RearLook::Solid(color) => (EffectGroup::Rainbow, color.hue()),
        };
        Self {
            service,
            aspect: display_aspect(cx),
            face: Face::Front,
            effect_group,
            dragging: None,
            tracks: slider::Tracks::default(),
            sv_hue,
            hsv_open: false,
            _observe: observe,
        }
    }

    fn end_drag(&mut self, cx: &mut Context<Self>) {
        if self.dragging.take().is_none() {
            return;
        }
        self.service.update(cx, |service, cx| {
            service.flush_now();
            cx.notify();
        });
    }
}

impl Render for HomeView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (status, connecting) = {
            let snap = self.service.read(cx);
            (
                snap.status.clone(),
                snap.connecting && !snap.connected(),
            )
        };

        div()
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_h(px(0.))
            .child(render_stage(self, connecting, cx))
            .child(self.render_face_tabs(cx))
            .child(console::render(self, cx))
            .child(render_footer(status))
            .when(self.hsv_open && self.face == Face::Rear, |root| {
                root.child(rear::hsv_popover(self, cx))
            })
    }
}

impl HomeView {
    fn render_face_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_4()
            .pt_3()
            .child(
                div()
                    .flex()
                    .w_full()
                    .p_0p5()
                    .rounded_lg()
                    .bg(rgb(0x181510))
                    .border_1()
                    .border_color(rgb(LINE))
                    .child(face_tab(
                        "face-front",
                        "前置主灯",
                        self.face == Face::Front,
                        cx.listener(|this, _, _, cx| {
                            this.face = Face::Front;
                            this.hsv_open = false;
                            cx.notify();
                        }),
                    ))
                    .child(face_tab(
                        "face-rear",
                        "后置氛围",
                        self.face == Face::Rear,
                        cx.listener(|this, _, _, cx| {
                            this.face = Face::Rear;
                            if let RearLook::Play(effect) = this.service.read(cx).rear.look {
                                this.effect_group = effect.group();
                            }
                            cx.notify();
                        }),
                    )),
            )
    }
}

fn face_tab(
    id: &'static str,
    label: &'static str,
    active: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_1()
        .h(px(32.))
        .px_2()
        .rounded_md()
        .flex()
        .justify_center()
        .items_center()
        .cursor_pointer()
        .when(active, |d| d.bg(rgb(0x2A2216)))
        .text_sm()
        .text_color(if active { rgb(AMBER) } else { rgb(STONE) })
        .hover(|s| s.bg(rgb(0x221E18)))
        .on_click(on_click)
        .child(label)
}

fn render_footer(status: gpui::SharedString) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .items_center()
        .px_4()
        .py_2()
        .border_t_1()
        .border_color(rgb(LINE))
        .child(div().text_xs().text_color(rgb(STONE)).child(status))
}

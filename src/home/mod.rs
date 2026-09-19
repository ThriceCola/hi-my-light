use gpui::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, Subscription, Window, div,
    prelude::*, px,
};

use hi_my_light::EffectGroup;

use crate::lamp::RearLook;
use crate::service::LampService;
use crate::theme::display_aspect;

use drag::Track;

mod drag;
mod front;
mod page;
mod preview;
mod rear;
mod slider;
mod stage;
mod tabs;
mod widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Face {
    Front,
    Rear,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RearPane {
    Solid,
    Motion,
}

pub struct HomeView {
    service: Entity<LampService>,
    aspect: f32,
    face: Face,
    rear_pane: RearPane,
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
        let rear = &service.read(cx).rear;
        let (rear_pane, effect_group, sv_hue) = match rear.look {
            RearLook::Play(effect) => (RearPane::Motion, effect.group(), rear.solid.hue()),
            RearLook::Solid(color) => (RearPane::Solid, rear.effect.group(), color.hue()),
        };
        Self {
            service,
            aspect: display_aspect(cx),
            face: Face::Front,
            rear_pane,
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
        let connecting = {
            let snap = self.service.read(cx);
            snap.connecting && !snap.connected()
        };

        div()
            .id("home-scroll")
            .relative()
            .flex()
            .flex_col()
            .size_full()
            .min_h(px(0.))
            .overflow_y_scroll()
            .child(page::render(self, connecting, cx))
            .when(
                self.hsv_open && self.face == Face::Rear && self.rear_pane == RearPane::Solid,
                |root| {
                root.child(rear::hsv_popover(self, cx))
            })
    }
}

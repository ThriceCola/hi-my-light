use gpui::{
    Bounds, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, canvas, div, fill,
    prelude::*, px, rgb,
};

use crate::theme::{AMBER, LINE, STONE};

use super::drag::{Track, TrackDrag};
use super::HomeView;

#[derive(Clone, Copy)]
pub enum Fill {
    Solid(u32),
    Cct,
    Hue,
}

#[derive(Default)]
pub struct Tracks {
    pub front_level: Bounds<Pixels>,
    pub front_cct: Bounds<Pixels>,
    pub rear_level: Bounds<Pixels>,
    pub rear_speed: Bounds<Pixels>,
    pub hue: Bounds<Pixels>,
    pub sat_val: Bounds<Pixels>,
}

impl Tracks {
    pub fn get(&self, track: Track) -> Bounds<Pixels> {
        match track {
            Track::FrontLevel => self.front_level,
            Track::FrontCct => self.front_cct,
            Track::RearLevel => self.rear_level,
            Track::RearSpeed => self.rear_speed,
            Track::Hue => self.hue,
            Track::SatVal => self.sat_val,
        }
    }

    pub fn set(&mut self, track: Track, bounds: Bounds<Pixels>) {
        match track {
            Track::FrontLevel => self.front_level = bounds,
            Track::FrontCct => self.front_cct = bounds,
            Track::RearLevel => self.rear_level = bounds,
            Track::RearSpeed => self.rear_speed = bounds,
            Track::Hue => self.hue = bounds,
            Track::SatVal => self.sat_val = bounds,
        }
    }
}

pub fn row(
    title: &'static str,
    value: String,
    track: Track,
    t: f32,
    fill: Fill,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_1p5()
        .child(
            div()
                .flex()
                .w_full()
                .items_center()
                .justify_between()
                .child(
                    div()
                        .px_0p5()
                        .text_xs()
                        .text_color(rgb(STONE))
                        .child(title),
                )
                .child(
                    div()
                        .px_0p5()
                        .text_xs()
                        .font_family("monospace")
                        .text_color(rgb(AMBER))
                        .child(value),
                ),
        )
        .child(track_bar(track, t, fill, cx))
}

pub fn track_bar(
    track: Track,
    t: f32,
    fill: Fill,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let t = t.clamp(0.0, 1.0);
    div()
        .id(track_id(track))
        .w_full()
        .h(px(28.))
        .cursor_pointer()
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.dragging = Some(track);
                let bounds = this.tracks.get(track);
                if bounds.size.width > px(0.) {
                    apply_track(this, track, ratio(ev.position.x, bounds), cx);
                }
            }),
        )
        .on_drag(TrackDrag(track), |drag, _, _, cx| cx.new(|_| *drag))
        .on_drag_move(cx.listener(move |this, ev: &DragMoveEvent<TrackDrag>, _, cx| {
            if ev.drag(cx).0 != track {
                return;
            }
            this.dragging = Some(track);
            apply_track(this, track, ratio(ev.event.position.x, ev.bounds), cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| this.end_drag(cx)),
        )
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| this.end_drag(cx)),
        )
        .child({
            let entity = cx.weak_entity();
            canvas(
                move |bounds, _, cx| {
                    let _ = entity.update(cx, |this, _| this.tracks.set(track, bounds));
                },
                move |bounds, _, window, _| paint_bar(bounds, t, fill, window),
            )
            .size_full()
        })
}

fn paint_bar(bounds: Bounds<Pixels>, t: f32, fill_kind: Fill, window: &mut Window) {
    let track_h = px(6.);
    let track = Bounds {
        origin: point_y_center(bounds, track_h),
        size: gpui::size(bounds.size.width, track_h),
    };
    window.paint_quad(fill(track, rgb(LINE)).corner_radii(px(3.)));

    match fill_kind {
        Fill::Solid(color) => {
            let lit = Bounds {
                origin: track.origin,
                size: gpui::size(track.size.width * t, track_h),
            };
            window.paint_quad(fill(lit, rgb(color)).corner_radii(px(3.)));
        }
        Fill::Cct => paint_gradient(track, 0xE39B4A, 0xC9D7EE, window),
        Fill::Hue => paint_hue(track, window),
    }

    let thumb_x = bounds.origin.x + bounds.size.width * t - px(8.);
    let thumb = Bounds {
        origin: gpui::point(thumb_x, bounds.center().y - px(8.)),
        size: gpui::size(px(16.), px(16.)),
    };
    window.paint_quad(fill(thumb, rgb(0xF3EDE4)).corner_radii(px(8.)));
    window.paint_quad(
        fill(
            Bounds {
                origin: gpui::point(thumb.origin.x + px(3.), thumb.origin.y + px(3.)),
                size: gpui::size(px(10.), px(10.)),
            },
            rgb(thumb_core(fill_kind, t)),
        )
        .corner_radii(px(5.)),
    );
}

fn thumb_core(fill: Fill, t: f32) -> u32 {
    match fill {
        Fill::Solid(color) => color,
        Fill::Cct => crate::theme::lerp_rgb(0xE39B4A, 0xC9D7EE, t),
        Fill::Hue => hi_my_light::Rgb::from_hue(t * 360.0).packed(),
    }
}

fn paint_gradient(track: Bounds<Pixels>, a: u32, b: u32, window: &mut Window) {
    let steps = 24;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let piece = Bounds {
            origin: gpui::point(track.origin.x + track.size.width * t, track.origin.y),
            size: gpui::size((track.size.width / steps as f32).max(px(1.)), track.size.height),
        };
        window.paint_quad(fill(piece, rgb(crate::theme::lerp_rgb(a, b, t))).corner_radii(px(2.)));
    }
}

fn paint_hue(track: Bounds<Pixels>, window: &mut Window) {
    let steps = 36;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let piece = Bounds {
            origin: gpui::point(track.origin.x + track.size.width * t, track.origin.y),
            size: gpui::size((track.size.width / steps as f32).max(px(1.)), track.size.height),
        };
        window.paint_quad(fill(piece, rgb(hi_my_light::Rgb::from_hue(t * 360.0).packed())).corner_radii(px(2.)));
    }
}

fn point_y_center(bounds: Bounds<Pixels>, h: Pixels) -> gpui::Point<Pixels> {
    gpui::point(bounds.origin.x, bounds.center().y - h / 2.)
}

fn apply_track(this: &mut HomeView, track: Track, t: f32, cx: &mut Context<HomeView>) {
    if matches!(track, Track::SatVal) {
        return;
    }
    if track == Track::Hue {
        this.sv_hue = t * 360.0;
    }
    let hue = this.sv_hue;
    this.service.update(cx, |service, cx| {
        match track {
            Track::FrontLevel => service.set_front_level(t * 100.0),
            Track::FrontCct => service.set_cct(3000.0 + t * 3000.0),
            Track::RearLevel => service.set_rear_level(t * 100.0),
            Track::RearSpeed => service.set_speed(t * 100.0),
            Track::Hue => {
                let (_, s, v) = match service.rear.look {
                    crate::lamp::RearLook::Solid(color) => color.hsv(),
                    crate::lamp::RearLook::Play(_) => (0.0, 1.0, 1.0),
                };
                service.set_rgb(hi_my_light::Rgb::from_hsv(hue, s, v));
            }
            Track::SatVal => {}
        }
        cx.notify();
    });
}

pub fn apply_sat_val(this: &mut HomeView, x: f32, y: f32, cx: &mut Context<HomeView>) {
    let s = x.clamp(0.0, 1.0);
    let v = (1.0 - y).clamp(0.0, 1.0);
    let hue = this.sv_hue;
    this.service.update(cx, |service, cx| {
        service.set_rgb(hi_my_light::Rgb::from_hsv(hue, s, v));
        cx.notify();
    });
}

pub fn ratio(x: Pixels, bounds: Bounds<Pixels>) -> f32 {
    ((x - bounds.origin.x) / bounds.size.width).clamp(0.0, 1.0)
}

pub fn ratio_y(y: Pixels, bounds: Bounds<Pixels>) -> f32 {
    ((y - bounds.origin.y) / bounds.size.height).clamp(0.0, 1.0)
}

const fn track_id(track: Track) -> &'static str {
    match track {
        Track::FrontLevel => "track-front-level",
        Track::FrontCct => "track-front-cct",
        Track::RearLevel => "track-rear-level",
        Track::RearSpeed => "track-rear-speed",
        Track::Hue => "track-hue",
        Track::SatVal => "track-sat-val",
    }
}

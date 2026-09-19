use gpui::{
    Bounds, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, canvas, div, fill,
    prelude::*, px, rgb,
};

use crate::theme::{PAPER, STONE, TRACK, lerp_rgb};

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
        .gap_2()
        .child(
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
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(rgb(PAPER))
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
    let h = match fill {
        Fill::Cct | Fill::Hue => px(28.),
        Fill::Solid(_) => px(18.),
    };
    div()
        .id(track_id(track))
        .w_full()
        .h(h)
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
    match fill_kind {
        Fill::Cct => paint_tube(bounds, t, window),
        Fill::Hue => paint_hue_tube(bounds, t, window),
        Fill::Solid(color) => paint_thin(bounds, t, color, window),
    }
}

fn paint_tube(bounds: Bounds<Pixels>, t: f32, window: &mut Window) {
    let h = px(10.);
    let track = Bounds {
        origin: gpui::point(bounds.origin.x, bounds.center().y - h / 2.),
        size: gpui::size(bounds.size.width, h),
    };
    window.paint_quad(fill(track, rgb(0x111111)));
    paint_cct_spectrum(track, window);
    paint_needle(bounds, t, window);
}

fn paint_hue_tube(bounds: Bounds<Pixels>, t: f32, window: &mut Window) {
    let h = px(10.);
    let track = Bounds {
        origin: gpui::point(bounds.origin.x, bounds.center().y - h / 2.),
        size: gpui::size(bounds.size.width, h),
    };
    paint_hue(track, window);
    paint_needle(bounds, t, window);
}

fn paint_thin(bounds: Bounds<Pixels>, t: f32, _color: u32, window: &mut Window) {
    let h = px(2.);
    let track = Bounds {
        origin: gpui::point(bounds.origin.x, bounds.center().y - h / 2.),
        size: gpui::size(bounds.size.width, h),
    };
    window.paint_quad(fill(track, rgb(TRACK)));
    let lit = Bounds {
        origin: track.origin,
        size: gpui::size(track.size.width * t, h),
    };
    window.paint_quad(fill(lit, rgb(PAPER)));
}

fn paint_needle(bounds: Bounds<Pixels>, t: f32, window: &mut Window) {
    let x = bounds.origin.x + bounds.size.width * t;
    let shadow = Bounds {
        origin: gpui::point(x - px(2.), bounds.center().y - px(11.)),
        size: gpui::size(px(4.), px(22.)),
    };
    window.paint_quad(fill(shadow, rgb(0x000000)));
    let needle = Bounds {
        origin: gpui::point(x - px(1.), bounds.center().y - px(10.)),
        size: gpui::size(px(2.), px(20.)),
    };
    window.paint_quad(fill(needle, rgb(PAPER)));
}

fn paint_cct_spectrum(track: Bounds<Pixels>, window: &mut Window) {
    const STOPS: [u32; 5] = [0xC47A32, 0xE8C48A, 0xF2EFE8, 0xC8D4E8, 0x7A90B8];
    let steps = 48;
    for i in 0..steps {
        let u = i as f32 / (steps - 1) as f32;
        let scaled = u * (STOPS.len() - 1) as f32;
        let i0 = scaled.floor() as usize;
        let i1 = (i0 + 1).min(STOPS.len() - 1);
        let f = scaled - i0 as f32;
        let color = lerp_rgb(STOPS[i0], STOPS[i1], f);
        let piece = Bounds {
            origin: gpui::point(
                track.origin.x + track.size.width * (i as f32 / steps as f32),
                track.origin.y,
            ),
            size: gpui::size(
                (track.size.width / steps as f32).max(px(1.)),
                track.size.height,
            ),
        };
        window.paint_quad(fill(piece, rgb(color)));
    }
}

fn paint_hue(track: Bounds<Pixels>, window: &mut Window) {
    let steps = 36;
    for i in 0..steps {
        let t = i as f32 / steps as f32;
        let piece = Bounds {
            origin: gpui::point(track.origin.x + track.size.width * t, track.origin.y),
            size: gpui::size(
                (track.size.width / steps as f32).max(px(1.)),
                track.size.height,
            ),
        };
        window.paint_quad(fill(
            piece,
            rgb(hi_my_light::Rgb::from_hue(t * 360.0).packed()),
        ));
    }
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

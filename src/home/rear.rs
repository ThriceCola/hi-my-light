use gpui::{
    Bounds, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, canvas, div, fill,
    prelude::*, px, rgb,
};

use hi_my_light::{Effect, EffectGroup, Rgb};

use crate::lamp::RearLook;
use crate::theme::{AMBER, AMBER_SOFT, INK, LINE, PANEL, PAPER, STONE, tone};

use super::drag::{Track, TrackDrag};
use super::front::{card, power_row};
use super::slider::{self, Fill};
use super::HomeView;

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let (on, level, speed, look) = {
        let snap = this.service.read(cx);
        (
            snap.rear.on,
            snap.rear.level.percent(),
            snap.rear.speed.percent(),
            snap.rear.look,
        )
    };
    let playing = matches!(look, RearLook::Play(_));
    let (sat, val, hex) = match look {
        RearLook::Solid(color) => {
            let (_, s, v) = color.hsv();
            (s, v, format!("#{:06x}", color.packed()))
        }
        RearLook::Play(_) => (1.0, 1.0, "灯效".into()),
    };

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .child(card().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(power_row("后置氛围灯", on, cx.listener(|this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.toggle_rear();
                        cx.notify();
                    });
                })))
                .child(slider::row(
                    "亮度",
                    format!("{:.0}%", level),
                    Track::RearLevel,
                    level / 100.0,
                    Fill::Solid(match look {
                        RearLook::Solid(c) => c.packed(),
                        RearLook::Play(_) => AMBER,
                    }),
                    cx,
                )),
        ))
        .child(card().child(
            div()
                .flex()
                .flex_col()
                .gap_3()
                .child(presets(look, cx))
                .child(color_trigger(look, &hex, sat, val, this.sv_hue, cx)),
        ))
        .when(playing, |col| {
            col.child(card().child(slider::row(
                "速度",
                format!("{:.0}%", speed),
                Track::RearSpeed,
                speed / 100.0,
                Fill::Solid(AMBER_SOFT),
                cx,
            )))
        })
        .child(effects(this, look, cx))
}

fn color_trigger(
    look: RearLook,
    hex: &str,
    sat: f32,
    val: f32,
    hue: f32,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let swatch = match look {
        RearLook::Solid(color) => color.packed(),
        RearLook::Play(_) => AMBER,
    };
    div()
        .id("hsv-open")
        .w_full()
        .h(px(44.))
        .px_2()
        .rounded_lg()
        .border_1()
        .border_color(rgb(LINE))
        .bg(rgb(0x181510))
        .cursor_pointer()
        .flex()
        .items_center()
        .gap_3()
        .hover(|s| s.bg(rgb(0x221E18)))
        .on_click(cx.listener(|this, _, _, cx| {
            this.hsv_open = true;
            cx.notify();
        }))
        .child(
            div()
                .size(px(28.))
                .rounded_md()
                .border_1()
                .border_color(rgb(0xF3EDE4))
                .bg(rgb(swatch)),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w(px(0.))
                .child(
                    div()
                        .text_sm()
                        .text_color(rgb(PAPER))
                        .child(hex.to_string()),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(STONE))
                        .child(format!("H {:.0}°  S {:.0}%  V {:.0}%", hue, sat * 100.0, val * 100.0)),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(AMBER))
                .child("色板"),
        )
}

pub fn hsv_popover(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let look = this.service.read(cx).rear.look;
    let hue_t = (this.sv_hue / 360.0).clamp(0.0, 1.0);
    let (sat, val, hex) = match look {
        RearLook::Solid(color) => {
            let (_, s, v) = color.hsv();
            (s, v, format!("#{:06x}", color.packed()))
        }
        RearLook::Play(_) => (1.0, 1.0, "灯效".into()),
    };

    div()
        .id("hsv-veil")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .bg(tone(INK, 0.62))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.hsv_open = false;
                this.end_drag(cx);
                cx.notify();
            }),
        )
        .child(
            div()
                .id("hsv-window")
                .w(px(360.))
                .px_4()
                .py_4()
                .rounded_xl()
                .border_1()
                .border_color(rgb(LINE))
                .bg(rgb(PANEL))
                .shadow(vec![gpui::BoxShadow {
                    color: gpui::hsla(0., 0., 0., 0.5),
                    offset: gpui::point(px(0.), px(8.)),
                    blur_radius: px(24.),
                    spread_radius: px(0.),
                    inset: false,
                }])
                .flex()
                .flex_col()
                .gap_3()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                .child(
                    div()
                        .flex()
                        .w_full()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_sm()
                                .text_color(rgb(AMBER))
                                .child("色板"),
                        )
                        .child(
                            div()
                                .id("hsv-done")
                                .px_2()
                                .py_1()
                                .rounded_md()
                                .cursor_pointer()
                                .text_xs()
                                .text_color(rgb(STONE))
                                .hover(|s| s.bg(rgb(0x221E18)).text_color(rgb(PAPER)))
                                .child("完成")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.hsv_open = false;
                                    this.end_drag(cx);
                                    cx.notify();
                                })),
                        ),
                )
                .child(hsv_board(this.sv_hue, sat, val, cx))
                .child(slider::row(
                    "色相",
                    hex,
                    Track::Hue,
                    hue_t,
                    Fill::Hue,
                    cx,
                ))
                .child(
                    div()
                        .flex()
                        .w_full()
                        .justify_between()
                        .px_0p5()
                        .text_xs()
                        .font_family("monospace")
                        .text_color(rgb(STONE))
                        .child(format!("H {:.0}°", this.sv_hue))
                        .child(format!("S {:.0}%", sat * 100.0))
                        .child(format!("V {:.0}%", val * 100.0)),
                ),
        )
}

fn hsv_board(hue: f32, sat: f32, val: f32, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .id("hsv-board")
        .w_full()
        .h(px(140.))
        .rounded_lg()
        .overflow_hidden()
        .cursor_pointer()
        .border_1()
        .border_color(rgb(LINE))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, ev: &MouseDownEvent, _, cx| {
                this.dragging = Some(Track::SatVal);
                let bounds = this.tracks.get(Track::SatVal);
                if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    slider::apply_sat_val(
                        this,
                        slider::ratio(ev.position.x, bounds),
                        slider::ratio_y(ev.position.y, bounds),
                        cx,
                    );
                }
            }),
        )
        .on_drag(TrackDrag(Track::SatVal), |drag, _, _, cx| cx.new(|_| *drag))
        .on_drag_move(cx.listener(move |this, ev: &DragMoveEvent<TrackDrag>, _, cx| {
            if ev.drag(cx).0 != Track::SatVal {
                return;
            }
            this.dragging = Some(Track::SatVal);
            slider::apply_sat_val(
                this,
                slider::ratio(ev.event.position.x, ev.bounds),
                slider::ratio_y(ev.event.position.y, ev.bounds),
                cx,
            );
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
                    let _ = entity.update(cx, |this, _| this.tracks.set(Track::SatVal, bounds));
                },
                move |bounds, _, window, _| paint_sv(bounds, hue, sat, val, window),
            )
            .size_full()
        })
}

fn paint_sv(bounds: Bounds<Pixels>, hue: f32, sat: f32, val: f32, window: &mut Window) {
    let cols = 24;
    let rows = 16;
    let cw = bounds.size.width / cols as f32;
    let ch = bounds.size.height / rows as f32;
    for row in 0..rows {
        let v = 1.0 - (row as f32 + 0.5) / rows as f32;
        for col in 0..cols {
            let s = (col as f32 + 0.5) / cols as f32;
            let piece = Bounds {
                origin: gpui::point(
                    bounds.origin.x + cw * col as f32,
                    bounds.origin.y + ch * row as f32,
                ),
                size: gpui::size(cw + px(0.6), ch + px(0.6)),
            };
            window.paint_quad(fill(piece, rgb(Rgb::from_hsv(hue, s, v).packed())));
        }
    }

    let x = bounds.origin.x + bounds.size.width * sat.clamp(0.0, 1.0);
    let y = bounds.origin.y + bounds.size.height * (1.0 - val.clamp(0.0, 1.0));
    let outer = Bounds {
        origin: gpui::point(x - px(8.), y - px(8.)),
        size: gpui::size(px(16.), px(16.)),
    };
    window.paint_quad(fill(outer, rgb(0xF3EDE4)).corner_radii(px(8.)));
    let inner = Bounds {
        origin: gpui::point(outer.origin.x + px(3.), outer.origin.y + px(3.)),
        size: gpui::size(px(10.), px(10.)),
    };
    window.paint_quad(
        fill(inner, rgb(Rgb::from_hsv(hue, sat, val).packed())).corner_radii(px(5.)),
    );
}

fn presets(look: RearLook, cx: &mut Context<HomeView>) -> impl IntoElement {
    let current = match look {
        RearLook::Solid(rgb) => Some(rgb),
        RearLook::Play(_) => None,
    };
    div()
        .flex()
        .w_full()
        .items_center()
        .justify_between()
        .children(Rgb::PRESETS.into_iter().map(|(name, color)| {
            let active = current == Some(color);
            div()
                .id(name)
                .size(px(28.))
                .rounded_full()
                .cursor_pointer()
                .border_2()
                .border_color(if active { rgb(0xF3EDE4) } else { rgb(LINE) })
                .bg(rgb(color.packed()))
                .shadow(if active {
                    vec![gpui::BoxShadow {
                        color: gpui::hsla(0.08, 0.4, 0.5, 0.45),
                        offset: gpui::point(px(0.), px(0.)),
                        blur_radius: px(8.),
                        spread_radius: px(0.),
                        inset: false,
                    }]
                } else {
                    Vec::new()
                })
                .on_click(cx.listener(move |this, _, _, cx| {
                    let (h, s, _) = color.hsv();
                    if s > 0.04 {
                        this.sv_hue = h;
                    }
                    this.service.update(cx, |service, cx| {
                        service.set_rgb(color);
                        service.flush_now();
                        cx.notify();
                    });
                }))
        }))
}

fn effects(this: &HomeView, look: RearLook, cx: &mut Context<HomeView>) -> impl IntoElement {
    let current = match look {
        RearLook::Play(effect) => Some(effect),
        RearLook::Solid(_) => None,
    };
    let group = this.effect_group;
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .flex()
                .w_full()
                .p_0p5()
                .rounded_lg()
                .bg(rgb(0x181510))
                .border_1()
                .border_color(rgb(LINE))
                .children(EffectGroup::ALL.into_iter().map(|g| {
                    let active = group == g;
                    div()
                        .id(g.name())
                        .flex_1()
                        .h(px(30.))
                        .px_1()
                        .rounded_md()
                        .cursor_pointer()
                        .flex()
                        .justify_center()
                        .items_center()
                        .when(active, |d| d.bg(rgb(0x2A2216)))
                        .text_xs()
                        .text_color(if active { rgb(AMBER) } else { rgb(STONE) })
                        .child(g.name())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.effect_group = g;
                            cx.notify();
                        }))
                })),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .children(
                    Effect::ALL
                        .into_iter()
                        .filter(|e| e.group() == group)
                        .map(|effect| {
                            let active = current == Some(effect);
                            div()
                                .id(gpui::SharedString::from(format!("fx-{:02x}", effect.byte())))
                                .w_full()
                                .h(px(36.))
                                .px_3()
                                .rounded_md()
                                .cursor_pointer()
                                .flex()
                                .items_center()
                                .border_1()
                                .border_color(if active { rgb(AMBER) } else { rgb(LINE) })
                                .bg(if active { rgb(0x241C12) } else { rgb(PANEL) })
                                .hover(|s| s.bg(rgb(0x221E18)))
                                .text_sm()
                                .text_color(if active { rgb(AMBER_SOFT) } else { rgb(0xD8D0C4) })
                                .child(effect.name())
                                .on_click(cx.listener(move |this, _, _, cx| {
                                    this.effect_group = effect.group();
                                    this.service.update(cx, |service, cx| {
                                        service.set_effect(effect);
                                        cx.notify();
                                    });
                                }))
                        }),
                ),
        )
}

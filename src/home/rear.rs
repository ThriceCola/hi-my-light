use gpui::prelude::*;
use gpui::{
    Bounds, Context, DragMoveEvent, InteractiveElement, IntoElement, MouseButton, MouseDownEvent,
    ParentElement, Pixels, StatefulInteractiveElement, Styled, Window, canvas, div, fill, px,
    relative, rgb,
};

use hi_my_light::{Effect, EffectGroup, Rgb};

use crate::lamp::RearLook;
use crate::theme::{
    HOVER, INK, LINE, MIST, PANEL, PAPER, STONE, TRACK, power_switch, step_btn, tone,
};

use super::drag::{Track, TrackDrag};
use super::slider::{self, Fill};
use super::{HomeView, RearPane, tabs};

pub fn render(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let (on, level, speed, look, solid, effect, rms, loud, audio_caption, sensitivity, dynamic) = {
        let snap = this.service.read(cx);
        (
            snap.rear.on,
            snap.rear.level.percent(),
            snap.rear.speed.percent(),
            snap.rear.look,
            snap.rear.solid,
            snap.rear.effect,
            snap.audio_rms,
            snap.audio_loud,
            snap.audio_caption(),
            snap.audio_sensitivity.percent(),
            snap.audio_dynamic,
        )
    };
    let motion = this.rear_pane == RearPane::Motion;
    let audio = this.rear_pane == RearPane::Audio;
    let glow = if on {
        if motion {
            PAPER
        } else if audio {
            if loud {
                PAPER
            } else if audio_caption == "暂缓" {
                MIST
            } else {
                LINE
            }
        } else {
            solid.packed()
        }
    } else {
        LINE
    };
    let right = if motion {
        effect.name().into()
    } else if audio {
        audio_caption.into()
    } else {
        format!("{:06X}", solid.packed())
    };

    div()
        .id("rear-page")
        .flex()
        .flex_col()
        .w_full()
        .min_h(px(0.))
        .child(pane_tabs(this, cx))
        .child(readout(
            on,
            level,
            speed,
            sensitivity,
            rms,
            glow,
            right,
            motion,
            audio,
            cx,
        ))
        .when(!audio, |col| {
            col.child(slider_step(
                Track::RearLevel,
                level / 100.0,
                Fill::Solid(glow),
                "rear-bri-dec",
                "rear-bri-inc",
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_rear_level(level - 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_rear_level(level + 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx,
            ))
        })
        .when(motion, |col| {
            col.child(slider_step(
                Track::RearSpeed,
                speed / 100.0,
                Fill::Solid(PAPER),
                "rear-spd-dec",
                "rear-spd-inc",
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_speed(speed - 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_speed(speed + 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx,
            ))
        })
        .when(!motion && !audio, |col| {
            col.child(presets(look, cx)).child(custom_color(solid, cx))
        })
        .when(motion, |col| {
            col.child(groups(this, cx)).child(effects(this, look, cx))
        })
        .when(audio, |col| {
            col.child(slider_step(
                Track::AudioSense,
                sensitivity / 100.0,
                Fill::Solid(if on { PAPER } else { LINE }),
                "rear-sense-dec",
                "rear-sense-inc",
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_audio_sensitivity(sensitivity - 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx.listener(move |this, _, _, cx| {
                    this.service.update(cx, |service, cx| {
                        service.set_audio_sensitivity(sensitivity + 5.0);
                        service.flush_now();
                        cx.notify();
                    });
                }),
                cx,
            ))
            .child(dynamic_sense_btn(dynamic, cx))
        })
        .child(
            div()
                .flex_none()
                .flex()
                .w_full()
                .items_center()
                .pt_4()
                .child(power_switch(
                    "power-rear",
                    on,
                    cx.listener(|this, _, _, cx| {
                        this.service.update(cx, |service, cx| {
                            service.toggle_rear();
                            cx.notify();
                        });
                    }),
                )),
        )
}

fn dynamic_sense_btn(on: bool, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .id("rear-dynamic-sense")
        .flex_none()
        .h(px(40.))
        .w_full()
        .cursor_pointer()
        .flex()
        .justify_center()
        .items_center()
        .border_t_1()
        .border_color(rgb(LINE))
        .when(on, |d| d.bg(rgb(PAPER)))
        .hover(|s| if on { s } else { s.bg(rgb(HOVER)) })
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(if on { rgb(INK) } else { rgb(STONE) })
        .child("动态灵敏")
        .on_click(cx.listener(|this, _, _, cx| {
            this.service.update(cx, |service, cx| {
                service.toggle_audio_dynamic();
                cx.notify();
            });
        }))
}

fn pane_tabs(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    div()
        .flex()
        .w_full()
        .flex_none()
        .border_b_1()
        .border_color(rgb(LINE))
        .child(tabs::tab(
            "rear-pane-solid",
            "纯色",
            this.rear_pane == RearPane::Solid,
            false,
            cx.listener(|this, _, _, cx| {
                this.rear_pane = RearPane::Solid;
                this.hsv_open = false;
                let solid = this.service.read(cx).rear.solid;
                let (h, s, _) = solid.hsv();
                if s > 0.04 {
                    this.sv_hue = h;
                }
                this.service.update(cx, |service, cx| {
                    service.show_rear_solid();
                    cx.notify();
                });
            }),
        ))
        .child(tabs::tab(
            "rear-pane-motion",
            "动效",
            this.rear_pane == RearPane::Motion,
            false,
            cx.listener(|this, _, _, cx| {
                this.rear_pane = RearPane::Motion;
                this.hsv_open = false;
                let effect = this.service.read(cx).rear.effect;
                this.effect_group = effect.group();
                this.service.update(cx, |service, cx| {
                    service.show_rear_effect();
                    cx.notify();
                });
            }),
        ))
        .child(tabs::tab(
            "rear-pane-audio",
            "拾音",
            this.rear_pane == RearPane::Audio,
            true,
            cx.listener(|this, _, _, cx| {
                this.rear_pane = RearPane::Audio;
                this.hsv_open = false;
                this.service.update(cx, |service, cx| {
                    service.show_rear_audio();
                    cx.notify();
                });
            }),
        ))
}

fn readout(
    on: bool,
    level: f32,
    speed: f32,
    sensitivity: f32,
    rms: f32,
    glow: u32,
    right: String,
    motion: bool,
    audio: bool,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    let row = div()
        .flex_none()
        .flex()
        .w_full()
        .items_end()
        .justify_between()
        .py_4()
        .opacity(if on { 1.0 } else { 0.32 });
    if audio {
        div()
            .flex_none()
            .flex()
            .flex_col()
            .w_full()
            .opacity(if on { 1.0 } else { 0.32 })
            .gap_3()
            .child(
                div()
                    .flex()
                    .w_full()
                    .items_end()
                    .justify_between()
                    .pt_4()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_start()
                            .child(
                                div()
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(rgb(STONE))
                                    .child("音频"),
                            )
                            .child(
                                div()
                                    .mt_1()
                                    .text_size(px(72.))
                                    .font_weight(gpui::FontWeight::BOLD)
                                    .line_height(px(64.))
                                    .text_color(rgb(glow))
                                    .child(right),
                            ),
                    )
                    .child(metric(
                        "灵敏",
                        format!("{:.0}", sensitivity),
                        PAPER,
                        false,
                        true,
                    )),
            )
            .child(audio_meter(rms, glow))
    } else {
        row.child(metric("亮度", format!("{:.0}", level), glow, true, false))
            .child(
                div()
                    .id("rear-color")
                    .flex()
                    .items_end()
                    .gap_3()
                    .when(!motion, |d| {
                        d.cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                            this.hsv_open = true;
                            cx.notify();
                        }))
                    })
                    .child(metric(
                        if motion { "灯效" } else { "颜色" },
                        right,
                        glow,
                        false,
                        true,
                    ))
                    .when(motion, |d| {
                        d.child(div().w(px(64.)).flex_none().child(metric(
                            "速度",
                            format!("{:.0}", speed),
                            glow,
                            false,
                            true,
                        )))
                    }),
            )
    }
}

fn metric(
    label: &'static str,
    value: String,
    color: u32,
    large: bool,
    end: bool,
) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .when(end, |d| d.items_end())
        .when(!end, |d| d.items_start())
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(STONE))
                .child(label),
        )
        .child(
            div()
                .mt_1()
                .text_size(if large { px(96.) } else { px(36.) })
                .font_weight(gpui::FontWeight::BOLD)
                .line_height(if large { px(82.) } else { px(42.) })
                .text_color(rgb(color))
                .child(value),
        )
}

fn slider_step(
    track: Track,
    t: f32,
    fill: Fill,
    dec_id: &'static str,
    inc_id: &'static str,
    on_dec: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    on_inc: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
    cx: &mut Context<HomeView>,
) -> impl IntoElement {
    div()
        .flex_none()
        .flex()
        .w_full()
        .items_center()
        .gap_3()
        .pt_2()
        .pb_4()
        .child(
            div()
                .flex_1()
                .min_w(px(0.))
                .child(slider::track_bar(track, t, fill, cx)),
        )
        .child(
            div()
                .flex()
                .gap_1p5()
                .child(step_btn(dec_id, "−", on_dec))
                .child(step_btn(inc_id, "+", on_inc)),
        )
}

pub fn hsv_popover(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let color = this.service.read(cx).rear.solid;
    let hue_t = (this.sv_hue / 360.0).clamp(0.0, 1.0);
    let (_, sat, val) = color.hsv();
    let hex = format!("#{:06x}", color.packed());

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
                .px_5()
                .py_5()
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
                    div().flex().w_full().items_center().justify_end().child(
                        div()
                            .id("hsv-done")
                            .px_2()
                            .py_1()
                            .cursor_pointer()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .border_1()
                            .border_color(rgb(LINE))
                            .text_color(rgb(STONE))
                            .hover(|s| s.bg(rgb(HOVER)).text_color(rgb(PAPER)))
                            .child("关闭")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.hsv_open = false;
                                this.end_drag(cx);
                                cx.notify();
                            })),
                    ),
                )
                .child(hsv_board(this.sv_hue, sat, val, cx))
                .child(slider::row("色相", hex, Track::Hue, hue_t, Fill::Hue, cx))
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
        .on_drag_move(
            cx.listener(move |this, ev: &DragMoveEvent<TrackDrag>, _, cx| {
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
            }),
        )
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
    window.paint_quad(fill(outer, rgb(PAPER)));
    let inner = Bounds {
        origin: gpui::point(outer.origin.x + px(3.), outer.origin.y + px(3.)),
        size: gpui::size(px(10.), px(10.)),
    };
    window.paint_quad(fill(inner, rgb(Rgb::from_hsv(hue, sat, val).packed())));
}

fn audio_meter(rms: f32, fill_color: u32) -> impl IntoElement {
    let fill = ((rms / 0.25).clamp(0.0, 1.0) * 100.0).round() as f32 / 100.0;
    div()
        .flex_none()
        .w_full()
        .pb_6()
        .child(
            div()
                .w_full()
                .h(px(8.))
                .bg(rgb(TRACK))
                .child(div().h_full().w(relative(fill)).bg(rgb(fill_color))),
        )
}

fn presets(look: RearLook, cx: &mut Context<HomeView>) -> impl IntoElement {
    let current = match look {
        RearLook::Solid(rgb) => Some(rgb),
        RearLook::Play(_) | RearLook::Audio => None,
    };
    let last = Rgb::PRESETS.len().saturating_sub(1);
    div()
        .id("rear-presets")
        .flex_none()
        .flex()
        .w_full()
        .border_t_1()
        .border_color(rgb(LINE))
        .children(
            Rgb::PRESETS
                .into_iter()
                .enumerate()
                .map(|(i, (name, color))| {
                    let active = current == Some(color);
                    div()
                        .id(name)
                        .flex_1()
                        .h(px(48.))
                        .cursor_pointer()
                        .flex()
                        .justify_center()
                        .items_center()
                        .when(i != last, |d| d.border_r_1().border_color(rgb(LINE)))
                        .bg(rgb(color.packed()))
                        .when(active, |d| d.border_1().border_color(rgb(PAPER)))
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
                }),
        )
}

fn custom_color(solid: Rgb, cx: &mut Context<HomeView>) -> impl IntoElement {
    let swatch = solid.packed();
    let custom = Rgb::PRESETS.iter().all(|(_, c)| *c != solid);
    div()
        .id("rear-custom")
        .flex_none()
        .flex()
        .w_full()
        .h(px(48.))
        .items_center()
        .justify_center()
        .gap_3()
        .border_t_1()
        .border_color(rgb(LINE))
        .cursor_pointer()
        .when(custom, |d| d.bg(rgb(PAPER)))
        .hover(|s| if custom { s } else { s.bg(rgb(HOVER)) })
        .on_click(cx.listener(|this, _, _, cx| {
            this.hsv_open = true;
            cx.notify();
        }))
        .child(
            div()
                .size(px(18.))
                .border_1()
                .border_color(rgb(if custom { INK } else { LINE }))
                .bg(rgb(swatch)),
        )
        .child(
            div()
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(rgb(if custom { INK } else { STONE }))
                .child("自定义"),
        )
}

fn groups(this: &HomeView, cx: &mut Context<HomeView>) -> impl IntoElement {
    let group = this.effect_group;
    let last = EffectGroup::ALL.len().saturating_sub(1);
    div()
        .id("rear-groups")
        .flex_none()
        .flex()
        .w_full()
        .border_t_1()
        .border_b_1()
        .border_color(rgb(LINE))
        .children(EffectGroup::ALL.into_iter().enumerate().map(|(i, g)| {
            let active = group == g;
            div()
                .id(g.name())
                .flex_1()
                .h(px(48.))
                .cursor_pointer()
                .flex()
                .justify_center()
                .items_center()
                .when(i != last, |d| d.border_r_1().border_color(rgb(LINE)))
                .when(active, |d| d.bg(rgb(PAPER)))
                .hover(|s| if active { s } else { s.bg(rgb(HOVER)) })
                .text_xs()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(if active { rgb(INK) } else { rgb(STONE) })
                .child(g.name())
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.effect_group = g;
                    cx.notify();
                }))
        }))
}

fn effects(this: &HomeView, look: RearLook, cx: &mut Context<HomeView>) -> impl IntoElement {
    let current = match look {
        RearLook::Play(effect) => Some(effect),
        RearLook::Solid(_) | RearLook::Audio => None,
    };
    let group = this.effect_group;
    div()
        .id("fx-list")
        .flex_1()
        .min_h(px(0.))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .children(
            Effect::ALL
                .into_iter()
                .filter(|e| e.group() == group)
                .map(|effect| {
                    let active = current == Some(effect);
                    div()
                        .id(gpui::SharedString::from(format!(
                            "fx-{:02x}",
                            effect.byte()
                        )))
                        .w_full()
                        .h(px(48.))
                        .cursor_pointer()
                        .flex()
                        .justify_center()
                        .items_center()
                        .border_b_1()
                        .border_color(rgb(LINE))
                        .when(active, |d| d.bg(rgb(PAPER)))
                        .hover(|s| if active { s } else { s.bg(rgb(HOVER)) })
                        .text_xs()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(if active { rgb(INK) } else { rgb(STONE) })
                        .child(effect.name())
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.effect_group = effect.group();
                            this.service.update(cx, |service, cx| {
                                service.set_effect(effect);
                                cx.notify();
                            });
                        }))
                }),
        )
}

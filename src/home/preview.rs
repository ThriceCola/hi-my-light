use gpui::{
    Bounds, Pixels, Window, fill, point, px, rgb, size,
};

use hi_my_light::{Effect, Rgb};

use crate::lamp::RearLook;
use crate::theme::{INK, glow_from_kelvin, lerp_rgb, tone};

const BAR_H: f32 = 7.0;

const DUSTY_RED: Rgb = Rgb::new(0xFF, 0x8A, 0x78);
const DUSTY_GREEN: Rgb = Rgb::new(0x6A, 0xE0, 0xA4);
const DUSTY_BLUE: Rgb = Rgb::new(0x7F, 0xB4, 0xFF);
const DUSTY_YELLOW: Rgb = Rgb::new(0xFF, 0xD6, 0x6A);
const DUSTY_CYAN: Rgb = Rgb::new(0x6A, 0xEE, 0xE8);
const DUSTY_PURPLE: Rgb = Rgb::new(0xD4, 0x9A, 0xE8);
const DUSTY_WHITE: Rgb = Rgb::new(0xFF, 0xF6, 0xEE);

#[derive(Clone, Copy)]
pub struct LampPreview {
    pub aspect: f32,
    pub front_on: bool,
    pub front_level: f32,
    pub front_kelvin: u16,
    pub rear_on: bool,
    pub rear_level: f32,
    pub rear: RearLook,
    pub phase: f32,
}

struct Layout {
    bar: Bounds<Pixels>,
    bezel: Bounds<Pixels>,
    screen: Bounds<Pixels>,
}

pub fn paint(bounds: Bounds<Pixels>, preview: LampPreview, window: &mut Window) {
    window.paint_quad(fill(bounds, rgb(INK)));

    let aspect = preview.aspect.clamp(1.2, 2.5);
    let front = if preview.front_on {
        (preview.front_level / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let rear = if preview.rear_on {
        (preview.rear_level / 100.0).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let kelvin = glow_from_kelvin(preview.front_kelvin);
    let layout = layout(bounds, aspect);

    paint_wall_wash(bounds, &layout, preview, rear, window);
    paint_monitor(&layout, kelvin, front, window);
    paint_front_bar(layout.bar, kelvin, front, window);
}

fn layout(canvas: Bounds<Pixels>, aspect: f32) -> Layout {
    let pad = px(16.);
    let bar_h = px(BAR_H);
    let bezel = px(6.);
    let gap = px(8.);
    let wash = px(26.);

    let max_w = (canvas.size.width - pad * 2.).max(px(96.));
    let top = pad + wash + bar_h + gap;
    let visible = (canvas.size.height - top - px(12.)).max(px(48.));
    let mut screen_h = visible / 0.5;
    let mut screen_w = screen_h * aspect;
    if screen_w > max_w {
        screen_w = max_w;
        screen_h = screen_w * (1.0 / aspect);
    }

    let screen_x = canvas.center().x - screen_w / 2.;
    let screen_y = canvas.origin.y + top;
    let screen = Bounds {
        origin: point(screen_x, screen_y),
        size: size(screen_w, screen_h),
    };
    let frame = Bounds {
        origin: point(screen.origin.x - bezel, screen.origin.y - bezel),
        size: size(
            screen.size.width + bezel * 2.,
            screen.size.height + bezel * 2.,
        ),
    };
    let bar_w = screen_w * 1.02;
    let bar = Bounds {
        origin: point(
            canvas.center().x - bar_w / 2.,
            screen_y - gap - bar_h,
        ),
        size: size(bar_w, bar_h),
    };
    Layout {
        bar,
        bezel: frame,
        screen,
    }
}

/// 后置光打在灯条后面的墙上：沿灯条中心左右对称向上洗开。
fn paint_wall_wash(
    canvas: Bounds<Pixels>,
    layout: &Layout,
    preview: LampPreview,
    rear: f32,
    window: &mut Window,
) {
    if rear < 0.01 {
        return;
    }

    let sample = |t: f32| rear_rgb(preview.rear, t, preview.phase).scale(rear);
    let mid = layout.bar.center().x;
    let bar_w = layout.bar.size.width;
    let bottom = layout.bar.origin.y + layout.bar.size.height * 0.45;
    let top = canvas.origin.y + px(6.);
    let rise = (bottom - top).max(px(8.));

    let cols = 15;
    let layers = 8;
    for layer in 0..layers {
        let lt = layer as f32 / (layers as f32 - 1.0).max(1.0);
        let y = bottom - rise * (0.12 + lt * 0.88);
        let h = (rise / layers as f32) * 1.7;
        let flare = px(lt * 16.0);
        let span = bar_w + flare * 2.;
        let alpha = (1.0 - lt).powf(1.2) * (0.08 + rear * 0.30);
        let col_w = span / cols as f32;

        for i in 0..cols {
            let u = (i as f32 + 0.5) / cols as f32;
            let cx = mid - span / 2. + span * u;
            let lamp_t = ((cx - (mid - bar_w / 2.)) / bar_w).clamp(0.0, 1.0);
            let piece = Bounds {
                origin: point(cx - col_w * 0.62, y),
                size: size(col_w * 1.24, h),
            };
            window.paint_quad(
                fill(piece, tone(sample(lamp_t).packed(), alpha)).corner_radii(h * 0.55),
            );
        }
    }
}

fn paint_monitor(layout: &Layout, kelvin: u32, front: f32, window: &mut Window) {
    window.paint_quad(fill(layout.bezel, rgb(0x1A1814)).corner_radii(px(10.)));
    window.paint_quad(fill(layout.screen, rgb(0x0E0D0B)).corner_radii(px(4.)));

    if front > 0.01 {
        let layers = 7;
        for i in 0..layers {
            let t = i as f32 / layers as f32;
            let h = layout.screen.size.height * (0.16 + front * 0.10);
            let band = Bounds {
                origin: point(
                    layout.screen.origin.x,
                    layout.screen.origin.y + h * t * 0.85,
                ),
                size: size(layout.screen.size.width, h),
            };
            window.paint_quad(fill(
                band,
                tone(kelvin, front * (0.20 - t * 0.16).max(0.0)),
            ));
        }
    }
}

fn paint_front_bar(bar: Bounds<Pixels>, kelvin: u32, front: f32, window: &mut Window) {
    let r = bar.size.height / 2.;
    if front > 0.01 {
        let bloom = 3.0 + front * 7.0;
        let halo = Bounds {
            origin: point(bar.origin.x - px(bloom), bar.origin.y - px(bloom * 0.45)),
            size: size(
                bar.size.width + px(bloom * 2.0),
                bar.size.height + px(bloom * 0.9),
            ),
        };
        window.paint_quad(fill(halo, tone(kelvin, front * 0.22)).corner_radii(halo.size.height / 2.));
    }
    window.paint_quad(fill(bar, rgb(0x2A261F)).corner_radii(r));
    let lit = if front > 0.01 {
        lerp_rgb(0x3A3226, kelvin, 0.35 + front * 0.65)
    } else {
        0x2A261F
    };
    window.paint_quad(fill(bar, rgb(lit)).corner_radii(r));
}

fn rear_rgb(look: RearLook, t: f32, phase: f32) -> Rgb {
    match look {
        RearLook::Solid(rgb) => rgb,
        RearLook::Play(effect) => effect_rgb(effect, t, phase),
    }
}

pub fn period_ms(look: RearLook, speed: f32) -> u64 {
    let s = speed.clamp(0.0, 100.0) / 100.0;
    let tween = |slow: f32, fast: f32| (slow + (fast - slow) * s).round() as u64;
    match look {
        RearLook::Solid(_) => tween(18000.0, 8000.0),
        RearLook::Play(effect) => match effect {
            Effect::RainbowEnergy
            | Effect::RainbowJump
            | Effect::RgbJump
            | Effect::YcmJump => tween(7200.0, 2800.0),
            Effect::RainbowStrobe | Effect::RgbStrobe | Effect::YcmStrobe => tween(5600.0, 2200.0),
            Effect::SevenFade
            | Effect::RedYellowFade
            | Effect::RedPurpleFade
            | Effect::GreenCyanFade
            | Effect::GreenYellowFade
            | Effect::BluePurpleFade => tween(16000.0, 7000.0),
            Effect::RainbowBrushFwd
            | Effect::RainbowBrushRev
            | Effect::RgbBrushFwd
            | Effect::RgbBrushRev
            | Effect::YcmBrushFwd
            | Effect::YcmBrushRev
            | Effect::RainbowBrushClose
            | Effect::RainbowBrushOpen
            | Effect::RgbBrushClose
            | Effect::RgbBrushOpen
            | Effect::YcmBrushClose
            | Effect::YcmBrushOpen => tween(14000.0, 6000.0),
            _ => tween(18000.0, 8000.0),
        },
    }
}

fn effect_rgb(effect: Effect, t: f32, phase: f32) -> Rgb {
    let p = phase.rem_euclid(1.0);
    let u = t.rem_euclid(1.0);
    let pal = palette(effect);
    match effect {
        Effect::AutoLoop | Effect::RainbowFwd => chroma_chase(pal, u, p, false),
        Effect::RainbowRev => chroma_chase(pal, u, p, true),
        Effect::RainbowEnergy | Effect::RainbowJump | Effect::RgbJump | Effect::YcmJump => {
            jump(pal, p)
        }
        Effect::RainbowStrobe | Effect::RgbStrobe | Effect::YcmStrobe => strobe(pal, p),
        Effect::SevenFade
        | Effect::RedYellowFade
        | Effect::RedPurpleFade
        | Effect::GreenCyanFade
        | Effect::GreenYellowFade
        | Effect::BluePurpleFade => fade_cycle(pal, p),
        Effect::RedHorse
        | Effect::GreenHorse
        | Effect::BlueHorse
        | Effect::YellowHorse
        | Effect::CyanHorse
        | Effect::PurpleHorse
        | Effect::WhiteHorse => {
            let color = horse_color(effect);
            let d = wrap_dist(u, p);
            color.scale((1.0 - d * 5.0).clamp(0.06, 1.0))
        }
        Effect::RainbowFollowFwd
        | Effect::RgbFollowFwd
        | Effect::YcmFollowFwd => follow(pal, u, p, false),
        Effect::RainbowFollowRev
        | Effect::RgbFollowRev
        | Effect::YcmFollowRev => follow(pal, u, p, true),
        Effect::RainbowDriftFwd | Effect::RgbDriftFwd | Effect::YcmDriftFwd => {
            drift(pal, u, p, false)
        }
        Effect::RainbowDriftRev | Effect::RgbDriftRev | Effect::YcmDriftRev => {
            drift(pal, u, p, true)
        }
        Effect::RainbowBrushFwd | Effect::RgbBrushFwd | Effect::YcmBrushFwd => {
            brush(pal, u, p, false)
        }
        Effect::RainbowBrushRev | Effect::RgbBrushRev | Effect::YcmBrushRev => {
            brush(pal, u, p, true)
        }
        Effect::RainbowBrushClose | Effect::RgbBrushClose | Effect::YcmBrushClose => {
            curtain(brush(pal, u, p, false), u, p, true)
        }
        Effect::RainbowBrushOpen | Effect::RgbBrushOpen | Effect::YcmBrushOpen => {
            curtain(brush(pal, u, p, false), u, p, false)
        }
    }
}

/// 幻彩：同时约三色，每个灯单元都渐变到下一色，不是整块纯色。
fn chroma_chase(pal: &[Rgb], u: f32, p: f32, rev: bool) -> Rgb {
    let x = three_band(u, p, rev, pal.len());
    let i = wrap_index(x.floor() as i32, pal.len());
    let j = wrap_index(i as i32 + 1, pal.len());
    pal[i].lerp(pal[j], x.fract())
}

/// 追光：同时三色硬切，各约占三分之一；下一色接调色板顺序。
fn follow(pal: &[Rgb], u: f32, p: f32, rev: bool) -> Rgb {
    sample_step(pal, three_band(u, p, rev, pal.len()))
}

/// 飘动：相邻光单元直接是下一个颜色。
fn drift(pal: &[Rgb], u: f32, p: f32, rev: bool) -> Rgb {
    let cells = 15.0;
    sample_step(pal, along(u, p, rev) * cells)
}

fn jump(pal: &[Rgb], p: f32) -> Rgb {
    sample_step(pal, p * pal.len() as f32)
}

fn strobe(pal: &[Rgb], p: f32) -> Rgb {
    let on = (p * pal.len() as f32 * 2.0).fract() < 0.55;
    if on {
        jump(pal, p)
    } else {
        Rgb::new(0x18, 0x14, 0x10)
    }
}

/// 渐变：同一色由明到暗，灭掉后再换下一色。
fn fade_cycle(pal: &[Rgb], p: f32) -> Rgb {
    let x = p * pal.len() as f32;
    let i = wrap_index(x.floor() as i32, pal.len());
    pal[i].scale((1.0 - x.fract()).clamp(0.04, 1.0))
}

/// 刷色：本色追着淡亮版刷过去，刷完跳下一色。正向往左，反向往右。
fn brush(pal: &[Rgb], u: f32, p: f32, rev: bool) -> Rgb {
    let n = pal.len() as f32;
    let x = p * n;
    let i = wrap_index(x.floor() as i32, pal.len());
    let f = x.fract();
    let deep = shade(pal[i]);
    let light = wash(pal[i]);
    let head = if rev { f } else { 1.0 - f };
    let mix = ((u - head) / 0.12 + 0.5).clamp(0.0, 1.0);
    if rev {
        deep.lerp(light, mix)
    } else {
        light.lerp(deep, mix)
    }
}

fn curtain(color: Rgb, u: f32, p: f32, close: bool) -> Rgb {
    let from_center = (u - 0.5).abs() * 2.0;
    let open = if close { 1.0 - p } else { p };
    if from_center <= open {
        color
    } else {
        color.scale(0.08)
    }
}

fn shade(color: Rgb) -> Rgb {
    color.scale(0.38)
}

fn wash(color: Rgb) -> Rgb {
    color.lerp(Rgb::new(0xFF, 0xF8, 0xF0), 0.55)
}

fn along(u: f32, p: f32, rev: bool) -> f32 {
    if rev {
        (u - p).rem_euclid(1.0)
    } else {
        (u + p).rem_euclid(1.0)
    }
}

fn three_band(u: f32, p: f32, rev: bool, n: usize) -> f32 {
    let pos = if rev { 1.0 - u } else { u };
    pos * 3.0 + p * n as f32
}

fn sample_step(pal: &[Rgb], x: f32) -> Rgb {
    pal[wrap_index(x.floor() as i32, pal.len())]
}

fn wrap_index(i: i32, n: usize) -> usize {
    let n = n as i32;
    (((i % n) + n) % n) as usize
}

fn wrap_dist(a: f32, b: f32) -> f32 {
    (a - b)
        .abs()
        .min((a - b + 1.0).abs())
        .min((a - b - 1.0).abs())
}

fn horse_color(effect: Effect) -> Rgb {
    match effect {
        Effect::RedHorse => DUSTY_RED,
        Effect::GreenHorse => DUSTY_GREEN,
        Effect::BlueHorse => DUSTY_BLUE,
        Effect::YellowHorse => DUSTY_YELLOW,
        Effect::CyanHorse => DUSTY_CYAN,
        Effect::PurpleHorse => DUSTY_PURPLE,
        Effect::WhiteHorse => DUSTY_WHITE,
        _ => DUSTY_BLUE,
    }
}

fn palette(effect: Effect) -> &'static [Rgb] {
    const SEVEN: [Rgb; 7] = [
        DUSTY_RED,
        DUSTY_GREEN,
        DUSTY_BLUE,
        DUSTY_YELLOW,
        DUSTY_CYAN,
        DUSTY_PURPLE,
        DUSTY_WHITE,
    ];
    const RGB: [Rgb; 3] = [DUSTY_RED, DUSTY_GREEN, DUSTY_BLUE];
    const YCM: [Rgb; 3] = [DUSTY_YELLOW, DUSTY_CYAN, DUSTY_PURPLE];
    const RED_YELLOW: [Rgb; 2] = [DUSTY_RED, DUSTY_YELLOW];
    const RED_PURPLE: [Rgb; 2] = [DUSTY_RED, DUSTY_PURPLE];
    const GREEN_CYAN: [Rgb; 2] = [DUSTY_GREEN, DUSTY_CYAN];
    const GREEN_YELLOW: [Rgb; 2] = [DUSTY_GREEN, DUSTY_YELLOW];
    const BLUE_PURPLE: [Rgb; 2] = [DUSTY_BLUE, DUSTY_PURPLE];
    match effect {
        Effect::RgbJump
        | Effect::RgbStrobe
        | Effect::RgbFollowFwd
        | Effect::RgbFollowRev
        | Effect::RgbDriftFwd
        | Effect::RgbDriftRev
        | Effect::RgbBrushFwd
        | Effect::RgbBrushRev
        | Effect::RgbBrushClose
        | Effect::RgbBrushOpen => &RGB,
        Effect::YcmJump
        | Effect::YcmStrobe
        | Effect::YcmFollowFwd
        | Effect::YcmFollowRev
        | Effect::YcmDriftFwd
        | Effect::YcmDriftRev
        | Effect::YcmBrushFwd
        | Effect::YcmBrushRev
        | Effect::YcmBrushClose
        | Effect::YcmBrushOpen => &YCM,
        Effect::RedYellowFade => &RED_YELLOW,
        Effect::RedPurpleFade => &RED_PURPLE,
        Effect::GreenCyanFade => &GREEN_CYAN,
        Effect::GreenYellowFade => &GREEN_YELLOW,
        Effect::BluePurpleFade => &BLUE_PURPLE,
        _ => &SEVEN,
    }
}

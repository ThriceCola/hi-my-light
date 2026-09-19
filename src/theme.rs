use gpui::prelude::*;
use gpui::{
    App, Bounds, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Pixels, Point, SharedString, Size, StatefulInteractiveElement, Styled, Window, div, point, px,
    rgb, size,
};

pub const INK: u32 = 0x050506;
pub const PANEL: u32 = 0x000000;
pub const LINE: u32 = 0x3F3F46;
pub const AMBER: u32 = 0xD4B896;
pub const STONE: u32 = 0x8B8B93;
pub const PAPER: u32 = 0xFAFAFA;
pub const MIST: u32 = 0xD6D6DA;
pub const HOVER: u32 = 0x141416;
pub const TRACK: u32 = 0x27272A;
pub const BAD: u32 = 0xEF4444;

pub fn tone(hex: u32, alpha: f32) -> Hsla {
    Hsla::from(rgb(hex)).opacity(alpha)
}

pub const FALLBACK_ASPECT: f32 = 16.0 / 9.0;

/// 当前主屏宽高比（宽/高）。竖屏会夹到常见显示器范围。
pub fn display_aspect(cx: &App) -> f32 {
    let bounds = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next())
        .map(|display| display.bounds());
    let Some(bounds) = bounds else {
        return FALLBACK_ASPECT;
    };
    let ratio = bounds.size.width / bounds.size.height;
    if ratio >= 1.0 {
        ratio.clamp(1.25, 2.4)
    } else {
        (1.0 / ratio).clamp(1.25, 2.4)
    }
}

pub const TITLEBAR_H: f32 = 36.0;

/// 横向窗口。高度含标题栏，按内容估算，避免开局裁掉底部。
pub fn home_window_size(cx: &App) -> Size<Pixels> {
    let work = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| size(px(1920.), px(1080.)));

    let content_w = px(880.);
    let content_h = px(TITLEBAR_H) + px(645.);
    let width = content_w.min(work.width * 0.92);
    let height = content_h.min(work.height * 0.90);
    size(width.max(px(760.)), height.max(px(640.)))
}

pub fn glow_from_kelvin(kelvin: u16) -> u32 {
    let t = ((kelvin as f32 - 3000.0) / 3000.0).clamp(0.0, 1.0);
    let r = (232.0 - t * 70.0).round() as u32;
    let g = (160.0 + t * 40.0).round() as u32;
    let b = (96.0 + t * 130.0).round() as u32;
    (r << 16) | (g << 8) | b
}

pub fn lerp_rgb(a: u32, b: u32, t: f32) -> u32 {
    let mix = |shift: u32| {
        let ca = ((a >> shift) & 0xff) as f32;
        let cb = ((b >> shift) & 0xff) as f32;
        ((ca + (cb - ca) * t).round() as u32) & 0xff
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

pub fn ghost_btn(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .px_3()
        .py_1()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .font_weight(FontWeight::SEMIBOLD)
        .border_1()
        .border_color(rgb(LINE))
        .text_color(if disabled { rgb(0x52525B) } else { rgb(PAPER) })
        .when(!disabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(rgb(HOVER)))
                .on_click(on_click)
        })
        .child(label.into())
}

pub fn chip_btn(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    ghost_btn(id, label, disabled, on_click)
}

pub fn step_btn(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .size(px(28.))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgb(LINE))
        .text_color(rgb(PAPER))
        .text_size(px(16.))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(HOVER)))
        .on_click(on_click)
        .child(label)
}

pub fn check_box(checked: bool) -> impl IntoElement {
    div()
        .size(px(14.))
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(rgb(PAPER))
        .bg(if checked { rgb(PAPER) } else { rgb(PANEL) })
        .child(if checked {
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(INK))
                .child("✓")
        } else {
            div().child("")
        })
}

pub fn status_pill(connected: bool, scanning: bool) -> impl IntoElement {
    let (label, color) = if connected {
        ("已连接", AMBER)
    } else if scanning {
        ("扫描中", 0xC8D4E8)
    } else {
        ("未连接", BAD)
    };
    div()
        .flex()
        .gap_2()
        .items_center()
        .child(div().size(px(6.)).bg(rgb(color)).shadow(if connected {
            vec![gpui::BoxShadow {
                color: tone(color, 0.85),
                blur_radius: px(8.),
                spread_radius: px(0.),
                inset: false,
                offset: point(px(0.), px(0.)),
            }]
        } else {
            Vec::new()
        }))
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if connected { rgb(PAPER) } else { rgb(STONE) })
                .child(label),
        )
}

pub fn power_switch(
    id: impl Into<gpui::ElementId>,
    on: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .flex()
        .items_center()
        .gap(px(10.))
        .cursor_pointer()
        .on_click(on_click)
        .child(
            div()
                .w(px(28.))
                .h(px(14.))
                .relative()
                .border_1()
                .border_color(rgb(PAPER))
                .child(
                    div()
                        .absolute()
                        .top(px(2.))
                        .when(on, |d| d.left(px(16.)))
                        .when(!on, |d| d.left(px(2.)))
                        .size(px(8.))
                        .bg(rgb(if on { AMBER } else { PAPER })),
                ),
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(if on { rgb(PAPER) } else { rgb(STONE) })
                .child(if on { "开" } else { "关" }),
        )
}

pub fn paint_spinner_ring(bounds: Bounds<Pixels>, delta: f32, window: &mut Window) {
    let center = bounds.center();
    let radius = f32::from(bounds.size.width.min(bounds.size.height)) * 0.38;
    if let Some(path) = stroke_arc(center, radius, 0.0, 359.0, 2.0) {
        window.paint_path(path, tone(LINE, 1.0));
    }
    let start = delta * 360.0;
    if let Some(path) = stroke_arc(center, radius, start, start + 92.0, 2.6) {
        window.paint_path(path, tone(PAPER, 0.95));
    }
}

pub fn stroke_arc(
    center: Point<Pixels>,
    radius: f32,
    start_deg: f32,
    end_deg: f32,
    width: f32,
) -> Option<gpui::Path<Pixels>> {
    let mut builder = PathBuilder::stroke(px(width));
    let mut first = true;
    let steps = 32;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let deg = start_deg + (end_deg - start_deg) * t;
        let rad = deg.to_radians();
        let p = point(
            center.x + px(radius * rad.cos()),
            center.y + px(radius * rad.sin()),
        );
        if first {
            builder.move_to(p);
            first = false;
        } else {
            builder.line_to(p);
        }
    }
    builder.build().ok()
}

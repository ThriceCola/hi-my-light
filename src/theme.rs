use gpui::{
    App, Bounds, FontWeight, Hsla, InteractiveElement, IntoElement, ParentElement, PathBuilder,
    Pixels, Point, SharedString, Size, StatefulInteractiveElement, Styled, Window, div, point,
    prelude::*, px, rgb, size,
};

pub const INK: u32 = 0x0B0A08;
pub const PANEL: u32 = 0x141310;
pub const LINE: u32 = 0x2A261F;
pub const AMBER: u32 = 0xD4A054;
pub const AMBER_SOFT: u32 = 0xE8C27A;
pub const STONE: u32 = 0x8A8478;
pub const PAPER: u32 = 0xE8E0D4;

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

/// 主页窗口跟主屏同比例，缩进显示器里。
pub fn home_window_size(cx: &App) -> Size<Pixels> {
    let aspect = display_aspect(cx);
    let work = cx
        .primary_display()
        .or_else(|| cx.displays().into_iter().next())
        .map(|display| display.bounds().size)
        .unwrap_or_else(|| size(px(1920.), px(1080.)));

    let mut width = (work.width * 0.56).max(px(840.)).min(px(1180.));
    let mut height = width * (1.0 / aspect);
    let max_h = work.height * 0.78;
    if height > max_h {
        height = max_h;
        width = height * aspect;
    }
    size(width, height)
}

pub fn glow_from_kelvin(kelvin: u16) -> u32 {
    let t = ((kelvin as f32 - 3000.0) / 3000.0).clamp(0.0, 1.0);
    lerp_rgb(AMBER_SOFT, 0xC9D7EE, t)
}

pub fn lerp_rgb(a: u32, b: u32, t: f32) -> u32 {
    let mix = |shift: u32| {
        let ca = ((a >> shift) & 0xff) as f32;
        let cb = ((b >> shift) & 0xff) as f32;
        ((ca + (cb - ca) * t).round() as u32) & 0xff
    };
    (mix(16) << 16) | (mix(8) << 8) | mix(0)
}

pub fn chip_btn(
    id: impl Into<gpui::ElementId>,
    label: impl Into<SharedString>,
    disabled: bool,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id.into())
        .px_3()
        .py_1p5()
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .border_1()
        .border_color(rgb(LINE))
        .text_color(if disabled {
            rgb(0x5A564C)
        } else {
            rgb(AMBER_SOFT)
        })
        .when(!disabled, |d| {
            d.cursor_pointer()
                .hover(|s| s.bg(rgb(0x221E18)))
                .on_click(on_click)
        })
        .child(label.into())
}

pub fn check_box(checked: bool) -> impl IntoElement {
    div()
        .size(px(16.))
        .rounded_sm()
        .flex()
        .items_center()
        .justify_center()
        .border_1()
        .border_color(if checked { rgb(AMBER) } else { rgb(0x5A564C) })
        .bg(if checked { rgb(AMBER) } else { rgb(INK) })
        .child(if checked {
            div()
                .text_xs()
                .font_weight(FontWeight::BOLD)
                .text_color(rgb(0x1A140C))
                .child("✓")
        } else {
            div().child("")
        })
}

pub fn status_pill(connected: bool, scanning: bool) -> impl IntoElement {
    div()
        .flex()
        .gap_2()
        .items_center()
        .px_3()
        .py_1()
        .rounded_full()
        .bg(rgb(0x181612))
        .border_1()
        .border_color(rgb(LINE))
        .child(
            div()
                .size_2()
                .rounded_full()
                .bg(if connected {
                    rgb(AMBER)
                } else if scanning {
                    rgb(0x6B8CAE)
                } else {
                    rgb(0x4A453C)
                }),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(STONE))
                .child(if connected {
                    "已连接"
                } else if scanning {
                    "扫描"
                } else {
                    "未连接"
                }),
        )
}

pub fn section_label(text: &'static str) -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgb(STONE))
        .child(text)
}

pub fn paint_spinner_ring(bounds: Bounds<Pixels>, delta: f32, window: &mut Window) {
    let center = bounds.center();
    let radius = f32::from(bounds.size.width.min(bounds.size.height)) * 0.38;
    if let Some(path) = stroke_arc(center, radius, 0.0, 359.0, 2.0) {
        window.paint_path(path, tone(LINE, 1.0));
    }
    let start = delta * 360.0;
    if let Some(path) = stroke_arc(center, radius, start, start + 92.0, 2.6) {
        window.paint_path(path, tone(AMBER, 0.95));
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

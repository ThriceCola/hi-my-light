use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Bounds, Context, CursorStyle, Decorations, HitboxBehavior,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Pixels, Point, ResizeEdge, Size,
    StatefulInteractiveElement, Styled, Window, WindowBackgroundAppearance, WindowBounds,
    WindowControls, WindowDecorations, WindowOptions, canvas, div, point, prelude::*, px, rgb, size,
    transparent_black,
};

use crate::theme::{AMBER, INK, LINE, PAPER, STONE, home_window_size, paint_spinner_ring, tone};

const SHADOW: f32 = 10.0;
const ROUND: f32 = 10.0;

pub fn window_options(cx: &gpui::App) -> WindowOptions {
    let bounds = gpui::Bounds::centered(None, home_window_size(cx), cx);
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: Some("HML".into()),
            appears_transparent: true,
            traffic_light_position: None,
        }),
        window_background: WindowBackgroundAppearance::Opaque,
        window_decorations: Some(WindowDecorations::Client),
        window_min_size: Some(size(px(560.), px(640.))),
        app_id: Some("hi-my-light".into()),
        ..Default::default()
    }
}

pub fn frame(
    window: &mut Window,
    title: impl IntoElement,
    body: impl IntoElement,
) -> impl IntoElement {
    let decorations = window.window_decorations();
    let rounding = px(ROUND);
    let shadow_size = px(SHADOW);
    window.set_client_inset(match decorations {
        Decorations::Client { tiling } if !tiling.is_tiled() => shadow_size,
        _ => px(0.),
    });

    div()
        .id("window-backdrop")
        .size_full()
        .bg(transparent_black())
        .map(|el| match decorations {
            Decorations::Server => el,
            Decorations::Client { tiling, .. } => el
                .child(
                    canvas(
                        |_bounds, window, _cx| {
                            window.insert_hitbox(
                                Bounds::new(
                                    point(px(0.0), px(0.0)),
                                    window.window_bounds().get_bounds().size,
                                ),
                                HitboxBehavior::Normal,
                            )
                        },
                        move |_bounds, hitbox, window, _cx| {
                            let mouse = window.mouse_position();
                            let size = window.window_bounds().get_bounds().size;
                            let Some(edge) = resize_edge(mouse, shadow_size, size) else {
                                return;
                            };
                            window.set_cursor_style(cursor_for_edge(edge), &hitbox);
                        },
                    )
                    .size_full()
                    .absolute(),
                )
                .when(!(tiling.top || tiling.left), |d| d.rounded_tl(rounding))
                .when(!(tiling.top || tiling.right), |d| d.rounded_tr(rounding))
                .when(!tiling.top, |d| d.pt(shadow_size))
                .when(!tiling.bottom, |d| d.pb(shadow_size))
                .when(!tiling.left, |d| d.pl(shadow_size))
                .when(!tiling.right, |d| d.pr(shadow_size))
                .on_mouse_move(|_, window, _| window.refresh())
                .on_mouse_down(MouseButton::Left, move |e, window, _| {
                    let size = window.window_bounds().get_bounds().size;
                    if let Some(edge) = resize_edge(e.position, shadow_size, size) {
                        window.start_window_resize(edge);
                    }
                }),
        })
        .child(
            div()
                .size_full()
                .flex()
                .flex_col()
                .bg(rgb(INK))
                .text_color(rgb(PAPER))
                .font_family(".SystemUIFont")
                .overflow_hidden()
                .map(|el| match decorations {
                    Decorations::Server => el,
                    Decorations::Client { tiling } => el
                        .border_color(rgb(LINE))
                        .when(!(tiling.top || tiling.left), |d| d.rounded_tl(rounding))
                        .when(!(tiling.top || tiling.right), |d| d.rounded_tr(rounding))
                        .when(!(tiling.bottom || tiling.left), |d| d.rounded_bl(rounding))
                        .when(!(tiling.bottom || tiling.right), |d| d.rounded_br(rounding))
                        .when(!tiling.top, |d| d.border_t_1())
                        .when(!tiling.bottom, |d| d.border_b_1())
                        .when(!tiling.left, |d| d.border_l_1())
                        .when(!tiling.right, |d| d.border_r_1())
                        .when(!tiling.is_tiled(), |d| {
                            d.shadow(vec![gpui::BoxShadow {
                                color: gpui::hsla(0., 0., 0., 0.45),
                                blur_radius: shadow_size / 2.,
                                spread_radius: px(0.),
                                inset: false,
                                offset: point(px(0.), px(0.)),
                            }])
                        }),
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .on_mouse_move(|_, _, cx| cx.stop_propagation())
                .child(title)
                .child(body),
        )
}

/// 标题栏拖动跟 Zed 一样：按下只记状态，移动才 `start_window_move`，避免点按钮也被拖走。
pub fn titlebar<T: TitleDrag + 'static>(
    controls: WindowControls,
    leading: impl IntoElement,
    trailing: impl IntoElement,
    cx: &mut Context<T>,
) -> impl IntoElement {
    div()
        .id("halo-titlebar")
        .window_control_area(gpui::WindowControlArea::Drag)
        .flex()
        .w_full()
        .h(px(40.))
        .px_3()
        .items_center()
        .justify_between()
        .border_b_1()
        .border_color(rgb(LINE))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.set_title_drag(true);
                cx.notify();
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|this, _, _, cx| {
                this.set_title_drag(false);
                cx.notify();
            }),
        )
        .on_mouse_down_out(cx.listener(|this, _, _, cx| {
            this.set_title_drag(false);
            cx.notify();
        }))
        .on_mouse_move(cx.listener(|this, _, window, cx| {
            if this.title_dragging() {
                this.set_title_drag(false);
                window.start_window_move();
                cx.notify();
            }
        }))
        .on_click(|e, window, _| {
            if e.is_right_click() {
                window.show_window_menu(e.position());
            } else if e.click_count() == 2 {
                window.zoom_window();
            }
        })
        .child(leading)
        .child(
            div()
                .flex()
                .gap_2()
                .items_center()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(trailing)
                .when(controls.minimize, |row| {
                    row.child(win_btn(
                        "win-min",
                        "—",
                        cx.listener(|_, _, window, _| window.minimize_window()),
                    ))
                })
                .when(controls.maximize, |row| {
                    row.child(win_btn(
                        "win-max",
                        "▢",
                        cx.listener(|_, _, window, _| window.zoom_window()),
                    ))
                })
                .child(win_btn(
                    "win-close",
                    "✕",
                    cx.listener(|this, _, window, cx| this.on_close_click(window, cx)),
                )),
        )
}

pub trait TitleDrag {
    fn title_dragging(&self) -> bool;
    fn set_title_drag(&mut self, dragging: bool);
    fn on_close_click(&mut self, window: &mut Window, cx: &mut Context<Self>)
    where
        Self: Sized;
}

fn win_btn(
    id: &'static str,
    label: &'static str,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .w(px(26.))
        .h(px(22.))
        .rounded_md()
        .flex()
        .items_center()
        .justify_center()
        .text_xs()
        .text_color(rgb(STONE))
        .cursor_pointer()
        .hover(|s| s.bg(rgb(0x221E18)).text_color(rgb(AMBER)))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_click(on_click)
        .child(label)
}

pub fn brand() -> impl IntoElement {
    div()
        .text_xs()
        .font_weight(gpui::FontWeight::MEDIUM)
        .text_color(rgb(AMBER))
        .child("HML")
}

pub fn connecting_overlay() -> impl IntoElement {
    div()
        .id("connecting-veil")
        .absolute()
        .inset_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .bg(tone(INK, 0.55))
        .child(
            div()
                .id("connecting-spin")
                .size(px(36.))
                .with_animation(
                    "connecting-spin",
                    Animation::new(Duration::from_millis(900)).repeat(),
                    |this, delta| {
                        this.child(
                            canvas(
                                |_, _, _| {},
                                move |bounds, _, window, _| {
                                    paint_spinner_ring(bounds, delta, window);
                                },
                            )
                            .size_full(),
                        )
                    },
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(rgb(AMBER))
                .child("正在连接"),
        )
}

fn resize_edge(pos: Point<Pixels>, shadow_size: Pixels, size: Size<Pixels>) -> Option<ResizeEdge> {
    if pos.y < shadow_size && pos.x < shadow_size {
        Some(ResizeEdge::TopLeft)
    } else if pos.y < shadow_size && pos.x > size.width - shadow_size {
        Some(ResizeEdge::TopRight)
    } else if pos.y < shadow_size {
        Some(ResizeEdge::Top)
    } else if pos.y > size.height - shadow_size && pos.x < shadow_size {
        Some(ResizeEdge::BottomLeft)
    } else if pos.y > size.height - shadow_size && pos.x > size.width - shadow_size {
        Some(ResizeEdge::BottomRight)
    } else if pos.y > size.height - shadow_size {
        Some(ResizeEdge::Bottom)
    } else if pos.x < shadow_size {
        Some(ResizeEdge::Left)
    } else if pos.x > size.width - shadow_size {
        Some(ResizeEdge::Right)
    } else {
        None
    }
}

fn cursor_for_edge(edge: ResizeEdge) -> CursorStyle {
    match edge {
        ResizeEdge::Top | ResizeEdge::Bottom => CursorStyle::ResizeUpDown,
        ResizeEdge::Left | ResizeEdge::Right => CursorStyle::ResizeLeftRight,
        ResizeEdge::TopLeft | ResizeEdge::BottomRight => CursorStyle::ResizeUpLeftDownRight,
        ResizeEdge::TopRight | ResizeEdge::BottomLeft => CursorStyle::ResizeUpRightDownLeft,
    }
}

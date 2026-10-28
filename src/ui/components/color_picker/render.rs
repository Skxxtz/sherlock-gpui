use std::{cell::Cell, rc::Rc};

use gpui::{
    App,
    Bounds,
    DispatchPhase,
    Hsla,
    IntoElement,
    MouseButton,
    MouseDownEvent,
    MouseMoveEvent,
    MouseUpEvent,
    ParentElement,
    Pixels,
    Point,
    Styled,
    Window,
    canvas,
    div,
    hsla,
    linear_color_stop,
    linear_gradient,
    px,
};

use super::{FIELD_H, HANDLE, HUE_H, INNER_W, data::Hsv, state::ColorPickerState};
use crate::app::theme::ThemeData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drag {
    Field,
    Hue,
}

/// Fraction of `bounds` at `pos`, clamped to `0..=1` on both axes.
fn fraction(bounds: Bounds<Pixels>, pos: Point<Pixels>) -> (f32, f32) {
    let x = ((pos.x - bounds.origin.x) / bounds.size.width).clamp(0., 1.);
    let y = ((pos.y - bounds.origin.y) / bounds.size.height).clamp(0., 1.);
    (x, y)
}

pub fn corner_mask(w: f32, h: f32, r: f32, color: Hsla) -> impl IntoElement {
    const RING: f32 = 4.;
    div()
        .absolute()
        .top(px(-RING))
        .left(px(-RING))
        .w(px(w + 2. * RING))
        .h(px(h + 2. * RING))
        .rounded(px(r + RING))
        .border(px(RING))
        .border_color(color)
}

/// Invisible layer over an area that turns press-and-drag into `on_drag(x, y)` fractions. The
/// drag keeps tracking outside the area until the button is released.
pub fn drag_area<F>(target: Drag, drag: Rc<Cell<Option<Drag>>>, on_drag: Rc<F>) -> impl IntoElement
where
    F: Fn(f32, f32, &mut Window, &mut App) + 'static,
{
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            {
                let (drag, on_drag) = (drag.clone(), on_drag.clone());
                window.on_mouse_event(move |e: &MouseDownEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && e.button == MouseButton::Left
                        && bounds.contains(&e.position)
                    {
                        drag.set(Some(target));
                        let (x, y) = fraction(bounds, e.position);
                        on_drag(x, y, window, cx);
                    }
                });
            }
            {
                let (drag, on_drag) = (drag.clone(), on_drag.clone());
                window.on_mouse_event(move |e: &MouseMoveEvent, phase, window, cx| {
                    if phase == DispatchPhase::Bubble
                        && drag.get() == Some(target)
                        && e.pressed_button == Some(MouseButton::Left)
                    {
                        let (x, y) = fraction(bounds, e.position);
                        on_drag(x, y, window, cx);
                    }
                });
            }
            window.on_mouse_event(move |_: &MouseUpEvent, phase, _, _| {
                if phase == DispatchPhase::Bubble && drag.get() == Some(target) {
                    drag.set(None);
                }
            });
        },
    )
    // pinned to the corner: an absolute element without an inset stays at its flow position,
    // i.e. below any in-flow siblings
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}

pub fn handle(color: Hsla) -> gpui::Div {
    div()
        .absolute()
        .size(px(HANDLE))
        .rounded_full()
        .border_2()
        .border_color(gpui::white())
        .bg(color)
        .shadow_sm()
}

/// Saturation (x) / brightness (y) field for the current hue.
pub fn field<F>(state: &ColorPickerState, theme: &ThemeData, on_drag: Rc<F>) -> impl IntoElement
where
    F: Fn(f32, f32, &mut Window, &mut App) + 'static,
{
    let Hsv { h, s, v } = state.hsv;
    div()
        .relative()
        .w(px(INNER_W))
        .h(px(FIELD_H))
        .bg(hsla(h, 1., 0.5, 1.))
        .cursor_crosshair()
        // square layers: white → transparent (saturation), transparent → black (brightness)
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(linear_gradient(
                    90.,
                    linear_color_stop(hsla(0., 0., 1., 1.), 0.),
                    linear_color_stop(hsla(0., 0., 1., 0.), 1.),
                )),
        )
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .bg(linear_gradient(
                    180.,
                    linear_color_stop(hsla(0., 0., 0., 0.), 0.),
                    linear_color_stop(hsla(0., 0., 0., 1.), 1.),
                )),
        )
        .child(corner_mask(INNER_W, FIELD_H, 6., theme.bg_app))
        .child(
            handle(state.color())
                .left(px(s * INNER_W - HANDLE / 2.))
                .top(px((1. - v) * FIELD_H - HANDLE / 2.)),
        )
        .child(drag_area(Drag::Field, state.drag.clone(), on_drag))
}

/// Hue slider: six gradient segments around the color wheel.
pub fn hue_slider<F>(
    state: &ColorPickerState,
    theme: &ThemeData,
    on_drag: Rc<F>,
) -> impl IntoElement
where
    F: Fn(f32, f32, &mut Window, &mut App) + 'static,
{
    let h = state.hsv.h;
    div()
        .relative()
        .w(px(INNER_W))
        .h(px(HUE_H))
        .cursor_pointer()
        .child(div().size_full().flex().children((0..6).map(|i| {
            let (a, b) = (i as f32 / 6., (i + 1) as f32 / 6.);
            div().flex_1().h_full().bg(linear_gradient(
                90.,
                linear_color_stop(hsla(a, 1., 0.5, 1.), 0.),
                linear_color_stop(hsla(b % 1., 1., 0.5, 1.), 1.),
            ))
        })))
        .child(corner_mask(INNER_W, HUE_H, HUE_H / 2., theme.bg_app))
        .child(
            handle(hsla(h, 1., 0.5, 1.))
                .left(px(h * INNER_W - HANDLE / 2.))
                .top(px((HUE_H - HANDLE) / 2.)),
        )
        .child(drag_area(Drag::Hue, state.drag.clone(), on_drag))
}

pub fn key_hint(key: &'static str, label: &'static str, theme: &ThemeData) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap(px(5.))
        .child(
            div()
                .px(px(5.))
                .py(px(1.))
                .rounded(px(4.))
                .bg(theme.bg_keybind)
                .text_color(theme.primary_text)
                .child(key),
        )
        .child(label)
}

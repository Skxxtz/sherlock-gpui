use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::app::theme::ThemeData;

const WIDTH: f32 = 36.;
const HEIGHT: f32 = 20.;
const KNOB: f32 = 16.;
const PAD: f32 = 2.;

/// A switch showing `on`; `on_click` decides what toggling means.
pub fn toggle(
    id: impl Into<ElementId>,
    on: bool,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let (track, knob) = if on {
        (theme.accent(), theme.bg_app)
    } else {
        (theme.border, theme.secondary_text)
    };
    div()
        .id(id)
        .flex()
        .flex_none()
        .w(px(WIDTH))
        .h(px(HEIGHT))
        .p(px(PAD))
        .rounded(px(6.))
        .bg(track)
        .cursor_pointer()
        .map(|this| {
            if on {
                this.justify_end()
            } else {
                this.justify_start()
            }
        })
        .child(div().size(px(KNOB)).rounded(px(4.)).bg(knob))
        .on_click(on_click)
}

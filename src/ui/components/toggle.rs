use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
};

use crate::app::theme::ThemeData;

const WIDTH: f32 = 36.;
const HEIGHT: f32 = 20.;
const KNOB: f32 = 14.;
const PAD: f32 = 3.;

/// A switch showing `on`; `on_click` decides what toggling means.
pub fn toggle(
    id: impl Into<ElementId>,
    on: bool,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex_none()
        .w(px(WIDTH))
        .h(px(HEIGHT))
        .p(px(PAD))
        .rounded_full()
        .border_1()
        .cursor_pointer()
        .map(|this| {
            if on {
                this.bg(theme.color_succ).border_color(theme.color_succ)
            } else {
                this.bg(theme.bg_muted).border_color(theme.border)
            }
        })
        .child(
            div()
                .size(px(KNOB))
                .rounded_full()
                .bg(theme.primary_text)
                .when(on, |this| this.ml(px(WIDTH - KNOB - 2. * PAD - 2.))),
        )
        .on_click(on_click)
}

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::app::theme::ThemeData;

fn button(
    id: ElementId,
    label: &'static str,
    enabled: bool,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let base = div()
        .id(id)
        .size(px(24.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .text_size(px(14.))
        .child(label);
    if enabled {
        base.cursor_pointer()
            .text_color(theme.primary_text)
            .hover(|s| s.bg(theme.bg_selected))
            .on_click(on_click)
    } else {
        base.text_color(theme.tertiary_text)
    }
}

/// `value` between − and +; the buttons are disabled at `min` / `max`.
pub fn stepper(
    id: impl Into<ElementId>,
    value: i64,
    min: i64,
    max: i64,
    theme: &ThemeData,
    on_dec: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_inc: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let id: ElementId = id.into();
    div()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(2.))
        .p(px(2.))
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.bg_idle)
        .child(button(
            ElementId::NamedChild(id.clone().into(), "dec".into()),
            "−",
            value > min,
            theme,
            on_dec,
        ))
        .child(
            div()
                .min_w(px(36.))
                .flex()
                .justify_center()
                .font_family(theme.monospace.clone())
                .text_size(px(12.))
                .text_color(theme.primary_text)
                .child(value.to_string()),
        )
        .child(button(
            ElementId::NamedChild(id.into(), "inc".into()),
            "+",
            value < max,
            theme,
            on_inc,
        ))
}

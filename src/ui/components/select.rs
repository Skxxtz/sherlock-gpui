use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::app::theme::ThemeData;

/// Shows `value` with a cycle hint; `on_click` picks the next option.
pub fn select(
    id: impl Into<ElementId>,
    value: impl Into<SharedString>,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .px(px(10.))
        .py(px(4.))
        .rounded_md()
        .border_1()
        .border_color(theme.border)
        .bg(theme.bg_idle)
        .cursor_pointer()
        .hover(|s| s.border_color(theme.border_selected))
        .text_size(px(12.))
        .text_color(theme.primary_text)
        .child(value.into())
        .child(div().text_color(theme.tertiary_text).child("⇅"))
        .on_click(on_click)
}

use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::app::theme::ThemeData;

/// Shows `value` (lowercase, mono) with a chevron; `on_click` picks the next option.
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
        .gap(px(10.))
        .h(px(28.))
        .px(px(10.))
        .rounded(px(7.))
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(|s| s.bg(theme.bg_selected))
        .font_family(theme.monospace.clone())
        .text_size(px(12.))
        .text_color(theme.primary_text)
        .child(value.into().to_lowercase())
        .child(
            div()
                .text_size(px(10.))
                .text_color(theme.secondary_text)
                .child("▾"),
        )
        .on_click(on_click)
}

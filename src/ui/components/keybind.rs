use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};

use crate::app::theme::ThemeData;

/// Key name as shown on a cap: short, lowercase words (`ctrl`, `esc`, `n`).
pub fn key_label(key: &str) -> SharedString {
    let key = key.to_lowercase();
    match key.as_str() {
        "control" => "ctrl".into(),
        "escape" => "esc".into(),
        "return" => "enter".into(),
        "cmd" | "meta" => "super".into(),
        _ => key.into(),
    }
}

/// `keys` (e.g. `["ctrl", "k"]`) as mono caps; `on_click` is where recording would start.
pub fn keybind(
    id: impl Into<ElementId>,
    keys: &[SharedString],
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_none()
        .gap(px(4.))
        .cursor_pointer()
        .children(keys.iter().map(|key| {
            div()
                .min_w(px(24.))
                .h(px(24.))
                .px(px(7.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(5.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.bg_keybind)
                .font_family(theme.monospace.clone())
                .text_size(px(12.))
                .line_height(relative(1.0))
                .text_color(theme.primary_text)
                .child(key_label(key))
        }))
        .on_click(on_click)
}

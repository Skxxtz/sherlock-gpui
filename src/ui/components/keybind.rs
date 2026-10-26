use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, Window, div, px, relative,
};

use crate::app::theme::ThemeData;

/// Pretty key name: modifiers as symbols, single letters upper-case.
pub fn key_label(key: &str) -> SharedString {
    match key {
        "ctrl" | "control" => "⌃".into(),
        "shift" => "⇧".into(),
        "alt" => "⌥".into(),
        "super" | "cmd" | "meta" => "◆".into(),
        "enter" | "return" => "↵".into(),
        "escape" | "esc" => "Esc".into(),
        "tab" => "⇥".into(),
        "space" => "Space".into(),
        k if k.chars().count() == 1 => k.to_uppercase().into(),
        k => SharedString::from(k.to_string()),
    }
}

/// `keys` (e.g. `["ctrl", "k"]`) as caps; `on_click` is where recording would start.
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
        .p(px(2.))
        .rounded_md()
        .cursor_pointer()
        .hover(|s| s.bg(theme.bg_idle))
        .children(keys.iter().map(|key| {
            div()
                .min_w(px(22.))
                .px(px(6.))
                .py(px(4.))
                .flex()
                .justify_center()
                .rounded_sm()
                .border_1()
                .border_color(theme.border)
                .bg(theme.bg_keybind)
                .text_size(px(11.))
                .line_height(relative(1.0))
                .text_color(theme.primary_text)
                .child(key_label(key))
        }))
        .on_click(on_click)
}

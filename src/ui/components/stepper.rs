use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::{app::theme::ThemeData, loader::resolve_icon_path};

const ICON: f32 = 12.;

fn button(
    id: ElementId,
    icon: &'static str,
    enabled: bool,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    let color = if enabled {
        theme.secondary_text
    } else {
        theme.tertiary_text
    };
    let base = div()
        .id(id)
        .w(px(28.))
        .h_full()
        .flex()
        .items_center()
        .justify_center()
        .children(
            resolve_icon_path(icon)
                .and_then(|i| i.svg())
                .map(|svg| svg.size(px(ICON)).text_color(color)),
        );
    if enabled {
        base.cursor_pointer()
            .hover(|s| s.bg(theme.bg_selected))
            .on_click(on_click)
    } else {
        base
    }
}

/// `value` between chevrons; the buttons are disabled at `min` / `max`.
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
        .h(px(28.))
        .rounded(px(7.))
        .border_1()
        .border_color(theme.border)
        .overflow_hidden()
        .font_family(theme.monospace.clone())
        .child(button(
            ElementId::NamedChild(id.clone().into(), "dec".into()),
            "sherlock-chevron-left",
            value > min,
            theme,
            on_dec,
        ))
        .child(
            div()
                .w(px(28.))
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .text_size(px(13.))
                .line_height(px(16.))
                .text_color(theme.primary_text)
                .child(value.to_string()),
        )
        .child(button(
            ElementId::NamedChild(id.into(), "inc".into()),
            "sherlock-chevron-right",
            value < max,
            theme,
            on_inc,
        ))
}

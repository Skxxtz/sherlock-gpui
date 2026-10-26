use gpui::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, Rgba,
    StatefulInteractiveElement, Styled, Window, div, px,
};

use crate::app::theme::ThemeData;

/// `#rrggbb` (or `#rrggbbaa` if not opaque).
pub fn hex(color: Hsla) -> String {
    let c = Rgba::from(color);
    let [r, g, b, a] = [c.r, c.g, c.b, c.a].map(|v| (v.clamp(0., 1.) * 255.).round() as u8);
    if a == 255 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("#{r:02x}{g:02x}{b:02x}{a:02x}")
    }
}

/// Shows `color`; `on_click` is where a picker would open.
pub fn color_swatch(
    id: impl Into<ElementId>,
    color: Hsla,
    theme: &ThemeData,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .flex_none()
        .items_center()
        .gap(px(8.))
        .h(px(28.))
        .px(px(8.))
        .rounded(px(7.))
        .border_1()
        .border_color(theme.border)
        .cursor_pointer()
        .hover(|s| s.bg(theme.bg_selected))
        .child(div().size(px(14.)).rounded(px(4.)).bg(color))
        .child(
            div()
                .font_family(theme.monospace.clone())
                .text_size(px(12.))
                .text_color(theme.primary_text)
                .child(hex(color)),
        )
        .on_click(on_click)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::rgb;

    #[test]
    fn hex_roundtrip() {
        assert_eq!(hex(rgb(0x89b4fa).into()), "#89b4fa");
        assert_eq!(hex(rgb(0x000000).into()), "#000000");
    }
}

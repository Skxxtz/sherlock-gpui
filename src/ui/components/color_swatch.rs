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

/// Parses `#rrggbb`, `rrggbb`, `#rgb` or `rgb` (any case). `None` for anything else.
pub fn parse_hex(text: &str) -> Option<Hsla> {
    let digits = text.strip_prefix('#').unwrap_or(text);
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let value = match digits.len() {
        6 => u32::from_str_radix(digits, 16).ok()?,
        3 => {
            let short = u32::from_str_radix(digits, 16).ok()?;
            let (r, g, b) = ((short >> 8) & 0xf, (short >> 4) & 0xf, short & 0xf);
            ((r * 0x11) << 16) | ((g * 0x11) << 8) | (b * 0x11)
        }
        _ => return None,
    };
    Some(gpui::rgb(value).into())
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
        .bg(theme.bg_app)
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

    #[test]
    fn parse_hex_forms() {
        let parsed = |s| parse_hex(s).map(hex);
        assert_eq!(parsed("#89B4FA").as_deref(), Some("#89b4fa"));
        assert_eq!(parsed("f80").as_deref(), Some("#ff8800"));
        assert_eq!(parsed("#89b4f"), None);
        assert_eq!(parsed("#gg0000"), None);
    }
}

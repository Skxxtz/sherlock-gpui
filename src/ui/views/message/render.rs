use gpui::{FontWeight, IntoElement, ParentElement, Styled, div, px};

use crate::{app::theme::ThemeData, ui::components::sherlock_pixel};

/// Nothing to report: a pixel smiley and a status in the terminal's voice, centred.
pub fn empty_state(theme: &ThemeData) -> impl IntoElement {
    let mono = theme.monospace.clone();
    let accent = theme.accent();

    let sherlock = sherlock_pixel::sherlock_pixel_art(theme);

    let status = div()
        .flex()
        .flex_col()
        .gap(px(6.))
        .font_family(mono)
        .text_size(px(12.))
        .line_height(px(16.))
        .child(
            div()
                .flex()
                .gap(px(8.))
                .child(div().text_color(accent).child("//"))
                .child(
                    div()
                        .text_color(theme.primary_text)
                        .font_weight(FontWeight::MEDIUM)
                        .child("all clear"),
                ),
        )
        .child(
            div()
                .text_color(theme.tertiary_text)
                .child("0 errors · 0 warnings"),
        );

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .gap(px(24.))
        .child(sherlock)
        .child(status)
}

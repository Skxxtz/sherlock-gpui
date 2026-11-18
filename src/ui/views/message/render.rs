use gpui::{FontWeight, IntoElement, ParentElement, Styled, div, px};

use crate::{
    app::theme::ThemeData,
    ui::{
        components::sherlock_pixel,
        widgets::message::{AT_W, COL_GAP, COUNT_W, KIND_W, LVL_W, ROW_PX},
    },
};

/// Nothing to report: a pixel smiley and a status in the terminal's voice, centred.
pub fn empty_state(theme: &ThemeData) -> impl IntoElement {
    let mono = theme.typography.monospace.clone();
    let accent = theme.accent;

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

pub fn column_header(theme: &ThemeData) -> impl IntoElement {
    let cell = |w: Option<f32>, text: &'static str| {
        let d = div().child(text);
        match w {
            Some(w) => d.w(px(w)).flex_none(),
            None => d.flex_1(),
        }
    };
    div()
        .flex_none()
        .h(px(26.))
        .flex()
        .items_center()
        .gap(px(COL_GAP))
        .px(px(ROW_PX + 1.))
        .border_b_1()
        .border_color(theme.border_idle)
        .font_family(theme.typography.monospace.clone())
        .text_size(px(10.))
        .text_color(theme.tertiary_text)
        .child(cell(Some(LVL_W), "LVL"))
        .child(cell(Some(KIND_W), "CODE"))
        .child(cell(None, "MESSAGE"))
        .child(cell(Some(AT_W), "AT"))
        .child(div().w(px(COUNT_W)).flex_none().flex())
}

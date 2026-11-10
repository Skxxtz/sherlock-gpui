use super::{answer::measure, *};

pub(super) fn suggestions_el(body: &str, closed: bool, ui: &mut Ui) -> AnyElement {
    if !(ui.ready && closed) {
        return div().into_any_element();
    }
    let theme = ui.theme;
    let mut rows = div().w_full().flex().flex_col().gap(SPACE_2XS);
    for (i, text) in suggestion_lines(body).enumerate() {
        let selected = ui.focus.suggestion() == Some(i);
        let view = ui.view.clone();
        let hover_bg = theme.bg_selected;
        rows = rows.child(
            div()
                .id(("ask-suggestion", i))
                .w_full()
                .flex_none()
                .flex()
                .items_center()
                .gap(SPACE_L)
                .px(SPACE_M)
                .py(SPACE_S)
                .rounded(px(theme.metrics.radius))
                .cursor_pointer()
                .when(selected, |el| el.bg(theme.bg_selected))
                .hover(move |s| s.bg(hover_bg))
                .child(
                    div()
                        .flex_none()
                        .text_color(if selected {
                            theme.accent
                        } else {
                            theme.tertiary_text
                        })
                        .child("→"),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .overflow_hidden()
                        .whitespace_nowrap()
                        .text_color(if selected {
                            theme.primary_text
                        } else {
                            theme.secondary_text
                        })
                        .child(SharedString::from(text.to_owned())),
                )
                .when(selected, |el| {
                    el.child(
                        div()
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(SPACE_S)
                            .text_size(px(theme.typography.font_size_sm))
                            .text_color(theme.tertiary_text)
                            .child(kbd(theme, "Tab"))
                            .child("edit")
                            .child(kbd(theme, "⏎"))
                            .child("ask"),
                    )
                })
                .on_click(move |_: &ClickEvent, _, cx| {
                    let _ = view.update(cx, |v, cx| v.ask_suggestion(i, cx));
                }),
        );
    }
    div()
        .w_full()
        .ml(-SPACE_M)
        .child(measure(rows))
        .into_any_element()
}

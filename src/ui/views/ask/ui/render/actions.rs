use super::*;

pub(super) fn actions_el(body: &str, closed: bool, ui: &mut Ui) -> AnyElement {
    let id = ui.code_index;
    ui.code_index += 1;
    let ready = ui.ready && closed;
    let mut rows = div().w_full().flex().flex_col().gap(SPACE_S);
    for (n, action) in resolve_block(body, ui.apps).into_iter().enumerate() {
        let item = (ready && action.is_valid()).then(|| {
            ui.runnable_index += 1;
            ui.runnable_index - 1
        });
        let selected = item.is_some() && item == ui.focus.selected_item();
        rows = rows.child(action_row(
            ("ask-action", id * 64 + n),
            &action,
            item,
            selected,
            ui,
        ));
    }
    rows.into_any_element()
}

/// One action as a card row.
pub(super) fn action_row(
    id: (&'static str, usize),
    action: &Resolved,
    item: Option<usize>,
    selected: bool,
    ui: &Ui,
) -> impl IntoElement {
    let theme = ui.theme;
    let (label, detail) = action.label();
    let valid = action.is_valid();
    let hover_bg = theme.bg_selected;
    let row = div()
        .id(id)
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(SPACE_XL)
        .px(CARD_X)
        .py(SPACE_L)
        .rounded(px(theme.metrics.radius))
        .border_1()
        .border_color(if selected {
            theme.border_selected
        } else {
            theme.border
        })
        .bg(if selected {
            theme.bg_selected
        } else {
            theme.bg_code
        })
        .when(!valid, |el| el.opacity(OPACITY_DISABLED))
        .child(
            div()
                .flex_none()
                .size(ACTION_DOT)
                .rounded_full()
                .bg(if selected { theme.accent } else { theme.border }),
        )
        .child(
            div()
                .flex_none()
                .text_size(px(theme.typography.font_size_sm))
                .text_color(if selected {
                    theme.accent
                } else {
                    theme.tertiary_text
                })
                .child(SharedString::from(label)),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_color(if valid {
                    theme.primary_text
                } else {
                    theme.tertiary_text
                })
                .child(SharedString::from(detail.to_owned())),
        )
        .when(item.is_some(), |el| {
            el.cursor_pointer()
                .hover(move |s| s.bg(hover_bg))
                .child(if selected {
                    kbd(theme, "⏎").into_any_element()
                } else {
                    div().into_any_element()
                })
        });
    match item {
        Some(item) => {
            let view = ui.view.clone();
            row.on_click(move |_: &ClickEvent, win, cx| {
                let _ = view.update(cx, |v, cx| v.run_item(item, false, win, cx));
            })
            .into_any_element()
        }
        None => row.into_any_element(),
    }
}

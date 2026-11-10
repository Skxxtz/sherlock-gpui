use super::*;

pub(super) fn code_el(block: &Block, lang: &str, body: &str, ui: &mut Ui) -> AnyElement {
    let theme = ui.theme;
    let id = ui.code_index;
    ui.code_index += 1;
    let runnable = ui.ready && block.is_runnable();
    let item = runnable.then(|| {
        ui.runnable_index += 1;
        ui.runnable_index - 1
    });
    let selected = item.is_some() && item == ui.focus.selected_item();

    // Blocks that need a real terminal
    let in_terminal = match block {
        Block::Code { terminal, .. } => *terminal || needs_terminal(&markdown::script(body)),
        _ => false,
    };

    let run = item.and_then(|i| ui.runs.iter().find(|r| r.item == i));
    let running = run.is_some_and(CommandRun::is_running);

    let label: SharedString = if lang.is_empty() {
        "shell".into()
    } else {
        lang.to_lowercase().into()
    };
    let body = body.replace('\t', "    ");
    let lines = body.lines().count().max(1);
    let mono = theme.typography.monospace.clone();
    let small = px(theme.typography.font_size_sm);

    // Full card width, so the spacer pushes Copy/Run to the right edge.
    let header = div()
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(SPACE_XS)
        .pl(CARD_X)
        .pr(SPACE_M)
        .h(BAR_H)
        .text_size(small)
        .text_color(theme.tertiary_text)
        .child(div().font_family(mono.clone()).child(label))
        .child(div().flex_1())
        .child(button(("ask-copy", id), "Copy", false, ui.theme).on_click({
            let (view, text) = (ui.view.clone(), body.clone());
            move |_, _, cx| {
                let _ = view.update(cx, |v, cx| {
                    v.copy_to_clipboard(text.clone(), "Copied code", cx)
                });
            }
        }))
        .when_some(item.filter(|_| in_terminal), |el, item| {
            let view = ui.view.clone();
            let label = if selected { "Terminal ⏎" } else { "Terminal" };
            el.child(
                button(("ask-term", id), label, true, ui.theme).on_click(move |_, win, cx| {
                    let _ = view.update(cx, |v, cx| v.run_item(item, true, win, cx));
                }),
            )
        })
        .when_some(item.filter(|_| !in_terminal), |el, item| {
            let (term_view, run_view) = (ui.view.clone(), ui.view.clone());
            el.child(
                button(("ask-term", id), "Terminal", false, ui.theme).on_click(
                    move |_, win, cx| {
                        let _ = term_view.update(cx, |v, cx| v.run_item(item, true, win, cx));
                    },
                ),
            )
            .child(if running {
                button(("ask-run", id), "Stop", true, ui.theme)
                    .on_click(move |_, _, cx| {
                        let _ = run_view.update(cx, |v, cx| v.stop_run(cx));
                    })
                    .into_any_element()
            } else {
                let label = match (selected, run.is_some()) {
                    (true, _) => "Run ⏎",
                    (false, true) => "Run again",
                    (false, false) => "Run",
                };
                button(("ask-run", id), label, true, ui.theme)
                    .on_click(move |_, win, cx| {
                        let _ = run_view.update(cx, |v, cx| v.run_item(item, false, win, cx));
                    })
                    .into_any_element()
            })
        });

    // Line numbers once a block is long enough to talk about lines.
    let gutter = (lines > 3).then(|| {
        let numbers = (1..=lines)
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("\n");
        div()
            .flex_none()
            .pr(SPACE_XL)
            .text_color(theme.tertiary_text)
            .opacity(OPACITY_MUTED)
            .child(SharedString::from(numbers))
    });

    let code = Rich {
        text: body,
        spans: Vec::new(),
    };

    let content = div()
        .id(("ask-code", id))
        .w_full()
        .flex_none()
        .flex()
        .px(CARD_X)
        .pt(SPACE_2XS)
        .pb(SPACE_2XL)
        .overflow_x_scroll()
        .font_family(mono)
        .text_size(small)
        .line_height(relative(LINE_CODE))
        .text_color(theme.primary_text)
        .children(gutter)
        .child(
            div()
                .flex_grow()
                .flex_shrink_0()
                .whitespace_nowrap()
                .child(selectable(code, ui)),
        );

    let card = div()
        .id(("ask-code-card", id))
        .w_full()
        .flex()
        .flex_col()
        .rounded(px(theme.metrics.radius))
        .border_1()
        .border_color(if selected { theme.accent } else { theme.border })
        .bg(if selected {
            theme.bg_selected
        } else {
            theme.bg_code
        })
        .child(header)
        .child(content);

    let card = match item {
        Some(item) => {
            let view = ui.view.clone();
            card.on_click(move |_, _, cx| {
                let _ = view.update(cx, |v, cx| v.select_item(item, cx));
            })
            .into_any_element()
        }
        None => card.into_any_element(),
    };
    match run.cloned() {
        Some(run) => div()
            .w_full()
            .flex()
            .flex_col()
            .gap(SPACE_S)
            .child(card)
            .child(output_panel(&run, id, ui))
            .into_any_element(),
        None => card,
    }
}

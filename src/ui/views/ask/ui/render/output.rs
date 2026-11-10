use super::*;

pub(super) fn output_panel(run: &CommandRun, id: usize, ui: &mut Ui) -> AnyElement {
    let theme = ui.theme;
    let small = px(theme.typography.font_size_sm);
    let running = run.is_running();
    let lines = run.line_count();
    let status = match &run.state {
        RunState::Running => "Running".to_owned(),
        RunState::Exited(0) => "Done".to_owned(),
        RunState::Exited(code) => format!("✗ Exit {code}"),
        RunState::Killed => "Stopped".to_owned(),
        RunState::Failed(e) => format!("✗ Couldn't start: {e}"),
    };
    // A failure reads as one, and offers the fix while its output hasn't gone out yet.
    let failed = run.failed();
    let fixable = failed && !run.sent;
    let mut meta = vec![format!("{:.1} s", run.elapsed.as_secs_f32())];
    if lines > 0 {
        meta.push(format!("{lines} line{}", if lines == 1 { "" } else { "s" }));
    }

    let dot = div().flex_none().size(DOT).rounded_full().bg(if failed {
        theme.color_err
    } else if running {
        theme.accent
    } else {
        theme.border
    });
    let dot = if running {
        dot.with_animation(
            ("ask-run-dot", id),
            Animation::new(Duration::from_millis(1000))
                .repeat()
                .with_easing(pulsating_between(0.3, 1.0)),
            |el, t| el.opacity(t),
        )
        .into_any_element()
    } else {
        dot.into_any_element()
    };

    let item = run.item;
    let (copy_view, clear_view) = (ui.view.clone(), ui.view.clone());
    let full = run.output.clone();
    let header = div()
        .w_full()
        .flex_none()
        .flex()
        .items_center()
        .gap(SPACE_M)
        .pl(CARD_X)
        .pr(SPACE_M)
        .h(OUTPUT_BAR_H)
        .text_size(small)
        .text_color(theme.tertiary_text)
        .child(dot)
        .child(
            div()
                .text_color(if failed {
                    theme.color_err
                } else {
                    theme.secondary_text
                })
                .when(failed, |el| el.font_weight(FontWeight::SEMIBOLD))
                .child(SharedString::from(status)),
        )
        .child(SharedString::from(format!("·  {}", meta.join("  ·  "))))
        .when(!run.sent && !running && !failed, |el| {
            el.child(
                div()
                    .text_color(theme.accent)
                    .child("·  sent with your next question"),
            )
        })
        .child(div().flex_1())
        .child(
            button(("ask-out-copy", id), "Copy", false, theme).on_click(move |_, _, cx| {
                let _ = copy_view.update(cx, |v, cx| {
                    v.copy_to_clipboard(full.clone(), "Copied output", cx)
                });
            }),
        )
        .child(
            button(("ask-out-clear", id), "Clear", false, theme).on_click(move |_, _, cx| {
                let _ = clear_view.update(cx, |v, cx| v.clear_run(item, cx));
            }),
        );

    let (shown, hidden) = run.tail(MAX_SHOWN_LINES);
    let body: AnyElement = if shown.is_empty() {
        div()
            .px(CARD_X)
            .pb(SPACE_XL)
            .text_size(small)
            .text_color(theme.tertiary_text)
            .child(if running {
                "Waiting for output…"
            } else {
                "No output"
            })
            .into_any_element()
    } else {
        let text = Rich {
            text: shown.to_owned(),
            spans: Vec::new(),
        };
        let content_h = px(small.as_f32() * LINE_OUTPUT * shown.lines().count() as f32) + SPACE_XL;
        let scrolls = content_h > OUTPUT_MAX_H;
        let body_view = ui.view.clone();
        let wheel_view = ui.view.clone();
        let handle = ui.output_scroll.clone();
        div()
            .id(("ask-output", id))
            .when(scrolls, |el| {
                el.occlude()
                    .on_click(move |_, _, cx| {
                        let _ = body_view.update(cx, |v, cx| v.focus_output(item, cx));
                    })
                    // At either end, the wheel goes on to the chat.
                    .on_scroll_wheel(move |e, win, cx| {
                        let dy = e.delta.pixel_delta(win.line_height()).y;
                        let (y, max) = (handle.offset().y, handle.max_offset().y);
                        let at_top = y >= px(-0.5);
                        let at_bottom = y <= -max + px(0.5);
                        if (dy > px(0.) && at_top) || (dy < px(0.) && at_bottom) {
                            let _ = wheel_view.update(cx, |v, cx| v.scroll_chat(dy, cx));
                        }
                    })
            })
            .w_full()
            .flex_none()
            .max_h(OUTPUT_MAX_H)
            .overflow_y_scroll()
            .track_scroll(ui.output_scroll)
            .px(CARD_X)
            .pb(SPACE_XL)
            .font_family(theme.typography.monospace.clone())
            .text_size(small)
            .line_height(relative(LINE_OUTPUT))
            .text_color(theme.primary_text)
            .when(hidden > 0 || run.dropped > 0, |el| {
                el.child(div().pb(SPACE_S).text_color(theme.tertiary_text).child(
                    SharedString::from(format!(
                        "… {hidden} earlier lines (Copy has everything kept)"
                    )),
                ))
            })
            .child(div().whitespace_nowrap().child(selectable(text, ui)))
            .into_any_element()
    };

    let focused = ui.focus.output() == Some(item);
    let focus_view = ui.view.clone();
    let fix_view = ui.view.clone();
    // A run that wanted a terminal is offered there; any other failure goes to the model.
    let (fix_label, fix_note) = if run.wants_terminal() {
        (
            "Open in terminal",
            "it needs a real terminal (a prompt or a full screen)",
        )
    } else {
        ("Fix it", "asks a follow-up with this output attached")
    };
    let fix_row = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(SPACE_XL)
        .px(CARD_X)
        .pb(SPACE_L)
        .text_size(small)
        .text_color(theme.tertiary_text)
        .child(
            div()
                .id(("ask-out-fix", id))
                .flex()
                .items_center()
                .gap(SPACE_S)
                .px(SPACE_L)
                .py(BUTTON_Y)
                .rounded(BUTTON_RADIUS)
                .cursor_pointer()
                .bg(theme.accent)
                .text_color(theme.bg_app)
                .font_weight(FontWeight::SEMIBOLD)
                .child(fix_label)
                .when(focused, |el| {
                    el.child(
                        div()
                            .font_family(theme.typography.monospace.clone())
                            .font_weight(FontWeight::NORMAL)
                            .child("⏎"),
                    )
                })
                .on_click(move |_, _, cx| {
                    let _ = fix_view.update(cx, |v, cx| v.fix_run(item, cx));
                }),
        )
        .child(fix_note);
    div()
        .id(("ask-output-panel", id))
        .w_full()
        .flex()
        .flex_col()
        .rounded(px(theme.metrics.radius))
        .border_1()
        .border_color(if focused { theme.accent } else { theme.border })
        .bg(theme.bg_app)
        .child(header)
        .child(body)
        .when(fixable, |el| el.child(fix_row))
        .when(focused && !fixable, |el| {
            el.child(
                div()
                    .flex_none()
                    .flex()
                    .justify_end()
                    .gap(SPACE_2XL)
                    .px(CARD_X)
                    .pb(SPACE_L)
                    .text_size(small)
                    .text_color(theme.tertiary_text)
                    .child(hint_el(theme, "⏎", "copy"))
                    .child(hint_el(theme, "⌫", "clear")),
            )
        })
        .on_click(move |_, _, cx| {
            let _ = focus_view.update(cx, |v, cx| v.focus_output(item, cx));
        })
        .into_any_element()
}

pub(super) fn button(
    id: (&'static str, usize),
    label: &'static str,
    accent: bool,
    theme: &ThemeData,
) -> gpui::Stateful<Div> {
    let hover_bg = theme.bg_muted;
    let hover_text = theme.primary_text;
    div()
        .id(id)
        .px(BUTTON_X)
        .py(BUTTON_Y)
        .rounded(BUTTON_RADIUS)
        .cursor_pointer()
        .text_color(if accent {
            theme.accent
        } else {
            theme.tertiary_text
        })
        .hover(move |s| s.bg(hover_bg).text_color(hover_text))
        .child(label)
}

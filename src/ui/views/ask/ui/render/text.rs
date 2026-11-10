use gpui::{DispatchPhase, MouseMoveEvent, Pixels, Point, TextLayout, canvas, point};

use super::*;

pub(super) fn selectable(rich: Rich, ui: &mut Ui) -> AnyElement {
    let id = ui.next_text_id();
    let sel = ui.text_sel.filter(|s| s.block == id);
    let selection = sel.map(TextSelection::range);
    let dragging = sel.is_some_and(|s| s.dragging);
    let plain: SharedString = rich.text.clone().into();
    let text = styled(rich, ui.theme, selection);
    let layout = text.layout().clone();

    let (down_view, move_view, up_view, out_view) = (
        ui.view.clone(),
        ui.view.clone(),
        ui.view.clone(),
        ui.view.clone(),
    );
    let move_layout = layout.clone();
    div()
        .id(("ask-text", id))
        .w_full()
        .cursor_text()
        .on_mouse_down(MouseButton::Left, move |ev, _, cx| {
            let at = index_at(&layout, ev.position);
            let _ = down_view.update(cx, |v, cx| {
                v.begin_text_selection(id, plain.clone(), at, ev.click_count, cx)
            });
        })
        .on_mouse_up(MouseButton::Left, move |_, _, cx| {
            let _ = up_view.update(cx, |v, cx| v.end_text_selection(cx));
        })
        .on_mouse_up_out(MouseButton::Left, move |_, _, cx| {
            let _ = out_view.update(cx, |v, cx| v.end_text_selection(cx));
        })
        .child(text)
        .when(dragging, |el| {
            el.child(
                canvas(
                    |_, _, _| (),
                    move |_, _, win, _| {
                        win.on_mouse_event(move |ev: &MouseMoveEvent, phase, _, cx| {
                            if phase == DispatchPhase::Bubble
                                && ev.pressed_button == Some(MouseButton::Left)
                            {
                                let at = index_at(&move_layout, ev.position);
                                let _ = move_view
                                    .update(cx, |v, cx| v.extend_text_selection(id, at, cx));
                            }
                        });
                    },
                )
                .absolute()
                .size_0(),
            )
        })
        .into_any_element()
}

/// The text index under `pos`, with the pointer held to the text's rows.
fn index_at(layout: &TextLayout, pos: Point<Pixels>) -> usize {
    let b = layout.bounds();
    let y = pos.y.clamp(b.top(), (b.bottom() - px(1.)).max(b.top()));
    layout
        .index_for_position(point(pos.x, y))
        .unwrap_or_else(|i| i)
}

/// `Rich` text as a gpui `StyledText`, with the mouse selection highlighted.
pub(super) fn styled(
    rich: Rich,
    theme: &ThemeData,
    selection: Option<std::ops::Range<usize>>,
) -> StyledText {
    let mut spans = rich.spans;
    let sel_index = match selection.filter(|r| !r.is_empty()) {
        Some(sel) => {
            spans.push((sel, Inline::Caret));
            Some(spans.len() - 1)
        }
        None => None,
    };

    let mut cuts: Vec<usize> = spans.iter().flat_map(|(r, _)| [r.start, r.end]).collect();
    cuts.sort_unstable();
    cuts.dedup();
    let mut highlights = Vec::new();
    for w in cuts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut style = HighlightStyle::default();
        let mut any = false;
        for (i, (r, kind)) in spans.iter().enumerate() {
            if !(r.start <= a && b <= r.end) {
                continue;
            }
            any = true;
            if Some(i) == sel_index {
                style.background_color = Some(theme.selection);
                continue;
            }
            match kind {
                Inline::Bold => {
                    style.font_weight = Some(FontWeight::SEMIBOLD);
                    style.color = style.color.or(Some(theme.primary_text));
                }
                Inline::Italic => style.font_style = Some(FontStyle::Italic),
                Inline::Strike => {
                    style.strikethrough = Some(StrikethroughStyle {
                        thickness: HAIRLINE,
                        color: None,
                    })
                }
                Inline::Code => {
                    style.color = Some(theme.accent);
                    style.background_color = style.background_color.or(Some(theme.bg_code));
                }
                Inline::Link => {
                    style.color = Some(theme.accent);
                    style.underline = Some(UnderlineStyle {
                        thickness: HAIRLINE,
                        ..Default::default()
                    });
                }
                Inline::Caret => style.color = Some(theme.accent),
            }
        }
        if any {
            highlights.push((a..b, style));
        }
    }
    StyledText::new(rich.text).with_highlights(highlights)
}

pub(super) fn thinking(theme: &ThemeData) -> impl IntoElement {
    div()
        .flex_none()
        .font_family(theme.typography.serif.clone())
        .text_size(px(theme.typography.font_size * LEAD_SCALE))
        .text_color(theme.tertiary_text)
        .child("Thinking…")
        .with_animation(
            "ask-thinking",
            Animation::new(Duration::from_millis(1400))
                .repeat()
                .with_easing(pulsating_between(0.35, 1.0)),
            |el, t| el.opacity(t),
        )
}

pub(super) fn error_card(msg: &str, ui: &mut Ui) -> impl IntoElement {
    let theme = ui.theme;
    let rich = markdown::parse(msg)
        .into_iter()
        .find_map(|b| match b {
            Block::Paragraph(r) => Some(r),
            _ => None,
        })
        .unwrap_or_default();
    div()
        .w_full()
        .flex_none()
        .mt(SPACE_XS)
        .px(CARD_X)
        .py(SPACE_XL)
        .rounded(px(theme.metrics.radius))
        .border_1()
        .border_color(theme.border)
        .bg(theme.bg_code)
        .text_color(theme.primary_text)
        .child(selectable(rich, ui))
}

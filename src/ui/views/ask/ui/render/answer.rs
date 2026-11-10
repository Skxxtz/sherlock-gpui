use std::borrow::Cow;

use super::*;

pub fn answer(ex: &Exchange, blocks: &[Block], ui: &mut Ui) -> AnyElement {
    let theme = ui.theme;
    let size = theme.typography.font_size;

    // Blocks are shared with the exchange's parse cache
    let mut blocks: Vec<Cow<Block>> = blocks.iter().map(Cow::Borrowed).collect();
    if ex.phase.is_live()
        && let Some(last) = blocks.last_mut()
    {
        markdown::append_caret(std::slice::from_mut(last.to_mut()));
    }

    // Fix to now make text overlap.
    let mut col = div()
        .w_full()
        .max_w(WIDE)
        .flex()
        .flex_col()
        .font_family(theme.typography.font_family.clone());

    // The question
    let question = Rich {
        text: ex.question.to_string(),
        spans: Vec::new(),
    };
    col = col.child(measure(
        div()
            .pb(SPACE_L)
            .text_size(px(theme.typography.font_size_sm))
            .text_color(theme.tertiary_text)
            .child(selectable(question, ui))
            .when(!ex.attached_cmds.is_empty(), |el| {
                el.child(
                    div()
                        .pt(SPACE_XS)
                        .flex()
                        .gap(SPACE_S)
                        .text_color(theme.accent)
                        .child("↳ with output of")
                        .child(
                            div()
                                .font_family(theme.typography.monospace.clone())
                                .child(SharedString::from(ex.attached_cmds.join(", "))),
                        ),
                )
            }),
    ));

    // The first sentence (answer summary)
    if let Some(Block::Paragraph(first)) = blocks.first().map(|b| b.as_ref().clone()) {
        let at = first.first_sentence_end().unwrap_or(first.text.len());
        let (lead, rest) = first.split_at(at);
        col = col.child(measure(
            div()
                .pb(SPACE_3XL)
                .font_family(theme.typography.serif.clone())
                .text_size(px(size * LEAD_SCALE))
                .line_height(relative(LINE_LEAD))
                .text_color(theme.primary_text)
                .child(selectable(lead, ui)),
        ));
        if rest.is_empty() {
            blocks.remove(0);
        } else {
            blocks[0] = Cow::Owned(Block::Paragraph(rest));
        }
    } else if blocks.is_empty() && ex.phase == Phase::Waiting {
        col = col.child(thinking(theme));
    }

    let body = div()
        .w_full()
        .flex_none()
        .flex()
        .flex_col()
        .gap(BLOCK_GAP)
        .text_size(px(size))
        .line_height(relative(LINE_BODY))
        .text_color(theme.secondary_text);
    col = col.child(blocks_el(body, blocks.iter().map(|b| b.as_ref()), ui));

    if let Phase::Failed(msg) = &ex.phase {
        col = col.child(error_card(msg, ui));
    }
    col.into_any_element()
}

/// Ensure readable line length at any window size
pub(super) fn measure(inner: Div) -> Div {
    div()
        .w_full()
        .flex_none()
        .flex()
        .child(inner.flex_basis(MEASURE).min_w_0())
}

pub(super) fn blocks_el<'b>(
    container: Div,
    blocks: impl IntoIterator<Item = &'b Block>,
    ui: &mut Ui,
) -> Div {
    let mut el = container;
    for block in blocks {
        el = el.child(div().w_full().flex_none().child(block_el(block, ui)));
    }
    el
}

pub(super) fn block_el(block: &Block, ui: &mut Ui) -> AnyElement {
    let theme = ui.theme;
    let size = theme.typography.font_size;
    match block {
        Block::Paragraph(rich) => {
            measure(div().child(selectable(rich.clone(), ui))).into_any_element()
        }
        Block::Heading(level, rich) => measure(
            div()
                .pt(SPACE_S)
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(theme.primary_text)
                .when(*level <= 2, |el| el.text_size(px(size * HEADING_SCALE)))
                .child(selectable(rich.clone(), ui)),
        )
        .into_any_element(),
        Block::List { start, items } => {
            let mut list = div().flex().flex_col().gap(SPACE_S);
            for (i, item) in items.iter().enumerate() {
                let marker: SharedString = match start {
                    Some(n) => format!("{}.", n + i as u64).into(),
                    None => "•".into(),
                };
                let content = blocks_el(
                    div().flex_1().min_w_0().flex().flex_col().gap(SPACE_S),
                    item,
                    ui,
                );
                list = list.child(
                    div()
                        .w_full()
                        .flex_none()
                        .flex()
                        .gap(SPACE_L)
                        .child(
                            div()
                                .flex_none()
                                .min_w(LIST_MARKER_W)
                                .text_color(theme.tertiary_text)
                                .child(marker),
                        )
                        .child(content),
                );
            }
            measure(list).into_any_element()
        }
        Block::Quote(inner) => measure(blocks_el(
            div()
                .flex()
                .flex_col()
                .gap(SPACE_M)
                .pl(CARD_X)
                .border_l_2()
                .border_color(theme.border)
                .text_color(theme.tertiary_text),
            inner,
            ui,
        ))
        .into_any_element(),
        Block::Rule => div()
            .w_full()
            .my(SPACE_XS)
            .h(HAIRLINE)
            .bg(theme.border)
            .into_any_element(),
        Block::Table { head, rows } => table_el(head, rows, ui),
        Block::Code {
            lang, body, closed, ..
        } if lang.eq_ignore_ascii_case("sherlock") => actions_el(body, *closed, ui),
        Block::Code {
            lang, body, closed, ..
        } if is_next_block(lang) => suggestions_el(body, *closed, ui),
        Block::Code { lang, body, .. } => code_el(block, lang, body, ui),
    }
}

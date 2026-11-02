use gpui::{FontStyle, FontWeight, TextRun};

use crate::{
    app::theme::ThemeData,
    ui::utils::pango::utils::{SpanState, TagKind, classify_tag},
};

/// Tokenise `content` into alternating text/tag slices and build
/// (final_text, runs).  Returns empty runs if there is no markup.
pub(super) fn parse_pango(
    content: &str,
    theme: &std::sync::Arc<ThemeData>,
) -> (String, Vec<TextRun>) {
    let mut final_text = String::with_capacity(content.len());
    let mut runs: Vec<TextRun> = Vec::new();
    let mut bold_depth: usize = 0;
    let mut italic_depth: usize = 0;
    let mut span_stack: Vec<SpanState> = Vec::new();
    let mut scratch = String::new();
    let mut rest = content;

    while !rest.is_empty() {
        if let Some(tag_start) = rest.find('<') {
            if tag_start > 0 {
                scratch.clear();
                unescape_into(&rest[..tag_start], &mut scratch);
                push_run(
                    &scratch,
                    &RunContext {
                        bold_depth,
                        italic_depth,
                        state: span_stack.last(),
                    },
                    theme,
                    &mut final_text,
                    &mut runs,
                );
            }
            rest = &rest[tag_start..];

            if let Some(tag_end) = rest.find('>') {
                let inner = rest[1..tag_end].trim();

                match classify_tag(inner) {
                    TagKind::BoldOpen => bold_depth += 1,
                    TagKind::BoldClose => bold_depth = bold_depth.saturating_sub(1),
                    TagKind::ItalicOpen => italic_depth += 1,
                    TagKind::ItalicClose => italic_depth = italic_depth.saturating_sub(1),
                    TagKind::Br => push_run(
                        "\n\n",
                        &RunContext {
                            bold_depth,
                            italic_depth,
                            state: span_stack.last(),
                        },
                        theme,
                        &mut final_text,
                        &mut runs,
                    ),
                    TagKind::SpanOpen => {
                        span_stack.push(SpanState::new(inner));
                    }
                    TagKind::SpanClose => {
                        span_stack.pop();
                    }
                    TagKind::Unknown => {
                        let entire_tag = &rest[..=tag_end];

                        scratch.clear();
                        unescape_into(entire_tag, &mut scratch);

                        push_run(
                            &scratch,
                            &RunContext {
                                bold_depth,
                                italic_depth,
                                state: span_stack.last(),
                            },
                            theme,
                            &mut final_text,
                            &mut runs,
                        );
                    }
                }
                rest = &rest[tag_end + 1..];
            } else {
                scratch.clear();
                unescape_into(rest, &mut scratch);
                push_run(
                    &scratch,
                    &RunContext {
                        bold_depth,
                        italic_depth,
                        state: span_stack.last(),
                    },
                    theme,
                    &mut final_text,
                    &mut runs,
                );
                break;
            }
        } else {
            scratch.clear();
            unescape_into(rest, &mut scratch);
            push_run(
                &scratch,
                &RunContext {
                    bold_depth,
                    italic_depth,
                    state: span_stack.last(),
                },
                theme,
                &mut final_text,
                &mut runs,
            );
            break;
        }
    }

    (final_text, runs)
}

pub fn strip_pango(content: &str) -> String {
    let mut out = String::with_capacity(content.len());
    let mut rest = content;

    while !rest.is_empty() {
        match rest.find('<') {
            Some(0) => match rest.find('>') {
                Some(end) => {
                    let inner = rest[1..end].trim();
                    if matches!(classify_tag(inner), TagKind::Br) {
                        out.push_str("\n\n");
                    }
                    rest = &rest[end + 1..];
                }
                None => {
                    unescape_into(rest, &mut out);
                    break;
                }
            },
            Some(tag_start) => {
                unescape_into(&rest[..tag_start], &mut out);
                rest = &rest[tag_start..];
            }
            None => {
                unescape_into(rest, &mut out);
                break;
            }
        }
    }
    out
}

#[inline]
fn unescape_into(s: &str, out: &mut String) {
    let bytes = s.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            out.push_str(&s[start..i]);
            let end = bytes[i..]
                .iter()
                .take(7)
                .position(|&b| b == b';')
                .map(|p| i + p)
                .unwrap_or(bytes.len().saturating_sub(1));
            match &s[i..=end] {
                "&quot;" => out.push('"'),
                "&amp;" => out.push('&'),
                "&lt;" => out.push('<'),
                "&gt;" => out.push('>'),
                "&nbsp;" => out.push(' '),
                "&apos;" => out.push('\''),
                other => out.push_str(other),
            }
            i = end + 1;
            start = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[start..]);
}

struct RunContext<'a> {
    bold_depth: usize,
    italic_depth: usize,
    state: Option<&'a SpanState>,
}

fn push_run(
    text: &str,
    ctx: &RunContext,
    theme: &std::sync::Arc<crate::app::theme::ThemeData>,
    final_text: &mut String,
    runs: &mut Vec<TextRun>,
) {
    if text.is_empty() {
        return;
    }

    let start = final_text.len();
    final_text.push_str(text);
    let len = final_text.len() - start;

    let target_color = ctx
        .state
        .and_then(|s| s.color())
        .unwrap_or(if ctx.bold_depth > 0 {
            theme.primary_text
        } else {
            theme.secondary_text
        });

    let target_weight = if ctx.bold_depth > 0 {
        FontWeight::BOLD
    } else {
        FontWeight::NORMAL
    };
    let target_style = if ctx.italic_depth > 0 {
        FontStyle::Italic
    } else {
        FontStyle::Normal
    };

    // Merge adjacent run if style AND font family are identical
    if let Some(last) = runs.last_mut() {
        let same_bold = last.font.weight == target_weight;
        let same_italic = last.font.style == target_style;
        let same_family = &last.font.family
            == ctx
                .state
                .and_then(|s| s.family())
                .unwrap_or(&theme.typography.font_family);
        let same_color = last.color == target_color;

        if same_bold && same_italic && same_family && same_color {
            last.len += len;
            return;
        }
    }

    runs.push(TextRun {
        len,
        color: target_color,
        font: gpui::Font {
            family: ctx
                .state
                .and_then(|s| s.family())
                .unwrap_or(&theme.typography.font_family)
                .clone(),
            weight: target_weight,
            style: target_style,
            ..Default::default()
        },
        ..Default::default()
    });
}

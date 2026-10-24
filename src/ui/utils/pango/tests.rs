use std::sync::Arc;

use gpui::{FontStyle, FontWeight, TextRun};

use super::*;
use crate::ui::utils::pango::parse::strip_pango;

fn dummy_theme() -> Arc<crate::app::theme::ThemeData> {
    Arc::new(crate::app::theme::ThemeData::dark())
}

fn parse(s: &str) -> (String, Vec<TextRun>) {
    parse_pango(s, &dummy_theme())
}

#[test]
fn plain_text_produces_one_run() {
    let (text, runs) = parse("hello world");
    assert_eq!(text, "hello world");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].len, 11);
    assert_eq!(runs[0].font.weight, FontWeight::NORMAL);
}

#[test]
fn bold_tag_sets_weight() {
    let (text, runs) = parse("<b>bold</b>");
    assert_eq!(text, "bold");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font.weight, FontWeight::BOLD);
}

#[test]
fn italic_tag_sets_style() {
    let (text, runs) = parse("<i>slanted</i>");
    assert_eq!(text, "slanted");
    assert_eq!(runs[0].font.style, FontStyle::Italic);
}

#[test]
fn mixed_bold_and_italic() {
    let (text, runs) = parse("<b><i>both</i></b>");
    assert_eq!(text, "both");
    assert_eq!(runs[0].font.weight, FontWeight::BOLD);
    assert_eq!(runs[0].font.style, FontStyle::Italic);
}

#[test]
fn bold_wrapping_plain_text() {
    let (text, runs) = parse("before <b>bold</b> after");
    assert_eq!(text, "before bold after");
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].font.weight, FontWeight::NORMAL);
    assert_eq!(runs[1].font.weight, FontWeight::BOLD);
    assert_eq!(runs[2].font.weight, FontWeight::NORMAL);
    // byte lengths
    assert_eq!(runs[0].len, 7); // "before "
    assert_eq!(runs[1].len, 4); // "bold"
    assert_eq!(runs[2].len, 6); // " after"
}

#[test]
fn br_tag_inserts_newline() {
    let (text, runs) = parse("line1<br/>line2");
    assert_eq!(text, "line1\n\nline2");
    let total_run_len: usize = runs.iter().map(|r| r.len).sum();
    assert_eq!(total_run_len, text.len());

    assert_eq!(runs.len(), 1);
}

#[test]
fn br_without_slash_also_works() {
    let (text, _) = parse("a<br>b");
    assert_eq!(text, "a\n\nb");
}

#[test]
fn html_entities_unescaped() {
    let (text, _) = parse("a &amp; b &lt;c&gt; &quot;d&quot;");
    assert_eq!(text, "a & b <c> \"d\"");
}

#[test]
fn nbsp_entity() {
    let (text, _) = parse("a&nbsp;b");
    assert_eq!(text, "a b");
}

#[test]
fn empty_string() {
    let (text, runs) = parse("");
    assert_eq!(text, "");
    assert!(runs.is_empty());
}

#[test]
fn unclosed_tag_treated_as_text() {
    let (text, _) = parse("hello <b world");
    assert!(text.contains("hello"));
}

#[test]
fn unknown_tag_emitted_as_literal() {
    let (text, _) = parse("hello <stan>world</stan>");
    assert!(text.contains("<stan>"));
    assert!(text.contains("world"));
    assert!(text.contains("</stan>"));
}

#[test]
fn nested_bold() {
    let (text, runs) = parse("<b>a<b>b</b>c</b>");
    assert_eq!(text, "abc");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].font.weight, FontWeight::BOLD);
}

#[test]
fn adjacent_same_style_runs_are_merged() {
    let (text, runs) = parse("hello <!-- comment --> world");
    let total_len: usize = runs.iter().map(|r| r.len).sum();
    assert_eq!(total_len, text.len());
}

#[test]
fn run_lengths_sum_to_text_length() {
    let cases = [
        "plain",
        "<b>bold</b> normal <i>italic</i>",
        "a &amp; <b>b &lt; <i>c</i></b> d",
        "<br/><br/>",
        "",
    ];
    let theme = dummy_theme();
    for case in &cases {
        let (text, runs) = parse_pango(case, &theme);
        let total: usize = runs.iter().map(|r| r.len).sum();
        assert_eq!(
            total,
            text.len(),
            "run lengths don't sum to text length for: {case:?}"
        );
    }
}

#[test]
fn span_applies_font_family() {
    let (text, runs) = parse("normal <span font_desc='Courier'>monospace</span> normal");
    assert_eq!(text, "normal monospace normal");
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[1].font.family.as_ref(), "Courier");
}

#[test]
fn nested_spans_restore_family() {
    let (_text, runs) =
        parse("<span font_desc='A'>outer <span font_desc='B'>inner</span> outer</span>");
    assert_eq!(runs.len(), 3);
    assert_eq!(runs[0].font.family.as_ref(), "A");
    assert_eq!(runs[1].font.family.as_ref(), "B");
    assert_eq!(runs[2].font.family.as_ref(), "A");
}

#[test]
fn strip_pango_comprehensive() {
    let cases = [
        ("hello world", "hello world"),
        ("<b>bold</b>", "bold"),
        ("<i>italic</i>", "italic"),
        ("a<br/>b", "a\n\nb"),
        ("a<br>b", "a\n\nb"),
        ("a<br />b", "a\n\nb"),
        ("&amp;", "&"),
        ("&lt;", "<"),
        ("&gt;", ">"),
        ("&quot;", "\""),
        ("&apos;", "'"),
        ("&nbsp;", " "),
        ("<b>a &amp; b</b>", "a & b"),
        ("<span font_desc='monospace'>code</span>", "code"),
        ("<b><i>both</i></b>", "both"),
        ("", ""),
        ("just plain text", "just plain text"),
        (
            "<b>Name</b>: foo &amp; bar<br/>second line",
            "Name: foo & bar\n\nsecond line",
        ),
    ];

    for (input, expected) in &cases {
        assert_eq!(strip_pango(input), *expected, "failed for: {input:?}");
    }
}

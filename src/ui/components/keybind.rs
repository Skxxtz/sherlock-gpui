use std::{cell::Cell, rc::Rc};

use gpui::{
    App,
    Bounds,
    ElementId,
    InteractiveElement,
    IntoElement,
    ParentElement,
    Pixels,
    SharedString,
    StatefulInteractiveElement,
    Styled,
    Window,
    canvas,
    div,
    prelude::FluentBuilder,
    px,
    relative,
};

use crate::app::theme::ThemeData;

/// Height of an editor line; the chord list shows [`VISIBLE_LINES`] of them, then scrolls.
pub const LINE_H: f32 = 36.;
pub const LINE_GAP: f32 = 2.;
pub const VISIBLE_LINES: usize = 4;

/// Key name as shown on a cap: short, lowercase words (`ctrl`, `esc`, `n`), arrows as symbols.
pub fn key_label(key: &str) -> SharedString {
    let key = key.to_lowercase();
    match key.as_str() {
        "control" => "ctrl".into(),
        "escape" => "esc".into(),
        "return" => "return".into(),
        "cmd" | "meta" | "platform" => "super".into(),
        "up" => "↑".into(),
        "down" => "↓".into(),
        "left" => "←".into(),
        "right" => "→".into(),
        "backspace" => "backspace".into(),
        _ => key.into(),
    }
}

/// The parts of a chord: `"ctrl-shift-k"` → `["ctrl", "shift", "k"]`, `"ctrl--"` → `["ctrl", "-"]`.
pub fn chord_parts(chord: &str) -> Vec<&str> {
    let (mods, key) = match chord.strip_suffix("--") {
        Some(mods) => (mods, "-"),
        None if chord == "-" => ("", "-"),
        None => chord.rsplit_once('-').unwrap_or(("", chord)),
    };
    mods.split('-')
        .filter(|m| !m.is_empty())
        .chain(std::iter::once(key))
        .collect()
}

/// A chord as readable text: `"ctrl-j"` → `"ctrl j"`.
pub fn chord_label(chord: &str) -> String {
    chord_parts(chord)
        .into_iter()
        .map(key_label)
        .collect::<Vec<_>>()
        .join(" ")
}

fn cap(text: SharedString, theme: &ThemeData) -> impl IntoElement {
    div()
        .min_w(px(20.))
        .h(px(20.))
        .px(px(5.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(4.))
        .bg(theme.bg_keybind)
        .font_family(theme.monospace.clone())
        .text_size(px(12.))
        .line_height(relative(1.0))
        .text_color(theme.primary_text)
        .child(text)
}

pub fn caps(chord: &str, theme: &ThemeData) -> impl IntoElement {
    div().flex().flex_none().gap(px(3.)).children(
        chord_parts(chord)
            .into_iter()
            .map(|part| cap(key_label(part), theme)),
    )
}

/// Records where an element is (put it inside, absolute): clicks there don't count as outside.
pub fn bounds_probe(cell: Rc<Cell<Option<Bounds<Pixels>>>>) -> impl IntoElement {
    canvas(move |bounds, _, _| cell.set(Some(bounds)), |_, _, _, _| {})
        .absolute()
        .top_0()
        .left_0()
        .size_full()
}

/// The row's control: the first chord and a `+N` badge for the others, or "Not bound". One line,
/// never wraps. `warn` marks a chord shared with another action.
pub fn keybind_summary(
    id: impl Into<ElementId>,
    chords: &[SharedString],
    open: bool,
    warn: bool,
    theme: &ThemeData,
    on_click: impl Fn(&mut Window, &mut App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .relative()
        .flex()
        .flex_none()
        .items_center()
        .gap(px(6.))
        .h(px(28.))
        .px(px(4.))
        .rounded(px(6.))
        .bg(theme.bg_app)
        .border_1()
        .cursor_pointer()
        .border_color(if warn { theme.color_warn } else { theme.border })
        .when(open, |this| this.bg(theme.bg_selected))
        .hover(|s| s.bg(theme.bg_selected))
        .map(|this| match chords.first() {
            Some(first) => this
                .child(caps(first, theme))
                .when(chords.len() > 1, |this| {
                    this.child(
                        div()
                            .px(px(5.))
                            .text_size(px(11.))
                            .text_color(theme.secondary_text)
                            .child(format!("+{}", chords.len() - 1)),
                    )
                }),
            None => this.child(
                div()
                    .px(px(4.))
                    .text_size(px(12.))
                    .text_color(theme.tertiary_text)
                    .child("Not bound"),
            ),
        })
        .on_click(move |_, window, cx| on_click(window, cx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chord_parts_split_modifiers_and_key() {
        assert_eq!(chord_parts("ctrl-shift-k"), ["ctrl", "shift", "k"]);
        assert_eq!(chord_parts("down"), ["down"]);
        assert_eq!(chord_parts("ctrl--"), ["ctrl", "-"]);
        assert_eq!(chord_parts("-"), ["-"]);
    }

    #[test]
    fn chord_label_is_readable() {
        assert_eq!(chord_label("ctrl-j"), "ctrl j");
        assert_eq!(chord_label("down"), "↓");
    }
}

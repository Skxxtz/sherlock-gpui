use gpui::{
    Context, EventEmitter, FocusHandle, Focusable, InteractiveElement, ParentElement, Render,
    SharedString, Styled, div, prelude::FluentBuilder, px,
};

use crate::{app::theme::ActiveTheme, ui::traits::HandleInputExt};

mod actions;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextEvent {
    /// The text changed
    Changed,
    /// Enter
    Submit,
    /// Cancel,
    Cancel,
}

pub struct TinyInput {
    /// The content of this input
    text: String,
    /// Byteoffset of the caret, always on a char boundary
    cursor: usize,
    /// The max length in chars.
    max_len: usize,
    /// Whic typed chars are allowed.
    filter: fn(&str, usize, char) -> bool,
    /// The placeholder text in the input.
    prefix: SharedString,
    placeholder: SharedString,
    focus: FocusHandle,
}

impl EventEmitter<TextEvent> for TinyInput {}

impl Focusable for TinyInput {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl TinyInput {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            max_len: usize::MAX,
            filter: |_, _, c| !c.is_control(),
            placeholder: SharedString::default(),
            prefix: SharedString::default(),
            focus: cx.focus_handle(),
        }
    }

    /// Sets the maxinum number of chars, and reserves room for them.
    #[inline(always)]
    pub fn max_len(mut self, max_len: usize) -> Self {
        self.max_len = max_len;
        self.text.reserve_exact(max_len);
        self
    }

    /// Sets the filter function for this input, allowing to white/blacklist chars.
    #[inline(always)]
    pub fn filter(mut self, filter: fn(&str, usize, char) -> bool) -> Self {
        self.filter = filter;
        self
    }

    /// Sets the placeholder text for this input.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn placeholder(mut self, placeholder: impl Into<SharedString>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Sets the prefix text for this input.
    #[inline(always)]
    #[allow(dead_code)]
    pub fn prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Initial text, run though the filter and `max_len` like typing.
    /// **Call it after `filter` and `max_len`.**
    #[inline]
    pub fn with_text(mut self, text: impl AsRef<str>) -> Self {
        self.text.clear();
        self.cursor = 0;
        self.insert(text.as_ref());
        self
    }

    /// Shared reference to this input's content.
    #[inline(always)]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Replaces the text without emitting.
    #[inline]
    pub fn set_text(&mut self, text: impl AsRef<str>, cx: &mut Context<Self>) {
        let text = text.as_ref();
        if self.text == text {
            if self.cursor != self.text.len() {
                self.cursor = self.text.len();
                cx.notify();
            }
            return;
        }
        self.text.clear();
        self.cursor = 0;
        self.insert(text);
        cx.notify();
    }
}

impl TinyInput {
    /// Get the previous caret position.
    #[inline(always)]
    fn prev(&self) -> usize {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i)
    }

    /// Get the next caret position.
    #[inline(always)]
    fn next(&self) -> usize {
        self.text[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |c| self.cursor + c.len_utf8())
    }

    /// Set the caret position.
    #[inline(always)]
    fn move_to(&mut self, cursor: usize, cx: &mut Context<Self>) -> bool {
        if cursor != self.cursor {
            self.cursor = cursor;
            cx.notify();
        }
        true
    }

    /// Deletes the previous char
    fn delete_back(&mut self) -> bool {
        let start = self.prev();
        self.text.replace_range(start..self.cursor, "");
        let changed = start != self.cursor;
        self.cursor = start;
        changed
    }

    /// Deletes the next char.
    fn delete_forward(&mut self) -> bool {
        let end = self.next();
        self.text.replace_range(self.cursor..end, "");
        end != self.cursor
    }

    /// Inserts the accepted chars of `typed` at the caret.
    fn insert(&mut self, typed: &str) -> bool {
        let mut changed = false;
        for c in typed.chars() {
            if self.text.chars().count() >= self.max_len {
                break;
            }
            if !(self.filter)(&self.text, self.cursor, c) {
                continue;
            }
            self.text.insert(self.cursor, c);
            self.cursor += c.len_utf8();
            changed = true;
        }
        changed
    }

    /// Whether the text input allows a char
    #[inline(always)]
    pub fn filter_accepts(&self, typed: char) -> bool {
        (self.filter)(&self.text, self.cursor, typed)
    }
}

impl Render for TinyInput {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> impl gpui::prelude::IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let focused = self.focus.is_focused(window);
        let (before, after) = self.text.split_at(self.cursor);
        let empty = self.text.is_empty();

        div()
            .track_focus(&self.focus)
            .handle_input(cx)
            .flex()
            .items_center()
            .text_color(theme.primary_text)
            .when(empty && !focused, |this| {
                this.text_color(theme.tertiary_text)
                    .child(self.placeholder.clone())
            })
            .when(!empty || focused, |this| {
                this.when(!self.prefix.is_empty(), |this| {
                    this.child(self.prefix.clone())
                })
                .child(SharedString::from(before.to_string()))
                .when(focused, |this| {
                    this.child(div().w(px(1.)).h(px(14.)).bg(theme.accent()))
                })
                .child(SharedString::from(after.to_string()))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(text: &str) -> TinyInput {
        TinyInput {
            text: text.into(),
            cursor: text.len(),
            max_len: 7,
            filter: |_, _, c| c == '#' || c.is_ascii_hexdigit(),
            placeholder: SharedString::default(),
            prefix: SharedString::default(),
            focus: test_focus(),
        }
    }

    /// A `FocusHandle` needs an app; the editing helpers don't touch it.
    fn test_focus() -> FocusHandle {
        gpui::TestAppContext::single().update(|cx| cx.focus_handle())
    }

    #[test]
    fn edits_respect_filter_max_len_and_char_boundaries() {
        let mut i = input("#ff");
        assert!(i.insert("0z0"), "z is filtered");
        assert_eq!(i.text, "#ff00");
        assert!(i.insert("aaaa"));
        assert_eq!(i.text, "#ff00aa", "capped at 7 chars");

        i.cursor = i.prev();
        assert!(i.delete_back());
        assert_eq!((i.text.as_str(), i.cursor), ("#ff00a", 5));
        assert!(i.delete_forward());
        assert_eq!(i.text, "#ff00");
        assert!(!i.delete_forward(), "nothing after the caret");

        let mut u = input("aé");
        u.filter = |_, _, _| true;
        assert_eq!(u.prev(), 1, "steps over the 2-byte é");
    }
}

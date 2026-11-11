use std::ops::Range;

use gpui::{
    ClipboardItem,
    Context,
    EntityInputHandler,
    MouseDownEvent,
    MouseMoveEvent,
    MouseUpEvent,
    Pixels,
    Point,
    SharedString,
    Window,
};
use unicode_segmentation::UnicodeSegmentation;

use super::{EmptyBackspace, TextInput};
use crate::ui::{actions::search_bar::*, views::Propagation};

/// `Continue` from an edit: let the action bubble on.
fn pass(propagation: Propagation, cx: &mut Context<TextInput>) {
    if propagation == Propagation::Continue {
        cx.propagate();
    }
}

// Editing. Each edit is a plain method returning `Propagation`, used by the main search bar's
// commands (`search_bar::commands`) and by the GPUI action listeners below (variable inputs).
impl TextInput {
    /// One left, or to the start of the selection. `Continue` at the start.
    pub(super) fn cursor_left(&mut self, cx: &mut Context<Self>) -> Propagation {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.start, cx);
            return Propagation::Stop;
        }
        let offset = self.cursor_offset();
        if offset == 0 {
            return Propagation::Continue;
        }
        self.move_to(self.previous_boundary(offset), cx);
        Propagation::Stop
    }

    /// One right, or to the end of the selection. `Continue` at the end.
    pub(super) fn cursor_right(&mut self, cx: &mut Context<Self>) -> Propagation {
        if !self.selected_range.is_empty() {
            self.move_to(self.selected_range.end, cx);
            return Propagation::Stop;
        }
        let offset = self.cursor_offset();
        if offset == self.content.len() {
            return Propagation::Continue;
        }
        self.move_to(self.next_boundary(offset), cx);
        Propagation::Stop
    }

    pub(super) fn extend_left(&mut self, cx: &mut Context<Self>) -> Propagation {
        self.select_to(self.previous_boundary(self.cursor_offset()), cx);
        Propagation::Stop
    }

    pub(super) fn extend_right(&mut self, cx: &mut Context<Self>) -> Propagation {
        self.select_to(self.next_boundary(self.cursor_offset()), cx);
        Propagation::Stop
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.move_to(0, cx);
        self.select_to(self.content.len(), cx)
    }

    pub(super) fn move_home(&mut self, cx: &mut Context<Self>) -> Propagation {
        self.move_to(0, cx);
        Propagation::Stop
    }

    pub(super) fn move_end(&mut self, cx: &mut Context<Self>) -> Propagation {
        self.move_to(self.content.len(), cx);
        Propagation::Stop
    }

    /// Deletes the selection or the character before the cursor. `Continue` when empty.
    pub(super) fn delete_back(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.content.is_empty() {
            return Propagation::Continue;
        }
        if self.selected_range.is_empty() {
            self.select_to(self.previous_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        Propagation::Stop
    }

    /// Deletes the selection or the character after the cursor.
    pub(super) fn delete_forward(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.selected_range.is_empty() {
            self.select_to(self.next_boundary(self.cursor_offset()), cx)
        }
        self.replace_text_in_range(None, "", window, cx);
        Propagation::Stop
    }

    pub(super) fn clear(&mut self, cx: &mut Context<Self>) -> Propagation {
        self.reset();
        cx.notify();
        Propagation::Stop
    }

    /// Accepts the ghost text. `Continue` without one.
    pub(super) fn accept_ghost(&mut self, cx: &mut Context<Self>) -> Propagation {
        let Some(ghost) = self.ghost_text.as_deref() else {
            return Propagation::Continue;
        };
        let new_content = format!("{}{}", self.content, ghost);
        if new_content.as_str() == self.content.as_str() {
            return Propagation::Continue;
        }
        let new_len = new_content.len();
        self.content = SharedString::from(new_content);
        self.selected_range = new_len..new_len;
        self.refresh_ghost_text();
        cx.notify();
        Propagation::Stop
    }

    /// Pastes the clipboard's text, newlines as spaces.
    pub(super) fn paste_clipboard(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.replace_text_in_range(None, &text.replace("\n", " "), window, cx);
        }
        Propagation::Stop
    }

    /// Copies the selection. `Continue` without one.
    pub(super) fn copy_selection(&mut self, cx: &mut Context<Self>) -> Propagation {
        if self.selected_range.is_empty() {
            return Propagation::Continue;
        }
        cx.write_to_clipboard(ClipboardItem::new_string(
            self.content[self.selected_range.clone()].to_string(),
        ));
        Propagation::Stop
    }

    /// Cuts the selection.
    pub(super) fn cut_selection(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.copy_selection(cx) == Propagation::Stop {
            self.replace_text_in_range(None, "", window, cx)
        }
        Propagation::Stop
    }
}

// GPUI action listeners.
impl TextInput {
    pub(super) fn left(&mut self, _: &CursorLeft, _: &mut Window, cx: &mut Context<Self>) {
        let p = self.cursor_left(cx);
        pass(p, cx);
    }

    pub(super) fn right(&mut self, _: &CursorRight, _: &mut Window, cx: &mut Context<Self>) {
        let p = self.cursor_right(cx);
        pass(p, cx);
    }

    pub(super) fn select_left(&mut self, _: &SelectLeft, _: &mut Window, cx: &mut Context<Self>) {
        self.extend_left(cx);
    }

    pub(super) fn select_right(&mut self, _: &SelectRight, _: &mut Window, cx: &mut Context<Self>) {
        self.extend_right(cx);
    }

    pub(super) fn select_all_internal(
        &mut self,
        _: &SelectAll,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.select_all(cx);
    }

    pub(super) fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_home(cx);
    }

    pub(super) fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_end(cx);
    }

    /// In an empty input: `EmptyBackspace`, the launcher turns it into `Back`.
    pub(super) fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.delete_back(window, cx) == Propagation::Continue {
            cx.emit(EmptyBackspace);
        }
    }

    pub(super) fn complete(&mut self, _: &Complete, _win: &mut Window, cx: &mut Context<Self>) {
        let p = self.accept_ghost(cx);
        pass(p, cx);
    }

    pub(super) fn delete(
        &mut self,
        _: &DeleteForward,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.delete_forward(window, cx);
    }

    pub(super) fn delete_all(
        &mut self,
        _: &DeleteAll,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.clear(cx);
    }

    pub(super) fn on_mouse_down(
        &mut self,
        event: &MouseDownEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.is_selecting = true;

        if event.modifiers.shift {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        } else {
            self.move_to(self.index_for_mouse_position(event.position), cx)
        }
    }

    pub(super) fn on_mouse_up(
        &mut self,
        _: &MouseUpEvent,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) {
        self.is_selecting = false;
    }

    pub(super) fn on_mouse_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.is_selecting {
            self.select_to(self.index_for_mouse_position(event.position), cx);
        }
    }

    pub(super) fn show_character_palette(
        &mut self,
        _: &ShowCharacterPalette,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        window.show_character_palette();
    }

    pub(super) fn paste(&mut self, _: &Paste, window: &mut Window, cx: &mut Context<Self>) {
        self.paste_clipboard(window, cx);
    }

    pub(super) fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) {
        let p = self.copy_selection(cx);
        pass(p, cx);
    }

    pub(super) fn cut(&mut self, _: &Cut, window: &mut Window, cx: &mut Context<Self>) {
        self.cut_selection(window, cx);
    }

    pub(super) fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.selected_range = offset..offset;
        cx.notify()
    }

    pub(super) fn cursor_offset(&self) -> usize {
        if self.selection_reversed {
            self.selected_range.start
        } else {
            self.selected_range.end
        }
    }

    pub(super) fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        if self.content.is_empty() {
            return 0;
        }

        let (Some(bounds), Some(line)) = (self.last_bounds.as_ref(), self.last_layout.as_ref())
        else {
            return 0;
        };
        if position.y < bounds.top() {
            return 0;
        }
        if position.y > bounds.bottom() {
            return self.content.len();
        }
        line.closest_index_for_x(position.x - bounds.left())
    }

    pub(super) fn select_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        if self.selection_reversed {
            self.selected_range.start = offset
        } else {
            self.selected_range.end = offset
        };
        if self.selected_range.end < self.selected_range.start {
            self.selection_reversed = !self.selection_reversed;
            self.selected_range = self.selected_range.end..self.selected_range.start;
        }
        cx.notify()
    }

    pub(super) fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;

        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }

        utf8_offset
    }

    pub(super) fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;

        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }

        utf16_offset
    }

    pub(super) fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    pub(super) fn range_from_utf16(&self, range_utf16: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range_utf16.start)..self.offset_from_utf16(range_utf16.end)
    }

    pub(super) fn previous_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .rev()
            .find_map(|(idx, _)| (idx < offset).then_some(idx))
            .unwrap_or(0)
    }

    pub(super) fn next_boundary(&self, offset: usize) -> usize {
        self.content
            .grapheme_indices(true)
            .find_map(|(idx, _)| (idx > offset).then_some(idx))
            .unwrap_or(self.content.len())
    }

    pub fn reset(&mut self) {
        self.content = "".into();
        self.selected_range = 0..0;
        self.selection_reversed = false;
        self.marked_range = None;
        self.last_layout = None;
        self.last_bounds = None;
        self.is_selecting = false;
    }

    pub fn set_text(&mut self, text: impl Into<SharedString>) {
        let new_content: SharedString = text.into();
        let new_len = new_content.len();

        self.content = new_content;

        self.selected_range = new_len..new_len;
        self.selection_reversed = false;

        self.marked_range = None;
        self.is_selecting = false;

        self.last_layout = None;
    }
}

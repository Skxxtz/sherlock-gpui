use gpui::{Context, KeyDownEvent, Window};

use super::{TextEvent, TinyInput};
use crate::ui::{
    actions::{
        launcher::{Execute, ExecuteInplace, ItemLeft, ItemRight, Quit},
        search_bar::{Backspace, Copy, Cut, DeleteAll, DeleteForward, End, Home, Paste},
    },
    traits::HandleActions,
    views::Propagation,
};

impl TinyInput {
    /// After an edit: tell the owner and redraw, but only if the text really changed.
    fn edited(&self, changed: bool, cx: &mut Context<Self>) -> Propagation {
        if changed {
            cx.emit(TextEvent::Changed);
            cx.notify();
        }
        Propagation::Stop
    }
    /// Inserts the clipboard text at the caret, through the filter and `max_len` like typing.
    /// Returns whether the text changed.
    fn paste_clipboard(&mut self, cx: &mut Context<Self>) -> bool {
        let pasted = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| self.insert(&text));
        if pasted {
            cx.emit(TextEvent::Changed);
            cx.notify();
        }
        pasted
    }

    fn copy_to_clipboard(&self, cx: &mut Context<Self>) {
        cx.write_to_clipboard(format!("{}{}", self.prefix, self.text).into());
    }
}

impl HandleActions for TinyInput {
    /// Nested in an owner: what the field doesn't use (↑↓, ctrl-l, …) goes to it.
    const UNHANDLED: Propagation = Propagation::Continue;

    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.copy_to_clipboard(cx);
        Propagation::Stop
    }
    fn cut(&mut self, _: &Cut, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.copy_to_clipboard(cx);
        let changed = !self.text.is_empty();
        self.text.clear();
        self.cursor = 0;
        self.edited(changed, cx)
    }
    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.paste_clipboard(cx); // emits + notifies itself
        Propagation::Stop
    }

    fn backspace(&mut self, _: &Backspace, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        let changed = self.delete_back();
        self.edited(changed, cx)
    }
    fn delete_forward(
        &mut self,
        _: &DeleteForward,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        let changed = self.delete_forward();
        self.edited(changed, cx)
    }
    fn delete_all(&mut self, _: &DeleteAll, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        let changed = !self.text.is_empty();
        self.text.clear();
        self.cursor = 0;
        self.edited(changed, cx)
    }

    // caret: `move_to` notifies when it moves
    fn item_left(&mut self, _: &ItemLeft, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_to(self.prev(), cx);
        Propagation::Stop
    }
    fn item_right(&mut self, _: &ItemRight, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_to(self.next(), cx);
        Propagation::Stop
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_to(0, cx);
        Propagation::Stop
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_to(self.text.len(), cx);
        Propagation::Stop
    }

    // ↵ / Esc: the owner decides what they mean
    fn execute(&mut self, _: &Execute, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        cx.emit(TextEvent::Submit);
        Propagation::Stop
    }
    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        cx.emit(TextEvent::Submit);
        Propagation::Stop
    }
    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        cx.emit(TextEvent::Cancel);
        Propagation::Stop
    }

    /// Typed text. Modified keys (ctrl-…, alt-…) aren't text: they go to the owner.
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        let ks = &event.keystroke;
        let m = ks.modifiers;
        if m.control || m.alt || m.platform || m.function {
            return Propagation::Continue;
        }
        match ks.key_char.as_deref() {
            Some(typed) => {
                let changed = self.insert(typed);
                self.edited(changed, cx)
            }
            None => Propagation::Continue,
        }
    }
}

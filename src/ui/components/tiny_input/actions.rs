use gpui::{Context, KeyDownEvent, Window};

use super::{TextEvent, TinyInput};
use crate::ui::{
    actions::{
        launcher::{Execute, ExecuteInplace, ItemLeft, ItemRight, Quit},
        search_bar::{Backspace, Copy, Cut, DeleteAll, DeleteForward, End, Home, Paste},
    },
    commands::{Command, ComponentCommands, Key},
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

/// Clears the text
fn clear(t: &mut TinyInput, cx: &mut Context<TinyInput>) -> Propagation {
    let changed = !t.text.is_empty();
    t.text.clear();
    t.cursor = 0;
    t.edited(changed, cx)
}

impl ComponentCommands for TinyInput {
    /// Nested in an owner: what the field doesn't use (↑↓, ctrl-l, …) goes to it.
    const UNHANDLED: Propagation = Propagation::Continue;

    const COMMANDS: &'static [Command<Self>] = &[
        Command::new(
            "input.copy",
            "copy",
            &[Key::action::<Copy>()],
            |t, _, cx| {
                t.copy_to_clipboard(cx);
                Propagation::Stop
            },
        ),
        Command::new("input.cut", "cut", &[Key::action::<Cut>()], |t, _, cx| {
            t.copy_to_clipboard(cx);
            clear(t, cx)
        }),
        Command::new(
            "input.paste",
            "paste",
            &[Key::action::<Paste>()],
            |t, _, cx| {
                t.paste_clipboard(cx); // emits + notifies itself
                Propagation::Stop
            },
        ),
        Command::new(
            "input.delete_back",
            "delete",
            &[Key::action::<Backspace>()],
            |t, _, cx| {
                let changed = t.delete_back();
                t.edited(changed, cx)
            },
        ),
        Command::new(
            "input.delete_forward",
            "delete forward",
            &[Key::action::<DeleteForward>()],
            |t, _, cx| {
                let changed = t.delete_forward();
                t.edited(changed, cx)
            },
        ),
        Command::new(
            "input.clear",
            "clear",
            &[Key::action::<DeleteAll>()],
            |t, _, cx| clear(t, cx),
        ),
        // caret: `move_to` notifies when it moves
        Command::new(
            "input.left",
            "left",
            &[Key::action::<ItemLeft>()],
            |t, _, cx| {
                t.move_to(t.prev(), cx);
                Propagation::Stop
            },
        ),
        Command::new(
            "input.right",
            "right",
            &[Key::action::<ItemRight>()],
            |t, _, cx| {
                t.move_to(t.next(), cx);
                Propagation::Stop
            },
        ),
        Command::new(
            "input.home",
            "start",
            &[Key::action::<Home>()],
            |t, _, cx| {
                t.move_to(0, cx);
                Propagation::Stop
            },
        ),
        Command::new("input.end", "end", &[Key::action::<End>()], |t, _, cx| {
            t.move_to(t.text.len(), cx);
            Propagation::Stop
        }),
        // ↵ / Esc: the owner decides what they mean
        Command::new(
            "input.submit",
            "submit",
            &[Key::action::<Execute>(), Key::action::<ExecuteInplace>()],
            |_, _, cx| {
                cx.emit(TextEvent::Submit);
                Propagation::Stop
            },
        ),
        Command::new(
            "input.cancel",
            "cancel",
            &[Key::action::<Quit>()],
            |_, _, cx| {
                cx.emit(TextEvent::Cancel);
                Propagation::Stop
            },
        ),
    ];

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

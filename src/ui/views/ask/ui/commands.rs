use gpui::{App, Context, SharedString};

use crate::ui::{
    actions::{Back, Copy, Execute, ExecuteInplace, ItemDown, ItemUp, NextVar, PrevVar, Quit},
    commands::{Command, Key},
    views::{
        Propagation,
        ask::{
            model::{focus, items::Item},
            ui::view::AskView,
        },
    },
};

const ENTER: &[Key] = &[Key::action::<Execute>()];
const CTRL_ENTER: &[Key] = &[Key::action::<ExecuteInplace>()];
const ESC: &[Key] = &[Key::action::<Quit>()];
/// `Back` is what ⌫ becomes in an empty search bar.
const BACK: &[Key] = &[Key::action::<Back>()];
const COPY: &[Key] = &[Key::action::<Copy>()];
const TAB: &[Key] = &[Key::action::<NextVar>()];
const SHIFT_TAB: &[Key] = &[Key::action::<PrevVar>()];
const UP: &[Key] = &[Key::action::<ItemUp>()];
const DOWN: &[Key] = &[Key::action::<ItemDown>()];

/// `run` for commands that always take the key.
fn done(f: impl FnOnce()) -> Propagation {
    f();
    Propagation::Stop
}

impl AskView {
    pub(super) const COMMANDS: &[Command<Self>] = &[
        // Esc
        Command {
            id: "ask.stop",
            label: "stop",
            keys: ESC,
            when: |v, _| v.is_running() || v.conv.is_live(),
            run: |v, _, cx| {
                done(|| {
                    if v.is_running() {
                        v.stop_run(cx);
                    } else {
                        v.stop(cx);
                        v.repaint(cx);
                    }
                })
            },
            hint: true,
        },
        Command {
            id: "ask.clear_selection",
            label: "clear selection",
            keys: ESC,
            when: |v, _| v.text_sel.is_some(),
            run: |v, _, cx| {
                done(|| {
                    v.text_sel = None;
                    v.repaint(cx);
                })
            },
            hint: false,
        },
        // Enter
        Command {
            id: "ask.ask",
            label: "ask",
            keys: ENTER,
            when: |v, _| !v.input.trim().is_empty(),
            run: |v, _, cx| {
                done(|| {
                    let question: SharedString = v.input.trim().to_owned().into();
                    v.ask(question, cx);
                    v.input = SharedString::default();
                    v.clear_input(cx);
                })
            },
            hint: false,
        },
        Command {
            id: "ask.open_chat",
            label: "open",
            keys: ENTER,
            when: |v, _| v.conv.shown().is_none() && v.focus.recent().is_some(),
            run: |v, _, cx| match v.focus.recent() {
                Some(i) => done(|| v.open_chat(i, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.ask_suggestion",
            label: "ask",
            keys: ENTER,
            when: |v, _| v.focus.suggestion().is_some(),
            run: |v, _, cx| match v.focus.suggestion() {
                Some(i) => done(|| v.ask_suggestion(i, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.open_terminal",
            label: "open in terminal",
            keys: ENTER,
            when: |v, _| v.unsent_failure().is_some_and(|(_, terminal)| terminal),
            run: |v, _, cx| match v.unsent_failure() {
                Some((item, _)) => done(|| v.fix_run(item, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.fix",
            label: "fix it",
            keys: ENTER,
            when: |v, _| v.unsent_failure().is_some(),
            run: |v, _, cx| match v.unsent_failure() {
                Some((item, _)) => done(|| v.fix_run(item, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.run",
            label: "run",
            keys: ENTER,
            when: |v, _| v.focus.selected_item().is_some(),
            run: |v, win, cx| match v.focus.selected_item() {
                Some(item) => done(|| v.run_item(item, false, win, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        // Ctrl+Enter
        Command {
            id: "ask.run_in_terminal",
            label: "in terminal",
            keys: CTRL_ENTER,
            when: |v, _| v.focus.item().is_some(),
            run: |v, win, cx| match v.focus.item() {
                Some(item) => done(|| v.run_item(item, true, win, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.copy_answer",
            label: "copy all",
            keys: CTRL_ENTER,
            when: |v, _| v.answer_text().is_some(),
            run: |v, _, cx| match v.answer_text() {
                Some(text) => done(|| v.copy_to_clipboard(text, "Copied answer", cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        // Ctrl+C: chat selection, focused output, selected command; else the search bar's copy.
        Command {
            id: "ask.copy_selection",
            label: "copy",
            keys: COPY,
            when: |v, _| v.selected_text().is_some(),
            run: |v, _, cx| match v.selected_text() {
                Some(text) => done(|| v.copy_to_clipboard(text, "Copied", cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.copy_output",
            label: "copy",
            keys: COPY,
            when: |v, _| v.focused_output().is_some(),
            run: |v, _, cx| match v.focused_output().map(|r| r.output.clone()) {
                Some(text) => done(|| v.copy_to_clipboard(text, "Copied output", cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.copy_command",
            label: "copy",
            keys: COPY,
            when: |v, _| v.selected_script().is_some(),
            run: |v, _, cx| match v.selected_script() {
                Some(text) => done(|| v.copy_to_clipboard(text, "Copied command", cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        // ⌫ in an empty search bar
        Command {
            id: "ask.clear_output",
            label: "clear",
            keys: BACK,
            when: |v, _| v.focus.output().is_some(),
            run: |v, _, cx| match v.focus.output() {
                Some(item) => done(|| {
                    v.clear_run(item, cx);
                    v.show_toast("Cleared output", cx);
                }),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.delete_chat",
            label: "delete",
            keys: BACK,
            when: |v, _| v.conv.shown().is_none() && v.focus.recent().is_some(),
            run: |v, _, cx| match v.focus.recent() {
                Some(chat) => done(|| v.delete_chat(chat, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        // Tab / Shift+Tab
        Command {
            id: "ask.edit_suggestion",
            label: "edit",
            keys: TAB,
            when: |v, _| v.focused_suggestion().is_some(),
            run: |v, _, cx| match v.focused_suggestion() {
                Some(text) => done(|| v.fill_input(text, cx)),
                None => Propagation::Continue,
            },
            hint: true,
        },
        Command {
            id: "ask.next",
            label: "next",
            keys: TAB,
            when: |v, _| v.input.is_empty(),
            run: |v, _, cx| done(|| v.cycle(true, cx)),
            hint: true,
        },
        Command {
            id: "ask.prev",
            label: "previous",
            keys: SHIFT_TAB,
            when: |v, _| v.input.is_empty(),
            run: |v, _, cx| done(|| v.cycle(false, cx)),
            hint: false,
        },
        // ↑ / ↓ (with text in the search bar they do nothing)
        Command {
            id: "ask.up",
            label: "up",
            keys: UP,
            when: always,
            run: |v, _, cx| done(|| v.move_up(cx)),
            hint: false,
        },
        Command {
            id: "ask.down",
            label: "down",
            keys: DOWN,
            when: always,
            run: |v, _, cx| done(|| v.move_down(cx)),
            hint: false,
        },
        // By id only (launcher binds: `"new_chat"`)
        Command {
            id: "ask.new_chat",
            label: "new",
            keys: &[],
            when: |v, _| !v.conv.is_empty(),
            run: |v, _, cx| done(|| v.new_conversation(cx)),
            hint: true,
        },
    ];

    /// The item of a focused failed run not yet sent back to the model, and whether it wants a
    /// terminal.
    fn unsent_failure(&self) -> Option<(usize, bool)> {
        self.focused_output()
            .filter(|r| r.failed() && !r.sent)
            .map(|r| (r.item, r.wants_terminal()))
    }

    /// The shown answer's text.
    fn answer_text(&self) -> Option<String> {
        self.conv
            .shown()
            .map(|e| e.text().to_owned())
            .filter(|t| !t.is_empty())
    }

    /// Selected chat text.
    fn selected_text(&self) -> Option<String> {
        self.text_sel
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.selected_text().to_owned())
    }

    /// The command of the selected script item.
    fn selected_script(&self) -> Option<String> {
        match self.items().get(self.focus.selected_item()?) {
            Some(Item::Script(script)) => Some(script.text.clone()),
            _ => None,
        }
    }

    fn move_up(&mut self, cx: &mut Context<Self>) {
        if !self.input.is_empty() {
            return;
        }
        if self.conv.shown().is_none() {
            self.set_focus(focus::recent_up(self.focus), cx);
        } else if self.focus.is_none() {
            // On a question: up to the answer above, entering at its last item.
            self.browse(false, cx);
        } else {
            // Back through this answer's stops, up to its question.
            self.cycle(false, cx);
        }
    }

    fn move_down(&mut self, cx: &mut Context<Self>) {
        if !self.input.is_empty() {
            return;
        }
        if self.conv.shown().is_none() {
            self.set_focus(focus::recent_down(self.focus, self.chats().len()), cx);
        } else {
            // Move through the answer's focus stops.
            let next = focus::down(self.focus, self.stops());
            if next == self.focus || self.stops().next().is_none() {
                self.browse(true, cx);
            } else {
                self.set_focus(next, cx);
            }
        }
    }
}

fn always(_: &AskView, _: &App) -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        let mut ids: Vec<_> = AskView::COMMANDS.iter().map(|c| c.id).collect();
        ids.sort_unstable();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len);
    }
}

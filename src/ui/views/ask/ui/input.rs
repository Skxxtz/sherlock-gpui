use gpui::{Context, Keystroke, SharedString, Window};

use crate::ui::{
    actions::{Back, Copy, Execute, ExecuteInplace, ItemDown, ItemUp, NextVar, PrevVar, Quit},
    traits::HandleActions,
    views::{
        Propagation,
        ask::{
            model::{focus, items::Item},
            ui::view::AskView,
        },
    },
};

impl HandleActions for AskView {
    const UNHANDLED: Propagation = Propagation::Continue;

    fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        let question: SharedString = self.input.trim().to_owned().into();
        if !question.is_empty() {
            self.ask(question, cx);
            self.input = SharedString::default();
            self.clear_input(cx);
        } else if self.conv.shown().is_none()
            && let Some(i) = self.focus.recent()
        {
            self.open_chat(i, cx);
        } else if let Some(i) = self.focus.suggestion() {
            self.ask_suggestion(i, cx);
        } else if let Some(item) = self
            .focused_output()
            .filter(|r| r.failed() && !r.sent)
            .map(|r| r.item)
        {
            self.fix_run(item, cx);
        } else if let Some(item) = self.focus.selected_item() {
            self.run_item(item, false, win, cx);
        }
        Propagation::Stop
    }

    /// `Back` in an empty search bar: clears a focused output, or deletes the focused recent
    /// chat; otherwise the usual back.
    fn back(&mut self, _: &Back, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if let Some(item) = self.focus.output() {
            self.clear_run(item, cx);
            self.show_toast("Cleared output", cx);
            Propagation::Stop
        } else if let Some(chat) = self.focus.recent().filter(|_| self.conv.shown().is_none()) {
            self.delete_chat(chat, cx);
            Propagation::Stop
        } else {
            Propagation::Continue
        }
    }

    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if let Some(item) = self.focus.item() {
            self.run_item(item, true, win, cx);
        } else if let Some(text) = self
            .conv
            .shown()
            .map(|e| e.text().to_owned())
            .filter(|t| !t.is_empty())
        {
            self.copy_to_clipboard(text, "Copied answer", cx);
        }
        Propagation::Stop
    }

    /// `Copy: selected text, when there is some (the search bar passes it on otherwise).
    fn copy(&mut self, _: &Copy, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if let Some(sel) = self.text_sel.as_ref().filter(|s| !s.is_empty()) {
            let text = sel.selected_text().to_owned();
            self.copy_to_clipboard(text, "Copied", cx);
            return Propagation::Stop;
        }

        if let Some(run) = self.focused_output() {
            let text = run.output.clone();
            self.copy_to_clipboard(text, "Copied output", cx);
            return Propagation::Stop;
        }

        if let Some(Item::Script(script)) = self
            .focus
            .selected_item()
            .and_then(|i| self.items().get(i).cloned())
        {
            self.copy_to_clipboard(script.text, "Copied command", cx);
            return Propagation::Stop;
        }

        Propagation::Continue
    }

    fn quit(&mut self, _: &Quit, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if self.is_running() {
            self.stop_run(cx);
        } else if self.text_sel.is_some() {
            self.text_sel = None;
            self.repaint(cx);
        } else {
            return Propagation::Continue;
        }
        Propagation::Stop
    }

    fn next_var(&mut self, _: &NextVar, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if let Some(text) = self.focused_suggestion() {
            self.fill_input(text, cx);
        } else if self.input.is_empty() {
            self.cycle(true, cx);
        }
        Propagation::Stop
    }

    fn prev_var(&mut self, _: &PrevVar, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if self.input.is_empty() {
            self.cycle(false, cx);
        }
        Propagation::Stop
    }

    fn item_up(&mut self, _: &ItemUp, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if !self.input.is_empty() {
            return Propagation::Stop;
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
        Propagation::Stop
    }

    fn item_down(&mut self, _: &ItemDown, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if !self.input.is_empty() {
            return Propagation::Stop;
        }
        if self.conv.shown().is_none() {
            self.set_focus(focus::recent_down(self.focus, self.chats().len()), cx);
        } else {
            // Through the shown answer's stops; past the last one, down to the next answer.
            let next = focus::down(self.focus, self.stops());
            if next == self.focus || self.stops().next().is_none() {
                self.browse(true, cx);
            } else {
                self.set_focus(next, cx);
            }
        }
        Propagation::Stop
    }

    fn bind(&mut self, key: &Keystroke, _win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        let Some(callback) = self
            .binds
            .as_deref()
            .and_then(|b| b.iter().find(|b| b.matches(key)))
            .map(|b| b.callback.clone())
        else {
            return Propagation::Continue;
        };

        let mut propagate = false;
        for action in callback.split(';').map(str::trim) {
            match action {
                "stop" if self.is_running() => self.stop_run(cx),
                "stop" if self.conv.is_live() => {
                    self.stop(cx);
                    self.repaint(cx);
                }
                "new_chat" if !self.conv.is_empty() => self.new_conversation(cx),
                "continue" => propagate = true,
                _ => {}
            }
        }
        if propagate {
            Propagation::Continue
        } else {
            Propagation::Stop
        }
    }
}

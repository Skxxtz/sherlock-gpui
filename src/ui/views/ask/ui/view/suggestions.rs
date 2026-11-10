use super::*;
use crate::ui::views::ask::model::items::suggestions;

impl AskView {
    /// Follow up questions as suggested by the model.
    pub(in super::super) fn suggestions(&self) -> Vec<String> {
        if self.conv.viewing.is_some() {
            return Vec::new();
        }
        match self.conv.current.as_ref().filter(|e| !e.phase.is_live()) {
            Some(ex) => suggestions(&ex.blocks()),
            None => Vec::new(),
        }
    }

    /// The suggestions with the keyboard focus.
    pub(in super::super) fn focused_suggestion(&self) -> Option<String> {
        let i = self.focus.suggestion()?;
        self.suggestions().into_iter().nth(i)
    }

    /// Asks sugggestion `i` right away.
    pub(in super::super) fn ask_suggestion(&mut self, i: usize, cx: &mut Context<Self>) {
        if let Some(text) = self.suggestions().into_iter().nth(i) {
            self.ask(text.into(), cx);
            self.input = SharedString::default();
            self.clear_input(cx);
        }
    }

    /// Puts `text` in the search bar, cursor at the end, to edit before asking.
    pub(in super::super) fn fill_input(&mut self, text: String, cx: &mut Context<Self>) {
        self.focus = Focus::None;
        self.input = text.clone().into();
        if let Some(shell) = self.shell.clone() {
            cx.defer(move |cx| {
                let _ = shell.update(cx, |shell, cx| {
                    shell.text_input.update(cx, |input, cx| {
                        input.set_text(text);
                        cx.notify();
                    });
                    cx.notify();
                });
            });
        }
        self.repaint(cx);
    }
}

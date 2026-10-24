use gpui::{Context, Window};

use crate::ui::{
    components::{
        choice::Choice,
        search_bar::{EmptyBackspace, actions::Backspace},
    },
    launcher::{SelectionDown, SelectionUp},
};

impl Choice {
    pub fn on_selection_up(
        &mut self,
        _ev: &SelectionUp,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.saturating_prev();
        cx.notify();
        cx.stop_propagation();
    }

    pub fn on_selection_down(
        &mut self,
        _ev: &SelectionDown,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.saturating_next();
        cx.notify();
        cx.stop_propagation();
    }

    pub fn on_backspace(&mut self, _ev: &Backspace, _win: &mut Window, cx: &mut Context<Self>) {
        cx.emit(EmptyBackspace);
    }
}

impl Choice {
    pub fn saturating_next(&mut self) {
        if self.options.is_empty() {
            self.selected = None;
            return;
        }

        self.selected = Some(match self.selected {
            None => 0,
            Some(idx) => (idx + 1).min(self.options.len().saturating_sub(1)),
        });
    }

    pub fn saturating_prev(&mut self) {
        if self.options.is_empty() {
            self.selected = None;
            return;
        }

        self.selected = match self.selected {
            Some(idx) if idx > 0 => Some(idx - 1),
            _ => None,
        };
    }
}

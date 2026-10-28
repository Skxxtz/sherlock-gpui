use gpui::{Context, Window};

use super::SettingsView;
use crate::ui::{actions::*, traits::HandleActions, views::Propagation};

/// The action was handled: redraw, skip the launcher's default.
fn handled(cx: &mut Context<SettingsView>) -> Propagation {
    cx.notify();
    Propagation::Stop
}

impl HandleActions for SettingsView {
    const UNHANDLED: Propagation = Propagation::Continue;

    fn item_up(&mut self, _: &ItemUp, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.select_row(self.row.saturating_sub(1));
        handled(cx)
    }

    fn item_down(&mut self, _: &ItemDown, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.select_row(self.row + 1);
        handled(cx)
    }

    fn item_left(&mut self, _: &ItemLeft, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if self.searching() {
            return Self::UNHANDLED;
        }
        self.select_section(self.active.saturating_sub(1));
        handled(cx)
    }

    fn item_right(&mut self, _: &ItemRight, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if self.searching() {
            return Self::UNHANDLED;
        }
        self.select_section(self.active + 1);
        handled(cx)
    }

    fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.activate(win, cx);
        handled(cx)
    }

    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.activate(win, cx);
        handled(cx)
    }
}

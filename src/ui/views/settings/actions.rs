use gpui::{Context, Window};

use super::SettingsView;
use crate::ui::{actions::*, traits::HandleActions, views::Propagation};

/// Notifes context of changes and stops propagation.
#[inline]
fn handled(cx: &mut Context<SettingsView>) -> Propagation {
    cx.notify();
    Propagation::Stop
}

impl HandleActions for SettingsView {
    const UNHANDLED: Propagation = Propagation::Continue;
    fn item_up(
        &mut self,
        _action: &ItemUp,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.select_row(self.row.saturating_sub(1));
        handled(cx)
    }
    fn item_down(
        &mut self,
        _action: &ItemDown,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.select_row(self.row.strict_add(1));
        handled(cx)
    }
    fn item_left(
        &mut self,
        _action: &ItemLeft,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.searching() {
            return Self::UNHANDLED;
        }
        self.select_section(self.active.saturating_sub(1));
        handled(cx)
    }
    fn item_right(
        &mut self,
        _action: &ItemRight,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        if self.searching() {
            return Self::UNHANDLED;
        }
        self.select_section(self.active.saturating_add(1));
        handled(cx)
    }
    fn execute(
        &mut self,
        _action: &Execute,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.handle_execute(win, cx)
    }
    fn execute_inplace(
        &mut self,
        _action: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.handle_execute(win, cx)
    }
}

impl SettingsView {
    fn handle_execute(&mut self, win: &mut Window, cx: &mut Context<SettingsView>) -> Propagation {
        let at = self.visible().get(self.row).copied();
        if let Some(item) = at.and_then(|at| self.keymaps.get(&at)) {
            item.update(cx, |item, cx| item.open(win, cx));
        } else if let Some(item) = at.and_then(|at| self.colors.get(&at)) {
            item.update(cx, |item, cx| item.open(win, cx));
        } else if let Some(select) = at.and_then(|at| self.selects.get(&at)) {
            select.update(cx, |select, cx| select.open(win, cx));
        } else {
            self.activate();
        }
        handled(cx)
    }
}

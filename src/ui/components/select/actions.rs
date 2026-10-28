use gpui::{Context, KeyDownEvent, Window};

use crate::ui::{
    actions::{End, Execute, ExecuteInplace, Home, ItemDown, ItemUp, Quit},
    traits::HandleActions,
    views::Propagation,
};

use super::Select;

impl HandleActions for Select {
    const UNHANDLED: Propagation = Propagation::Stop;

    fn item_up(&mut self, _: &ItemUp, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_cursor(self.cursor.saturating_sub(1), cx)
    }
    fn item_down(&mut self, _: &ItemDown, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_cursor(self.cursor + 1, cx)
    }
    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_cursor(0, cx)
    }
    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.move_cursor(usize::MAX, cx)
    }
    fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.choose(self.cursor, win, cx);
        self.close(win, cx);
        Propagation::Stop
    }
    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.choose(self.cursor, win, cx);
        Propagation::Stop
    }
    fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.close(win, cx);
        Propagation::Stop
    }
    /// Type-ahead: a letter jumps to the next option starting with it.
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        let found = (!event.keystroke.modifiers.modified())
            .then(|| event.keystroke.key_char.as_deref()?.chars().next())
            .flatten()
            .and_then(|c| self.find_next(c));
        if let Some(i) = found {
            self.move_cursor(i, cx);
        }
        Propagation::Stop
    }
}

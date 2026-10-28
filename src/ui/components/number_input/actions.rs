use gpui::{Context, KeyDownEvent, Window};

use crate::ui::{actions::*, traits::HandleActions, views::Propagation};

impl HandleActions for super::NumberInput {
    const UNHANDLED: Propagation = Propagation::Stop;

    fn item_up(&mut self, _: &ItemUp, window: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.step_by(1, window, cx);
        Propagation::Stop
    }
    fn item_right(
        &mut self,
        _: &ItemRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.step_by(1, window, cx);
        Propagation::Stop
    }
    fn item_down(
        &mut self,
        _: &ItemDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.step_by(-1, window, cx);
        Propagation::Stop
    }
    fn item_left(
        &mut self,
        _: &ItemLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.step_by(-1, window, cx);
        Propagation::Stop
    }

    fn execute(&mut self, _: &Execute, window: &mut Window, cx: &mut Context<Self>) -> Propagation {
        self.leave(window, cx);
        Propagation::Stop
    }
    fn execute_inplace(
        &mut self,
        _: &ExecuteInplace,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        self.leave(window, cx);
        Propagation::Stop
    }
    fn quit(&mut self, _: &Quit, window: &mut Window, cx: &mut Context<Self>) -> Propagation {
        if let Some(entry) = self.entry {
            self.set(entry, cx);
        }
        self.leave(window, cx);
        Propagation::Stop
    }

    /// A digit (or `-`) starts typing with that key; the field takes it from there.
    fn key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Propagation {
        let ks = &event.keystroke;
        let starts = !ks.modifiers.modified()
            && ks
                .key_char
                .as_deref()
                .is_some_and(|t| t.chars().all(|c| c.is_ascii_digit() || c == '-'));
        if starts && self.typing.is_none() {
            self.start_typing(true, window, cx);
            self.field.update(cx, |field, cx| {
                HandleActions::key_down(field, event, window, cx)
            });
        }
        Propagation::Stop
    }
}

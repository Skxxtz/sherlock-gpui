use gpui::{Context, KeyDownEvent, Window};

use super::NumberInput;
use crate::ui::{
    actions::*,
    commands::{Command, ComponentCommands, Key},
    views::Propagation,
};

fn step(
    n: &mut NumberInput,
    by: i64,
    win: &mut Window,
    cx: &mut Context<NumberInput>,
) -> Propagation {
    n.step_by(by, win, cx);
    Propagation::Stop
}

impl ComponentCommands for NumberInput {
    const UNHANDLED: Propagation = Propagation::Stop;

    const COMMANDS: &'static [Command<Self>] = &[
        Command::new(
            "number.increment",
            "+1",
            &[Key::action::<ItemUp>(), Key::action::<ItemRight>()],
            |n, win, cx| step(n, 1, win, cx),
        ),
        Command::new(
            "number.decrement",
            "-1",
            &[Key::action::<ItemDown>(), Key::action::<ItemLeft>()],
            |n, win, cx| step(n, -1, win, cx),
        ),
        Command::new(
            "number.done",
            "done",
            &[Key::action::<Execute>(), Key::action::<ExecuteInplace>()],
            |n, win, cx| {
                n.leave(win, cx);
                Propagation::Stop
            },
        ),
        Command::new(
            "number.cancel",
            "cancel",
            &[Key::action::<Quit>()],
            |n, win, cx| {
                if let Some(entry) = n.entry {
                    n.set(entry, cx);
                }
                n.leave(win, cx);
                Propagation::Stop
            },
        ),
    ];

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
                ComponentCommands::key_down(field, event, window, cx)
            });
        }
        Propagation::Stop
    }
}

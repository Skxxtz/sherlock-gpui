use gpui::{Context, KeyDownEvent, Window};

use super::Select;
use crate::ui::{
    actions::{End, Execute, ExecuteInplace, Home, ItemDown, ItemUp, Quit},
    commands::{Command, ComponentCommands, Key},
    views::Propagation,
};

impl ComponentCommands for Select {
    /// Open, it owns the keyboard.
    const UNHANDLED: Propagation = Propagation::Stop;

    const COMMANDS: &'static [Command<Self>] = &[
        Command::new("select.up", "up", &[Key::action::<ItemUp>()], |s, _, cx| {
            s.move_cursor(s.cursor.saturating_sub(1), cx)
        }),
        Command::new(
            "select.down",
            "down",
            &[Key::action::<ItemDown>()],
            |s, _, cx| s.move_cursor(s.cursor + 1, cx),
        ),
        Command::new(
            "select.first",
            "first",
            &[Key::action::<Home>()],
            |s, _, cx| s.move_cursor(0, cx),
        ),
        Command::new(
            "select.last",
            "last",
            &[Key::action::<End>()],
            |s, _, cx| s.move_cursor(usize::MAX, cx),
        ),
        Command::new(
            "select.choose",
            "choose",
            &[Key::action::<Execute>()],
            |s, win, cx| {
                s.choose(s.cursor, win, cx);
                s.close(win, cx);
                Propagation::Stop
            },
        ),
        Command::new(
            "select.preview",
            "choose, keep open",
            &[Key::action::<ExecuteInplace>()],
            |s, win, cx| {
                s.choose(s.cursor, win, cx);
                Propagation::Stop
            },
        ),
        Command::new(
            "select.close",
            "close",
            &[Key::action::<Quit>()],
            |s, win, cx| {
                s.close(win, cx);
                Propagation::Stop
            },
        ),
    ];

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

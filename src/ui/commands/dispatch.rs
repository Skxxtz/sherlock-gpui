use gpui::{App, Context, Keystroke, Window};

use super::{Command, Hint};
use crate::ui::views::Propagation;

/// Runs the first available command of one layer bound to `key`. `Continue`: none took it.
pub fn dispatch_key<V: 'static>(
    commands: impl IntoIterator<Item = &'static Command<V>>,
    this: &mut V,
    key: &Keystroke,
    win: &mut Window,
    cx: &mut Context<V>,
) -> Propagation {
    for cmd in commands {
        if cmd.matches(key) && (cmd.when)(this, cx) && (cmd.run)(this, win, cx) == Propagation::Stop
        {
            return Propagation::Stop;
        }
    }
    Propagation::Continue
}

/// Runs the command `id` of one layer if it's available. `Continue`: unknown, unavailable or
/// declined.
pub fn run_id<V: 'static>(
    commands: impl IntoIterator<Item = &'static Command<V>>,
    this: &mut V,
    id: &str,
    win: &mut Window,
    cx: &mut Context<V>,
) -> Propagation {
    match commands.into_iter().find(|c| c.id == id) {
        Some(cmd) if (cmd.when)(this, cx) => (cmd.run)(this, win, cx),
        _ => Propagation::Continue,
    }
}

/// The layer's available commands with `hint: true`, in order.
pub fn hints<V: 'static>(
    commands: impl IntoIterator<Item = &'static Command<V>>,
    this: &V,
    cx: &App,
) -> Vec<Hint> {
    commands
        .into_iter()
        .filter(|c| c.hint && (c.when)(this, cx))
        .map(|c| Hint {
            id: c.id,
            label: c.label.into(),
            keys: c.keystrokes().collect(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui::{IntoElement, Render, TestAppContext, VisualTestContext, div};

    use super::*;
    use crate::ui::commands::always;

    #[derive(Default)]
    struct V {
        armed: bool,
        log: Vec<&'static str>,
    }

    impl Render for V {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            div()
        }
    }

    const CMDS: &[Command<V>] = &[
        Command {
            id: "t.armed",
            label: "Armed",
            keys: &["ctrl-c"],
            when: |v, _| v.armed,
            run: |v, _, _| {
                v.log.push("armed");
                Propagation::Stop
            },
            hint: true,
        },
        Command {
            id: "t.declines",
            label: "Declines",
            keys: &["ctrl-c"],
            when: always,
            run: |v, _, _| {
                v.log.push("declines");
                Propagation::Continue
            },
            hint: false,
        },
        Command {
            id: "t.fallback",
            label: "Fallback",
            keys: &["ctrl-c", "backspace"],
            when: always,
            run: |v, _, _| {
                v.log.push("fallback");
                Propagation::Stop
            },
            hint: true,
        },
    ];

    #[gpui::test]
    fn first_available_wins_and_continue_falls_through(cx: &mut TestAppContext) {
        let (view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, _| V::default());
        let ctrl_c = Keystroke::parse("ctrl-c").unwrap();
        let ctrl_x = Keystroke::parse("ctrl-x").unwrap();

        cx.update(|win, cx| {
            view.update(cx, |v, cx| {
                // not armed: `declines` runs but passes on, `fallback` takes it
                assert_eq!(dispatch_key(CMDS, v, &ctrl_c, win, cx), Propagation::Stop);
                assert_eq!(v.log, ["declines", "fallback"]);
                assert_eq!(hints(CMDS, v, cx).len(), 1);

                v.log.clear();
                v.armed = true;
                dispatch_key(CMDS, v, &ctrl_c, win, cx);
                assert_eq!(v.log, ["armed"]);
                assert_eq!(hints(CMDS, v, cx).len(), 2);

                assert_eq!(
                    dispatch_key(CMDS, v, &ctrl_x, win, cx),
                    Propagation::Continue
                );
                assert_eq!(run_id(CMDS, v, "t.nope", win, cx), Propagation::Continue);
                v.armed = false;
                assert_eq!(run_id(CMDS, v, "t.armed", win, cx), Propagation::Continue);
            })
        });
    }
}

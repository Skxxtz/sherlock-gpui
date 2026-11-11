use gpui::{App, Context, Window};

use super::{Command, Hint, Key, Pressed};
use crate::ui::views::Propagation;

/// Runs the first available command of one layer bound to what was pressed. `Continue`: none
/// took it.
pub fn dispatch<V: 'static>(
    commands: impl IntoIterator<Item = &'static Command<V>>,
    this: &mut V,
    pressed: &Pressed,
    win: &mut Window,
    cx: &mut Context<V>,
) -> Propagation {
    for cmd in commands {
        if cmd.matches(pressed)
            && (cmd.when)(this, cx)
            && (cmd.run)(this, win, cx) == Propagation::Stop
        {
            return Propagation::Stop;
        }
    }
    Propagation::Continue
}

/// Runs the command `id` of one layer if it's available (`Command::named`). `Continue`: unknown,
/// unavailable or declined.
pub fn run_id<V: 'static>(
    commands: impl IntoIterator<Item = &'static Command<V>>,
    this: &mut V,
    id: &str,
    win: &mut Window,
    cx: &mut Context<V>,
) -> Propagation {
    match commands.into_iter().find(|c| c.named(id)) {
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
    let mut taken: Vec<Key> = Vec::new();
    let mut hints = Vec::new();
    for cmd in commands.into_iter().filter(|c| (c.when)(this, cx)) {
        let reachable = cmd.keys.is_empty() || cmd.keys.iter().any(|k| !taken.contains(k));
        if cmd.hint && reachable {
            hints.push(Hint {
                id: cmd.id,
                label: cmd.label.into(),
                keys: cmd.keys,
            });
        }
        taken.extend(cmd.keys.iter().copied());
    }
    hints
}

#[cfg(test)]
mod tests {
    use gpui::{IntoElement, Keystroke, Render, TestAppContext, VisualTestContext, div};

    use super::*;
    use crate::ui::commands::{Key, always};

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
            keys: &[Key::Stroke("ctrl-c")],
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
            keys: &[Key::Stroke("ctrl-c")],
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
            keys: &[Key::Stroke("ctrl-c"), Key::Stroke("backspace")],
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
        let stroke = |s| Pressed {
            key: Some(Keystroke::parse(s).unwrap()),
            action: None,
        };
        let (ctrl_c, ctrl_x) = (stroke("ctrl-c"), stroke("ctrl-x"));

        cx.update(|win, cx| {
            view.update(cx, |v, cx| {
                // not armed: `declines` runs but passes on, `fallback` takes it
                assert_eq!(dispatch(CMDS, v, &ctrl_c, win, cx), Propagation::Stop);
                assert_eq!(v.log, ["declines", "fallback"]);
                assert_eq!(hints(CMDS, v, cx).len(), 1);

                v.log.clear();
                v.armed = true;
                dispatch(CMDS, v, &ctrl_c, win, cx);
                assert_eq!(v.log, ["armed"]);
                assert_eq!(hints(CMDS, v, cx).len(), 2);

                assert_eq!(dispatch(CMDS, v, &ctrl_x, win, cx), Propagation::Continue);
                assert_eq!(run_id(CMDS, v, "t.nope", win, cx), Propagation::Continue);
                v.armed = false;
                assert_eq!(run_id(CMDS, v, "t.armed", win, cx), Propagation::Continue);
            })
        });
    }

    #[gpui::test]
    fn action_keys_follow_the_keymap(cx: &mut TestAppContext) {
        use crate::ui::actions::{Execute, Quit};

        cx.update(|cx| cx.bind_keys([gpui::KeyBinding::new("enter", Execute, None)]));
        let (_view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, _| V::default());
        let enter = Key::action::<Execute>();

        cx.update(|win, cx| {
            let pressed = Pressed::key(&Keystroke::parse("enter").unwrap(), win, cx);
            assert!(enter.matches(&pressed));
            assert!(!Key::action::<Quit>().matches(&pressed));
            assert!(!enter.matches(&Pressed::key(&Keystroke::parse("escape").unwrap(), win, cx)));
        });
        assert!(enter.matches(&Pressed::action(&Execute)));
        assert!(!enter.matches(&Pressed::action(&Quit)));
    }

    #[gpui::test]
    fn hints_skip_commands_their_keys_dont_reach(cx: &mut TestAppContext) {
        const SHADOWED: &[Command<V>] = &[
            Command {
                id: "t.first",
                label: "first",
                keys: &[Key::Stroke("enter")],
                when: always,
                run: |_, _, _| Propagation::Stop,
                hint: false,
            },
            Command {
                id: "t.second",
                label: "second",
                keys: &[Key::Stroke("enter")],
                when: always,
                run: |_, _, _| Propagation::Stop,
                hint: true,
            },
            Command {
                id: "t.bound",
                label: "bound",
                keys: &[],
                when: always,
                run: |_, _, _| Propagation::Stop,
                hint: true,
            },
        ];
        let (view, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, _| V::default());
        cx.update(|_, cx| {
            let ids: Vec<_> = hints(SHADOWED, view.read(cx), cx)
                .into_iter()
                .map(|h| h.id)
                .collect();
            assert_eq!(ids, ["t.bound"]);
        });
    }
}

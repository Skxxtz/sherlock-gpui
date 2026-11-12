use gpui::{Action, Context, InteractiveElement, KeyDownEvent, Window};

use super::{Command, Pressed, dispatch};
use crate::ui::{actions::*, views::Propagation};

/// A focusable widget whose keys are commands while it has focus.
pub trait ComponentCommands: Sized + 'static {
    const COMMANDS: &'static [Command<Self>];

    /// An action no command took: `Continue` bubbles it to the owner and the launcher, `Stop`
    /// swallows it (a popup that owns the keyboard while open). Key ups follow it too.
    const UNHANDLED: Propagation;

    /// A key down no action took (typed text, which isn't a command). Default: `UNHANDLED`.
    fn key_down(
        &mut self,
        _event: &KeyDownEvent,
        _win: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Propagation {
        Self::UNHANDLED
    }
}

/// One action on a focused component: its first available command decides (`Continue` from
/// it bubbles); none: `UNHANDLED`.
fn on_action<T: ComponentCommands>(
    this: &mut T,
    action: &dyn Action,
    win: &mut Window,
    cx: &mut Context<T>,
) {
    let pressed = Pressed::action(action);
    let available = T::COMMANDS
        .iter()
        .any(|c| c.matches(&pressed) && (c.when)(this, cx));
    let propagation = if available {
        dispatch(T::COMMANDS, this, &pressed, win, cx)
    } else {
        T::UNHANDLED
    };
    if propagation == Propagation::Continue {
        cx.propagate();
    }
}

macro_rules! component_listeners {
    ($($action:ty),* $(,)?) => {
        /// Puts `T`'s commands (and raw key downs) on `element`, the one that tracks the
        /// component's focus. Key ups follow `T::UNHANDLED`.
        pub fn handle_commands<T: ComponentCommands, E: InteractiveElement>(
            element: E,
            cx: &mut Context<T>,
        ) -> E {
            element
                $(
                    .on_action(cx.listener(|this, action: &$action, win, cx| {
                        on_action(this, action, win, cx)
                    }))
                )*
                .on_key_down(cx.listener(|this, event: &KeyDownEvent, win, cx| {
                    if this.key_down(event, win, cx) == Propagation::Stop {
                        cx.stop_propagation();
                    }
                }))
                .on_key_up(|_, _, cx| {
                    if T::UNHANDLED == Propagation::Stop {
                        cx.stop_propagation()
                    }
                })
        }
    };
}

// Every action `app::bindings::register_bindings` can bind. One missing here never reaches a
// focused component's commands: it goes straight past it to the launcher.
component_listeners! {
    ItemUp,
    ItemDown,
    ItemLeft,
    ItemRight,
    Execute,
    ExecuteInplace,
    Back,
    Quit,
    NextVar,
    PrevVar,
    ToggleContext,
    ShortcutAction,
    Complete,
    Backspace,
    DeleteForward,
    DeleteAll,
    SelectAll,
    Home,
    End,
    Paste,
    Copy,
    Cut,
}

/// `.handle_commands(cx)` on any element.
pub trait HandleCommandsExt: InteractiveElement + Sized {
    fn handle_commands<T: ComponentCommands>(self, cx: &mut Context<T>) -> Self {
        handle_commands(self, cx)
    }
}

impl<E: InteractiveElement> HandleCommandsExt for E {}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use gpui::{
        AppContext,
        Entity,
        FocusHandle,
        IntoElement,
        ParentElement,
        Render,
        TestAppContext,
        VisualTestContext,
        div,
    };

    use super::*;
    use crate::ui::commands::{Key, always};

    /// Lets ↑ through, takes ←, swallows everything else.
    struct Child {
        focus: FocusHandle,
        lefts: usize,
    }

    impl ComponentCommands for Child {
        const UNHANDLED: Propagation = Propagation::Stop;
        const COMMANDS: &'static [Command<Self>] = &[
            Command {
                id: "child.up",
                label: "up",
                keys: &[Key::action::<ItemUp>()],
                when: always,
                run: |_, _, _| Propagation::Continue,
                hint: false,
            },
            Command {
                id: "child.left",
                label: "left",
                keys: &[Key::action::<ItemLeft>()],
                when: always,
                run: |c, _, _| {
                    c.lefts += 1;
                    Propagation::Stop
                },
                hint: false,
            },
        ];
    }

    impl Render for Child {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            div().track_focus(&self.focus).handle_commands(cx)
        }
    }

    /// The launcher stand-in: counts what reaches it.
    struct Parent {
        child: Entity<Child>,
        reached: Rc<Cell<usize>>,
    }

    impl Render for Parent {
        fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
            let (up, down, left) = (
                self.reached.clone(),
                self.reached.clone(),
                self.reached.clone(),
            );
            div()
                .on_action(move |_: &ItemUp, _, _| up.set(up.get() + 1))
                .on_action(move |_: &ItemDown, _, _| down.set(down.get() + 100))
                .on_action(move |_: &ItemLeft, _, _| left.set(left.get() + 10_000))
                .child(self.child.clone())
        }
    }

    #[gpui::test]
    fn swallows_by_default_and_bubbles_on_continue(cx: &mut TestAppContext) {
        let reached = Rc::new(Cell::new(0));
        let (parent, cx): (_, &mut VisualTestContext) = cx.add_window_view(|_, cx| Parent {
            child: cx.new(|cx| Child {
                focus: cx.focus_handle(),
                lefts: 0,
            }),
            reached: reached.clone(),
        });
        let child = parent.read_with(cx, |p, _| p.child.clone());
        cx.update(|win, cx| child.read(cx).focus.clone().focus(win, cx));
        cx.run_until_parked();

        cx.dispatch_action(ItemUp);
        cx.dispatch_action(ItemDown);
        cx.dispatch_action(ItemLeft);

        assert_eq!(reached.get(), 1, "only ↑ (Continue) reaches the parent");
        assert_eq!(
            child.read_with(cx, |c, _| c.lefts),
            1,
            "← handled by the child"
        );
    }
}

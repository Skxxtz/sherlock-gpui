#![allow(dead_code)]

use gpui::{Context, InteractiveElement, KeyDownEvent, Window};

use crate::ui::{actions::*, views::Propagation};

/// Generates `HandleActions` (a default method per action) and `handle_input`
/// (registers every action on an element and routes it to those methods).
macro_rules! input_actions {
    ($($action:ty => $method:ident),* $(,)?) => {
        pub trait HandleActions: Sized + 'static {
            /// The default propagation that shuold be returned if an action isn't overwritten.
            const UNHANDLED: Propagation;

            $(
                #[doc = concat!("`", stringify!($action), "`. Default: `Self::UNHANDLED`.")]
                fn $method(
                    &mut self,
                    _action: &$action,
                    _win: &mut Window,
                    _cx: &mut Context<Self>,
                ) -> Propagation {
                    Self::UNHANDLED
                }
            )*

            /// A key down no action took (typed text). Default: swallowed, so the launcher's
            /// raw key listeners don't see it either.
            fn key_down(
                &mut self,
                _event: &KeyDownEvent,
                _win: &mut Window,
                _cx: &mut Context<Self>,
            ) -> Propagation {
                Self::UNHANDLED
            }
        }

        /// An input action that knows which `HandleActions` method handles it.
        pub trait InputAction: 'static {
            fn route<T: HandleActions>(&self, this: &mut T, window: &mut Window, cx: &mut Context<T>)
                -> Propagation;
        }

        $(
            impl InputAction for $action {
                #[inline]
                fn route<T: HandleActions>(&self, this: &mut T, window: &mut Window, cx: &mut Context<T>)
                    -> Propagation
                {
                    this.$method(self, window, cx)
                }
            }
        )*

        /// Routes every input action (and raw key downs) on `element` to `T`'s
        /// [`HandleActions`] methods; key ups are swallowed. Call it on the element that tracks
        /// the component's focus.
        pub fn handle_input<T: HandleActions, E: InteractiveElement>(
            element: E,
            cx: &mut Context<T>,
        ) -> E {
            element
                $(
                    .on_action(cx.listener(|this, action: &$action, win, cx| {
                        if this.$method(action, win, cx) == Propagation::Continue {
                            cx.propagate();
                        }
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

/// `.handle_input(cx)` on any element.
pub trait HandleInputExt: InteractiveElement + Sized {
    fn handle_input<T: HandleActions>(self, cx: &mut Context<T>) -> Self {
        handle_input(self, cx)
    }
}

impl<E: InteractiveElement> HandleInputExt for E {}

// Every action `app::bindings::register_bindings` can bind. A bound action missing here
// reaches the launcher from inside a focused component.
input_actions! {
    ItemUp => item_up,
    ItemDown => item_down,
    ItemLeft => item_left,
    ItemRight => item_right,
    Execute => execute,
    ExecuteInplace => execute_inplace,
    Back => back,
    Quit => quit,
    NextVar => next_var,
    PrevVar => prev_var,
    OpenContext => open_context,
    ShortcutAction => shortcut,
    Complete => complete,
    Backspace => backspace,
    DeleteForward => delete_forward,
    DeleteAll => delete_all,
    SelectAll => select_all,
    Home => home,
    End => end,
    Paste => paste,
    Copy => copy,
    Cut => cut,
}

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

    /// Lets ↑ through, takes ←, swallows everything else.
    struct Child {
        focus: FocusHandle,
        lefts: usize,
    }

    impl HandleActions for Child {
        const UNHANDLED: Propagation = Propagation::Stop;
        fn item_up(&mut self, _: &ItemUp, _: &mut Window, _: &mut Context<Self>) -> Propagation {
            Propagation::Continue
        }
        fn item_left(
            &mut self,
            _: &ItemLeft,
            _: &mut Window,
            _: &mut Context<Self>,
        ) -> Propagation {
            self.lefts += 1;
            Propagation::Stop
        }
    }

    impl Render for Child {
        fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
            handle_input(div().track_focus(&self.focus), cx)
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

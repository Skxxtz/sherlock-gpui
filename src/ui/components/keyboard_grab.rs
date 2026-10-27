use std::any::Any;

use gpui::{Context, Keystroke, Subscription, Window};

/// Hands every key down to `on_key` with the action it is bound to (resolved through the
/// keymap, so user binds apply). `on_key` returns whether it took the key; a taken key goes
/// nowhere else. Drop the subscription to release the keyboard.
///
/// Return `false` for keys a focused child should still get, e.g. typing in a text field.
pub fn grab_keyboard<T: 'static>(
    cx: &mut Context<T>,
    on_key: impl Fn(&mut T, &Keystroke, Option<&dyn Any>, &mut Window, &mut Context<T>) -> bool
    + 'static,
) -> Subscription {
    let this = cx.entity().downgrade();
    cx.intercept_keystrokes(move |event, window, cx| {
        let action = cx
            .key_bindings()
            .borrow()
            .bindings_for_input(std::slice::from_ref(&event.keystroke), &event.context_stack)
            .0
            .first() // highest precedence
            .map(|b| b.action().boxed_clone());
        let taken = this
            .update(cx, |this, cx| {
                on_key(
                    this,
                    &event.keystroke,
                    action.as_ref().map(|a| a.as_any()),
                    window,
                    cx,
                )
            })
            .unwrap_or(false);
        if taken {
            cx.stop_propagation();
        }
    })
}

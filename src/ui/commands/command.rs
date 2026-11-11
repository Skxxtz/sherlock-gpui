use gpui::{Action, App, Context, Keystroke, SharedString, Window};

use crate::ui::views::Propagation;

/// Something a layer can do, reachable by key, bind, hint or click.
pub struct Command<V> {
    /// Stable name, used by config binds and clicks: `"ask.stop"`.
    pub id: &'static str,
    /// Status bar / context menu text.
    pub label: &'static str,
    /// Default keys. Empty: reachable by id only.
    pub keys: &'static [Key],
    /// Whether the command is available right now.
    pub when: fn(&V, &App) -> bool,
    /// `Continue`: did nothing after all, the next layer gets the key.
    pub run: fn(&mut V, &mut Window, &mut Context<V>) -> Propagation,
    /// Shown in the status bar while available.
    pub hint: bool,
}

/// `when` for commands that are always available.
pub fn always<V>(_: &V, _: &App) -> bool {
    true
}

impl<V> Command<V> {
    /// Whether one of the keys is what was pressed.
    pub fn matches(&self, pressed: &Pressed) -> bool {
        self.keys.iter().any(|k| k.matches(pressed))
    }
}

/// A command's key.
#[derive(Clone, Copy)]
pub enum Key {
    /// A fixed keystroke: `"ctrl-n"`.
    Stroke(&'static str),
    /// Whatever the keymap binds to this action (so `[keybinds]` in the config still applies),
    /// and the action itself when dispatched (status-bar clicks).
    Action(fn() -> Box<dyn Action>),
}

impl Key {
    pub const fn action<A: Action + Default>() -> Self {
        Self::Action(|| Box::new(A::default()))
    }

    pub fn matches(&self, pressed: &Pressed) -> bool {
        match self {
            Self::Stroke(s) => pressed.key.as_ref().is_some_and(|key| {
                Keystroke::parse(s).is_ok_and(|k| k.modifiers == key.modifiers && k.key == key.key)
            }),
            Self::Action(make) => pressed
                .action
                .as_ref()
                .is_some_and(|action| action.partial_eq(&*make())),
        }
    }
}

/// What a command is offered: a keystroke (with the action the keymap binds it to), or a
/// dispatched action.
pub struct Pressed {
    pub key: Option<Keystroke>,
    pub action: Option<Box<dyn Action>>,
}

impl Pressed {
    /// `key`, resolved against the keymap in the focused context like GPUI would.
    pub fn key(key: &Keystroke, win: &Window, cx: &App) -> Self {
        let action = cx
            .key_bindings()
            .borrow()
            .bindings_for_input(std::slice::from_ref(key), &win.context_stack())
            .0
            .first()
            .map(|b| b.action().boxed_clone());
        Self {
            key: Some(key.clone()),
            action,
        }
    }

    /// An action dispatched without a key (a click, an event).
    pub fn action(action: &dyn Action) -> Self {
        Self {
            key: None,
            action: Some(action.boxed_clone()),
        }
    }
}

/// An available command, as the status bar shows it.
#[derive(Clone)]
pub struct Hint {
    pub id: &'static str,
    pub label: SharedString,
    pub keys: &'static [Key],
}

use gpui::{App, Context, Keystroke, SharedString, Window};

use crate::ui::views::Propagation;

/// Something a layer can do, reachable by key, bind, hint or click.
pub struct Command<V> {
    /// Stable name, used by config binds and clicks: `"ask.stop"`.
    pub id: &'static str,
    /// Status bar / context menu text.
    pub label: &'static str,
    /// Default keys (`"ctrl-c"`). Empty: reachable by id only.
    pub keys: &'static [&'static str],
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
    /// Whether one of the default keys is `key`.
    pub fn matches(&self, key: &Keystroke) -> bool {
        self.keystrokes()
            .any(|k| k.modifiers == key.modifiers && k.key == key.key)
    }

    /// The default keys, parsed. Invalid ones are skipped.
    pub fn keystrokes(&self) -> impl Iterator<Item = Keystroke> + '_ {
        self.keys.iter().filter_map(|k| Keystroke::parse(k).ok())
    }
}

/// An available command, as the status bar shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hint {
    pub id: &'static str,
    pub label: SharedString,
    pub keys: Vec<Keystroke>,
}

use std::sync::Arc;

use gpui::{App, AsKeystroke, Global, KeyBinding, Keystroke, Modifiers};
use smallvec::SmallVec;

use crate::{
    SHORTCUT_MOD,
    launcher::utils::binds::Bind,
    ui::{
        actions::{ShortcutAction, UIFunction},
        launcher::context_menu::action_keys::ContextActionBinds,
    },
    utils::config::{ConfigGuard, ConfigKeybinds},
};

pub struct ReservedKeys(pub Vec<ContextActionBinds>);
impl Global for ReservedKeys {}

/// `[commands]` from the config: keys that run commands in every view.
#[derive(Default)]
pub struct CommandBinds(pub Arc<[Bind]>);
impl Global for CommandBinds {}

/// `"variable.tab"` → `(Some("variable"), "tab")`; `"ctrl-n"` → `(None, "ctrl-n")`.
pub fn split_scope(key: &str) -> (Option<&str>, &str) {
    match key.rsplit_once('.') {
        Some((scope, keys)) if !keys.is_empty() => (Some(scope), keys),
        _ => (None, key),
    }
}

/// Clear current bindings and reload them from `ConfigGuard`.
#[inline(always)]
pub fn reload_bindings(cx: &mut App) {
    cx.clear_key_bindings();
    register_bindings(cx);
}

pub(super) fn register_bindings(cx: &mut App) {
    let command_binds = ConfigGuard::read_with(|c| c.commands.binds().0).unwrap_or_default();
    cx.set_global(CommandBinds(command_binds.into()));

    // one config read: the binds and the symbols for the shortcut modifiers
    let (binds, mod_symbols) =
        ConfigGuard::read_with(|c| (c.keybinds.effective(), c.appearance.mod_key_ascii))
            .unwrap_or_else(|_| {
                (
                    ConfigKeybinds::default().effective(),
                    crate::utils::config::AppearanceDefaults::modkey_ascii(),
                )
            });

    // set again below if shortcut keys are still bound
    ShortcutKeyMod::clear();
    let mut bindings: Vec<KeyBinding> = Vec::with_capacity(binds.len() + 8);
    for (key, function) in &binds {
        let (scope, keys) = split_scope(key);
        match function {
            // `ctrl-<digit>`: ctrl-1 … ctrl-9 run the n-th result
            UIFunction::Shortcut if keys.contains("<digit>") => {
                for index in 1..=9 {
                    let keys = keys.replace("<digit>", &index.to_string());
                    bindings.push(KeyBinding::new(&keys, ShortcutAction { index }, scope));
                }
                if let Some(mods) = shortcut_modifiers(keys) {
                    ShortcutKeyMod::set(&mods, &mod_symbols);
                }
            }
            function => bindings.extend(function.binding(keys, scope)),
        }
    }

    // bindings with a context come last: on equal keys, GPUI prefers the later one
    bindings.sort_by_key(|b| b.predicate().is_some());

    let reserved = bindings
        .iter()
        .filter_map(|b| match b.keystrokes() {
            [single] => ContextActionBinds::from_keystroke(single.as_keystroke()),
            _ => None,
        })
        .collect();
    cx.set_global(ReservedKeys(reserved));

    cx.bind_keys(bindings);
}

/// The modifiers of a pattern like `"ctrl-<digit>"`, read from its first expansion.
fn shortcut_modifiers(pattern: &str) -> Option<Modifiers> {
    Keystroke::parse(&pattern.replace("<digit>", "1"))
        .ok()
        .map(|k| k.modifiers)
}

/// The modifier symbols shown next to shortcut numbers (`^1`, `⌘1`, …).
pub struct ShortcutKeyMod {
    buf: SmallVec<[char; 4]>,
}

impl ShortcutKeyMod {
    /// Stores the symbols for `mods`; `symbols` are ⌘, ^, ⌥, ⇧ (from the config).
    fn set(mods: &Modifiers, symbols: &[char; 4]) {
        let buf = [mods.platform, mods.control, mods.alt, mods.shift]
            .into_iter()
            .zip(symbols)
            .filter_map(|(held, &symbol)| held.then_some(symbol))
            .collect();
        if let Ok(mut current) = SHORTCUT_MOD.write() {
            *current = Some(Self { buf });
        }
    }

    /// No shortcut keys bound (`"ctrl-<digit>" = "unbind"`).
    fn clear() {
        if let Ok(mut current) = SHORTCUT_MOD.write() {
            *current = None;
        }
    }

    /// The symbols, or `None` while no shortcut keys are bound (then results show no badges).
    pub fn get() -> Option<SmallVec<[char; 4]>> {
        SHORTCUT_MOD.read().ok()?.as_ref().map(|m| m.buf.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The symbols follow the keymap: replaced on every registration, gone when unbound.
    #[test]
    fn shortcut_symbols_follow_the_keymap() {
        let symbols = ['⌘', '^', '⎇', '⇧'];
        ShortcutKeyMod::set(&shortcut_modifiers("ctrl-<digit>").unwrap(), &symbols);
        assert_eq!(ShortcutKeyMod::get().as_deref(), Some(&['^'][..]));

        ShortcutKeyMod::set(&shortcut_modifiers("alt-<digit>").unwrap(), &symbols);
        assert_eq!(ShortcutKeyMod::get().as_deref(), Some(&['⎇'][..]));

        ShortcutKeyMod::clear();
        assert_eq!(ShortcutKeyMod::get(), None);
    }
}

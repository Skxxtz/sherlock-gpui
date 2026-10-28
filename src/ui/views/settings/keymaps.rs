use gpui::Context;

use super::{SettingsView, model::SettingValue};
use crate::ui::components::keybind::chord_label;

/// The context of a keybind row, from its id: `keybinds.variable.complete` → `Some("variable")`,
/// `keybinds.next_var` → `None`.
fn keybind_scope(id: &str) -> Option<&str> {
    id.strip_prefix("keybinds.")?
        .rsplit_once('.')
        .map(|(scope, _)| scope)
}

impl SettingsView {
    /// Labels of the other actions bound to `chord` in the same context as setting `id`. The
    /// same key in different contexts (`tab`, `variable.tab`) isn't a conflict.
    pub(super) fn conflicts(&self, id: &str, chord: &str) -> Vec<&'static str> {
        let scope = keybind_scope(id);
        self.settings()
            .filter(|st| {
                st.id != id
                    && keybind_scope(st.id) == scope
                    && matches!(&st.value, SettingValue::Keybind(c) if c.iter().any(|c| c == chord))
            })
            .map(|st| st.label)
            .collect()
    }

    /// "ctrl j is also bound to Previous item", for the first shared chord of setting `id`.
    pub(super) fn conflict_note(&self, id: &str) -> Option<String> {
        let SettingValue::Keybind(chords) = &self.setting(id)?.value else {
            return None;
        };
        chords.iter().find_map(|chord| {
            let others = self.conflicts(id, chord);
            (!others.is_empty()).then(|| {
                format!(
                    "{} is also bound to {}",
                    chord_label(chord),
                    others.join(", ")
                )
            })
        })
    }

    /// Tells each key recorder which of its chords other actions use too.
    pub(super) fn refresh_keymap_notes(&self, cx: &mut Context<Self>) {
        for setting in self.settings() {
            let SettingValue::Keybind(chords) = &setting.value else {
                continue;
            };
            let Some(control) = self.controls.get(setting.id) else {
                continue;
            };
            let notes = chords
                .iter()
                .map(|c| {
                    let others = self.conflicts(setting.id, c);
                    (!others.is_empty()).then(|| format!("also {}", others.join(", ")))
                })
                .collect();
            control.set_notes(notes, cx);
        }
    }
}

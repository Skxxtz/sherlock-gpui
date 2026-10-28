use gpui::Context;

use super::model::SettingValue;
use crate::ui::components::keybind::chord_label;

impl super::SettingsView {
    /// Tells each input which of its chords other actions use too.
    pub fn refresh_keymap_notes(&self, cx: &mut Context<Self>) {
        for (&at, input) in &self.keymaps {
            let notes: Vec<Option<String>> = match &self.sections[at.0].settings[at.1].value {
                SettingValue::Keybind(chords) => chords
                    .iter()
                    .map(|c| {
                        let others = self.conflicts(at, c);
                        (!others.is_empty()).then(|| format!("also {}", others.join(", ")))
                    })
                    .collect(),
                _ => continue,
            };
            input.update(cx, |input, cx| input.set_notes(notes, cx));
        }
    }

    /// "⌃ J is also bound to Previous item", for the first shared chord of a keybind setting.
    pub fn conflict_note(&self, section: usize, row: usize) -> Option<String> {
        let SettingValue::Keybind(chords) = &self.sections.get(section)?.settings.get(row)?.value
        else {
            return None;
        };
        chords.iter().find_map(|chord| {
            let others = self.conflicts((section, row), chord);
            (!others.is_empty()).then(|| {
                format!(
                    "{} is also bound to {}",
                    chord_label(chord),
                    others.join(", ")
                )
            })
        })
    }

    /// Other actions bound to `chord`, by label.
    pub fn conflicts(&self, at: (usize, usize), chord: &str) -> Vec<&'static str> {
        self.sections
            .iter()
            .enumerate()
            .flat_map(|(si, s)| {
                s.settings
                    .iter()
                    .enumerate()
                    .map(move |(ri, st)| ((si, ri), st))
            })
            .filter(|&(other, st)| {
                other != at
                    && matches!(&st.value, SettingValue::Keybind(c) if c.iter().any(|c| c == chord))
            })
            .map(|(_, st)| st.label)
            .collect()
    }
}

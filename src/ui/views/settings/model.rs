//! The settings data: values, settings, sections, and the edits that change them.

use gpui::{Hsla, SharedString};

/// The value of one setting; decides which control is shown.
#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum SettingValue {
    Toggle(bool),
    Color(Hsla),
    /// Every chord bound to the action (`"ctrl-n"`, `"down"`); an action can have several.
    Keybind(Vec<SharedString>),
    Choice {
        options: &'static [&'static str],
        selected: usize,
    },
    Number {
        value: i64,
        min: i64,
        max: i64,
        step: i64,
    },
    /// Free text (a command, a placeholder). Empty can mean "not set" for optional fields.
    Text(SharedString),
}

#[derive(Clone, Debug)]
pub struct Setting {
    /// Stable key (`"appearance.accent"`): finds the setting and its control, whatever the order.
    pub id: &'static str,
    /// Sub-heading within the section (`// startup`); consecutive settings share one.
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub value: SettingValue,
}

impl Setting {
    /// `query` must be lowercase.
    pub(super) fn matches(&self, query: &str) -> bool {
        self.label.to_lowercase().contains(query) || self.description.to_lowercase().contains(query)
    }
}

#[derive(Clone, Debug)]
pub struct SettingsSection {
    pub title: &'static str,
    /// One line under the section's title.
    pub description: &'static str,
    pub settings: Vec<Setting>,
}

/// A change to a value, from a click, Enter, or an editor (color, choice, keybind).
#[derive(Clone, Debug)]
pub(super) enum Edit {
    Toggle,
    SetNumber(i64),
    SetChoice(usize),
    SetColor(Hsla),
    SetChords(Vec<SharedString>),
    SetText(SharedString),
}

impl SettingValue {
    /// Applies `edit`; an edit for another kind of value is ignored.
    pub(super) fn apply(&mut self, edit: Edit) {
        match (self, edit) {
            (Self::Toggle(on), Edit::Toggle) => *on = !*on,
            (
                Self::Number {
                    value, min, max, ..
                },
                Edit::SetNumber(new),
            ) => *value = new.clamp(*min, *max),
            (Self::Choice { options, selected }, Edit::SetChoice(i)) => {
                *selected = i.min(options.len().saturating_sub(1));
            }
            (Self::Color(c), Edit::SetColor(new)) => *c = new,
            (Self::Keybind(chords), Edit::SetChords(new)) => *chords = new,
            (Self::Text(text), Edit::SetText(new)) => *text = new,
            _ => {}
        }
    }

    /// What Enter does to a value without an editor: flips a toggle. Everything else opens
    /// its editor.
    pub(super) fn activate(&self) -> Option<Edit> {
        match self {
            Self::Toggle(_) => Some(Edit::Toggle),
            _ => None,
        }
    }
}

use gpui::{Hsla, SharedString};

/// The value of one setting; decides which control is shown.
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
}

#[derive(Clone, Debug)]
pub struct Setting {
    /// Sub-heading within the section (`// startup`); consecutive settings share one.
    pub group: &'static str,
    pub label: &'static str,
    pub description: &'static str,
    pub value: SettingValue,
}

impl Setting {
    /// `query` must be lowercase.
    pub fn matches(&self, query: &str) -> bool {
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

/// What a click on a control does to its value.
#[derive(Clone, Copy, Debug)]
pub enum Edit {
    Toggle,
    SetChoice(usize),
    Step(i64),
    SetColor(Hsla),
}

impl SettingValue {
    pub fn apply(&mut self, edit: Edit) {
        match (self, edit) {
            (Self::Toggle(on), Edit::Toggle) => *on = !*on,
            (Self::Choice { options, selected }, Edit::SetChoice(i)) => {
                *selected = i.min(options.len().saturating_sub(1));
            }
            (
                Self::Number {
                    value, min, max, ..
                },
                Edit::Step(delta),
            ) => *value = (*value + delta).clamp(*min, *max),
            (Self::Color(c), Edit::SetColor(new)) => *c = new,
            _ => {}
        }
    }

    /// What Enter does: flip, or step up (wrapping to `min` at `max`).
    /// `None` for values that open their own editor (color, keybind, choice).
    pub fn activate(&self) -> Option<Edit> {
        match *self {
            Self::Toggle(_) => Some(Edit::Toggle),
            Self::Number {
                value,
                min,
                max,
                step,
            } => Some(Edit::Step(if value >= max { min - value } else { step })),
            Self::Color(_) | Self::Keybind(_) | Self::Choice { .. } => None,
        }
    }
}

/// Applies `edit` to setting `row` of `section`, if it exists.
pub fn apply_edit(sections: &mut [SettingsSection], section: usize, row: usize, edit: Edit) {
    if let Some(setting) = sections
        .get_mut(section)
        .and_then(|s| s.settings.get_mut(row))
    {
        setting.value.apply(edit);
    }
}

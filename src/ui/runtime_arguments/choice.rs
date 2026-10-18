use std::sync::Arc;

use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

use crate::ui::utils::pango::CachedPango;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct ChoiceVar {
    pub choice: ChoiceVarBody,
}

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChoiceVarBody {
    pub name: SharedString,
    pub default: Option<SharedString>,
    pub choices: Arc<[ChoiceOption]>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(remote = "Self")]
pub struct ChoiceOption {
    pub value: SharedString,
    pub label: CachedPango,
}

impl Serialize for ChoiceOption {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        Self::serialize(self, s)
    }
}

impl<'de> Deserialize<'de> for ChoiceOption {
    /// Config accepts `"text"` or `{ "value": .., "label": .. }` (label optional).
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        if !d.is_human_readable() {
            return Self::deserialize(d);
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Repr {
            Plain(SharedString),
            Full {
                value: SharedString,
                label: Option<CachedPango>,
            },
        }

        Ok(match Repr::deserialize(d)? {
            Repr::Plain(value) => Self {
                label: CachedPango::from(value.clone()),
                value,
            },
            Repr::Full { value, label } => Self {
                label: label.unwrap_or_else(|| CachedPango::from(value.clone())),
                value,
            },
        })
    }
}

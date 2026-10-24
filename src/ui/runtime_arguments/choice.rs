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

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use md_rs::{components::span::code, p};

    use super::ChoiceVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};

    impl Documentation for ChoiceVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "Choice Input",
                name: "choice",
                description: || {
                    p!(
                        "Presents the user with a predefined list of \
                            options to select from. Each choice has a",
                        code("label"),
                        "shown in the UI and a",
                        code("value"),
                        "passed to the command."
                    )
                },
                args: &[
                    FieldDoc {
                        name: "default",
                        ty: "String",
                        required: false,
                        default: None,
                        description: "The default value this field should show",
                    },
                    FieldDoc {
                        name: "default",
                        ty: "Choice",
                        required: false,
                        default: None,
                        description: "The choices that should be displayed in the dropdown menu.",
                    },
                ],
                example: indoc::indoc! {r#"{
                    "choice": {
                        "name": "temperature",
                        "choices": [
                            {"label": "5000", "value": "5000"},
                            {"label": "6000 <span color='#555555'><i>default</i></span>", "value": "6000"},
                            {"label": "7000", "value": "7000"},
                            {"label": "8000", "value": "8000"}
                        ]
                    }
                }"#},
            }
        }
    }
}

use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

use crate::ui::utils::input_rule::CharSet;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct StringVar {
    pub string_input: SharedString,
    pub default: Option<SharedString>,
    pub validation: Option<CharSet>,
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::StringVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};
    impl Documentation for StringVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "String Input",
                name: "string_input",
                description: || "A plain text input field.".into(),
                args: &[
                    FieldDoc {
                        name: "default",
                        ty: "String",
                        required: false,
                        default: None,
                        description: "The default value this field should show",
                    },
                    FieldDoc {
                        name: "validation",
                        ty: "Character class",
                        required: false,
                        default: None,
                        // TODO: use a function here so that we can use spans
                        description: "A plain text input field. The entered value replaces \
                            `{variable:<name>}` in `exec`, where `<name>` is the value of `string_input`.",
                    },
                ],
                example: r#"{
                  "string_input": "repository",
                  "default": "sherlock-gpui",
                  "validation": "[a-zA-Z0-9._-]"
                }"#,
            }
        }
    }
}

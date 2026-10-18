use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct PasswordVar {
    pub password_input: SharedString,
    pub default: Option<SharedString>,
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::PasswordVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};
    impl Documentation for PasswordVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "Password Input",
                name: "password_input",
                description: || "Like string_input but the value is masked in the UI.".into(),
                args: &[FieldDoc {
                    name: "default",
                    ty: "String",
                    required: false,
                    default: None,
                    description: "The default value this field should show",
                }],
                example: r#"{ "password_input": "sudo" }"#,
            }
        }
    }
}

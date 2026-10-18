use crate::ui::utils::input_rule::CharSet;
use gpui::SharedString;
use serde::{Deserialize, Serialize};
use sherlock_derive::Finalize;

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize, Finalize)]
pub struct NumberVar {
    pub number_input: SharedString,
    pub default: Option<f64>,
    pub validation: Option<CharSet>,
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::NumberVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};
    impl Documentation for NumberVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "Number Input",
                name: "number_input",
                description: || "A number input field.".into(),
                args: &[FieldDoc {
                    name: "default",
                    ty: "Number",
                    required: false,
                    default: None,
                    description: "The default value this field should show",
                }],
                example: r#"{ "number_input": "example"}"#,
            }
        }
    }
}

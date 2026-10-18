use std::borrow::Cow;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::{
    ui::runtime_arguments::utils::{Finalize, expand_one},
    utils::files::home_str,
};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathVar {
    pub path_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
}

impl Finalize for PathVar {
    fn finalize<'a>(&self, raw: &'a str) -> Cow<'a, str> {
        expand_one(raw, home_str().unwrap_or("."))
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::PathVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};
    use md_rs::{components::span::code, p};
    impl Documentation for PathVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "Path Input",
                name: "path_input",
                description: || {
                    p!(
                        "A text input featuring path completion. \
                            By default, completion paths are resolved relative to the",
                        code("$HOME"),
                        "directory. Starting the input with a",
                        code("/"),
                        "prefix will search from the system root instead.",
                    )
                },
                args: &[FieldDoc {
                    name: "default",
                    ty: "String",
                    required: false,
                    default: None,
                    description: "The default value this field should show",
                }],
                example: r#"{ "path_input": "path" }"#,
            }
        }
    }
}

use std::borrow::Cow;

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::{
    ui::runtime_arguments::utils::{Finalize, expand_one, looks_like_path},
    utils::files::home_str,
};

#[derive(Default, Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommandVar {
    pub command_input: SharedString,
    pub default: Option<SharedString>,
    #[serde(skip)]
    pub index: usize,
    #[serde(skip)]
    pub is_scoped: bool,
}

impl From<SharedString> for CommandVar {
    fn from(value: SharedString) -> Self {
        Self {
            command_input: value,
            ..Default::default()
        }
    }
}

impl Finalize for CommandVar {
    fn finalize<'a>(&self, raw: &'a str) -> Cow<'a, str> {
        if !self.is_scoped {
            return Cow::Borrowed(raw);
        }
        let home = home_str().unwrap_or(".");
        let words: Vec<_> = raw
            .split(' ')
            .map(|w| {
                if looks_like_path(w) {
                    expand_one(w, home)
                } else {
                    Cow::Borrowed(w)
                }
            })
            .collect();
        if words.iter().all(|w| matches!(w, Cow::Borrowed(_))) {
            Cow::Borrowed(raw)
        } else {
            Cow::Owned(words.join(" "))
        }
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use md_rs::{components::span::code, p};

    use super::CommandVar;
    use crate::docs::{Documentation, launcher::FieldDoc, runtime_args::RuntimeArgumentDoc};
    impl Documentation for CommandVar {
        type Docs = RuntimeArgumentDoc;
        fn docs() -> Self::Docs {
            RuntimeArgumentDoc {
                title: "Command Input",
                name: "command_input",
                description: || {
                    p!(
                        "A text input featuring path completion. Unlike",
                        code("path_input"),
                        ", this will only look at executeable files. \
                            First, it will look at the",
                        code("$PATH"),
                        ", then it will try to complete like",
                        code("path_input"),
                        "."
                    )
                },
                args: &[FieldDoc {
                    name: "default",
                    ty: "String",
                    required: false,
                    default: None,
                    description: "The default value this field should show",
                }],
                example: r#"{ "command_input": "command" }"#,
            }
        }
    }
}

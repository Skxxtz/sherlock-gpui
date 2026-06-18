use std::sync::Arc;

use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    display_name,
    docs::launcher::{Example, LauncherDoc, LauncherDocEntry},
    launcher::{LauncherProvider, LauncherType, LoadContext},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, translator::TranslationData},
    utils::errors::SherlockMessage,
    variant_name,
};

/// No user-side arguments
#[derive(Clone, Debug, Deserialize)]
pub struct Translator {}

impl LauncherProvider for Translator {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Translator(Translator {})
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        _ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        Ok(vec![RenderableChild::Translator {
            launcher,
            inner: TranslationData::new(cx),
        }])
    }
}

// DOCS
impl LauncherDoc for Translator {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(Translator),
            variant_name: variant_name!(Translator),
            description: "Translate your queries into other languages.",
            examples: &[Example {
                description: "Basic translator",
                json: indoc! {
                    r#"{
                        "name": "Translator",
                        "alias": "trans",
                        "type": "translator",
                        "args": {},
                        "on_return": "inner.run",
                        "exit": false,
                        "priority": 1,
                        "shortcut": false
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

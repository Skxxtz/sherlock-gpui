use indoc::indoc;
use serde_json::Value;

use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry, capabilities_section},
    launcher::{LauncherProvider, LauncherType},
    loader::utils::RawLauncher,
    ui::widgets::{RenderableChild, clipboard::ClipWidget},
    utils::{errors::SherlockMessage, intent::Capabilities},
    variant_name,
};

/// The following arguments are available to users:
/// - `capabilities`
#[derive(Clone, Debug)]
pub struct ClipboardLauncher {
    pub capabilities: Capabilities,
}
impl LauncherProvider for ClipboardLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let caps: Vec<String> = match raw.args.get("capabilities") {
            Some(Value::Array(arr)) => arr
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect(),
            _ => vec![String::from("calc.math"), String::from("calc.units")],
        };
        let capabilities = Capabilities::from_strings(&caps);
        Ok(LauncherType::Clipboard(ClipboardLauncher { capabilities }))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        Ok(vec![RenderableChild::Clip {
            launcher,
            inner: ClipWidget::new(cx),
        }])
    }
}

// DOCS
impl LauncherDoc for ClipboardLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(ClipboardLauncher),
            variant_name: variant_name!(Clipboard),
            description: "Executes commands based on the clipboard content.",
            args: &[FieldDoc {
                name: "capabilities",
                ty: "Capabilities[]",
                required: false,
                default: Some(r#"[ "calc.units", "calc.math" ]"#),
                description: "The capabilities the clipboard executor should have.",
            }],
            args_explanations: &[capabilities_section],
            examples: &[Example {
                description: "Power Menu Example",
                json: indoc! {
                    r#" {
                        "name": "Clipboard",
                        "type": "clipboard",
                        "args": {
                            "capabilities": [
                                "url",
                                "colors",
                                "calc.math"
                            ]
                        },
                        "on_return": "copy",
                        "priority": 3,
                        "home": "OnlyHome"
                    } "#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

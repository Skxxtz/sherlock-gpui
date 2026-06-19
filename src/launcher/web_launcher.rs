use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    launcher::{LauncherProvider, LauncherType, app_launcher::app_data::AppData},
    loader::utils::{PriorityGuard, RawLauncher},
    sherlock_msg,
    ui::widgets::RenderableChild,
    utils::errors::{SherlockMessage, types::SherlockErrorType},
    variant_name,
};
use gpui::SharedString;
use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;

/// The following arguments are available to users:
/// - `engine`: The engine to be used for the query
/// - `browser`: The browser to be used for opening the query, defaults
/// - `display_name`: The display name for this tile, replacing `{keyword}` with query
#[derive(Clone, Debug, Deserialize)]
pub struct WebLauncher {
    #[serde(rename = "search_engine")]
    pub engine: String,
    #[serde(default)]
    pub browser: Option<String>,
}

impl LauncherProvider for WebLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        serde_json::from_value::<WebLauncher>(raw.args.as_ref().clone())
            .map(LauncherType::Web)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let name: Option<SharedString> = opts
            .get("display_name")
            .and_then(Value::as_str)
            .map(String::from)
            .map(SharedString::from);

        let inner = AppData {
            name,
            icon: launcher.icon.clone(),
            priority: PriorityGuard::new_with_launcher(&launcher, 0),
            ..AppData::new()
        };

        Ok(vec![RenderableChild::App { launcher, inner }])
    }
}

// DOCS
/// - `engine`: The engine to be used for the query
/// - `browser`: The browser to be used for opening the query, defaults
/// - `display_name`: The display name for this tile, replacing `{keyword}` with query
impl LauncherDoc for WebLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(WebLauncher),
            variant_name: variant_name!(Web),
            description: "Seach the current query in the specified engine using the specified browser.",
            args: &[
                FieldDoc {
                    name: "search_engine",
                    ty: "string",
                    required: true,
                    default: None,
                    description: "The search engine used for the query.",
                },
                FieldDoc {
                    name: "browser",
                    ty: "u64",
                    required: false,
                    default: Some("Default Browser"),
                    description: "The browser in which to open the query.",
                },
                FieldDoc {
                    name: "display_name",
                    ty: "string",
                    required: false,
                    default: None,
                    description: "The display name for this tile, replacing `{keyword}` with the actual contents of the search bar.",
                },
            ],
            examples: &[Example {
                description: "Basic web launcher",
                json: indoc! {
                    r#"{
                        "name": "Web Search",
                        "alias": "gg",
                        "type": "web",
                        "args": {
                            "search_engine": "google",
                            "icon": "google",
                            "display_name": "Google Search {keyword}"
                        },
                        "home": "Persist",
                        "priority": 100
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

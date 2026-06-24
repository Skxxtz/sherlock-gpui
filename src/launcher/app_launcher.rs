use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    launcher::{LauncherProvider, LauncherType, LoadContext},
    loader::{application_loader::ApplicationLoader, utils::RawLauncher},
    sherlock_msg,
    ui::widgets::RenderableChild,
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

pub mod app_data;
pub mod app_serde;

/// The following arguments are available to users:
/// - `use_keywords`: Whether the search should use the keywords or only the app name
#[derive(Clone, Debug, Deserialize)]
pub struct AppLauncher {
    #[serde(default)]
    pub use_keywords: bool,
}

impl LauncherProvider for AppLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        serde_json::from_value::<AppLauncher>(raw.args.as_ref().clone())
            .map(LauncherType::Apps)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        ApplicationLoader::load_applications(
            Arc::clone(&launcher),
            &ctx.counts,
            self.use_keywords,
            ctx.changes,
        )
        .map(|apps| {
            Arc::unwrap_or_clone(apps)
                .into_iter()
                .map(|inner| RenderableChild::App {
                    launcher: Arc::clone(&launcher),
                    inner,
                })
                .collect()
        })
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::AppLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };
    use indoc::indoc;

    impl LauncherDoc for AppLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(AppLauncher),
                variant_name: variant_name!(Apps),
                description: "Launches installed desktop applications",
                args: &[FieldDoc {
                    name: "use_keywords",
                    ty: "bool",
                    required: false,
                    default: Some("true"),
                    description: "Whether the search should use the keywords defined in the .desktop file.",
                }],
                examples: &[Example {
                    description: "Basic app launcher",
                    json: indoc! {
                        r#"{
                        "name": "App Launcher",
                        "alias": "app",
                        "type": "apps",
                        "args": {
                            "use_keywords": false
                        },
                        "priority": 4,
                        "home": "Home"
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    launcher::{
        LauncherProvider, LauncherType, LoadContext, plugin_launcher::sandbox::PluginSandBox,
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::widgets::{RenderableChild, plugin::PluginWidget},
    utils::errors::{
        SherlockMessage,
        types::{PluginAction, SherlockErrorType},
    },
    variant_name,
};
use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

pub mod lua;
pub mod sandbox;

#[derive(Clone, Debug, Deserialize)]
pub struct PluginLauncher {
    pub path: String,
}

impl LauncherProvider for PluginLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        match serde_json::from_value::<PluginLauncher>(raw.args.as_ref().clone()) {
            Ok(launcher) => LauncherType::Plugin(launcher),
            Err(_) => LauncherType::Empty,
        }
    }

    fn objects(
        &self,
        launcher: Arc<super::Launcher>,
        _ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let path = std::path::Path::new(&self.path);

        let sandbox = PluginSandBox::from_file(path).map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::Plugin(PluginAction::Load, self.path.clone()),
                e
            )
        })?;

        let sandbox = Arc::new(sandbox);

        // call the plugin's `tiles()` function to get tile descriptors
        let tiles: Vec<mlua::Table> = sandbox.call("tiles", ()).map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::Plugin(PluginAction::TileInit, self.path.clone()),
                e
            )
        })?;

        let children = tiles
            .into_iter()
            .map(|tile| RenderableChild::Plugin {
                launcher: Arc::clone(&launcher),
                inner: PluginWidget {
                    sandbox: Arc::clone(&sandbox),
                    tile,
                },
            })
            .collect();

        Ok(children)
    }
}

impl LauncherDoc for PluginLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(PluginLauncher),
            variant_name: variant_name!(Plugin),
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

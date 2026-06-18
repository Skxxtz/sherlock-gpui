use std::{path::PathBuf, sync::Arc};

use gpui::SharedString;
use serde::{
    Deserialize, Deserializer,
    de::{MapAccess, Visitor},
};

use crate::{
    launcher::{LauncherConfig, app_launcher::app_data::AppData},
    loader::{
        resolve_icon_path,
        utils::{ApplicationAction, ExecVariable, PriorityGuard},
    },
    ui::launcher::context_menu::ContextMenuAction,
};

/// Deserializes a map of `{ "App Name": AppData }` where the key becomes
/// `AppData.name`. This is needed because the app name lives as the map key
/// in the config format, not as a field inside the value.
pub fn deserialize_named_appdata<'de, D>(
    deserializer: D,
    launcher: &LauncherConfig,
) -> Result<Vec<AppData>, D::Error>
where
    D: Deserializer<'de>,
{
    struct AppDataMapVisitor<'a> {
        launcher: &'a LauncherConfig,
    }

    impl<'de, 'a> Visitor<'de> for AppDataMapVisitor<'a> {
        type Value = Vec<AppData>;

        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("a map of AppData keyed by name")
        }

        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            #[derive(Deserialize)]
            pub struct AppDataSerde {
                pub exec: Option<String>,
                pub search_string: String,
                #[serde(default)]
                pub priority: Option<u16>,
                pub icon: Option<String>,
                pub desktop_file: Option<PathBuf>,
                pub original_name: Option<String>,
                #[serde(default)]
                pub actions: Vec<AppActionSerde>,
                #[serde(default)]
                #[serde(rename = "variables")]
                pub vars: Vec<ExecVariable>,
                #[serde(default)]
                pub terminal: bool,
            }

            #[derive(Deserialize)]
            pub struct AppActionSerde {
                pub name: SharedString,

                #[serde(default)]
                pub exec: Option<String>,

                #[serde(default)]
                pub icon: Option<String>,

                pub method: String,

                #[serde(default)]
                pub exit: Option<bool>,
            }

            let mut collection = Vec::with_capacity(map.size_hint().unwrap_or(0));

            while let Some((key, value)) = map.next_entry::<String, AppDataSerde>()? {
                let icon = value.icon.as_deref().and_then(resolve_icon_path);
                collection.push(AppData {
                    name: Some(SharedString::from(key)),
                    exec: value.exec,
                    search_string: value.search_string,
                    priority: PriorityGuard::new(
                        value.priority.unwrap_or(self.launcher.priority),
                        0,
                    ),
                    actions: value
                        .actions
                        .into_iter()
                        .map(|a| {
                            let ctx_action = ContextMenuAction::App(ApplicationAction {
                                name: a.name,
                                icon: a
                                    .icon
                                    .as_deref()
                                    .and_then(resolve_icon_path)
                                    .or(icon.clone()),
                                method: a.method,
                                exec: a.exec,
                                exit: a.exit.unwrap_or(true),
                            });
                            Arc::new(ctx_action)
                        })
                        .collect(),
                    icon,
                    desktop_file: value.desktop_file,
                    original_name: value.original_name,
                    vars: value.vars,
                    terminal: value.terminal,
                });
            }
            Ok(collection)
        }
    }

    deserializer.deserialize_map(AppDataMapVisitor { launcher })
}

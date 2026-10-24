use std::sync::Arc;

use serde::de::IntoDeserializer;

use crate::{
    launcher::{LauncherConfig, LauncherProvider, LauncherType},
    loader::{named_items::deserialize_named_items, utils::RawLauncher},
    sherlock_msg,
    ui::widgets::{RenderableChild, list_item::category::CategoryPayload},
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

/// The following arguments are available to users:
/// - `categories`: The available categories. Is a named AppData. On execution, will apply the
///   alias, provided as the `exec` field.
#[derive(Clone, Debug)]
pub struct CategoryLauncher {}

impl LauncherProvider for CategoryLauncher {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Categories(CategoryLauncher {}))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let cmds = opts.get("categories").ok_or_else(|| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Invalid launcher configuration.".into()),
                "Category launcher does not contain any categories."
            )
        })?;

        let items =
            deserialize_named_items(cmds.into_deserializer(), |exec| CategoryPayload { exec })
                .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))?;

        let children: Vec<RenderableChild> = items
            .into_iter()
            .map(|mut item| {
                item.inherit_icons(launcher.icon.as_ref());
                RenderableChild::new(Arc::clone(&launcher), item)
            })
            .collect();

        Ok(children)
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use indoc::indoc;

    use super::CategoryLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };

    impl LauncherDoc for CategoryLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(CategoryLauncher),
                variant_name: variant_name!(Categories),
                description: "Applies aliases to restrict search to certain launchers.",
                args: &[FieldDoc {
                    name: "categories",
                    ty: "{Name: AppData}",
                    required: true,
                    default: None,
                    description: "The available categories. On execution, will apply the aslias, privodes as the `exec` field.",
                }],
                examples: &[Example {
                    description: "Power Menu Example",
                    json: indoc! {
                        r#" {
                        "name": "Categories",
                        "alias": "cat",
                        "type": "categories",
                        "args": {
                            "categories": {
                                "Power Menu": {
                                    "icon": "battery-full-symbolic",
                                    "icon_class": "reactive",
                                    "exec": "pm",
                                    "search_string": "powermenu;",
                                    "actions": [
                                        {
                                            "name": "Shutdown",
                                            "icon": "system-shutdown",
                                            "exec": "systemctl poweroff",
                                            "method": "command"
                                        },
                                        {
                                            "name": "Sleep",
                                            "icon": "system-suspend",
                                            "exec": "systemctl suspend",
                                            "method": "command"
                                        },
                                        {
                                            "name": "Lock",
                                            "icon": "system-lock-screen",
                                            "exec": "systemctl suspend & swaylock",
                                            "method": "command"
                                        },
                                        {
                                            "name": "Reboot",
                                            "icon": "system-reboot",
                                            "exec": "systemctl reboot",
                                            "method": "command"
                                        },
                                        {
                                            "name": "Log Out",
                                            "icon": "system-log-out",
                                            "exec": "hyprctl dispatch exit",
                                            "method": "command"
                                        }
                                    ]
                                }
                            }
                        },
                        "priority": 4,
                        "home": "Home"
                    } "#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

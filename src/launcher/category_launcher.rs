use std::sync::Arc;

use indoc::indoc;
use serde::de::IntoDeserializer;

use crate::docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry};
use crate::launcher::app_launcher::app_serde::deserialize_named_appdata;
use crate::launcher::{LauncherProvider, LauncherType};
use crate::loader::utils::{ApplicationAction, RawLauncher};
use crate::ui::launcher::context_menu::ContextMenuAction;
use crate::ui::widgets::RenderableChild;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::SherlockErrorType;
use crate::{display_name, sherlock_msg, variant_name};

/// The following arguments are available to users:
/// - `categories`: The available categories. Is a named AppData. On execution, will apply the
///   alias, provided as the `exec` field.
#[derive(Clone, Debug)]
pub struct CategoryLauncher {}

impl LauncherProvider for CategoryLauncher {
    fn parse(_raw: &RawLauncher) -> LauncherType {
        LauncherType::Categories(CategoryLauncher {})
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<super::LauncherConfig>,
        ctx: &crate::loader::LoadContext,
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
        let app_data = deserialize_named_appdata(cmds.clone().into_deserializer(), &launcher)
            .unwrap_or_default();

        let children: Vec<RenderableChild> = app_data
            .into_iter()
            .map(|mut inner| {
                let count = inner
                    .exec
                    .as_deref()
                    .and_then(|exec| ctx.counts.get(exec))
                    .copied()
                    .unwrap_or(0u16);
                inner.icon = inner.icon.clone();
                inner.priority.set_launcher(&launcher, count);
                inner.actions = inner
                    .actions
                    .iter()
                    .map(|action_arc| match action_arc.as_ref() {
                        ContextMenuAction::App(app_action) => {
                            let resolved_icon = app_action.icon.clone();

                            Arc::new(ContextMenuAction::App(ApplicationAction {
                                icon: resolved_icon,
                                ..app_action.clone()
                            }))
                        }
                        ContextMenuAction::Fn(_) => Arc::clone(action_arc),
                        ContextMenuAction::Emoji(_) => Arc::clone(action_arc),
                    })
                    .collect();

                RenderableChild::App {
                    launcher: Arc::clone(&launcher),
                    inner,
                }
            })
            .collect();

        Ok(children)
    }
}

// DOCS
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

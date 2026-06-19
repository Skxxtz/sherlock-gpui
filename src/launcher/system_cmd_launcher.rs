use std::sync::Arc;

use indoc::indoc;
use serde::de::IntoDeserializer;

use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    launcher::{
        LauncherProvider, LauncherType, app_launcher::app_serde::deserialize_named_appdata,
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::{launcher::context_menu::ContextMenuAction, widgets::RenderableChild},
    utils::errors::{SherlockMessage, types::SherlockErrorType},
    variant_name,
};

/// The following arguments are available to users:
/// - `commands`: Named AppData containing executable commands
#[derive(Clone, Debug)]
pub struct CommandLauncher {}

impl LauncherProvider for CommandLauncher {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Commands(CommandLauncher {}))
    }

    fn objects(
        &self,
        launcher: std::sync::Arc<super::LauncherConfig>,
        ctx: &crate::loader::LoadContext,
        opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let cmds = opts.get("commands").ok_or_else(|| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Invalid launcher configuration.".into()),
                "Command launcher does not contain any commands."
            )
        })?;
        let app_data =
            deserialize_named_appdata(cmds.into_deserializer(), &launcher).unwrap_or_default();
        let children: Vec<RenderableChild> = app_data
            .into_iter()
            .map(|mut inner| {
                let count = inner
                    .exec
                    .as_deref()
                    .and_then(|exec| ctx.counts.get(exec))
                    .copied()
                    .unwrap_or(0u16);

                let parent_icon = inner.icon.or(launcher.icon.clone());

                inner.icon = parent_icon.clone();

                inner.actions = inner
                    .actions
                    .iter()
                    .map(|action| match action.as_ref() {
                        ContextMenuAction::App(app_action) => {
                            let mut resolved = app_action.clone();
                            resolved.icon = app_action.icon.clone().or_else(|| parent_icon.clone());
                            Arc::new(ContextMenuAction::App(resolved))
                        }
                        _ => action.clone(),
                    })
                    .collect();

                inner.priority.set_launcher(&launcher, count);

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
impl LauncherDoc for CommandLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(CommandLauncher),
            variant_name: variant_name!(Commands),
            description: "Launches user-specified commands.",
            args: &[FieldDoc {
                name: "commands",
                ty: "{Name: AppData}",
                required: true,
                default: None,
                description: "The commands to show in Sherlock.",
            }],
            examples: &[Example {
                description: "Basic command launcher",
                json: indoc! {
                    r#"{
                        "name": "Throw Confetti",
                        "type": "commands",
                        "args": {
                            "commands": {
                                "Confetti": {
                                    "icon": "sherlock-confetti",
                                    "exec": "confetti",
                                    "search_string": "confetti;party"
                                }
                            }
                        },
                        "priority": 4
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

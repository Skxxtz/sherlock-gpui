use serde::de::IntoDeserializer;

use crate::{
    launcher::{LauncherProvider, LauncherType},
    loader::{named_items::deserialize_named_items, utils::RawLauncher},
    sherlock_msg,
    ui::widgets::{RenderableChild, list_item::command::CommandPayload},
    utils::errors::{SherlockMessage, types::SherlockErrorType},
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
        _ctx: &crate::loader::LoadContext,
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

        let items =
            deserialize_named_items(cmds.into_deserializer(), |exec| CommandPayload { exec })
                .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))?;

        let children: Vec<RenderableChild> = items
            .into_iter()
            .map(|mut item| {
                item.inherit_icons(launcher.icon.as_ref());
                RenderableChild::new(launcher.clone(), item)
            })
            .collect();

        Ok(children)
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::CommandLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };
    use indoc::indoc;

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
}

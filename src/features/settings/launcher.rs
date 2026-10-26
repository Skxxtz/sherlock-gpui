use crate::{
    launcher::{LauncherProvider, LauncherType, utils::exec_mode::ExecMode},
    ui::{
        launcher::navigator::handle::ViewKind,
        widgets::{
            RenderableChild,
            list_item::{ItemMeta, ListItem, fixed::FixedPayload},
        },
    },
};

#[derive(Clone, Debug, Default)]
pub struct SettingsLauncher;

impl LauncherProvider for SettingsLauncher {
    fn try_parse(
        _raw: &crate::loader::utils::RawLauncher,
    ) -> Result<crate::launcher::LauncherType, crate::utils::errors::SherlockMessage> {
        Ok(LauncherType::Settings(Self {}))
    }
    fn objects(
        &self,
        launcher: std::sync::Arc<crate::launcher::LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<crate::utils::errors::SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<crate::ui::widgets::RenderableChild>, crate::utils::errors::SherlockMessage>
    {
        let item = ListItem {
            meta: ItemMeta::new("Settings").icon_name("settings"),
            payload: FixedPayload(|launcher| ExecMode::CreateView {
                mode: ViewKind::Settings,
                launcher: launcher.clone(),
            }),
        };
        Ok(vec![RenderableChild::new(launcher, item)])
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use indoc::indoc;

    use super::SettingsLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, LauncherDoc, LauncherDocEntry},
        variant_name,
    };

    impl LauncherDoc for SettingsLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(SettingsLauncher),
                variant_name: variant_name!(Settings),
                description: "Opens Sherlock's settings page inside the launcher.",
                examples: &[Example {
                    description: "Settings entry, also reachable via its alias",
                    json: indoc! {
                        r#"{
                        "name": "Settings",
                        "type": "settings",
                        "alias": "settings",
                        "args": {},
                        "priority": 1,
                        "home": "Search",
                        "exit": false
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

use std::sync::Arc;

use serde::Deserialize;

use crate::{
    launcher::{LauncherProvider, utils::exec_mode::ExecMode, variant_type::LauncherType},
    loader::{resolve_icon_path, utils::RawLauncher},
    ui::widgets::{
        RenderableChild,
        list_item::{ItemMeta, ListItem, fixed::FixedPayload},
    },
    utils::errors::SherlockMessage,
};

#[derive(Clone, Debug, Deserialize)]
pub struct MessageLauncher {}
impl LauncherProvider for MessageLauncher {
    fn try_parse(_raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        Ok(LauncherType::Message(Self {}))
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let item = ListItem {
            meta: ItemMeta {
                search: "messages;errors;warnings;show".into(),
                icon: resolve_icon_path("sherlock-devtools"),
                ..ItemMeta::new("Show Messages")
            },
            payload: FixedPayload(|_| ExecMode::ShowMessages),
        };
        Ok(vec![RenderableChild::new(launcher, item)])
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::MessageLauncher;
    use crate::{
        display_name,
        docs::launcher::{LauncherDoc, LauncherDocEntry},
        variant_name,
    };

    impl LauncherDoc for MessageLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry::new_hidden(
                display_name!(MessageLauncher),
                variant_name!(Message),
                "The launcher to provide the message view",
            )
        }
    }
}

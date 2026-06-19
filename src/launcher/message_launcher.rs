use std::sync::Arc;

use serde::Deserialize;

use crate::{
    display_name,
    docs::launcher::{LauncherDoc, LauncherDocEntry},
    launcher::{LauncherProvider, app_launcher::app_data::AppData, variant_type::LauncherType},
    loader::{
        resolve_icon_path,
        utils::{PriorityGuard, RawLauncher},
    },
    ui::widgets::RenderableChild,
    utils::errors::SherlockMessage,
    variant_name,
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
        let inner = AppData {
            name: Some("Show Messages".into()),
            search_string: "messages;errors;warnings;show".into(),
            icon: resolve_icon_path("sherlock-devtools"),
            priority: PriorityGuard::new_with_launcher(&launcher, 0),
            ..AppData::new()
        };
        Ok(vec![RenderableChild::App {
            launcher: Arc::clone(&launcher),
            inner,
        }])
    }
}

// DOCS
impl LauncherDoc for MessageLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry::new_hidden(
            display_name!(MessageLauncher),
            variant_name!(Message),
            "The launcher to provide the message view",
        )
    }
}

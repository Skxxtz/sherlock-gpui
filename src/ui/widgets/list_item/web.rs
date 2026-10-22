use gpui::SharedString;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{
    launcher::{LauncherConfig, utils::exec_mode::ExecMode, variant_type::LauncherType},
    ui::widgets::list_item::ItemMeta,
};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WebPayload {
    pub url: Option<SharedString>,
}

impl super::Payload for WebPayload {
    fn exec(&self, _: &ItemMeta, launcher: &Arc<LauncherConfig>) -> Option<ExecMode> {
        let (engine, browser) = match &launcher.launcher_type {
            LauncherType::Web(w) => (Some(w.engine.clone()), w.browser.clone()),
            LauncherType::Bookmarks(b) => (None, Some(b.target_browser.to_string())),
            _ => return None,
        };

        Some(ExecMode::Web {
            engine,
            browser: browser.map(Into::into),
            exec: self.url.clone(),
        })
    }

    fn content(&self, meta: &ItemMeta, field: Option<&str>) -> Option<String> {
        match field {
            Some("exec") => self.url.as_deref().map(str::to_owned),
            _ => Some(meta.name.to_string()),
        }
    }
}

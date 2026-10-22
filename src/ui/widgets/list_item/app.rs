use std::sync::Arc;

use gpui::{AnyElement, IntoElement, SharedString};
use serde::{Deserialize, Serialize};

use super::{ListItem, Payload};
use crate::launcher::{LauncherConfig, utils::exec_mode::ExecMode};
use sidebar::AppDataSidebar;

mod sidebar;

pub type AppItem = ListItem<AppPayload>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppPayload {
    pub exec: SharedString,
    pub terminal: bool,
    pub desktop_file: Arc<std::path::Path>,
    pub original_name: SharedString,
}

impl Payload for AppPayload {
    const HAS_SIDEBAR: bool = true;
    const HIDEABLE: bool = true;
    fn exec(&self, _meta: &super::ItemMeta, _launcher: &Arc<LauncherConfig>) -> Option<ExecMode> {
        Some(ExecMode::App {
            exec: self.exec.clone().into(),
            terminal: self.terminal,
        })
    }

    fn set_exec(&mut self, exec: SharedString) {
        self.exec = exec;
    }

    fn content(&self, meta: &super::ItemMeta, field: Option<&str>) -> Option<String> {
        match field {
            Some("exec") => Some(self.exec.to_string()),
            _ => Some(meta.name.to_string()),
        }
    }

    fn sidebar(
        &self,
        meta: &super::ItemMeta,
        launcher: &Arc<LauncherConfig>,
    ) -> Option<AnyElement> {
        Some(AppDataSidebar::new(meta, self, launcher).into_any_element())
    }
}

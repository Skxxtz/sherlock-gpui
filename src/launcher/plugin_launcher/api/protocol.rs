use std::{path::Path, sync::Arc};

use crate::launcher::plugin_launcher::ui_schema::PluginTileContent;
use crate::utils::errors::types::PluginAction;

pub enum PluginDeferFunction {
    Update {
        plugin_id: Arc<Path>,
        tile_id: String,
        node: Box<PluginTileContent>,
    },
    WriteClipboard(String),
    Error {
        plugin: String,
        action: PluginAction,
        message: String,
    },
}

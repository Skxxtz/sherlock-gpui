use std::{path::Path, sync::Arc};

use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

pub enum PluginDeferFunction {
    Update {
        plugin_id: Arc<Path>,
        tile_id: String,
        node: Box<PluginUiNode>,
    },
    WriteClipboard(String),
}

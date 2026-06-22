use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

pub enum PluginDeferFunction {
    Update {
        tile_id: String,
        node: Box<PluginUiNode>,
    },
    WriteClipboard(String),
}

use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

// ui/widgets/plugin_tile_state.rs
use gpui::Context;

pub struct PluginTileState {
    pub data: Option<PluginUiNode>,
    pub loading: bool,
    pub error: Option<String>,
}

impl PluginTileState {
    pub fn set_data(&mut self, data: PluginUiNode, cx: &mut Context<Self>) {
        self.data = Some(data);
        self.loading = false;
        self.error = None;
        cx.notify();
    }

    pub fn set_error(&mut self, err: String, cx: &mut Context<Self>) {
        self.error = Some(err);
        self.loading = false;
        cx.notify();
    }
}

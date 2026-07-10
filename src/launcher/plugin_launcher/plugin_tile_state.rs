use gpui::{Context, Task};

use crate::launcher::plugin_launcher::ui_schema::PluginUiNode;

#[derive(Default)]
pub struct PluginTileState {
    pub data: Option<Box<PluginUiNode>>,
    pub loading: bool,
    pub error: Option<String>,
    pub update_task: Option<Task<()>>,
}

impl PluginTileState {
    pub fn set_data(&mut self, data: Box<PluginUiNode>, cx: &mut Context<Self>) {
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

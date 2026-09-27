use gpui::{Context, Task};

use crate::launcher::plugin_launcher::ui_schema::PluginTileContent;

#[derive(Default)]
pub struct PluginTileState {
    pub data: Option<Box<PluginTileContent>>,
    pub loading: bool,
    pub error: Option<String>,
    pub update_task: Option<Task<()>>,
}

impl PluginTileState {
    pub fn set_data(&mut self, data: Box<PluginTileContent>, cx: &mut Context<Self>) -> bool {
        let was_hidden = self.data.as_ref().is_some_and(|d| d.meta.hidden);
        let changed = was_hidden != data.meta.hidden;
        self.data = Some(data);
        self.loading = false;
        self.error = None;
        cx.notify();
        changed
    }

    pub fn set_error(&mut self, err: String, cx: &mut Context<Self>) {
        self.error = Some(err);
        self.loading = false;
        cx.notify();
    }
}

use gpui::{IntoElement, RenderOnce, SharedString, Styled};

use crate::{
    app::theme::ActiveTheme,
    ui::{
        launcher::sidebar::wrapper::Sidebar,
        widgets::process::{ProcessData, ProcessMeta},
    },
};

#[derive(IntoElement)]
pub struct ProcessSidebar {
    meta: ProcessMeta,
    name: SharedString,
}

impl ProcessSidebar {
    pub fn new(value: &ProcessData, meta: ProcessMeta) -> Self {
        ProcessSidebar {
            meta,
            name: value.name.clone(),
        }
    }
}
impl RenderOnce for ProcessSidebar {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let m = &self.meta;

        Sidebar::new(theme)
            .min_h_full()
            .header(m.icon.as_ref(), self.name.clone(), std::iter::empty())
            .location(m.exe.clone())
            .separator()
            .section("Info")
            .row("Parent PID", m.ppid.to_string())
            .row("PID", m.pid.to_string())
            .separator()
            .section("Snapshot")
            .row("Memory", format!("{:.1} MB", m.memory_mb))
            .row("Memory Peak", format!("{:.1} MB", m.mem_peak))
            .row("Threads", m.threads.to_string())
    }
}

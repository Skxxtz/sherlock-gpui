use gpui::SharedString;

use crate::launcher::utils::exec_mode::ExecMode;

#[derive(Clone, Debug)]
pub struct CategoryPayload {
    pub exec: SharedString,
}

impl super::Payload for CategoryPayload {
    fn exec(
        &self,
        meta: &super::ItemMeta,
        launcher: &std::sync::Arc<crate::launcher::LauncherConfig>,
    ) -> Option<ExecMode> {
        Some(ExecMode::Category {
            category: crate::ui::launcher::LauncherMode::Alias {
                short: self.exec.clone(),
                name: meta.name.clone(),
                launcher: launcher.clone(),
            },
        })
    }
}

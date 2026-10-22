use gpui::SharedString;

use crate::launcher::utils::exec_mode::ExecMode;

#[derive(Clone, Debug)]
pub struct CategoryPayload {
    pub exec: SharedString,
}

impl super::Payload for CategoryPayload {
    fn exec(
        &self,
        _meta: &super::ItemMeta,
        _launcher: &std::sync::Arc<crate::launcher::LauncherConfig>,
    ) -> Option<ExecMode> {
        Some(ExecMode::Command {
            exec: self.exec.to_string(),
        })
    }
}

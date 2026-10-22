use std::sync::Arc;

use crate::launcher::{LauncherConfig, utils::exec_mode::ExecMode};

use super::{ItemMeta, Payload};

/// Payload for a launcher's own entry. Builds its `ExecMode` on demand.
#[derive(Clone, Copy)]
pub struct FixedPayload(pub fn(&Arc<LauncherConfig>) -> ExecMode);

impl Payload for FixedPayload {
    fn exec(&self, _meta: &ItemMeta, launcher: &Arc<LauncherConfig>) -> Option<ExecMode> {
        Some((self.0)(launcher))
    }
}

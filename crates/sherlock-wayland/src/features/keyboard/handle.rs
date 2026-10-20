use std::sync::Arc;

use super::{Layout, Shared};

#[derive(Clone)]
pub struct LayoutHandle(pub(super) Shared);

impl LayoutHandle {
    /// Active layout, or `None` before the first keymap arrived.
    /// Load once per search, then call `pos()` per character.
    pub fn load(&self) -> Option<Arc<Layout>> {
        self.0.read().ok()?.clone()
    }
}

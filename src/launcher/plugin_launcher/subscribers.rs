use gpui::WeakEntity;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::launcher::plugin_launcher::plugin_tile_state::PluginTileState;

#[derive(Clone)]
pub struct TileSubscribersGlobal(pub TileSubscribers);
impl gpui::Global for TileSubscribersGlobal {}

#[derive(Clone, Default)]
pub struct TileSubscribers {
    inner: Arc<Mutex<HashMap<String, WeakEntity<PluginTileState>>>>,
}

impl TileSubscribers {
    pub fn register(&self, tile_id: String, entity: WeakEntity<PluginTileState>) {
        self.inner.lock().unwrap().insert(tile_id, entity);
    }

    pub fn unregister(&self, tile_id: &str) {
        self.inner.lock().unwrap().remove(tile_id);
    }

    pub fn get(&self, tile_id: &str) -> Option<WeakEntity<PluginTileState>> {
        self.inner.lock().unwrap().get(tile_id).cloned()
    }
}

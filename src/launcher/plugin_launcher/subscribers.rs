use gpui::{Entity, WeakEntity};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::launcher::plugin_launcher::plugin_tile_state::PluginTileState;

#[derive(Clone)]
pub struct TileSubscribersGlobal(pub TileSubscribers);
impl gpui::Global for TileSubscribersGlobal {}

/// Tiles are keyed by `(plugin path, tile_id)` so plugins can reuse tile ids.
type TileKey = (Arc<Path>, String);

#[derive(Clone, Default)]
pub struct TileSubscribers {
    inner: Arc<Mutex<HashMap<TileKey, WeakEntity<PluginTileState>>>>,
}

impl TileSubscribers {
    pub fn register(
        &self,
        plugin: Arc<Path>,
        tile_id: String,
        entity: WeakEntity<PluginTileState>,
    ) {
        self.inner.lock().unwrap().insert((plugin, tile_id), entity);
    }

    /// Removes the entry only if it still points at `entity`, so dropping a
    /// stale widget can't unregister its replacement.
    pub fn unregister(&self, plugin: &Arc<Path>, tile_id: &str, entity: &Entity<PluginTileState>) {
        let mut map = self.inner.lock().unwrap();
        let key = (plugin.clone(), tile_id.to_string());
        if map
            .get(&key)
            .is_some_and(|weak| weak.entity_id() == entity.entity_id())
        {
            map.remove(&key);
        }
    }

    /// Removes every tile belonging to `plugin`.
    pub fn clear_plugin(&self, plugin: &Path) {
        self.inner
            .lock()
            .unwrap()
            .retain(|(p, _), _| p.as_ref() != plugin);
    }

    pub fn get(&self, plugin: &Arc<Path>, tile_id: &str) -> Option<WeakEntity<PluginTileState>> {
        self.inner
            .lock()
            .unwrap()
            .get(&(plugin.clone(), tile_id.to_string()))
            .cloned()
    }
}

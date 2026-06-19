use mlua::prelude::*;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

#[derive(Default)]
pub struct PluginRegistry {
    plugins: HashMap<PathBuf, LoadedPlugin>,
}

pub struct LoadedPlugin {
    pub env_key: LuaRegistryKey,
}

impl PluginRegistry {
    /// Returns `Ok(())` on success, `Err(())` if already loaded.
    pub fn insert(&mut self, path: &Path, env_key: LuaRegistryKey) -> Result<(), ()> {
        if self.plugins.contains_key(path) {
            return Err(());
        }
        self.plugins
            .insert(path.to_owned(), LoadedPlugin { env_key });
        Ok(())
    }

    pub fn get(&self, path: &Path) -> Option<&LoadedPlugin> {
        self.plugins.get(path)
    }

    pub fn remove(&mut self, path: &Path) -> Option<LoadedPlugin> {
        self.plugins.remove(path)
    }

    pub fn is_loaded(&self, path: &Path) -> bool {
        self.plugins.contains_key(path)
    }
}

use mlua::prelude::*;
use std::collections::HashMap;

pub struct LoadedPlugin {
    pub env_key: LuaRegistryKey,
    pub name: String,
}

#[derive(Default)]
pub struct PluginRegistry {
    next_id: u64,
    plugins: HashMap<u64, LoadedPlugin>,
}

impl PluginRegistry {
    pub fn insert(&mut self, name: String, env_key: LuaRegistryKey) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.plugins.insert(id, LoadedPlugin { env_key, name });
        id
    }

    pub fn get(&self, id: u64) -> Option<&LoadedPlugin> {
        self.plugins.get(&id)
    }

    pub fn remove(&mut self, id: u64) -> Option<LoadedPlugin> {
        self.plugins.remove(&id)
    }
}

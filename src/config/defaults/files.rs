use std::{path::Path, sync::Arc};

use crate::{config::types::CfgPath, utils::paths};

pub struct FileDefaults {}
impl FileDefaults {
    pub fn cache() -> Arc<Path> {
        paths::get_cache_dir()
            .unwrap()
            .join("desktop_files.bin")
            .into()
    }
    pub fn config() -> Arc<Path> {
        paths::get_config_dir().unwrap().join("config.toml").into()
    }
    pub fn fallback() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("fallback.json")
            .into()
    }
    pub fn alias() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlock_alias.json")
            .into()
    }
    pub fn ignore() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlockignore")
            .into()
    }
    pub fn actions() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlock_actions.json")
            .into()
    }
    pub fn icon_paths() -> Vec<CfgPath> {
        vec![paths::get_config_dir().unwrap().join("icons/").into()]
    }
}

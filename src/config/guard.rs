use std::sync::{LazyLock, RwLock, RwLockReadGuard, RwLockWriteGuard};

use crate::{
    CONFIG,
    config::SherlockConfig,
    sherlock_msg,
    utils::errors::{SherlockMessage, types::ErrorKind},
};

pub struct ConfigGuard;
impl<'g> ConfigGuard {
    fn get_config() -> Result<&'g RwLock<SherlockConfig>, SherlockMessage> {
        CONFIG.get().ok_or_else(|| {
            sherlock_msg!(
                Error,
                ErrorKind::Config("Failed to get global CONFIG singleton.".into()),
                "Config not initialized"
            )
        })
    }

    fn get_read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockMessage> {
        Self::get_config()?.read().map_err(|_| {
            sherlock_msg!(
                Warning,
                ErrorKind::Config("Failed get immutable CONFIG singleton.".into()),
                "Failed to acquire write lock on config"
            )
        })
    }

    fn get_write() -> Result<RwLockWriteGuard<'g, SherlockConfig>, SherlockMessage> {
        Self::get_config()?.write().map_err(|_| {
            sherlock_msg!(
                Warning,
                ErrorKind::Config("Failed to get mutable CONFIG singleton.".into()),
                "Failed to acquire write lock on config"
            )
        })
    }

    pub fn read() -> Result<RwLockReadGuard<'g, SherlockConfig>, SherlockMessage> {
        Self::get_read()
    }

    pub fn read_with<F, R>(key_fn: F) -> Result<R, SherlockMessage>
    where
        F: FnOnce(&SherlockConfig) -> R,
    {
        let config = Self::get_read()?;
        Ok(key_fn(&config))
    }
    /// Reads from the config, or from the defaults while there is none.
    pub fn get<F, R>(key_fn: F) -> R
    where
        F: FnOnce(&SherlockConfig) -> R,
    {
        static DEFAULT: LazyLock<SherlockConfig> = LazyLock::new(SherlockConfig::default);
        match Self::get_read() {
            Ok(config) => key_fn(&config),
            Err(_) => key_fn(&DEFAULT),
        }
    }

    pub fn write_with<F>(key_fn: F) -> Result<(), SherlockMessage>
    where
        F: FnOnce(&mut SherlockConfig),
    {
        let mut config = Self::get_write()?;
        key_fn(&mut config);
        Ok(())
    }
}

impl ConfigGuard {
    pub fn is_initialized() -> bool {
        Self::get_read().is_ok_and(|s| s.initialized)
    }
}

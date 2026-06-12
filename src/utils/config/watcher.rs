use std::{path::Path, time::SystemTime};

#[cfg(feature = "nixos")]
use crate::loader::application_loader;
use crate::{
    loader::application_loader::ApplicationLoader,
    sherlock_msg,
    utils::{
        config::ConfigGuard,
        errors::{
            SherlockMessage,
            types::{DirAction, SherlockErrorType},
        },
    },
};

/// This struct aims at providing an audit function to check for config file changes and
/// application data changes. This should be run on every startup.
pub struct ConfigWatcher {
    latest_audit: SystemTime,
    root_dir: Box<Path>,
    last_audit_result: ConfigFileChange,
}

impl ConfigWatcher {
    pub fn new(root_dir: Box<Path>) -> Self {
        Self {
            latest_audit: SystemTime::now(),
            root_dir,
            last_audit_result: ConfigFileChange::empty(),
        }
    }

    pub fn audit(&mut self) -> Result<ConfigFileChange, SherlockMessage> {
        let current_audit_time = SystemTime::now();
        let since = self.latest_audit;
        self.last_audit_result = ConfigFileChange::empty();

        // check desktop files
        let app_change =
            (!ApplicationLoader::get_new_apps(since).is_empty()).then_some(ConfigFileChange::Apps);

        // get entries
        let entries = std::fs::read_dir(&self.root_dir).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DirError(DirAction::Read, self.root_dir.to_path_buf()),
                e
            )
        })?;

        let files = ConfigGuard::read()
            .map(|c| c.files.clone())
            .unwrap_or_default();

        // collect out-of-date entries
        entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                #[cfg(feature = "nixos")]
                if application_loader::nixos::system_creation_time().is_some_and(|t| t > since) {
                    return true;
                }

                entry
                    .metadata()
                    .and_then(|m| m.modified())
                    // TODO: make nixos-home-manager compatible
                    .map(|modified| entry.path().is_file() && modified > since)
                    .unwrap_or(false)
            })
            .map(|entry| {
                let path_buf = entry.path().to_path_buf();
                match path_buf {
                    _ if path_buf == files.config.as_ref() => ConfigFileChange::Config,
                    _ if path_buf == files.fallback.as_ref() => ConfigFileChange::Fallback,
                    _ if path_buf == files.alias.as_ref() => ConfigFileChange::Alias,
                    _ if path_buf == files.ignore.as_ref() => ConfigFileChange::Ignore,
                    _ if path_buf == files.actions.as_ref() => ConfigFileChange::Actions,
                    _ => ConfigFileChange::Other,
                }
            })
            .chain(app_change)
            .for_each(|change| self.last_audit_result |= change);

        self.latest_audit = current_audit_time;

        Ok(self.last_audit_result)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ConfigFileChange: u8 {
        const Actions  = 1 << 0;
        const Alias    = 1 << 1;
        const Apps     = 1 << 2;
        const Config   = 1 << 3;
        const Ignore   = 1 << 4;
        const Fallback = 1 << 5;
        const Other    = 1 << 6;
    }
}

impl ConfigFileChange {
    #[inline(always)]
    pub fn config(&self) -> bool {
        self.contains(ConfigFileChange::Config)
    }
    #[inline(always)]
    pub fn aliases(&self) -> bool {
        self.contains(ConfigFileChange::Alias)
    }
    #[inline(always)]
    pub fn ignores(&self) -> bool {
        self.contains(ConfigFileChange::Ignore)
    }
    #[inline(always)]
    pub fn apps(&self) -> bool {
        self.contains(ConfigFileChange::Apps)
    }
    #[inline(always)]
    pub fn launchers(&self) -> bool {
        self.contains(ConfigFileChange::Fallback | ConfigFileChange::Actions)
    }
}

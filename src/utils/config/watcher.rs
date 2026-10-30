use std::{path::Path, time::SystemTime};

#[cfg(feature = "nixos")]
use crate::loader::application_loader;
use crate::{
    loader::application_loader::ApplicationLoader,
    sherlock_msg,
    utils::{
        config::{ConfigGuard, writer::config_sources},
        errors::{
            SherlockMessage,
            types::{DirAction, SherlockErrorType},
        },
        files::modified_since,
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

        // if nix has rebuilt since last audit
        #[cfg(feature = "nixos")]
        let nix_rebuilt =
            application_loader::nixos::system_creation_time().is_some_and(|t| t > since);
        #[cfg(not(feature = "nixos"))]
        let nix_rebuilt = false;

        let entries = std::fs::read_dir(&self.root_dir).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DirError(DirAction::Read, self.root_dir.to_path_buf()),
                e
            )
        })?;

        let (config_file, fallback_file, alias_file, ignore_file, actions_file, cache_file) =
            ConfigGuard::read().map(|c| {
                (
                    c.files.config.clone(),
                    c.files.fallback.clone(),
                    c.files.alias.clone(),
                    c.files.ignore.clone(),
                    c.files.actions.clone(),
                    c.caching.cache.clone(),
                )
            })?;

        // all files pulled in via [[source]], wherever they live
        let sources = config_sources(&config_file);
        let source_change = sources
            .iter()
            .any(|p| nix_rebuilt || modified_since(p, since))
            .then_some(ConfigFileChange::Config);

        // collect out-of-date entries
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && (nix_rebuilt || modified_since(path, since)))
            .map(|path| match path {
                _ if path == config_file.as_ref() => ConfigFileChange::Config,
                _ if path == fallback_file.as_ref() => ConfigFileChange::Fallback,
                _ if path == alias_file.as_ref() => ConfigFileChange::Alias,
                _ if path == ignore_file.as_ref() => ConfigFileChange::Ignore,
                _ if path == actions_file.as_ref() => ConfigFileChange::Actions,
                _ if path == cache_file.as_ref() => ConfigFileChange::Cache,
                _ if sources.contains(&path) => ConfigFileChange::Config,
                _ => ConfigFileChange::Other,
            })
            .chain(app_change)
            .chain(source_change)
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
        const Cache    = 1 << 6;
        const Other    = 1 << 7;
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
    pub fn cache(&self) -> bool {
        self.contains(ConfigFileChange::Cache)
    }
    #[inline(always)]
    pub fn launchers(&self) -> bool {
        self.intersects(ConfigFileChange::Fallback | ConfigFileChange::Actions)
    }
}

use std::{collections::HashSet, path::Path, time::SystemTime};

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
}

impl ConfigWatcher {
    pub fn new(root_dir: Box<Path>) -> Self {
        Self {
            latest_audit: SystemTime::now(),
            root_dir,
        }
    }

    pub fn audit(&mut self) -> Result<HashSet<ConfigFileChange>, SherlockMessage> {
        let current_audit_time = SystemTime::now();
        let since = self.latest_audit;

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
        let changes: HashSet<ConfigFileChange> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .metadata()
                    .and_then(|m| m.modified())
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
            .collect();

        self.latest_audit = current_audit_time;

        Ok(changes)
    }
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub enum ConfigFileChange {
    Actions,
    Alias,
    Apps,
    Config,
    Ignore,
    Fallback,
    Other,
}

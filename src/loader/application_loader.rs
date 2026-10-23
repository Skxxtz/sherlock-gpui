use glob::Pattern;
use rayon::prelude::*;
use std::collections::HashSet;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

use crate::loader::application_loader::parser::DesktopFileParser;
use crate::prelude::PathHelpers;
use crate::ui::widgets::list_item::app::AppItem;
use crate::utils::cache::BinaryCache;
use crate::utils::config::{ConfigFileChange, SherlockAlias, SherlockIgnore};
use crate::utils::errors::types::{FileAction, SherlockErrorType};
use crate::utils::files::{expand_path, home_dir_owned};
use crate::utils::{config::ConfigGuard, errors::SherlockMessage};

mod parser;

#[cfg(feature = "nixos")]
pub mod nixos;

pub struct ApplicationLoader;
impl ApplicationLoader {
    /// Loads and synchronizes the application registry from disk and cache.
    ///
    /// Uses following paths as a reference for the `.desktop` files:
    /// * XDG_DATA_HOME (or `~/.local/share/`)
    /// * XDG_DATA_DIRS (or `/usr/share/applications/` and `/usr/local/share/`),
    /// * User-specified locations using `SherlockConfig.debug.app_paths`
    ///
    /// Update Strategy:
    /// 1. **Cache**: Read the `BinaryCache`. A missing, corrupt or outdated cache is
    ///    treated as empty, which forces a full parse — the cache is never load-critical.
    /// 2. **Discovery**: Identifies `.desktop` files modified since the last cache write.
    /// 3. **Differential Loading**: Only parses new or changed files from disk, while
    ///    recycling valid entries from the cache.
    /// 4. **Smart Persistence**: Spawns a background task to update the cache if
    ///    any discrepancies (stale entries or new files) are detected.
    ///
    /// Returns an `Arc<Vec<AppItem>>` for zero-copy sharing across threads.
    ///
    /// # Errors
    /// Returns a `SherlockMessage` if configuration is unreachable or disk parsing
    /// fails critically.
    pub fn load_applications(
        use_keywords: bool,
        changes: Option<ConfigFileChange>,
    ) -> Result<Arc<Vec<AppItem>>, SherlockMessage> {
        // Don't hold the config lock while parsing (the parser reads it too).
        let cache_path: Arc<Path> = ConfigGuard::read()?.caching.cache.clone();

        let cached: Option<Vec<AppItem>> = match BinaryCache::read(&cache_path) {
            Ok(apps) => Some(apps),
            Err(e) => {
                if !matches!(
                    e.error_type,
                    SherlockErrorType::FileError(FileAction::Find, _)
                ) {
                    eprintln!("app cache unusable, rebuilding: {}", e.error_type);
                }
                None
            }
        };

        // No usable cache, or changed ignore rules → reparse everything.
        let force_full = cached.is_none() || changes.is_some_and(|c| c.ignores());
        let last_cached = (!force_full)
            .then(|| cache_path.modtime())
            .flatten()
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let need_update = Self::get_new_apps(last_cached);
        let new_apps = Self::load_applications_from_disk(&need_update, use_keywords)?;

        let Some(cached) = cached else {
            let apps = Arc::new(new_apps);
            Self::spawn_cache_write(cache_path, apps.clone());
            return Ok(apps);
        };

        let cached_len = cached.len();
        let update_lookup = LookupCache::new(&need_update);

        let all_apps: Arc<Vec<AppItem>> = Arc::new(
            cached
                .into_iter()
                .filter(|item| {
                    let path = &*item.payload.desktop_file;
                    path.exists() && !update_lookup.contains(path)
                })
                .chain(new_apps)
                .collect(),
        );

        if !need_update.is_empty() || all_apps.len() != cached_len {
            Self::spawn_cache_write(cache_path, all_apps.clone());
        }

        Ok(all_apps)
    }

    /// `.desktop` files modified after `last_cache`.
    pub fn get_new_apps(last_cache: SystemTime) -> Vec<PathBuf> {
        // On NixOS all store timestamps are UNIX_EPOCH (deterministic builds), so compare
        // against the system generation's creation time instead.
        #[cfg(feature = "nixos")]
        let force_refresh = nixos::system_creation_time().is_some_and(|system| system > last_cache);
        #[cfg(not(feature = "nixos"))]
        let force_refresh = false;

        Self::desktop_files()
            .filter(|path| force_refresh || path.modtime().is_some_and(|t| t > last_cache))
            .collect()
    }

    /// All `.desktop` files in the application directories.
    pub fn get_desktop_files() -> Arc<[PathBuf]> {
        Self::desktop_files().collect()
    }

    fn desktop_files() -> impl Iterator<Item = PathBuf> {
        Self::get_applications_dir()
            .into_iter()
            .flat_map(|dir| dir.read_dir().into_iter().flatten())
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                p.extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("desktop"))
            })
    }

    fn load_applications_from_disk(
        files: &[PathBuf],
        use_keywords: bool,
    ) -> Result<Vec<AppItem>, SherlockMessage> {
        if files.is_empty() {
            return Ok(Vec::new());
        }

        let (ignore, aliases) = {
            let config = ConfigGuard::read()?;
            (
                SherlockIgnore::load(&config.files.ignore)?,
                RwLock::new(SherlockAlias::load(&config.files.alias)?),
            )
        };
        let parser = DesktopFileParser::new(&ignore, use_keywords);

        Ok(files
            .par_iter()
            .filter_map(|path| parser.parse(path, &aliases))
            .collect())
    }

    fn spawn_cache_write(path: Arc<Path>, apps: Arc<Vec<AppItem>>) {
        rayon::spawn_fifo(move || {
            if let Err(e) = BinaryCache::write(&path, &*apps) {
                eprintln!("{}", e.error_type);
            }
        });
    }

    pub fn get_applications_dir() -> Vec<PathBuf> {
        let home = home_dir_owned().unwrap_or_default();
        let xdg_data_home = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| home.join(".local/share"));

        let xdg_data_dirs =
            env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".into());

        let mut dirs: Vec<PathBuf> = xdg_data_dirs
            .split(':')
            .map(|p| PathBuf::from(p).join("applications"))
            .chain(std::iter::once(xdg_data_home))
            .collect();

        if let Ok(c) = ConfigGuard::read() {
            dirs.extend(c.debug.app_paths.iter().map(|p| expand_path(p, &home)));
        }

        dirs.retain(|p| p.exists());
        dirs
    }
}

fn should_ignore(ignore_apps: &[Pattern], app: &str) -> bool {
    let app_name = app.to_lowercase();
    ignore_apps.iter().any(|pattern| pattern.matches(&app_name))
}

pub fn file_has_changed(file_path: &Path, compare_to: &Path) -> bool {
    match (file_path.modtime(), compare_to.modtime()) {
        (Some(t1), Some(t2)) => t1 >= t2, // newer (or same) → treat as changed
        _ => true,                        // a modtime is missing
    }
}

impl PathHelpers for Path {
    fn modtime(&self) -> Option<SystemTime> {
        let meta = self.metadata().ok()?;
        meta.modified().or_else(|_| meta.created()).ok()
    }
}

/// Adaptive contains-check. Usually only a handful of `.desktop` files change, where a
/// linear scan beats hashing; for large sets (first run, big package update) use a
/// `HashSet` for O(1) lookups.
pub enum LookupCache<'a> {
    Set(HashSet<&'a Path>),
    Slice(&'a [PathBuf]),
}

impl<'a> LookupCache<'a> {
    pub fn new(input: &'a [PathBuf]) -> Self {
        if input.len() > 25 {
            Self::Set(input.iter().map(PathBuf::as_path).collect())
        } else {
            Self::Slice(input)
        }
    }

    pub fn contains(&self, path: &Path) -> bool {
        match self {
            Self::Set(s) => s.contains(path),
            Self::Slice(v) => v.iter().any(|p| p == path),
        }
    }
}

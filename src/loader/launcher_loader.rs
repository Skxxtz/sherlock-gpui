use gpui::App;
use std::{
    collections::HashMap,
    io::ErrorKind,
    path::{Path, PathBuf},
    rc::Rc,
    sync::Arc,
};

use crate::{
    app::LauncherEntity,
    launcher::{Launcher, LauncherConfig, variant_type::LauncherType},
    loader::utils::RawLauncher,
    sherlock_msg,
    ui::launcher::LauncherMode,
    utils::{
        cache::BinaryCache,
        config::{ConfigFileChange, ConfigGuard},
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
    },
};

use super::Loader;
use super::utils::CounterReader;

pub struct LoadContext {
    pub counts: HashMap<String, u16>,
    pub path: PathBuf,
    pub changes: Option<ConfigFileChange>,
}
impl LoadContext {
    fn new(changes: Option<ConfigFileChange>) -> Result<Self, SherlockMessage> {
        let counter_reader = CounterReader::new()?;
        let counts: HashMap<String, u16> =
            BinaryCache::read(&counter_reader.path).unwrap_or_default();

        Ok(Self {
            counts,
            path: counter_reader.path,
            changes,
        })
    }
}

pub struct LauncherLoadResult {
    pub modes: Arc<[LauncherMode]>,
    pub messages: Vec<SherlockMessage>,
}
impl Loader {
    pub fn load_launchers(
        cx: &mut App,
        data_handle: LauncherEntity,
        changes: Option<ConfigFileChange>,
    ) -> Result<LauncherLoadResult, SherlockMessage> {
        // read config
        let config = ConfigGuard::read()?;

        // Read fallback data here:
        let (raw_launchers, mut messages) = parse_launcher_configs(&config.files.fallback);

        // Read cached counter file
        let ctx = LoadContext::new(changes)?;

        // Parse the launchers
        let mut launchers: Vec<(Arc<LauncherConfig>, Arc<serde_json::Value>)> = raw_launchers
            .into_iter()
            .map(|raw| {
                let launcher_type: LauncherType = raw.r#type.into_launcher_type(&raw);

                let icon = raw
                    .args
                    .get("icon")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string());

                let opts = Arc::clone(&raw.args);

                (
                    Arc::new(LauncherConfig::from_raw(raw, launcher_type, icon)),
                    opts,
                )
            })
            .collect();

        launchers.sort_by_key(|(l, _)| l.priority);

        let mut modes = Vec::with_capacity(launchers.len());
        let renders: Vec<Launcher> = launchers
            .into_iter()
            .inspect(|(launcher, _)| {
                // Collect modes
                if let Some((alias, name)) = launcher.alias.as_ref().zip(launcher.name.as_ref()) {
                    modes.push(LauncherMode::Alias {
                        short: alias.into(),
                        name: name.into(),
                        launcher: launcher.clone(),
                    });
                }
            })
            .filter_map(|(launcher, opts)| {
                let children = match launcher.launcher_type.get_render_obj(
                    Arc::clone(&launcher),
                    &ctx,
                    opts,
                    &mut messages,
                    cx,
                ) {
                    Ok(vec) => (!vec.is_empty()).then_some(vec),
                    Err(e) => {
                        messages.push(e);
                        None
                    }
                }?;

                Some(Launcher {
                    config: launcher,
                    children,
                })
            })
            .collect();

        Self::sync_cache_if_empty(&ctx, &renders, &mut messages);

        data_handle.update(cx, |items, cx| {
            *items = Rc::new(renders);
            cx.notify();
        });

        Ok(LauncherLoadResult {
            modes: Arc::from(modes),
            messages,
        })
    }

    fn sync_cache_if_empty(
        ctx: &LoadContext,
        renders: &[Launcher],
        warnings: &mut Vec<SherlockMessage>,
    ) {
        if ctx.counts.is_empty() {
            let counts: HashMap<String, u16> = renders
                .iter()
                .flat_map(|l| l.children.iter())
                .filter_map(|render| render.get_exec())
                .map(|exec| (exec, 0))
                .collect();
            if let Err(e) = BinaryCache::write(&ctx.path, &counts) {
                warnings.push(e)
            };
        }
    }
}

/// Incrementally parses launchers from the `fallback.json` file.
///
/// Each launcher is deserialized individually. If an entry is invalid—for instance,
/// due to an unknown `LauncherVariant`—a warning is appended to the
/// returned list and the specific launcher is skipped, allowing the rest
/// of the configuration to load.
///
/// # Returns
/// A tuple containing the successfully parsed `Vec<RawLauncher>` and
/// a `Vec<SherlockError>` containing any collected warnings.
fn parse_launcher_configs<P: AsRef<Path>>(p: P) -> (Vec<RawLauncher>, Vec<SherlockMessage>) {
    let path = p.as_ref();
    let mut warnings = Vec::new();
    let mut launchers = Vec::new();

    let raw_bytes: Vec<u8> = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            warnings.push(sherlock_msg!(
                Info,
                SherlockErrorType::FileError(FileAction::Find, path.to_path_buf()),
                "Using default fallback.json configuration."
            ));
            include_bytes!("../../assets/fallback.json").to_vec()
        }
        Err(e) => {
            warnings.push(sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            ));
            return (launchers, warnings);
        }
    };

    let mut buffer = raw_bytes;
    let raw_values: Vec<serde_json::Value> = match simd_json::from_slice(&mut buffer) {
        Ok(v) => v,
        Err(e) => {
            warnings.push(sherlock_msg!(
                Warning,
                SherlockErrorType::DeserializationError(path.to_string_lossy().to_string()),
                e
            ));
            return (launchers, warnings);
        }
    };

    for value in raw_values.into_iter() {
        match serde_json::from_value::<RawLauncher>(value) {
            Ok(launcher) => launchers.push(launcher),
            Err(e) => {
                warnings.push(sherlock_msg!(
                    Warning,
                    SherlockErrorType::ConfigError("Invalid launcher configuration".into()),
                    e
                ));
            }
        }
    }

    (launchers, warnings)
}

use std::{path::Path, sync::Arc};

use gpui::{App, AppContext, AsyncApp};

use super::{SherlockConfig, watcher::ConfigFileChange};
use crate::{
    app::{LauncherEntity, LauncherViewGlobal},
    launcher::variant_type::LauncherType,
    loader::{Loader, RawLaunchers},
    ui::{launcher::LauncherMode, widgets::ChildKind},
    utils::{
        config::{ConfigGuard, SherlockAlias},
        errors::SherlockMessage,
    },
};

/// `reload` with the file reading and parsing on a background thread; only swapping in
/// the results runs on the main thread.
pub async fn reload_async(
    cx: &mut AsyncApp,
    data: &LauncherEntity,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: ConfigFileChange,
) -> Option<Arc<[LauncherMode]>> {
    let fallback = ConfigGuard::read_with(|c| c.files.fallback.clone()).ok();
    let (config, raw) = cx
        .background_spawn(async move { read_changes(changes, fallback) })
        .await;
    cx.update(|cx| finish(cx, data, initial_messages, changes, config, raw))
}

/// Reloads what `changes` names, on the calling thread.
pub fn reload(
    cx: &mut App,
    data: &LauncherEntity,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: ConfigFileChange,
) -> Option<Arc<[LauncherMode]>> {
    let fallback = ConfigGuard::read_with(|c| c.files.fallback.clone()).ok();
    let (config, raw) = read_changes(changes, fallback);
    finish(cx, data, initial_messages, changes, config, raw)
}

type LoadedConfig = (SherlockConfig, Vec<SherlockMessage>);

/// The file part of a reload (no app access): the new config, if it changed, and the parsed
/// launchers, if they need a reload. `fallback` is the current launcher file.
fn read_changes(
    changes: ConfigFileChange,
    fallback: Option<Arc<Path>>,
) -> (Option<LoadedConfig>, Option<RawLaunchers>) {
    let config = changes.config().then(load_config).flatten();
    let needs_launchers =
        changes.launchers() || changes.apps() || changes.ignores() || changes.cache();
    let raw = needs_launchers
        .then(|| {
            // a new config may point at another launcher file
            let path = config
                .as_ref()
                .map(|(c, _)| c.files.fallback.clone())
                .or(fallback)?;
            Some(Loader::read_launchers(&path))
        })
        .flatten();
    (config, raw)
}

/// Reads the config file again, with the flags Sherlock was started with.
fn load_config() -> Option<LoadedConfig> {
    let mut flags = Loader::load_flags()?.flags;
    Some(match flags.get_config() {
        Ok(loaded) => loaded,
        Err(e) => {
            let mut cfg = SherlockConfig::default();
            cfg.apply_flags(&mut flags);
            (cfg, vec![e])
        }
    })
}

/// The app part of a reload: swaps in the config, rebuilds the launchers, applies aliases.
fn finish(
    cx: &mut App,
    data: &LauncherEntity,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: ConfigFileChange,
    config: Option<LoadedConfig>,
    raw: Option<RawLaunchers>,
) -> Option<Arc<[LauncherMode]>> {
    let mut messages: Vec<SherlockMessage> = Vec::new();

    if let Some((config, msgs)) = config {
        messages.extend(msgs);
        // Update global config
        if let Err(e) = ConfigGuard::write_with(|cfg| *cfg = config) {
            messages.push(e);
        }
    }

    // Reload launchers
    let modes =
        raw.and_then(
            |raw| match Loader::build_launchers(cx, data.clone(), Some(changes), raw) {
                Ok(result) => {
                    // Only updates when the reload was started with the open launcher.
                    if let Some(view) = cx
                        .try_global::<LauncherViewGlobal>()
                        .and_then(|g| g.0.upgrade())
                    {
                        view.update(cx, |view, cx| {
                            view.modes = result.modes.clone();
                            cx.notify();
                        });
                    }
                    messages.extend(result.messages);
                    Some(result.modes)
                }
                Err(e) => {
                    messages.push(e);
                    None
                }
            },
        );

    // reload aliases
    if changes.aliases() {
        let _ = reload_aliases(data, cx);
    }

    *initial_messages = messages;
    modes
}

fn reload_aliases<C: AppContext>(data: &LauncherEntity, cx: &mut C) -> Result<(), SherlockMessage> {
    let alias_path = ConfigGuard::read_with(|cfg| cfg.files.alias.clone())?;
    let mut aliases = SherlockAlias::load(&alias_path)?;
    data.update(cx, |this, _cx| {
        for launcher in this.make_mut().values_mut() {
            let LauncherType::Apps(apps) = &launcher.config.launcher_type else {
                continue;
            };
            let use_keywords = apps.use_keywords;

            for child in launcher.children.values_mut() {
                if let ChildKind::App(item) = &mut child.kind
                    && let Some(alias) = aliases.remove(item.payload.original_name.as_str())
                {
                    item.apply_alias(Some(alias), None, use_keywords);
                }
            }
        }
    });
    Ok(())
}

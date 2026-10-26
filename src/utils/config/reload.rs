use std::sync::Arc;

use gpui::{App, AppContext, AsyncApp};

use super::{SherlockConfig, watcher::ConfigFileChange};
use crate::{
    CONFIG,
    app::LauncherEntity,
    launcher::variant_type::LauncherType,
    loader::Loader,
    ui::{launcher::LauncherMode, widgets::ChildKind},
    utils::{
        config::{ConfigGuard, SherlockAlias},
        errors::SherlockMessage,
    },
};

pub fn reload_async(
    cx: &mut AsyncApp,
    data: &LauncherEntity,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: ConfigFileChange,
) -> Option<Arc<[LauncherMode]>> {
    cx.update(|cx| reload(cx, data, initial_messages, changes))
}

pub fn reload(
    cx: &mut App,
    data: &LauncherEntity,
    initial_messages: &mut Vec<SherlockMessage>,
    changes: ConfigFileChange,
) -> Option<Arc<[LauncherMode]>> {
    let mut messages: Vec<SherlockMessage> = Vec::new();

    if changes.config() {
        let mut flags = Loader::load_flags()?.flags;
        let config = match flags.get_config() {
            Err(e) => {
                messages.push(e);
                let mut cfg = SherlockConfig::default();
                cfg.apply_flags(&mut flags);
                cfg
            }
            Ok((cfg, msgs)) => {
                messages.extend(msgs);
                cfg
            }
        };
        // Update global config
        if let Ok(mut guard) = CONFIG.get()?.write() {
            *guard = config;
        }
    }

    // Reload launchers
    let modes = if changes.launchers() || changes.apps() || changes.ignores() || changes.cache() {
        let result = match Loader::load_launchers(cx, data.clone(), Some(changes)) {
            Ok(result) => result,
            Err(e) => {
                messages.push(e);
                return None;
            }
        };
        messages.extend(result.messages);
        Some(result.modes)
    } else {
        None
    };

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

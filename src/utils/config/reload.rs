use std::{rc::Rc, sync::Arc};

use gpui::AsyncApp;

use super::{SherlockConfig, watcher::ConfigFileChange};
use crate::{
    CONFIG,
    app::RenderableChildEntity,
    launcher::variant_type::LauncherType,
    loader::{Loader, application_loader::ApplicationLoader},
    ui::{launcher::LauncherMode, widgets::RenderableChild},
    utils::{config::ConfigGuard, errors::SherlockMessage},
};

pub fn reload(
    cx: &mut AsyncApp,
    data: &RenderableChildEntity,
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
    let modes = if changes.launchers() || changes.apps() || changes.ignores() {
        let result = match cx.update(|cx| Loader::load_launchers(cx, data.clone(), Some(changes))) {
            Ok(result) => result,
            Err(e) => {
                messages.push(e);
                return None;
            }
        };
        messages.extend(result.messages);
        Some(result.modes)
    } else {
        None // caller keeps existing modes
    };

    // reload aliases
    if changes.aliases() {
        let _ = reload_aliases(data, cx);
    }

    *initial_messages = messages;
    modes
}

fn reload_aliases(data: &RenderableChildEntity, cx: &mut AsyncApp) -> Result<(), SherlockMessage> {
    let alias_path = ConfigGuard::read_with(|cfg| cfg.files.alias.clone())?;
    let mut aliases = ApplicationLoader::load_aliases(&alias_path)?;
    data.update(cx, |this, _cx| {
        for c in Rc::make_mut(this).iter_mut() {
            if let RenderableChild::App { inner, launcher } = c
                && let Some(alias) = inner.original_name.as_ref().and_then(|n| aliases.remove(n))
                && let LauncherType::Apps(launcher) = &launcher.launcher_type
            {
                inner.apply_alias_raw(alias, launcher.use_keywords);
            }
        }
    });
    Ok(())
}

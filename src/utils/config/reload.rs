use std::{collections::HashSet, rc::Rc, sync::Arc};

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
    changes: HashSet<ConfigFileChange>,
) -> Option<Arc<[LauncherMode]>> {
    let needs = ReloadNeeds::from_changes(&changes);
    let mut messages: Vec<SherlockMessage> = Vec::new();

    if needs.config {
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
    let modes = if needs.launchers || needs.apps {
        let result = match cx.update(|cx| Loader::load_launchers(cx, data.clone())) {
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
    if needs.aliases {
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

#[derive(Default)]
struct ReloadNeeds {
    config: bool,
    launchers: bool,
    aliases: bool,
    apps: bool,
}

impl ReloadNeeds {
    fn from_changes(changes: &HashSet<ConfigFileChange>) -> Self {
        changes.iter().fold(Self::default(), |mut needs, change| {
            match change {
                ConfigFileChange::Config => needs.config = true,
                ConfigFileChange::Fallback
                | ConfigFileChange::Actions
                | ConfigFileChange::Ignore => needs.launchers = true,
                ConfigFileChange::Alias => needs.aliases = true,
                ConfigFileChange::Apps => needs.apps = true,
                ConfigFileChange::Other => {}
            }
            needs
        })
    }
}

use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
    sync::Arc,
};

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::{
    launcher::{Launcher, variant_type::LauncherType},
    loader::{
        IconType, resolve_icon_path,
        utils::{ApplicationAction, ExecVariable, PriorityGuard, SherlockAlias, construct_search},
    },
    ui::launcher::context_menu::ContextMenuAction,
};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct AppData {
    #[serde(default)]
    pub name: Option<SharedString>,
    pub exec: Option<String>,
    pub search_string: String,
    #[serde(default)]
    pub priority: PriorityGuard, // to enable new count instantly having effect
    pub icon: Option<IconType>,
    pub desktop_file: Option<PathBuf>,
    pub original_name: Option<String>,
    #[serde(default)]
    pub actions: Arc<[Arc<ContextMenuAction>]>,
    #[serde(default)]
    #[serde(rename = "variables")]
    pub vars: Vec<ExecVariable>,
    #[serde(default)]
    pub terminal: bool,
}
impl Eq for AppData {}
impl Hash for AppData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Make more efficient and handle error using f32
        self.exec.hash(state);
        self.desktop_file.hash(state);
    }
}
impl AppData {
    pub fn new() -> Self {
        Self {
            name: None,
            exec: None,
            search_string: String::new(),
            priority: PriorityGuard::default(),
            icon: None,
            desktop_file: None,
            original_name: None,
            actions: Arc::new([]),
            vars: vec![],
            terminal: false,
        }
    }

    pub fn apply_alias_raw(&mut self, alias: SherlockAlias, use_keywords: bool) {
        if let Some(name) = alias.name {
            self.name = Some(name.into());
        }

        if let Some(icon) = alias.icon {
            self.icon = resolve_icon_path(&icon);
        }

        if let Some(exec) = alias.exec {
            self.exec = Some(exec);
        }

        if let Some(vars) = alias.variables {
            self.vars.extend(vars);
        }

        if let Some(keywords) = alias.keywords {
            self.search_string = construct_search(self.name.as_deref(), &keywords, use_keywords);
        }

        if let Some(add_actions) = alias.add_actions {
            self.actions = self
                .actions
                .iter()
                .cloned()
                .chain(add_actions.into_iter().map(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    a.into()
                }))
                .collect()
        }

        if let Some(actions) = alias.actions {
            self.actions = actions
                .into_iter()
                .map(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    a.into()
                })
                .collect();
        }
    }

    pub fn apply_alias(
        &mut self,
        launcher: &Arc<Launcher>,
        alias: Option<SherlockAlias>,
        use_keywords: bool,
        mut buffer: Vec<ApplicationAction>,
    ) {
        let Some(alias) = alias else {
            let name: Option<&str> = self
                .name
                .as_ref()
                .map(|s| s.as_str())
                .or(launcher.name.as_ref().map(|s| s.as_str()));
            self.search_string = construct_search(name, &self.search_string, use_keywords);
            self.actions = buffer.into_iter().map(Into::into).collect();
            return;
        };

        if let Some(name) = alias.name {
            self.name = Some(name.into());
        }

        if let Some(icon) = alias.icon {
            self.icon = resolve_icon_path(&icon);
        }

        if let Some(exec) = alias.exec {
            self.exec = Some(exec);
        }

        if let Some(vars) = alias.variables {
            self.vars.extend(vars);
        }

        let name: Option<&str> = self
            .name
            .as_ref()
            .map(|s| s.as_str())
            .or(launcher.name.as_ref().map(|s| s.as_str()));
        if let Some(alias_keywords) = alias.keywords.as_ref() {
            self.search_string = construct_search(name, alias_keywords, use_keywords);
        } else {
            self.search_string = construct_search(name, &self.search_string, use_keywords);
        }

        if let Some(add_actions) = alias.add_actions {
            add_actions.into_iter().for_each(|mut a| {
                if a.icon.is_none() {
                    a.icon = self.icon.clone();
                }
                buffer.push(a);
            });
        }

        if let Some(actions) = alias.actions {
            self.actions = actions
                .into_iter()
                .map(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    a.into()
                })
                .collect();
        } else {
            self.actions = buffer
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>()
                .into();
        }
    }
    pub fn get_exec(&self, launcher: &Arc<Launcher>) -> Option<String> {
        match &launcher.launcher_type {
            LauncherType::Web(web) => Some(format!("websearch-{}", web.engine)),

            LauncherType::Apps(_) | LauncherType::Commands(_) | LauncherType::Categories(_) => {
                self.exec.clone()
            }

            // None-Home Launchers
            LauncherType::Calculator(_) => None,
            _ => None,
        }
    }
}

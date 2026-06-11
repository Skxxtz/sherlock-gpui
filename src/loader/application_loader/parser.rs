use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, RwLock},
};

use glob::Pattern;
use gpui::SharedString;

use crate::{
    launcher::{Launcher, app_launcher::app_data::AppData},
    loader::{
        application_loader::should_ignore,
        resolve_icon_path,
        utils::{ApplicationAction, SherlockAlias},
    },
    utils::files::read_lines,
};

#[derive(PartialEq, Eq)]
enum Section {
    DesktopEntry,
    Action,
    Other,
}

impl Section {
    #[inline]
    fn from_header(header: &str) -> Self {
        match header {
            "Desktop Entry" => Self::DesktopEntry,
            h if h.starts_with("Desktop Action") => Self::Action,
            _ => Self::Other,
        }
    }
}

pub struct DesktopFileParser<'a> {
    launcher: &'a Arc<Launcher>,
    ignore: &'a [Pattern],
    counts: &'a HashMap<String, u16>,
    use_keywords: bool,
}

impl<'a> DesktopFileParser<'a> {
    pub fn new(
        launcher: &'a Arc<Launcher>,
        ignore: &'a [Pattern],
        counts: &'a HashMap<String, u16>,
        use_keywords: bool,
    ) -> Self {
        Self {
            launcher,
            ignore,
            counts,
            use_keywords,
        }
    }

    pub fn parse(
        &self,
        path: &Path,
        aliases: &RwLock<HashMap<String, SherlockAlias>>,
    ) -> Option<AppData> {
        let content = read_lines(path.to_str()?).ok()?;

        let mut data = AppData {
            desktop_file: Some(path.into()),
            ..AppData::new()
        };
        let mut actions: Vec<ApplicationAction> = Vec::new();
        let mut current_action = ApplicationAction::new("app_launcher", "");
        let mut section = Section::Other;
        let mut key_buf = String::with_capacity(32);

        for line in content.map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // handle section headers
            if let Some(header) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                if section == Section::Action && current_action.is_valid() {
                    actions.push(current_action);
                    current_action = ApplicationAction::new("app_launcher", "");
                }
                section = Section::from_header(header);
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let value = value.trim();

            key_buf.clear();
            key_buf.extend(key.trim().chars().map(|c| c.to_ascii_lowercase()));

            match section {
                Section::DesktopEntry => {
                    if !self.handle_entry_field(&key_buf, value, &mut data)? {
                        // false means skip file
                        return None;
                    }
                }
                Section::Action => {
                    self.handle_action_field(&key_buf, value, &data, &mut current_action);
                    if current_action.is_full() {
                        actions.push(current_action);
                        current_action = ApplicationAction::new("app_launcher", "");
                        section = Section::Other;
                    }
                }
                Section::Other => {}
            }
        }

        // flush
        if section == Section::Action && current_action.is_valid() {
            actions.push(current_action);
        }

        let alias = {
            aliases
                .write()
                .unwrap()
                .remove(data.name.as_ref()?.as_str())
        };
        data.apply_alias(self.launcher, alias, self.use_keywords, actions);

        let count = data
            .exec
            .as_ref()
            .and_then(|exec| self.counts.get(exec))
            .copied()
            .unwrap_or(0);

        data.priority.set_launcher(self.launcher, count);

        Some(data)
    }

    /// Returns `Some(true)` to continue, `Some(false)` to skip file, `None` on hard error. Using
    /// the return value avoids a flag variable.
    #[inline]
    fn handle_entry_field(&self, key: &str, value: &str, data: &mut AppData) -> Option<bool> {
        match key {
            "name" if should_ignore(self.ignore, value) => {
                return Some(false);
            }
            "name" => {
                let name = value.to_string();
                data.name = Some(SharedString::from(name.clone()));
                data.original_name = Some(name);
            }
            "icon" => data.icon = resolve_icon_path(value),
            "exec" => data.exec = Some(value.to_string()),
            "terminal" => data.terminal = value.eq_ignore_ascii_case("true"),
            "keywords" if self.use_keywords => {
                data.search_string = value.to_lowercase();
            }
            "nodisplay" | "hidden" if value.eq_ignore_ascii_case("true") => return Some(false),
            _ => {}
        }

        Some(true)
    }

    #[inline]
    fn handle_action_field(
        &self,
        key: &str,
        value: &str,
        data: &AppData,
        current_action: &mut ApplicationAction,
    ) {
        match key {
            "name" => current_action.name = SharedString::from(value.to_string()),
            "exec" => current_action.exec = Some(value.to_string()),
            "icon" => current_action.icon = resolve_icon_path(value),
            _ => {}
        }

        if current_action.icon.is_none() {
            current_action.icon = data.icon.clone();
        }
    }
}

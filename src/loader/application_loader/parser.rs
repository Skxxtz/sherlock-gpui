use std::{collections::HashMap, path::Path, sync::RwLock};

use glob::Pattern;
use gpui::SharedString;

use crate::{
    loader::{
        IconType, application_loader::should_ignore, resolve_icon_path, utils::ApplicationAction,
    },
    ui::widgets::list_item::{ItemMeta, ListItem, app::AppPayload},
    utils::{config::SherlockAlias, files::read_lines},
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

/// Raw `[Desktop Entry]` fields, collected before the item is built.
#[derive(Default)]
struct Entry {
    name: Option<SharedString>,
    icon: Option<IconType>,
    exec: Option<SharedString>,
    terminal: bool,
    keywords: String,
}

enum Field {
    Continue,
    Skip,
}

pub struct DesktopFileParser<'a> {
    ignore: &'a [Pattern],
    use_keywords: bool,
}

impl<'a> DesktopFileParser<'a> {
    pub fn new(ignore: &'a [Pattern], use_keywords: bool) -> Self {
        Self {
            ignore,
            use_keywords,
        }
    }

    pub fn parse(
        &self,
        path: &Path,
        aliases: &RwLock<HashMap<String, SherlockAlias>>,
    ) -> Option<ListItem<AppPayload>> {
        let content = read_lines(path.to_str()?).ok()?;

        let mut entry = Entry::default();
        let mut actions: Vec<ApplicationAction> = Vec::new();
        let mut current_action = ApplicationAction::new("app_launcher", "");
        let mut section = Section::Other;
        let mut key_buf = String::with_capacity(32);

        for line in content.map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

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
                    if let Field::Skip = self.entry_field(&key_buf, value, &mut entry) {
                        return None;
                    }
                }
                Section::Action => {
                    Self::action_field(&key_buf, value, &mut current_action);
                    if current_action.is_full() {
                        actions.push(current_action);
                        current_action = ApplicationAction::new("app_launcher", "");
                        section = Section::Other;
                    }
                }
                Section::Other => {}
            }
        }

        if section == Section::Action && current_action.is_valid() {
            actions.push(current_action);
        }

        // An app without name or exec can't be shown or launched.
        let name = entry.name?;
        let exec = entry.exec?;

        let mut item = ListItem {
            meta: ItemMeta {
                icon: entry.icon,
                actions: actions.into_iter().map(Into::into).collect(),
                ..ItemMeta::new(name.clone())
            },
            payload: AppPayload {
                exec,
                terminal: entry.terminal,
                desktop_file: path.into(),
                original_name: name.clone(),
            },
        };

        let alias = aliases.write().ok()?.remove(name.as_ref());
        item.apply_alias(alias, Some(&entry.keywords), self.use_keywords);
        // After the alias, so actions inherit an alias-overridden icon.
        item.inherit_icons(None);

        Some(item)
    }

    #[inline]
    fn entry_field(&self, key: &str, value: &str, entry: &mut Entry) -> Field {
        match key {
            "name" if should_ignore(self.ignore, value) => return Field::Skip,
            "name" => entry.name = Some(value.to_owned().into()),
            "icon" => entry.icon = resolve_icon_path(value),
            "exec" => entry.exec = Some(value.to_owned().into()),
            "terminal" => entry.terminal = value.eq_ignore_ascii_case("true"),
            "keywords" if self.use_keywords => entry.keywords = value.to_lowercase(),
            "nodisplay" | "hidden" if value.eq_ignore_ascii_case("true") => return Field::Skip,
            _ => {}
        }
        Field::Continue
    }

    #[inline]
    fn action_field(key: &str, value: &str, action: &mut ApplicationAction) {
        match key {
            "name" => action.name = value.to_owned().into(),
            "exec" => action.exec = Some(value.to_owned()),
            "icon" => action.icon = resolve_icon_path(value),
            _ => {}
        }
    }
}

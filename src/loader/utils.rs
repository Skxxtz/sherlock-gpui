use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    borrow::Cow,
    fmt::Debug,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use crate::utils::config::HomeType;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ApplicationAction {
    pub name: Option<String>,
    pub exec: Option<String>,
    pub icon: Option<String>,
    pub method: String,
    #[serde(default = "default_true")]
    pub exit: bool,
}
impl ApplicationAction {
    pub fn new(method: &str) -> Self {
        Self {
            name: None,
            exec: None,
            icon: None,
            method: method.to_string(),
            exit: true,
        }
    }
    pub fn is_valid(&self) -> bool {
        self.name.is_some() && self.exec.is_some()
    }
    pub fn is_full(&self) -> bool {
        self.name.is_some() && self.exec.is_some() && self.icon.is_some()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct AppData {
    #[serde(default)]
    pub name: String,
    pub exec: Option<String>,
    pub search_string: String,
    #[serde(default)]
    pub priority: f32,
    pub icon: Option<String>,
    pub icon_class: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub desktop_file: Option<PathBuf>,
    #[serde(default)]
    pub actions: Vec<ApplicationAction>,
    #[serde(default)]
    #[serde(rename = "variables")]
    pub vars: Vec<ExecVariable>,
    #[serde(default)]
    pub terminal: bool,
}
impl AppData {
    pub fn new() -> Self {
        Self {
            name: String::new(),
            exec: None,
            search_string: String::new(),
            priority: 0.0,
            icon: None,
            icon_class: None,
            tag_start: None,
            tag_end: None,
            desktop_file: None,
            actions: vec![],
            vars: vec![],
            terminal: false,
        }
    }
    pub fn new_for_theme<'a, T, S>(name: T, path: Option<S>, raw: &RawLauncher) -> Self
    where
        T: Into<Cow<'a, str>>,
        S: Into<Cow<'a, str>>,
    {
        let name: Cow<'a, str> = name.into();
        let path = path.map(|s| s.into().into_owned());
        let name_string = name.into_owned();
        let icon = raw
            .args
            .get("icon")
            .and_then(|s| s.as_str())
            .unwrap_or("sherlock-devtools")
            .to_string();
        Self {
            name: name_string.clone(),
            exec: path,
            search_string: name_string,
            priority: raw.priority,
            icon: Some(icon),
            icon_class: None,
            tag_start: None,
            tag_end: None,
            desktop_file: None,
            actions: vec![],
            vars: vec![],
            terminal: false,
        }
    }
    pub fn from_raw_launcher(raw: &RawLauncher) -> Self {
        let search_string = format!(
            "{};{}",
            raw.name.as_deref().unwrap_or_default(),
            raw.args
                .get("search_string")
                .and_then(Value::as_str)
                .unwrap_or_default()
        );
        let mut data = Self {
            name: raw.name.clone().unwrap_or_default(),
            exec: Default::default(),
            search_string,
            priority: raw.priority,
            icon: None,
            icon_class: None,
            tag_start: raw.tag_start.clone(),
            tag_end: raw.tag_end.clone(),
            desktop_file: None,
            actions: raw.actions.clone().unwrap_or_default(),
            vars: raw.variables.clone().unwrap_or_default(),
            terminal: false,
        };
        data.icon_class = raw
            .args
            .get("icon_class")
            .and_then(Value::as_str)
            .map(|s| s.to_string());
        data.icon = raw
            .args
            .get("icon")
            .and_then(Value::as_str)
            .map(|s| s.to_string());

        data
    }
    pub fn with_priority(mut self, priority: f32) -> Self {
        self.priority = priority;
        self
    }
    pub fn apply_alias(&mut self, alias: Option<SherlockAlias>, use_keywords: bool) {
        if let Some(alias) = alias {
            if let Some(alias_name) = alias.name.as_ref() {
                self.name = alias_name.to_string();
            }
            if let Some(alias_icon) = alias.icon.as_ref() {
                self.icon = Some(alias_icon.to_string());
            }
            if let Some(alias_keywords) = alias.keywords.as_ref() {
                self.search_string =
                    Self::construct_search(&self.name, &alias_keywords, use_keywords);
            } else {
                self.search_string =
                    Self::construct_search(&self.name, &self.search_string, use_keywords);
            }
            if let Some(alias_exec) = alias.exec.as_ref() {
                self.exec = Some(alias_exec.to_string());
            }
            if let Some(add_actions) = alias.add_actions {
                add_actions.into_iter().for_each(|mut a| {
                    if a.icon.is_none() {
                        a.icon = self.icon.clone();
                    }
                    self.actions.push(a);
                });
            }
            if let Some(actions) = alias.actions {
                self.actions = actions
                    .into_iter()
                    .map(|mut a| {
                        if a.icon.is_none() {
                            a.icon = self.icon.clone();
                        }
                        a
                    })
                    .collect();
            }
            if let Some(variables) = alias.variables {
                self.vars.extend(variables);
            }
        } else {
            self.search_string =
                Self::construct_search(&self.name, &self.search_string, use_keywords);
        }
    }
    fn construct_search(name: &str, search_str: &str, use_keywords: bool) -> String {
        if use_keywords {
            format!("{};{}", name, search_str)
        } else {
            name.to_string()
        }
    }
}
impl AppData {
    pub fn name(&self) -> String {
        self.name.clone()
    }
}
impl Eq for AppData {}
impl Hash for AppData {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Make more efficient and handle error using f32
        self.exec.hash(state);
        self.desktop_file.hash(state);
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct SherlockAlias {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub exec: Option<String>,
    pub keywords: Option<String>,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
    pub variables: Option<Vec<ExecVariable>>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecVariable {
    StringInput(String),
    PasswordInput(String),
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RawLauncher {
    pub name: Option<String>,
    pub alias: Option<String>,
    pub tag_start: Option<String>,
    pub tag_end: Option<String>,
    pub display_name: Option<String>,
    pub on_return: Option<String>,
    pub next_content: Option<String>,
    pub r#type: String,
    pub priority: f32,

    #[serde(default = "default_true")]
    pub exit: bool,
    #[serde(default = "default_true")]
    pub shortcut: bool,
    #[serde(default = "default_true")]
    pub spawn_focus: bool,
    #[serde(default)]
    pub r#async: bool,
    #[serde(default)]
    pub home: HomeType,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(default)]
    pub actions: Option<Vec<ApplicationAction>>,
    #[serde(default)]
    pub add_actions: Option<Vec<ApplicationAction>>,
    #[serde(default)]
    pub variables: Option<Vec<ExecVariable>>,
}

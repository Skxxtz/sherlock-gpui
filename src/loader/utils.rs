use gpui::SharedString;
use serde::{Deserialize, Deserializer, Serialize};
use std::{
    collections::{BTreeSet, HashMap},
    fmt::Debug,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

use crate::{
    launcher::{Launcher, utils::binds::BindSerde, variant_type::LauncherVariant},
    loader::{IconType, resolve_icon_path},
    sherlock_msg,
    ui::choice::ChoiceOption,
    utils::{
        cache::BinaryCache,
        config::HomeType,
        errors::{
            SherlockMessage,
            types::{DirAction, SherlockErrorType},
        },
        paths,
    },
};

#[derive(PartialEq, PartialOrd, Serialize, Deserialize, Copy, Clone, Debug)]
pub struct Priority {
    pub base: u16,
    pub count: u16,
}
impl Priority {
    pub fn new(base: u16, count: u16) -> Self {
        Self { base, count }
    }
    pub fn new_with_launcher(launcher: &Launcher, count: u16) -> Self {
        Self {
            base: launcher.priority,
            count,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PriorityGuard(Arc<RwLock<Priority>>);

impl PriorityGuard {
    pub fn new(base: u16, count: u16) -> Self {
        Self(Arc::new(RwLock::new(Priority::new(base, count))))
    }
    pub fn new_with_launcher(launcher: &Launcher, count: u16) -> Self {
        Self(Arc::new(RwLock::new(Priority::new_with_launcher(
            launcher, count,
        ))))
    }

    pub fn get(&self) -> Priority {
        *self.0.read().unwrap()
    }

    pub fn set_count(&self, count: u16) {
        self.0.write().unwrap().count = count;
    }

    pub fn increment_count(&self) {
        self.0.write().unwrap().count += 1;
    }

    pub fn set_launcher(&self, launcher: &Launcher, count: u16) {
        let mut inner = self.0.write().unwrap();
        inner.base = launcher.priority;
        inner.count = count;
    }
}

impl Default for PriorityGuard {
    fn default() -> Self {
        Self(Arc::new(RwLock::new(Priority { base: 1, count: 0 })))
    }
}

impl PartialEq for PriorityGuard {
    fn eq(&self, other: &Self) -> bool {
        self.get() == other.get()
    }
}

impl PartialOrd for PriorityGuard {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.get().partial_cmp(&other.get())
    }
}

impl Serialize for PriorityGuard {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.get().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PriorityGuard {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Priority::deserialize(deserializer).map(|inner| Self(Arc::new(RwLock::new(inner))))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ApplicationAction {
    pub name: SharedString,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconType>,

    pub method: String,

    #[serde(default = "default_true")]
    pub exit: bool,
}

impl ApplicationAction {
    pub fn new(method: &str, name: &str) -> Self {
        Self {
            name: name.to_string().into(),
            exec: None,
            icon: None,
            method: method.to_string(),
            exit: true,
        }
    }
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && self.exec.is_some()
    }
    pub fn is_full(&self) -> bool {
        !self.name.is_empty() && self.exec.is_some() && self.icon.is_some()
    }

    pub fn name(mut self, name: impl Into<SharedString>) -> Self {
        self.name = name.into();
        self
    }
    pub fn icon(mut self, icon: IconType) -> Self {
        self.icon = Some(icon);
        self
    }
    pub fn icon_name(mut self, icon_name: &str) -> Self {
        self.icon = resolve_icon_path(icon_name);
        self
    }
    pub fn exec(mut self, exec: String) -> Self {
        self.exec = Some(exec);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Default)]
pub struct ApplicationActionSerde {
    pub name: SharedString,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    pub method: String,

    #[serde(default = "default_true")]
    pub exit: bool,
}

impl From<ApplicationActionSerde> for ApplicationAction {
    fn from(value: ApplicationActionSerde) -> Self {
        Self {
            name: value.name,
            exec: value.exec,
            icon: value.icon.as_deref().and_then(resolve_icon_path),
            method: value.method,
            exit: value.exit,
        }
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
pub enum ExecVariable {
    #[serde(rename = "string_input")]
    String(SharedString),
    #[serde(rename = "password_input")]
    Password(SharedString),
    #[serde(rename = "path_input")]
    Path(PathData),
    #[serde(rename = "command_input")]
    Command(CommandData),
    #[serde(rename = "choice")]
    Choice {
        name: SharedString,
        choices: Arc<[ChoiceOption]>,
    },
}

/// A path placeholder that deserializes from a plain string.
/// The `index` field tracks cursor position in the UI and is not persisted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "SharedString")]
pub struct PathData {
    pub path: SharedString,
    #[serde(skip)]
    pub index: usize,
}

// Implement the conversion logic
impl From<SharedString> for PathData {
    fn from(path: SharedString) -> Self {
        Self { path, index: 0 }
    }
}

/// A command placeholder that deserializes from a plain string.
/// The `index` field tracks cursor position in the UI and is not persisted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(from = "SharedString")]
pub struct CommandData {
    pub command: SharedString,
    #[serde(skip)]
    pub index: usize,
    #[serde(skip)]
    pub is_scoped: bool,
}

// Implement the conversion logic
impl From<SharedString> for CommandData {
    fn from(command: SharedString) -> Self {
        Self {
            command,
            index: 0,
            is_scoped: false,
        }
    }
}

impl ExecVariable {
    pub fn placeholder(&self) -> SharedString {
        match self {
            Self::String(s) => s.clone(),
            Self::Path(p) => p.path.clone(),
            Self::Password(s) => s.clone(),
            Self::Command(c) => c.command.clone(),
            Self::Choice { name, .. } => name.clone(),
        }
    }
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Debug, Serialize)]
pub struct RawLauncher {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_return: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_content: Option<String>,

    pub r#type: LauncherVariant,
    pub priority: u16,

    #[serde(default)]
    pub limit: Option<u16>,

    #[serde(default = "default_true")]
    #[serde(skip_serializing_if = "is_true")]
    pub exit: bool,

    #[serde(default = "default_true")]
    #[serde(skip_serializing_if = "is_true")]
    pub shortcut: bool,

    #[serde(default = "default_true")]
    #[serde(skip_serializing_if = "is_true")]
    pub spawn_focus: bool,

    #[serde(default)]
    #[serde(skip_serializing_if = "is_default")]
    pub home: HomeType,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binds: Option<Vec<BindSerde>>,

    #[serde(default)]
    pub args: Arc<serde_json::Value>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<ApplicationActionSerde>>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_actions: Option<Vec<ApplicationActionSerde>>,

    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<ExecVariable>>,
}

fn is_default<T: Default + PartialEq>(t: &T) -> bool {
    t == &T::default()
}
fn is_true(t: &bool) -> bool {
    *t
}

/// Persists and normalizes application launch counts across sessions.
///
/// On every increment, counts are re-ranked to contiguous integers (1, 2, 3...)
/// rather than raw hit counts. This prevents frequently-used apps from
/// dominating the sort order unboundedly over time.
pub struct CounterReader {
    pub path: PathBuf,
}
impl CounterReader {
    pub fn new() -> Result<Self, SherlockMessage> {
        let data_dir = paths::get_data_dir()?;
        let path = data_dir.join("counts.bin");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DirError(DirAction::Create, parent.to_path_buf()),
                    e
                )
            })?;
        }
        Ok(CounterReader { path })
    }
    /// Re-ranks all existing counts to contiguous values before incrementing,
    /// so the ordering stays stable regardless of absolute hit counts.
    pub fn increment(&self, key: &str) -> Result<(), SherlockMessage> {
        let mut content: HashMap<String, u16> = BinaryCache::read(&self.path)?;

        let val = content.entry(key.to_string()).or_insert(0);

        if *val == u16::MAX {
            // compress all values to 1..=n preserving relative order
            let unique: Vec<u16> = content
                .values()
                .copied()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let rank: HashMap<u16, u16> = unique
                .iter()
                .enumerate()
                .map(|(i, &v)| (v, (i + 1) as u16))
                .collect();
            content.values_mut().for_each(|v| *v = rank[v]);
        } else {
            *val += 1;
        }

        BinaryCache::write(&self.path, &content)
    }
}

/// Builds the search string used for fuzzy matching.
///
/// If `use_keywords` is true, produces `"name;keywords"` — the semicolon
/// separates the display name from the keyword blob so both are searchable.
/// If false, only the name is used.
pub fn construct_search(name: Option<&str>, search_str: &str, use_keywords: bool) -> String {
    let mut s = if use_keywords {
        let name_val = name.unwrap_or("");
        let mut s = String::with_capacity(name_val.len() + 1 + search_str.len());
        s.push_str(name_val);
        s.push(';');
        s.push_str(search_str);
        s
    } else {
        name.unwrap_or_default().to_string()
    };

    s.make_ascii_lowercase();
    s
}

pub fn deserialize_path_buf<'de, D>(deserializer: D) -> Result<PathBuf, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if let Some(stripped) = s.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        return Ok(PathBuf::from(home).join(stripped.trim_start_matches('/')));
    }

    Ok(PathBuf::from(s))
}

pub fn deserialize_arc_path<'de, D>(deserializer: D) -> Result<Arc<Path>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;

    if let Some(stripped) = s.strip_prefix('~')
        && let Ok(home) = std::env::var("HOME")
    {
        return Ok(PathBuf::from(home)
            .join(stripped.trim_start_matches('/'))
            .into());
    }

    Ok(PathBuf::from(s).into())
}

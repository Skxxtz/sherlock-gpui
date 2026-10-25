pub mod utils;
pub mod variant_type;

use slotmap::{SlotMap, new_key_type};
use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use gpui::{App, SharedString};
pub use variant_type::{LauncherType, LauncherVariant};

use crate::{
    features::dmenu::DmenuLauncher,
    launcher::{utils::binds::Bind, variant_type::InnerFunction},
    loader::{
        IconType, LoadContext, resolve_icon_path,
        utils::{ApplicationAction, ApplicationActionSerde, Priority, RawLauncher},
    },
    sherlock_msg,
    ui::{launcher::context_menu::ContextMenuAction, widgets::RenderableChild},
    utils::{
        config::HomeType,
        errors::{SherlockMessage, types::SherlockErrorType},
    },
};

pub trait LauncherProvider {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage>;
    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        ctx: &LoadContext,
        opts: Arc<serde_json::Value>,
        messages: &mut Vec<SherlockMessage>,
        cx: &mut App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage>;
    fn binds(&self) -> Option<Arc<Vec<Bind>>> {
        None
    }
    fn execute_function(
        &self,
        func: InnerFunction,
        _child: &RenderableChild,
        _variables: &[(SharedString, SharedString)],
        _cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        Err(sherlock_msg!(
            Warning,
            SherlockErrorType::InvalidFunction,
            format!("{} does not provide function: {:?}", stringify!(self), func)
        ))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LauncherConfig {
    /// The name of the launcher. Might get displayed in the widget
    pub name: Option<SharedString>,

    /// May not apply to all widgets
    pub icon: Option<IconType>,

    /// A short alias like `app` to launcher launcher-specific search (`alias` => only show items
    /// belonging to that launcher)
    pub alias: Option<String>,

    /// The action to be executed when the user executes a widget
    pub on_return: Option<String>,

    /// If true, Sherlock will close after the widget was exectued
    pub exit: bool,

    /// Sorting weight for display order. Lower values appear first, 0 appears only in alias mode
    pub priority: u16,

    /// The maximum number of items to show for this launcher
    pub limit: Option<u16>,

    /// Determines when to show the widgets
    pub home: HomeType,

    /// The category and functional variant for the launcher
    pub launcher_type: LauncherType,

    /// If true, enables UI shortcut for this widgets
    pub shortcut: bool,

    /// If true, allows the sidebar to be toggled on without the info action. Note: this will still
    /// only show the sidebar if it actually has data connted to it.
    pub sidebar_default_on: bool,

    /// If true, this widget can spawn focus
    pub spawn_focus: bool,

    /// The list of primary actions. This will overwrite actions defined in possible desktop files
    pub actions: Option<Arc<[Arc<ContextMenuAction>]>>,

    /// The list of supplementary actions that extend the primary actions
    pub add_actions: Option<Arc<[Arc<ContextMenuAction>]>>,
}

new_key_type! { pub struct LocalId; }

#[derive(Clone, Debug, Default)]
pub struct Children {
    slots: SlotMap<LocalId, RenderableChild>,
    order: Vec<LocalId>,
}

impl Children {
    pub fn get(&self, id: LocalId) -> Option<&RenderableChild> {
        self.slots.get(id)
    }
    pub fn first(&self) -> Option<&RenderableChild> {
        self.slots.get(*self.order.first()?)
    }
    pub fn len(&self) -> usize {
        self.order.len()
    }
    pub fn iter(&self) -> impl Iterator<Item = &RenderableChild> + '_ {
        self.order.iter().filter_map(|&id| self.slots.get(id))
    }
    pub fn iter_with_ids(&self) -> impl Iterator<Item = (LocalId, &RenderableChild)> + '_ {
        self.order
            .iter()
            .filter_map(|&id| self.slots.get(id).map(|c| (id, c)))
    }
    pub fn ids(&self) -> impl ExactSizeIterator<Item = LocalId> + '_ {
        self.order.iter().copied()
    }
    pub fn values_mut(&mut self) -> impl Iterator<Item = &mut RenderableChild> + '_ {
        self.slots.values_mut()
    }
    pub fn push(&mut self, c: RenderableChild) -> LocalId {
        let id = self.slots.insert(c);
        self.order.push(id);
        id
    }
    pub fn remove(&mut self, id: LocalId) -> Option<RenderableChild> {
        let c = self.slots.remove(id)?;
        self.order.retain(|&i| i != id);
        Some(c)
    }
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            slots: SlotMap::with_capacity_and_key(cap),
            order: Vec::with_capacity(cap),
        }
    }
    pub fn truncate(&mut self, len: usize) {
        for id in self.order.drain(len.min(self.order.len())..) {
            self.slots.remove(id);
        }
    }
}

impl FromIterator<RenderableChild> for Children {
    fn from_iter<T: IntoIterator<Item = RenderableChild>>(iter: T) -> Self {
        let iter = iter.into_iter();
        let cap = iter.size_hint().0;

        let mut slots = SlotMap::with_capacity_and_key(cap);
        let mut order = Vec::with_capacity(cap);

        for c in iter {
            order.push(slots.insert(c))
        }

        Self { slots, order }
    }
}
impl From<Vec<RenderableChild>> for Children {
    fn from(value: Vec<RenderableChild>) -> Self {
        value.into_iter().collect()
    }
}
impl Extend<RenderableChild> for Children {
    fn extend<T: IntoIterator<Item = RenderableChild>>(&mut self, iter: T) {
        let iter = iter.into_iter();
        let additional = iter.size_hint().0;
        self.slots.reserve(additional);
        self.order.reserve(additional);
        for c in iter {
            self.order.push(self.slots.insert(c));
        }
    }
}

#[derive(Clone, Debug)]
pub struct Launcher {
    pub config: Arc<LauncherConfig>,
    pub children: Children,
}

#[allow(dead_code)]
pub trait LauncherValues<'a> {
    fn name(&'a self) -> Option<&'a str>;
    fn alias(&'a self) -> Option<&'a str>;
    fn priority(&self) -> Priority;
    fn is_async(&self) -> bool;
    fn home(&self) -> HomeType;
    fn spawn_focus(&self) -> bool;
    fn sidebar_default_on(&self) -> bool;
    fn launcher_type(&'a self) -> &'a LauncherType;
    fn launcher_variant(&'a self) -> LauncherVariant;
    fn shortcut(&self) -> bool;
}

impl LauncherConfig {
    pub fn id(&self) -> LauncherId {
        LauncherId::from(self)
    }
    pub fn from_raw(raw: RawLauncher, launcher_type: LauncherType, icon: Option<String>) -> Self {
        let icon = icon.as_deref().and_then(resolve_icon_path);

        // build actions
        type ActionType = Option<Arc<[Arc<ContextMenuAction>]>>;
        let from_action = |field: Option<Vec<ApplicationActionSerde>>| -> ActionType {
            field.map(|a| {
                a.into_iter()
                    .map(|action| {
                        let mut app_action: ApplicationAction = action.into();
                        app_action.icon = app_action.icon.or_else(|| icon.clone());
                        Arc::new(ContextMenuAction::App(app_action))
                    })
                    .collect::<Vec<_>>()
                    .into()
            })
        };

        Self {
            actions: from_action(raw.actions),
            add_actions: from_action(raw.add_actions),
            name: raw.name.map(|n| n.into()),
            icon,
            alias: raw.alias,
            on_return: raw.on_return,
            exit: raw.exit,
            priority: raw.priority,
            limit: raw.limit,
            home: raw.home,
            sidebar_default_on: raw.sidebar_default_on,
            launcher_type,
            shortcut: raw.shortcut,
            spawn_focus: raw.spawn_focus,
        }
    }
    pub fn default_dmenu() -> Self {
        Self {
            priority: 1,
            home: HomeType::Home,
            // TODO: into dmenu module?
            launcher_type: LauncherType::Dmenu(DmenuLauncher::default()),
            ..Default::default()
        }
    }
    pub fn needs_stack_push(&self) -> bool {
        matches!(
            (&self.launcher_type).into(),
            LauncherVariant::Emoji | LauncherVariant::Process | LauncherVariant::Files
        )
    }
}
impl Display for LauncherConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(name) = self.name.as_ref() {
            return f.write_str(name);
        }

        f.write_str(&format!("{:?}", self.launcher_type))
    }
}

impl Hash for Launcher {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.config.hash(state);
    }
}

impl Hash for LauncherConfig {
    fn hash<H: std::hash::Hasher>(&self, h: &mut H) {
        self.name.hash(h);
        self.alias.hash(h);
        LauncherVariant::from(&self.launcher_type).hash(h);
        self.priority.hash(h);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LauncherId(pub u64);
impl From<&LauncherConfig> for LauncherId {
    fn from(value: &LauncherConfig) -> Self {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        Self(hasher.finish())
    }
}

pub enum ExecEffect {
    InsertMessages(Vec<SherlockMessage>),
    ClearMessages,
    UpdateAsync,
    None,
}

#[macro_export]
macro_rules! define_inner_functions {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident { $($body:tt)* }
    ) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, PartialEq,
            ::strum::VariantNames, ::strum::EnumString,
        )]
        #[strum(serialize_all = "snake_case")]
        $vis enum $name { $($body)* }
    };
}

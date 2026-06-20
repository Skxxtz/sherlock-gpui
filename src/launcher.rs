pub mod app_launcher;
pub mod audio_launcher;
pub mod bookmark_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod clipboard_launcher;
pub mod debug_launcher;
pub mod dmenu_launcher;
pub mod emoji_launcher;
pub mod event_launcher;
pub mod file_launcher;
pub mod message_launcher;
pub mod plugin_launcher;
pub mod process_launcher;
pub mod script_launcher;
pub mod system_cmd_launcher;
pub mod theme_launcher;
pub mod timer_launcher;
pub mod translator_launcher;
pub mod utils;
pub mod variant_type;
pub mod weather_launcher;
pub mod web_launcher;
// Integrate later: TODO
// pub mod pipe_launcher;

use crate::{
    launcher::{
        utils::binds::Bind,
        variant_type::{InnerFunction, LauncherType, LauncherVariant},
    },
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
use gpui::{App, SharedString};
use std::{
    fmt::Display,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
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

#[derive(Debug, Default, PartialEq)]
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

    /// If true, this widget can spawn focus
    pub spawn_focus: bool,

    /// The list of primary actions. This will overwrite actions defined in possible desktop files
    pub actions: Option<Arc<[Arc<ContextMenuAction>]>>,

    /// The list of supplementary actions that extend the primary actions
    pub add_actions: Option<Arc<[Arc<ContextMenuAction>]>>,
}

#[derive(Clone, Debug)]
pub struct Launcher {
    pub config: Arc<LauncherConfig>,
    pub children: Vec<RenderableChild>,
}

#[allow(dead_code)]
pub trait LauncherValues<'a> {
    fn name(&'a self) -> Option<&'a str>;
    fn alias(&'a self) -> Option<&'a str>;
    fn priority(&self) -> Priority;
    fn is_async(&self) -> bool;
    fn home(&self) -> HomeType;
    fn spawn_focus(&self) -> bool;
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
            launcher_type,
            shortcut: raw.shortcut,
            spawn_focus: raw.spawn_focus,
        }
    }
    pub fn default_dmenu() -> Self {
        Self {
            priority: 1,
            home: HomeType::Home,
            launcher_type: LauncherType::Dmenu(dmenu_launcher::DmenuLauncher::default()),
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
        $vis:vis enum $name:ident {
            $( $variant:ident $( { $($field_name:ident : $field_type:ty),* $(,)? } )? ),* $(,)?
        }
    ) => {
        #[derive(Debug, Clone, PartialEq, strum::VariantNames, strum::EnumString)]
        #[strum(serialize_all = "snake_case")]
        $vis enum $name {
            $( $variant $( { $($field_name : $field_type),* } )? ),*
        }
    };
}

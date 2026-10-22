use std::sync::Arc;

use gpui::{App, SharedString};

use crate::{
    launcher::{
        LauncherConfig, LauncherValues,
        utils::binds::Bind,
        variant_type::{InnerFunction, LauncherType},
    },
    ui::{
        launcher::{
            LauncherMode,
            context_menu::{
                ContextMenuAction,
                quick_actions::{Hideable, QuickAction},
            },
            navigator::handle::ViewKind,
        },
        traits::RenderableChildDelegate,
        widgets::{
            RenderableChild,
            emoji::{get_emoji, get_selected_skin_tones},
        },
    },
};

/// Helper enum for providing context to the execution of a launcher widget.
///
/// # Cloning
///
/// Cloning `ExecMode` is inexpensive, since it should only ever get called on the execution of an
/// item, therefore not on the hotpath.
#[derive(Default, Clone)]
pub enum ExecMode {
    Inner {
        func: InnerFunction,
        exit: bool,
    },
    App {
        exec: String,
        terminal: bool,
    },
    Command {
        exec: String,
    },
    Category {
        category: LauncherMode,
    },
    CreateView {
        mode: ViewKind,
        launcher: Arc<LauncherConfig>,
    },
    DynamicContextMenuFunc {
        action: Arc<ContextMenuAction>,
    },
    Hide {
        exec: String,
    },
    Info,
    ShowMessages,
    Web {
        engine: Option<SharedString>,
        browser: Option<SharedString>,
        exec: Option<SharedString>,
    },
    Copy {
        content: String,
    },
    Print {
        content: String,
    },
    #[default]
    None,
}

impl ExecMode {
    /// The user chose a result (as opposed to navigating, toggling or editing).
    /// Only picks are counted and learned.
    pub fn is_pick(&self) -> bool {
        match self {
            Self::App { .. }
            | Self::Command { .. }
            | Self::Web { .. }
            | Self::Copy { .. }
            | Self::Print { .. }
            | Self::Inner { .. } => true,

            Self::Category { .. }              // step into a submenu
            | Self::CreateView { .. }          // step into a view
            | Self::ShowMessages               // navigation
            | Self::Info                       // sidebar
            | Self::Hide { .. }                // config edit
            | Self::DynamicContextMenuFunc { .. }
            | Self::None => false,
        }
    }

    /// Parse a method string + context into an ExecMode.
    /// This is the single source of truth for string-based dispatch.
    fn from_method(
        method: &str,
        exec: Option<impl Into<String>>,
        launcher_type: &LauncherType,
        exit: bool,
        content: impl FnOnce() -> Option<String>,
    ) -> Option<Self> {
        let exec = exec.map(Into::into);

        match method {
            "app_launcher" | "command" | "app" => Some(Self::Command {
                exec: exec.unwrap_or_default(),
            }),

            "web_launcher" | "web" | "web_search" => Some(Self::Web {
                engine: None,
                browser: None,
                exec: exec.map(|s| s.to_string().into()),
            }),

            "copy" => content().map(|content| Self::Copy { content }),
            "print" => content().map(|content| Self::Print { content }),
            "hide" => Some(Self::Hide {
                exec: exec.unwrap_or_default(),
            }),
            "info" => Some(Self::Info),

            k if k.starts_with("inner.") => {
                let func = InnerFunction::from_str(launcher_type, k.trim_start_matches("inner."));
                (func != InnerFunction::Empty).then_some(Self::Inner { func, exit })
            }

            _ => content().map(|content| Self::Print { content }),
        }
    }

    pub fn from_bind(bind: &Bind, child: &RenderableChild, cx: &mut App) -> Option<Self> {
        if let Some(exec) = child.get_exec() {
            return Some(exec);
        }

        Self::from_method(
            &bind.callback,
            None::<&str>,
            child.launcher_type(),
            bind.exit,
            || child.get_content(cx),
        )
    }

    pub fn from_app_action(
        action: Arc<ContextMenuAction>,
        data: &RenderableChild,
        cx: &mut App,
    ) -> Self {
        match action.as_ref() {
            ContextMenuAction::App(action) => Self::from_method(
                &action.method,
                action.exec.clone(),
                data.launcher_type(),
                action.exit,
                || action.exec.clone().or(data.get_content(cx)),
            )
            .unwrap_or_default(),

            ContextMenuAction::Fn(_) => Self::DynamicContextMenuFunc { action },
            ContextMenuAction::Emoji(emj) => emj
                .entry()
                .map(|entry| {
                    let content = get_emoji(entry, &get_selected_skin_tones())
                        .as_str()
                        .to_string();
                    Self::Copy { content }
                })
                .unwrap_or_default(),
            ContextMenuAction::QuickAction(quick) => match quick {
                QuickAction::Hide => {
                    Self::from_method("hide", data.hide_id(), data.launcher_type(), false, || None)
                        .unwrap_or_default()
                }
                QuickAction::Info => {
                    Self::from_method("info", None::<&str>, data.launcher_type(), false, || None)
                        .unwrap_or_default()
                }
            },
        }
    }

    pub fn from_child(data: &RenderableChild, cx: &mut App) -> Option<Self> {
        if let Some(exec) = data.get_exec() {
            return Some(exec);
        }
        if let Some(on_return) = &data.launcher.on_return
            && let Some(result) = Self::from_method(
                on_return,
                None::<&str>,
                data.launcher_type(),
                data.launcher.exit,
                || data.get_content(cx),
            )
        {
            return Some(result);
        }

        data.build_exec(cx)
    }
}

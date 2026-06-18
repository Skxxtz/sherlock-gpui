use crate::launcher::LauncherConfig;
use crate::tokio_utils::SizedMessageObj;
use crate::ui::backdrop::Backdrop;
use crate::ui::choice::Choice;
use crate::ui::{launcher::context_menu::ContextMenuAction, launcher::views::NavigationStack};
use crate::utils::networking::ServerResponse;
use crate::utils::sized_message_sync::SizedMessage;
use gpui::WindowHandle;
use gpui::{AnyElement, IntoElement};
use gpui::{App, Entity, FocusHandle, Focusable, SharedString, Subscription};
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use crate::ui::search_bar::TextInput;

pub mod actions;
pub mod context_menu;
pub mod filter_and_sort;
pub mod render;
pub mod views;

pub use actions::{
    Execute, NextVar, OpenContext, PrevVar, Quit, SelectionDown, SelectionLeft, SelectionRight,
    SelectionUp,
};

#[derive(Clone, Debug)]
pub enum VariableInput {
    Text(Entity<TextInput>),
    Choice(Entity<Choice>),
}
impl Focusable for VariableInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Text(ent) => ent.read(cx).focus_handle.clone(),
            Self::Choice(ent) => ent.read(cx).focus_handle.clone(),
        }
    }
}

impl IntoElement for VariableInput {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        match self {
            Self::Text(ent) => ent.into_any_element(),
            Self::Choice(ent) => ent.into_any_element(),
        }
    }
}

pub struct LauncherView {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub _subs: Vec<Subscription>,

    // mode
    pub modes: Arc<[LauncherMode]>,

    // context menu
    pub context_idx: Option<usize>,
    pub context_actions: Arc<[Arc<ContextMenuAction>]>,
    pub has_actions: bool,

    // variable input fields
    pub variable_input: Vec<VariableInput>,
    pub active_bar: usize,

    // Model
    pub navigation: NavigationStack,

    // State
    pub config_initialized: bool,

    // Responses
    pub response_socket: Option<Arc<UnixStream>>,

    pub backdrop: Option<WindowHandle<Backdrop>>,
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Drop for LauncherView {
    fn drop(&mut self) {
        if let Some(socket) = self.response_socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl LauncherView {
    pub fn write_response(&self, what: String, cx: &mut App) {
        if let Some(stream) = self.response_socket.as_ref() {
            let mut stream = &**stream;
            let response = match SizedMessageObj::from_struct(&ServerResponse::Print(what)) {
                Ok(r) => r,
                Err(e) => {
                    self.navigation.push_message(e, cx);
                    return;
                }
            };

            let _ = stream.write_sized(response);
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub enum LauncherMode {
    #[default]
    Home,
    Search,
    Alias {
        short: SharedString,
        name: SharedString,
        launcher: Arc<LauncherConfig>,
    },
}

pub enum ModeTransition {
    None,
    PushStack(Arc<LauncherConfig>),
    ClearInput,
}

impl LauncherMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Home | Self::Search => "all",
            Self::Alias { short, .. } => short.as_ref(),
        }
    }
    pub fn display_str(&self) -> SharedString {
        match self {
            // "".into() uses static literals (no allocation) → efficient
            Self::Home => "All".into(),
            Self::Search => "Search".into(),
            Self::Alias { name, .. } => name.clone(),
        }
    }
    pub fn transition_for_query(&mut self, query: &str, modes: &[Self]) -> ModeTransition {
        match (self, query.is_empty()) {
            (m @ Self::Search, true) => *m = Self::Home,
            (m @ Self::Home, false) => *m = Self::Search,
            (m @ Self::Search, false) | (m @ Self::Alias { .. }, false) => {
                if let Some(alias_input) = query.strip_suffix(' ') {
                    let found_mode = modes.iter().find(|mode| {
                        if let Self::Alias { short, .. } = mode {
                            short.eq_ignore_ascii_case(alias_input)
                        } else {
                            false
                        }
                    });

                    if let Some(new_mode) = found_mode {
                        *m = new_mode.clone();
                        if let Self::Alias { launcher, .. } = new_mode
                            && launcher.needs_stack_push()
                        {
                            return ModeTransition::PushStack(launcher.clone());
                        }
                        // should clear search bar
                        return ModeTransition::ClearInput;
                    }
                }
            }
            _ => {}
        }

        // only minor change
        ModeTransition::None
    }
}

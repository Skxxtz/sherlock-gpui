use std::{os::unix::net::UnixStream, sync::Arc};

use gpui::{
    AnyElement,
    App,
    Entity,
    FocusHandle,
    Focusable,
    IntoElement,
    Subscription,
    WindowHandle,
};

use crate::{
    tokio_utils::SizedMessageObj,
    ui::{
        actions::launcher::*,
        backdrop::Backdrop,
        components::{choice::Choice, search_bar::TextInput},
        launcher::{
            context_menu::wrapper::ContextMenu,
            navigator::Navigator,
            sidebar_controller::SidebarController,
        },
        model::item::SelectionKey,
    },
    utils::{networking::ServerResponse, sized_message_sync::SizedMessage},
};

pub mod actions;
pub mod context_menu;
mod execute;
pub mod filter_and_sort;
mod mode;
pub mod navigator;
pub mod render;
pub mod sidebar;
pub mod sidebar_controller;

pub use mode::{LauncherMode, Route};

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
    pub context_menu: ContextMenu,
    pub sidebar_controller: SidebarController,

    // variable input fields
    pub variable_input: Vec<VariableInput>,
    pub active_bar: usize,

    // Model
    pub navigation: Navigator,

    // State
    pub config_initialized: bool,

    // Responses
    pub response_socket: Option<Arc<UnixStream>>,

    pub backdrop: Option<WindowHandle<Backdrop>>,

    /// Last rendered selection; its inner focus is reset when the selection moves.
    pub last_selected: Option<SelectionKey>,
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

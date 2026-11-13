use std::{os::unix::net::UnixStream, sync::Arc};

use gpui::{
    AnyElement,
    App,
    Context,
    Entity,
    FocusHandle,
    Focusable,
    IntoElement,
    Subscription,
    WindowHandle,
};

use crate::{
    app::LauncherEntity,
    config::ConfigGuard,
    tokio_utils::SizedMessageObj,
    ui::{
        actions::launcher::*,
        backdrop::Backdrop,
        components::{
            choice::Choice,
            search_bar::{EmptyBackspace, TextInput},
        },
        launcher::{
            context_menu::wrapper::ContextMenu,
            navigator::{Navigator, handle::ViewKind},
            sidebar_controller::SidebarController,
        },
        model::item::SelectionKey,
    },
    utils::{
        errors::SherlockMessage,
        networking::ServerResponse,
        sized_message_sync::SizedMessage,
    },
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

#[cfg(feature = "ai")]
pub(crate) use execute::ExecRequest;
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
    _subs: Vec<Subscription>,
    /// Interceptor listener that intercepts keystrokes before actions will be handled. This allows
    /// us to give user-binds prevendence over actions. For example, ctrl-c will first check user
    /// binds, then copy.
    _bind_interceptor: Subscription,

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
    pub fn new(
        text_input: Entity<TextInput>,
        backdrop: Option<WindowHandle<Backdrop>>,
        data: LauncherEntity,
        modes: Arc<[LauncherMode]>,
        initial_messages: Vec<SherlockMessage>,
        response_socket: Option<Arc<UnixStream>>,
        cx: &mut Context<Self>,
    ) -> Self {
        // Re-run the filter when launcher data changes (e.g. plugin tiles
        // arriving asynchronously).
        let data_sub = cx.observe(&data, |this: &mut LauncherView, _, cx| {
            this.force_filter_and_sort(cx);
        });

        let sub = cx.observe(&text_input, |this: &mut LauncherView, _, cx| {
            this.context_menu.close();
            this.navigation.reset_selection(cx);
            this.filter_and_sort(cx);
        });

        // routed through `Back`, so the current view can override it
        let backspace_sub = cx.subscribe(&text_input, |_, _, _: &EmptyBackspace, cx| {
            cx.dispatch_action(&Back);
        });

        text_input.update(cx, |this, _| this._sub = Some(backspace_sub));

        let mut navigation = Navigator::new(data, initial_messages, cx);

        let mode = ConfigGuard::get(|c| {
            c.runtime
                .sub_menu
                .as_deref()
                .and_then(|submenu| {
                    modes.iter().find(|m| {
                        matches!(
                            m,
                            LauncherMode::Alias { short, .. } if short.eq_ignore_ascii_case(submenu)
                        )
                    })
                })
                .cloned()
                .unwrap_or(LauncherMode::Home)
        });

        if let LauncherMode::Alias { launcher, .. } = &mode
            && let Some(view) = ViewKind::try_from(&launcher.launcher_type)
                .ok()
                .and_then(|kind| kind.build(launcher.clone(), cx))
        {
            navigation.push(view);
        }

        let weak_self = cx.entity().downgrade();

        LauncherView {
            text_input,
            focus_handle: cx.focus_handle(),
            _subs: vec![sub, data_sub],
            modes,
            context_menu: ContextMenu::default(),
            sidebar_controller: SidebarController::default(),
            variable_input: Vec::new(),
            active_bar: 0,
            navigation,
            config_initialized: ConfigGuard::is_initialized(),
            response_socket,
            backdrop,
            last_selected: None,
            _bind_interceptor: cx.intercept_keystrokes(move |ev, win, cx| {
                let handled = weak_self
                    .update(cx, |launcher, cx| launcher.run_bind(&ev.keystroke, win, cx))
                    .unwrap_or(false);
                if handled {
                    cx.propagate();
                }
            }),
        }
    }
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

use std::sync::Arc;

use gpui::{App, AppContext, Entity, Global, WeakEntity};

use crate::{
    app::LauncherEntity,
    launcher::LauncherId,
    ui::{
        launcher::{
            LauncherMode,
            context_menu::ContextMenuAction,
            navigator::{handle::ViewKind, views::NavView},
        },
        model::{Model, message::MessageView},
        traits::RenderableChildDelegate,
        widgets::RenderableChild,
    },
    utils::errors::SherlockMessage,
};

pub mod handle;
pub mod views;

/// Global handle to the message view, so any code can push messages
/// without access to the `Navigator`.
#[derive(Clone)]
pub struct MessageViewGlobal(WeakEntity<MessageView>);
impl Global for MessageViewGlobal {}
impl MessageViewGlobal {
    /// Adds a message to the message view.
    /// Does nothing if the view has been dropped.
    pub fn push_message<C: AppContext>(&self, message: SherlockMessage, cx: &mut C) {
        let _ = self.0.update(cx, |this, cx| {
            this.push_message(message, cx.weak_entity(), cx);
        });
    }
}

/// Holds all launcher views and tracks which one is active.
///
/// * `home` is the permanent root.
/// * `messages` is an overlay reachable from any depth.
/// * `stack` holds every view pushed on top of home.
///
/// Provides functions to:
///
/// * navigate, push and pop views
/// * interact with items of the active view
/// * add and show errors, warnings, and infos
pub struct Navigator {
    home: NavView,
    messages: NavView,
    // Typed handle to the message view, so messages can be pushed without matching
    message_entity: Entity<MessageView>,
    stack: Vec<NavView>,
    // When true, `current()` returns `messages` instead of the stack top.
    showing_messages: bool,
}

impl Navigator {
    /// Creates the navigator with the home view and the message overlay.
    ///
    /// * `initial`: home's launcher entity
    /// * `messages`: startup messages
    /// * `len`: home's initial item count
    ///
    /// Also registers `MessageViewGlobal`, so messages can be pushed from anywhere.
    pub fn new(
        initial: LauncherEntity,
        messages: Vec<SherlockMessage>,
        len: usize,
        cx: &mut App,
    ) -> Self {
        let (messages, message_entity) = NavView::messages(messages, cx);
        cx.set_global(MessageViewGlobal(message_entity.downgrade()));

        Self {
            home: NavView::home(initial, len, cx),
            messages,
            message_entity,
            stack: Vec::new(),
            showing_messages: false,
        }
    }
}

// Stack implementations
impl Navigator {
    /// Returns the currently active view
    pub fn current(&self) -> &NavView {
        if self.showing_messages {
            &self.messages
        } else {
            self.stack.last().unwrap_or(&self.home)
        }
    }

    /// Returns the currently active view as a mutable reference
    pub fn current_mut(&mut self) -> &mut NavView {
        if self.showing_messages {
            &mut self.messages
        } else {
            self.stack.last_mut().unwrap_or(&mut self.home)
        }
    }

    /// Returns the kind of view for the currently active view
    pub fn current_kind(&self) -> &ViewKind {
        &self.current().kind
    }

    /// Returns the `LauncherMode` for the currently active view
    pub fn current_mode(&self) -> &LauncherMode {
        &self.current().mode
    }

    /// Pushes a new temporary view onto the view stack
    pub fn push(&mut self, view: NavView) {
        self.showing_messages = false;
        self.stack.push(view);
    }

    /// Pops the last temporary view from the stack
    pub fn pop(&mut self) -> Option<NavView> {
        self.stack.pop()
    }

    /// Clears the entire view stack
    pub fn clear(&mut self) {
        self.stack.clear();
        self.showing_messages = false;
    }

    /// Pops the currently active view and moves back to the view before. Also moves out of the
    /// message view to the home view
    pub fn back(&mut self) {
        if !std::mem::take(&mut self.showing_messages) {
            self.stack.pop();
        }
        self.current_mut().layout.reset();
    }

    /// Sets the message view the active view
    pub fn show_messages(&mut self) {
        self.showing_messages = true;
    }
}

// Messages implementations
impl Navigator {
    /// Add a new error/warning/info message to the message view
    pub fn push_message(&self, message: SherlockMessage, cx: &mut App) {
        self.message_entity.update(cx, |this, cx| {
            this.push_message(message, cx.weak_entity(), cx);
        });
    }

    /// Get the numver of messages for the message view
    pub fn message_count(&self, cx: &App) -> usize {
        self.message_entity.read(cx).count.get()
    }

    /// Clears all messages for the message view
    pub fn clear_messages(&self, cx: &mut App) {
        self.message_entity
            .update(cx, |this, cx| this.clear_messages(cx));
    }
}

// Model Implementations
impl Navigator {
    /// Read the active view's model.
    /// Can't update entities inside `f`; return what you need instead.
    pub fn with_model<R>(&self, cx: &App, f: impl FnOnce(&Model) -> R) -> R {
        self.current().handle.with_model(cx, f)
    }

    /// Mutate the active view's model.
    /// Takes `&self` because the model lives in a GPUI entity, not in the navigator.
    /// Don't re-enter the same view's entity inside `f`, or GPUI panics.
    pub fn with_model_mut<R, C: AppContext>(
        &self,
        cx: &mut C,
        f: impl FnOnce(&mut Model, &mut App) -> R,
    ) -> R {
        self.current().handle.with_model_mut(cx, f)
    }

    /// Returns the currently active data index
    pub fn selected_data_idx(&self, cx: &App) -> Option<(LauncherId, usize)> {
        let ui_idx = self.current().layout.selected;
        self.with_model(cx, |mdl| {
            let indices = mdl.filtered_indices();
            let last = indices.len().checked_sub(1)?;
            indices.get(ui_idx.min(last)).copied()
        })
    }

    /// Sets the currently active data index
    pub fn set_selected_data_idx(&mut self, data_idx: (LauncherId, usize), cx: &App) {
        let pos = self.with_model(cx, |mdl| {
            mdl.filtered_indices().iter().position(|i| *i == data_idx)
        });
        if let Some(pos) = pos {
            self.current_mut().layout.selected = pos;
        }
    }

    /// Returns a shared reference to the currently active widget
    pub fn selected_item_ref<'a>(&self, cx: &'a App) -> Option<&'a RenderableChild> {
        let idx = self.selected_data_idx(cx)?;
        let data = self.with_model(cx, |mdl| mdl.data());
        data.read(cx)
            .get(&idx.0)
            .and_then(|l| l.children.get(idx.1))
    }

    /// Read the widget at the given index.
    /// Can't update entities inside `f`: return what you need instead.
    pub fn with_item<R>(
        &self,
        idx: (LauncherId, usize),
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        let data = self.with_model(cx, |mdl| mdl.data());
        data.update(cx, |data, cx| {
            data.get(&idx.0)
                .and_then(|l| l.children.get(idx.1))
                .map(|item| f(item, cx))
        })
    }

    /// Read the currently active widget.
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_selected_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        let idx = self.selected_data_idx(cx)?;
        self.with_item(idx, cx, f)
    }

    /// Read the nth item that allows shortcuts.
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_nth_shortcut_item<R>(
        &self,
        n: usize,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        let skip = n.checked_sub(1)?;
        let idx = self.with_model(cx, |mdl| {
            let entity = mdl.data();
            let data = entity.read(cx);
            mdl.filtered_indices()
                .iter()
                .copied()
                .filter(|i| data.get(&i.0).is_some_and(|l| l.config.shortcut))
                .nth(skip)
        })?;
        self.with_item(idx, cx, f)
    }

    /// Returns the collection of `ContextMenuAction` for the currently active item.
    pub fn current_actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        self.with_selected_item(cx, |item, cx| item.actions(cx))
            .flatten()
    }
}

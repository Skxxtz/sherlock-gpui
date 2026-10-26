use std::sync::Arc;

use gpui::{App, AppContext, Entity, Global, SharedString, WeakEntity};

use crate::{
    app::LauncherEntity,
    ui::{
        launcher::{
            LauncherMode,
            context_menu::ContextMenuAction,
            navigator::{handle::ViewKind, views::NavView},
        },
        model::{
            AnyModel, Model,
            item::{ItemRef, SelectionKey},
        },
        traits::RenderableChildDelegate,
        views::message::MessageView,
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
    pub fn back(&mut self, cx: &mut App) {
        if !std::mem::take(&mut self.showing_messages) {
            self.stack.pop();
        }
        self.reset_selection(cx);
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
    /// Read the active view's launcher model. `None` if it doesn't show launcher data.
    /// Can't update entities inside `f`; return what you need instead.
    pub fn with_model<R>(&self, cx: &App, f: impl FnOnce(&Model) -> R) -> Option<R> {
        self.current().handle.with_model(cx, f)
    }

    /// Mutate the active view's launcher model. `None` if it doesn't show launcher data.
    /// Takes `&self` because the model lives in a GPUI entity, not in the navigator.
    /// Don't re-enter the same view's entity inside `f`, or GPUI panics.
    pub fn with_model_mut<R, C: AppContext>(
        &self,
        cx: &mut C,
        f: impl FnOnce(&mut Model, &mut App) -> R,
    ) -> Option<R> {
        self.current().handle.with_model_mut(cx, f)
    }

    /// Read the active view's model without knowing its data type.
    /// `None` if the view has no model.
    pub fn with_any_model<R>(&self, cx: &App, f: impl FnOnce(&dyn AnyModel) -> R) -> Option<R> {
        self.current().handle.with_any_model(cx, f)
    }

    /// Mutate the active view's model without knowing its data type.
    /// `None` if the view has no model.
    pub fn with_any_model_mut<R, C: AppContext>(
        &self,
        cx: &mut C,
        f: impl FnOnce(&mut dyn AnyModel) -> R,
    ) -> Option<R> {
        self.current().handle.with_any_model_mut(cx, f)
    }

    /// Number of visible items in the active view (0 without a model).
    pub fn len(&self, cx: &App) -> usize {
        self.with_any_model(cx, |m| m.len()).unwrap_or(0)
    }

    /// The query that produced the active view's results, if any.
    pub fn last_query(&self, cx: &App) -> Option<SharedString> {
        self.with_any_model(cx, |m| m.last_query()).flatten()
    }

    /// Forget the active view's last query, so the next one always re-runs.
    pub fn clear_last_query(&self, cx: &mut App) {
        self.with_any_model_mut(cx, |m| m.clear_last_query());
    }

    /// Identity of the selected item of the active view.
    pub fn selected_key(&self, cx: &App) -> Option<SelectionKey> {
        self.with_any_model(cx, |m| m.selected_key()).flatten()
    }

    /// Handle to the selected item of the active view.
    pub fn selected_item(&self, cx: &App) -> Option<ItemRef> {
        self.with_any_model(cx, |m| m.selected_item()).flatten()
    }

    /// Selected position in the active view's visible items (clamped).
    pub fn selected_index(&self, cx: &App) -> usize {
        self.with_any_model(cx, |m| m.selected_index()).unwrap_or(0)
    }

    /// Selects position `n` and scrolls it into view. Returns `false` if out of range.
    pub fn select(&self, n: usize, cx: &mut App) -> bool {
        let ok = self
            .with_any_model_mut(cx, |m| m.select(n))
            .unwrap_or(false);
        if ok {
            self.current().layout.reveal(n);
        }
        ok
    }

    /// Selects the visible item with `key`.
    pub fn select_key(&self, key: SelectionKey, cx: &mut App) {
        self.with_any_model_mut(cx, |m| m.select_key(key));
    }

    /// Back to the first item of the active view.
    pub fn reset_selection(&self, cx: &mut App) {
        self.with_any_model_mut(cx, |m| m.reset_selection());
    }

    /// Shared reference to the selected launcher item.
    pub fn selected_item_ref<'a>(&self, cx: &'a App) -> Option<&'a RenderableChild> {
        let item = self.selected_item(cx)?;
        item.read(cx)?.as_launcher_item()
    }

    /// Read the selected launcher item.
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_selected_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        self.selected_item(cx)?.with_launcher_item(cx, f)
    }

    /// Read the nth item that allows shortcuts (launcher items only).
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_nth_shortcut_item<R>(
        &self,
        n: usize,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        self.nth_shortcut_item(n, cx)?.with_launcher_item(cx, f)
    }

    /// Read the nth item that allows shortcuts as a shared reference
    pub fn nth_shortcut_item_ref<'a>(&self, n: usize, cx: &'a App) -> Option<&'a RenderableChild> {
        let item = self.nth_shortcut_item(n, cx)?;
        item.read(cx)?.as_launcher_item()
    }

    /// The nth (1-based) visible item that allows shortcuts.
    fn nth_shortcut_item(&self, n: usize, cx: &App) -> Option<ItemRef> {
        let skip = n.checked_sub(1)?;
        self.with_any_model(cx, |m| {
            (0..m.len())
                .filter_map(|i| m.item_at(i))
                .filter(|item| item.read(cx).is_some_and(|i| i.shortcut()))
                .nth(skip)
        })
        .flatten()
    }

    /// Returns the collection of `ContextMenuAction` for the currently active item.
    pub fn current_actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        self.with_selected_item(cx, |item, cx| item.actions(cx))
            .flatten()
    }
}

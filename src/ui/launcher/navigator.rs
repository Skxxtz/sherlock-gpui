use std::sync::Arc;

use gpui::{App, AppContext, Entity, Global, SharedString, WeakEntity};

use crate::{
    app::LauncherEntity,
    ui::{
        launcher::{LauncherMode, context_menu::ContextMenuAction, navigator::views::NavView},
        model::item::{SelectionKey, ShellItem},
        views::{home::HomeView, message::MessageView},
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
    ///
    /// Also registers `MessageViewGlobal`, so messages can be pushed from anywhere.
    pub fn new(initial: LauncherEntity, messages: Vec<SherlockMessage>, cx: &mut App) -> Self {
        let message_entity = cx.new(|cx| MessageView::new(messages, cx));
        cx.set_global(MessageViewGlobal(message_entity.downgrade()));
        let messages = NavView::new(message_entity.clone(), cx);

        let home = cx.new(|cx| HomeView::new(initial, cx));

        Self {
            home: NavView::new(home, cx),
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

    /// True if the home view is active (no pushed view, no message overlay).
    pub fn is_home(&self) -> bool {
        !self.showing_messages && self.stack.is_empty()
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
    /// Number of visible items in the active view (0 without a model).
    pub fn len(&self, cx: &App) -> usize {
        self.current().handle.len(cx)
    }

    /// The query that produced the active view's results, if any.
    pub fn last_query(&self, cx: &App) -> Option<SharedString> {
        self.current().handle.last_query(cx)
    }

    /// Forget the active view's last query, so the next one always re-runs.
    pub fn clear_last_query(&self, cx: &mut App) {
        self.current().handle.clear_last_query(cx);
    }

    /// Identity of the selected item of the active view.
    pub fn selected_key(&self, cx: &App) -> Option<SelectionKey> {
        self.current().handle.selected_key(cx)
    }

    /// Selected position in the active view's visible items (clamped).
    pub fn selected_index(&self, cx: &App) -> usize {
        self.current().handle.selected_index(cx)
    }

    /// Selects position `n` and scrolls it into view. Returns `false` if out of range.
    pub fn select(&self, n: usize, cx: &mut App) -> bool {
        let ok = self.current().handle.select(n, cx);
        if ok {
            self.current().layout.reveal(n);
        }
        ok
    }

    /// Selects the visible item with `key`.
    pub fn select_key(&self, key: SelectionKey, cx: &mut App) {
        self.current().handle.select_key(key, cx);
    }

    /// Back to the first item of the active view.
    pub fn reset_selection(&self, cx: &mut App) {
        self.current().handle.reset_selection(cx);
    }

    /// Shared reference to the visible item at position `n`, whatever its type.
    pub fn item_ref<'a>(&self, n: usize, cx: &'a App) -> Option<&'a dyn ShellItem> {
        self.current().handle.item_ref(n, cx)
    }

    /// Shared reference to the selected item, whatever its type.
    pub fn selected_shell_item_ref<'a>(&self, cx: &'a App) -> Option<&'a dyn ShellItem> {
        self.item_ref(self.selected_index(cx), cx)
    }

    /// Shared reference to the selected launcher item.
    pub fn selected_item_ref<'a>(&self, cx: &'a App) -> Option<&'a RenderableChild> {
        self.selected_shell_item_ref(cx)?.as_launcher_item()
    }

    /// Run `f` with the selected item, whatever its type.
    /// Don't re-enter the item's data entity inside `f`, or GPUI panics.
    pub fn with_selected_shell_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
    ) -> Option<R> {
        let n = self.selected_index(cx);
        self.current().handle.with_item(n, cx, f)
    }

    /// Read the selected launcher item.
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_selected_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        self.with_selected_shell_item(cx, |item, cx| item.as_launcher_item().map(|c| f(c, cx)))
            .flatten()
    }

    /// Run `f` with the visible item with `key`, if it is still visible.
    pub fn with_item_by_key<R>(
        &self,
        key: SelectionKey,
        cx: &mut App,
        f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
    ) -> Option<R> {
        let n = self.current().handle.position_of(key, cx)?;
        self.current().handle.with_item(n, cx, f)
    }

    /// Position of the nth (1-based) visible item that allows shortcuts.
    fn shortcut_position(&self, n: usize, cx: &App) -> Option<usize> {
        let skip = n.checked_sub(1)?;
        (0..self.len(cx))
            .filter(|&i| self.item_ref(i, cx).is_some_and(|item| item.shortcut()))
            .nth(skip)
    }

    /// Shared reference to the nth item that allows shortcuts, whatever its type.
    pub fn shortcut_item_ref<'a>(&self, n: usize, cx: &'a App) -> Option<&'a dyn ShellItem> {
        self.item_ref(self.shortcut_position(n, cx)?, cx)
    }

    /// Run `f` with the nth item that allows shortcuts, whatever its type.
    pub fn with_nth_shortcut_shell_item<R>(
        &self,
        n: usize,
        cx: &mut App,
        f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
    ) -> Option<R> {
        let pos = self.shortcut_position(n, cx)?;
        self.current().handle.with_item(pos, cx, f)
    }

    /// Read the nth item that allows shortcuts (launcher items only).
    /// Don't re-enter the same widget's entity inside `f`, or GPUI panics.
    pub fn with_nth_shortcut_item<R>(
        &self,
        n: usize,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        self.with_nth_shortcut_shell_item(n, cx, |item, cx| {
            item.as_launcher_item().map(|c| f(c, cx))
        })
        .flatten()
    }

    /// Read the nth item that allows shortcuts as a shared reference (launcher items only).
    pub fn nth_shortcut_item_ref<'a>(&self, n: usize, cx: &'a App) -> Option<&'a RenderableChild> {
        self.shortcut_item_ref(n, cx)?.as_launcher_item()
    }

    /// Returns the collection of `ContextMenuAction` for the currently active item.
    pub fn current_actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        self.with_selected_shell_item(cx, |item, cx| item.actions(cx))
            .flatten()
    }
}

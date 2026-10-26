//! What the shell needs from items of any data type, and their type-erased identity.

use std::{
    any::TypeId,
    hash::{DefaultHasher, Hash, Hasher},
    sync::Arc,
};

use gpui::App;

use crate::{
    launcher::{LauncherValues, utils::exec_mode::ExecMode, variant_type::QuickActionCapabilities},
    ui::{
        launcher::context_menu::{ContextMenuAction, wrapper::ContextMenu},
        model::data::ModelData,
        query_memory::QueryMemoryGuard,
        traits::RenderableChildDelegate,
        widgets::RenderableChild,
    },
    utils::item_key::ItemKey,
};

/// What the shell needs from an item, whatever its data type. Every method has a default, so a
/// plain item only needs `impl ShellItem for MyItem {}`.
pub trait ShellItem: 'static {
    /// Focus this item first when results change.
    fn spawn_focus(&self) -> bool {
        false
    }

    /// Item can be run with a numbered shortcut.
    fn shortcut(&self) -> bool {
        false
    }

    /// What Enter does. `None`: nothing.
    fn exec(&self, _cx: &mut App) -> Option<ExecMode> {
        None
    }

    /// Context-menu actions of this item.
    fn actions(&self, _cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        None
    }

    /// Whether `actions` returns anything (checked for the selected item on every frame).
    fn has_actions(&self, _cx: &mut App) -> bool {
        false
    }

    /// What running one of this item's context-menu actions does.
    fn action_exec(&self, _action: Arc<ContextMenuAction>, _cx: &mut App) -> ExecMode {
        ExecMode::default()
    }

    /// Built-in quick actions (hide, info) this item supports.
    fn quick_actions(&self) -> QuickActionCapabilities {
        QuickActionCapabilities::empty()
    }

    /// Key under which picks are learned (query memory). `None`: picks aren't learned.
    fn pick_key(&self) -> Option<ItemKey> {
        None
    }

    /// Called after the item was picked, to refresh its popularity.
    fn refresh_popularity(&self) {}

    /// The launcher item behind this item, if any. Sidebars, vars, binds and inner functions
    /// only work for launcher items.
    fn as_launcher_item(&self) -> Option<&RenderableChild> {
        None
    }
}

impl ShellItem for RenderableChild {
    #[inline]
    fn spawn_focus(&self) -> bool {
        LauncherValues::spawn_focus(self)
    }

    #[inline]
    fn shortcut(&self) -> bool {
        LauncherValues::shortcut(self)
    }

    fn exec(&self, cx: &mut App) -> Option<ExecMode> {
        ExecMode::from_child(self, cx)
    }

    fn actions(&self, cx: &mut App) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        RenderableChildDelegate::actions(self, cx)
    }

    fn has_actions(&self, cx: &mut App) -> bool {
        RenderableChildDelegate::has_actions(self, cx)
    }

    fn action_exec(&self, action: Arc<ContextMenuAction>, cx: &mut App) -> ExecMode {
        self.build_action_exec(action, cx)
    }

    fn quick_actions(&self) -> QuickActionCapabilities {
        ContextMenu::availability_mask(self)
    }

    fn pick_key(&self) -> Option<ItemKey> {
        self.launcher_type()
            .is_pickable()
            .then(|| self.item_key())
            .flatten()
    }

    fn refresh_popularity(&self) {
        if let Some(id) = self.item_key_ref() {
            self.set_count(QueryMemoryGuard::popularity_count(id));
        }
    }

    #[inline]
    fn as_launcher_item(&self) -> Option<&RenderableChild> {
        Some(self)
    }
}

/// Type-erased identity of an item: a hash of its data type and id.
///
/// Used where the shell remembers an item across frames (context menu cache, sidebar, inner
/// focus) without knowing the id type.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SelectionKey(u64);

impl SelectionKey {
    /// The raw hash, e.g. for element ids.
    #[inline]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    pub fn new<D: ModelData>(id: &D::Id) -> Self {
        let mut h = DefaultHasher::new();
        TypeId::of::<D>().hash(&mut h);
        id.hash(&mut h);
        Self(h.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::model::list_data::{ListData, ListId, ListItem};

    struct A;
    struct B;
    impl ShellItem for A {}
    impl ShellItem for B {}
    impl ListItem for A {
        fn search(&self) -> &str {
            ""
        }
    }
    impl ListItem for B {
        fn search(&self) -> &str {
            ""
        }
    }

    #[test]
    fn keys_differ_by_id_and_data_type() {
        let a1 = SelectionKey::new::<ListData<A>>(&ListId(1));
        assert_eq!(a1, SelectionKey::new::<ListData<A>>(&ListId(1)));
        assert_ne!(a1, SelectionKey::new::<ListData<A>>(&ListId(2)));
        assert_ne!(a1, SelectionKey::new::<ListData<B>>(&ListId(1)));
    }
}

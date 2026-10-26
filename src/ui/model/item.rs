//! Type-erased access to items, so the shell can work with the selected item of any view.

use std::{
    any::TypeId,
    hash::{DefaultHasher, Hash, Hasher},
    rc::Rc,
};

use gpui::{App, Entity};

use crate::{
    launcher::LauncherValues,
    ui::{model::data::ModelData, widgets::RenderableChild},
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

    /// The launcher item behind this item, if any. Running, the context menu, sidebars, vars and
    /// binds currently only work for launcher items; other items are selectable only.
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

/// Handle to one item of some view's data. Cheap to clone; resolves the item on use, so it is
/// `None` once the item is gone.
#[derive(Clone)]
pub struct ItemRef(Rc<dyn ErasedItemRef>);

impl ItemRef {
    pub fn new<D: ModelData>(data: Entity<D>, id: D::Id) -> Self
    where
        D::Item: ShellItem,
    {
        Self(Rc::new(EntityItem { data, id }))
    }

    pub fn key(&self) -> SelectionKey {
        self.0.key()
    }

    /// Shared reference to the item.
    pub fn read<'a>(&self, cx: &'a App) -> Option<&'a dyn ShellItem> {
        self.0.read(cx)
    }

    /// Runs `f` with the item and `&mut App`. Updates the item's data entity, so `f` must not
    /// update that same entity.
    pub fn with<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&dyn ShellItem, &mut App) -> R,
    ) -> Option<R> {
        let mut f = Some(f);
        let mut out = None;
        self.0.with(cx, &mut |item, cx| {
            if let Some(f) = f.take() {
                out = Some(f(item, cx));
            }
        });
        out
    }

    /// Like `with`, for launcher items only.
    pub fn with_launcher_item<R>(
        &self,
        cx: &mut App,
        f: impl FnOnce(&RenderableChild, &mut App) -> R,
    ) -> Option<R> {
        self.with(cx, |item, cx| item.as_launcher_item().map(|c| f(c, cx)))
            .flatten()
    }
}

trait ErasedItemRef {
    fn key(&self) -> SelectionKey;
    fn read<'a>(&self, cx: &'a App) -> Option<&'a dyn ShellItem>;
    fn with(&self, cx: &mut App, f: &mut dyn FnMut(&dyn ShellItem, &mut App));
}

struct EntityItem<D: ModelData> {
    data: Entity<D>,
    id: D::Id,
}

impl<D: ModelData> ErasedItemRef for EntityItem<D>
where
    D::Item: ShellItem,
{
    fn key(&self) -> SelectionKey {
        SelectionKey::new::<D>(&self.id)
    }

    fn read<'a>(&self, cx: &'a App) -> Option<&'a dyn ShellItem> {
        self.data
            .read(cx)
            .get(self.id)
            .map(|item| item as &dyn ShellItem)
    }

    fn with(&self, cx: &mut App, f: &mut dyn FnMut(&dyn ShellItem, &mut App)) {
        let id = self.id;
        self.data.update(cx, |data, cx| {
            if let Some(item) = data.get(id) {
                f(item, cx);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::model::list_data::{ListData, ListId, ListItem};

    struct A;
    struct B;
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

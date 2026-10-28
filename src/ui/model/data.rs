use std::{fmt::Debug, hash::Hash, sync::Arc};

use gpui::{App, AppContext, SharedString};

use crate::{
    app::data::LauncherData,
    launcher::variant_type::LauncherType,
    ui::{
        model::{
            ItemIdx,
            all_indices,
            index::{IndexBuilder, SearchIndex},
            item::ShellItem,
        },
        traits::RenderableChildDelegate,
        widgets::RenderableChild,
    },
};

pub trait ModelData: Clone + 'static {
    /// Stable identity of an item. Survives refilters and data updates, so it is used to keep the
    /// selection.
    type Id: Copy + Eq + Hash + Debug + Send + Sync + 'static;

    /// The item type views render and act on.
    type Item: ShellItem;

    /// Changes whenever the data changes; invalidates the search index.
    fn revision(&self) -> u64;

    /// The item with `id`, if it still exists.
    fn get(&self, id: Self::Id) -> Option<&Self::Item>;

    /// Every item, unfiltered
    fn all_ids(&self) -> Arc<[Self::Id]>;

    /// Builds the thread-saffe search index for this revision.
    fn build_index(&self) -> SearchIndex<Self::Id>;

    /// Rule 5 for an contextual item. `Some(false)` for items that no longer exists.
    fn based_show<C: AppContext>(&self, id: Self::Id, query: &str, cx: &mut C) -> Option<bool>;

    /// Called after new results were applied, with the visible ids. Items that keep per-query
    /// state (e.g. launcher widgets) update it here.
    fn on_results(&self, _visible: &[Self::Id], _query: &SharedString, _cx: &mut App) {}

    /// Refreshes live items (weather, music, clipboard, ...). Called periodically and after
    /// actions that may change them.
    fn refresh(&self, _cx: &mut App) {}
}

/// Data of views without items (plain pages): `type Data = NoData;`.
#[allow(dead_code)] // until the first plain page uses it
#[derive(Clone, Copy, Debug, Default)]
pub struct NoData;

/// Item type of `NoData`; can't be constructed.
#[allow(dead_code)]
pub enum NoItem {}

impl ShellItem for NoItem {}

impl ModelData for NoData {
    type Id = ();
    type Item = NoItem;

    fn revision(&self) -> u64 {
        0
    }

    fn get(&self, _id: ()) -> Option<&NoItem> {
        None
    }

    fn all_ids(&self) -> Arc<[()]> {
        Arc::from([])
    }

    fn build_index(&self) -> SearchIndex<()> {
        IndexBuilder::new(0).finish()
    }

    fn based_show<C: AppContext>(&self, _id: (), _query: &str, _cx: &mut C) -> Option<bool> {
        None
    }
}

impl ModelData for LauncherData {
    type Id = ItemIdx;
    type Item = RenderableChild;

    #[inline(always)]
    fn revision(&self) -> u64 {
        LauncherData::revision(self)
    }

    #[inline]
    fn get(&self, id: Self::Id) -> Option<&RenderableChild> {
        self.child(id)
    }

    fn all_ids(&self) -> Arc<[Self::Id]> {
        all_indices(self)
    }

    fn build_index(&self) -> SearchIndex<Self::Id> {
        SearchIndex::build(self)
    }

    fn based_show<C: AppContext>(&self, id: Self::Id, query: &str, cx: &mut C) -> Option<bool> {
        self.child(id)
            .map_or(Some(false), |child| child.based_show(query, cx))
    }

    fn on_results(&self, visible: &[ItemIdx], query: &SharedString, cx: &mut App) {
        for &id in visible {
            if let Some(child) = self.child(id) {
                child.update_sync(query.clone(), cx);
            }
        }
    }

    fn refresh(&self, cx: &mut App) {
        self.values()
            .filter(|launcher| {
                matches!(
                    launcher.config.launcher_type,
                    LauncherType::Weather(_)
                        | LauncherType::MusicPlayer(_)
                        | LauncherType::Clipboard(_)
                        | LauncherType::Event(_)
                        | LauncherType::Plugin(_)
                )
            })
            .flat_map(|launcher| launcher.children.iter())
            .for_each(|item| item.update_async(cx));
    }
}

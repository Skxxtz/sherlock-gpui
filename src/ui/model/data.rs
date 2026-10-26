use std::{fmt::Debug, hash::Hash, sync::Arc};

use gpui::AppContext;

use crate::{
    app::data::LauncherData,
    ui::{
        model::{ItemIdx, all_indices, index::SearchIndex},
        traits::RenderableChildDelegate,
    },
};

pub trait ModelData: Clone + 'static {
    /// Stable identity of an item. Survives refilters and data updates, so it is used to keep the
    /// selection.
    type Id: Copy + Eq + Hash + Debug + Send + Sync + 'static;

    /// Changes whenever the data changes; invalidates the search index.
    fn revision(&self) -> u64;

    /// Every item, unfiltered
    fn all_ids(&self) -> Arc<[Self::Id]>;

    /// Builds the thread-saffe search index for this revision.
    fn build_index(&self) -> SearchIndex<Self::Id>;

    /// Rule 5 for an contextual item. `Some(false)` for items that no longer exists.
    fn based_show<C: AppContext>(&self, id: Self::Id, query: &str, cx: &mut C) -> Option<bool>;
}

impl ModelData for LauncherData {
    type Id = ItemIdx;

    #[inline(always)]
    fn revision(&self) -> u64 {
        LauncherData::revision(self)
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
}

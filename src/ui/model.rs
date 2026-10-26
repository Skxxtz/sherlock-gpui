use std::{collections::HashMap, sync::Arc};

use gpui::{App, AppContext, Entity, SharedString, Task, WeakEntity};

use crate::{
    app::{LauncherEntity, LauncherWeakEntity, data::LauncherData},
    launcher::{Launcher, LauncherConfig, LauncherId, LocalId, variant_type::LauncherType},
    ui::{
        launcher::{LauncherView, filter_and_sort::FilterPolicy},
        model::{
            cursor::Cursor,
            data::ModelData,
            index::SearchIndex,
            utils::{all_indices, single_launcher},
        },
        views::{file::FileSearchModel, process::ProcessModel},
    },
};

pub mod cursor;
pub mod data;
pub mod index;
pub mod pipeline;
mod utils;

pub type ItemIdx = (LauncherId, LocalId);

/// Filter state over one data entity: visible items, selection, cached search index.
///
/// Generic over the data (`ModelData`); `Model` alone means `Model<LauncherData>`.
pub struct Model<D: ModelData = LauncherData> {
    /// The data this model shows.
    pub data: Entity<D>,
    /// Ids of the visible items, in display order.
    pub filtered: Arc<[D::Id]>,
    /// Selected position in `filtered`.
    cursor: Cursor,
    /// Cached search index for `data`; rebuilt when the data revision changes.
    index: Option<Arc<SearchIndex<D::Id>>>,

    /// The query that produced `filtered` if any.
    pub last_query: Option<SharedString>,
    /// In-flight filter task; dropping it, cancels it.
    pub pending: Option<Task<Option<()>>>,
    /// Where the items come from: local filtering or an external search.
    pub source: ModelSource,
    /// Defines which filter rules should apply at runtime.
    pub filter_policy: FilterPolicy,
}

/// What `Model::set_results` changed.
#[derive(Clone, Copy, Debug)]
pub struct ResultsChange {
    /// The visible items differ from before.
    pub changed: bool,
    /// The previously selected item is still visible and stays selected.
    pub restored: bool,
}

// Constructors
impl<D: ModelData> Model<D> {
    /// Shared constructor. Starts with no query, no pending task and the first item selected.
    pub fn new(
        data: Entity<D>,
        filtered: Arc<[D::Id]>,
        source: ModelSource,
        filter_policy: FilterPolicy,
    ) -> Self {
        Self {
            data,
            filtered,
            cursor: Cursor::default(),
            last_query: None,
            index: None,
            pending: None,
            filter_policy,
            source,
        }
    }
}

impl Model<LauncherData> {
    /// Local model over owned launcher data, with every item visible.
    pub fn standard(data: HashMap<LauncherId, Launcher>, cx: &mut App) -> Self {
        Self::standard_with_entity(cx.new(|_| LauncherData::new(data)), cx)
    }

    pub fn sub_view(data: HashMap<LauncherId, Launcher>, cx: &mut App) -> Self {
        let filtered = all_indices(&data);
        let entity = cx.new(|_| LauncherData::new(data));
        Self::new(
            entity,
            filtered,
            ModelSource::Static,
            FilterPolicy::QueryOnly,
        )
    }

    /// Local model over an existing data entity, with every item visible.
    /// Used for home and dmenu, which share their launcher entity.
    pub fn standard_with_entity(data: LauncherEntity, cx: &App) -> Self {
        let filtered = all_indices(data.read(cx));
        Self::new(data, filtered, ModelSource::Static, FilterPolicy::All)
    }

    /// Process model. Starts empty; filled by `ProcessModel` searches.
    pub fn process(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let cap = match launcher.launcher_type.as_ref() {
            LauncherType::Process(p) => p.max_results,
            _ => 0,
        };
        let data = single_launcher(&launcher, cap, cx);
        Self::new(
            data,
            Arc::from([]),
            ModelSource::Process(ProcessModel::new(launcher)),
            FilterPolicy::All,
        )
    }

    /// File search model, optionally starting in `dir`.
    /// Starts empty; filled by `FileSearchModel` searches.
    pub fn file_search(
        launcher: Arc<LauncherConfig>,
        dir: Option<SharedString>,
        cx: &mut App,
    ) -> Self {
        let cap = match launcher.launcher_type.as_ref() {
            LauncherType::Files(p) => p.max_results,
            _ => 0,
        };
        let data = single_launcher(&launcher, cap, cx);
        Self::new(
            data,
            Arc::from([]),
            ModelSource::Files(FileSearchModel::new(launcher, dir)),
            FilterPolicy::All,
        )
    }
}

// Getters
impl<D: ModelData> Model<D> {
    /// Handle to the data this model filters.
    /// Cloning is cheap; it's a reference-counted entity handle.
    #[inline(always)]
    pub fn data(&self) -> Entity<D> {
        self.data.clone()
    }

    /// Ids of the visible items, in display order.
    /// Cloning is cheap; it's a shared `Arc`.
    #[inline(always)]
    pub fn filtered_indices(&self) -> Arc<[D::Id]> {
        self.filtered.clone()
    }

    /// The query that produced the current results, if any.
    #[inline(always)]
    pub fn last_query(&self) -> Option<SharedString> {
        self.last_query.clone()
    }

    /// Number of visible (filtered) items.
    #[inline(always)]
    pub fn len(&self) -> usize {
        self.filtered.len()
    }

    /// The search index for `data`, rebuilding it if `data` changed.
    /// Returns `(index, rebuilt)`.
    pub fn search_index(&mut self, data: &D) -> (Arc<SearchIndex<D::Id>>, bool) {
        match &self.index {
            Some(idx) if !idx.is_stale(data) => (idx.clone(), false),
            _ => {
                let idx = Arc::new(data.build_index());
                self.index = Some(idx.clone());
                (idx, true)
            }
        }
    }
}

// Selection
impl<D: ModelData> Model<D> {
    /// Selected position, clamped to the visible items.
    #[inline]
    pub fn selected_index(&self) -> usize {
        self.cursor.index(self.len())
    }

    /// Id of the selected item, if any items are visible.
    #[inline]
    pub fn selected_id(&self) -> Option<D::Id> {
        self.cursor.id(&self.filtered)
    }

    /// Selects position `n`. Returns `false` (and changes nothing) if out of range.
    pub fn select(&mut self, n: usize) -> bool {
        self.cursor.select(n, self.len())
    }

    /// Selects the item with `id`. Returns `false` if it isn't visible.
    pub fn select_id(&mut self, id: D::Id) -> bool {
        self.cursor.select_id(&id, &self.filtered)
    }

    /// Back to the first item; the next `set_results` won't try to keep the selection.
    pub fn reset_selection(&mut self) {
        self.cursor.reset();
    }

    /// Replaces the visible items. If the selection was explicit and the selected item is still
    /// visible, it stays selected (e.g. a plugin refresh doesn't move the cursor).
    pub fn set_results(&mut self, results: Arc<[D::Id]>, query: SharedString) -> ResultsChange {
        let changed = self.filtered != results;
        let restored = self.cursor.restore(&self.filtered, &results);
        self.filtered = results;
        self.last_query = Some(query);
        ResultsChange { changed, restored }
    }
}

/// Where the model's items come from.
pub enum ModelSource {
    /// Fixed data, filtered locally (home, dmenu, emoji, messages).
    Static,
    /// Results from a file search, updated as the query changes
    Files(FileSearchModel),
    /// Results from the running-process list, updtaed as the query changes.
    Process(ProcessModel),
}

impl ModelSource {
    /// True if items are filtered locally instead of by an external search.
    #[inline(always)]
    pub fn is_static(&self) -> bool {
        matches!(self, Self::Static)
    }

    /// Starts an external search. No-op for `Static`.
    /// Results are written into `data`, then handed back to `view` via `apply_results`.
    pub fn search(
        &mut self,
        query: Arc<str>,
        data: LauncherWeakEntity,
        view: WeakEntity<LauncherView>,
        cx: &mut App,
    ) {
        match self {
            Self::Files(s) => s.search(query, data, view, cx),
            Self::Process(s) => s.search(query, data, view, cx),
            Self::Static => {}
        }
    }
}

use std::{collections::HashMap, sync::Arc};

use gpui::{App, AppContext, Entity, SharedString, Task};

use crate::{
    app::{LauncherEntity, data::LauncherData},
    launcher::{Launcher, LauncherConfig, LauncherId, LocalId},
    ui::{
        launcher::filter_and_sort::FilterPolicy,
        model::{
            cursor::Cursor,
            data::ModelData,
            index::SearchIndex,
            utils::{all_indices, single_launcher},
        },
    },
};

pub mod cursor;
pub mod data;
pub mod index;
pub mod list_data;
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
    pub fn new(data: Entity<D>, filtered: Arc<[D::Id]>, filter_policy: FilterPolicy) -> Self {
        Self {
            data,
            filtered,
            cursor: Cursor::default(),
            last_query: None,
            index: None,
            pending: None,
            filter_policy,
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
        Self::new(entity, filtered, FilterPolicy::QueryOnly)
    }

    /// Local model over an existing data entity, with every item visible.
    /// Used for home and dmenu, which share their launcher entity.
    pub fn standard_with_entity(data: LauncherEntity, cx: &App) -> Self {
        let filtered = all_indices(data.read(cx));
        Self::new(data, filtered, FilterPolicy::All)
    }

    /// Empty model over a single launcher, for views that fill it with an external search
    /// (files, processes). `cap` preallocates room for the results.
    pub fn external(launcher: &Arc<LauncherConfig>, cap: usize, cx: &mut App) -> Self {
        let data = single_launcher(launcher, cap, cx);
        Self::new(data, Arc::from([]), FilterPolicy::All)
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

/// Object-safe view of a `Model<D>` for code that doesn't know `D` (shell, navigator):
/// size, selection and last query.
pub trait AnyModel {
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
    /// Selected position, clamped to the visible items.
    fn selected_index(&self) -> usize;
    /// Selects position `n`. Returns `false` (and changes nothing) if out of range.
    fn select(&mut self, n: usize) -> bool;
    /// Back to the first item.
    fn reset_selection(&mut self);
    /// The query that produced the current results, if any.
    fn last_query(&self) -> Option<SharedString>;
    /// Forget the last query, so the next one always re-runs.
    fn clear_last_query(&mut self);
}

impl<D: ModelData> AnyModel for Model<D> {
    #[inline]
    fn len(&self) -> usize {
        Model::len(self)
    }
    #[inline]
    fn selected_index(&self) -> usize {
        Model::selected_index(self)
    }
    #[inline]
    fn select(&mut self, n: usize) -> bool {
        Model::select(self, n)
    }
    #[inline]
    fn reset_selection(&mut self) {
        Model::reset_selection(self)
    }
    #[inline]
    fn last_query(&self) -> Option<SharedString> {
        self.last_query.clone()
    }
    #[inline]
    fn clear_last_query(&mut self) {
        self.last_query = None;
    }
}

use std::{collections::HashMap, rc::Rc, sync::Arc};

use gpui::{App, AppContext, SharedString, Task, WeakEntity};

use crate::{
    app::{LauncherEntity, LauncherWeakEntity},
    launcher::{Launcher, LauncherConfig, LauncherId, LocalId, variant_type::LauncherType},
    ui::{
        launcher::{LauncherView, filter_and_sort::FilterPolicy},
        model::{
            file::FileSearchModel,
            process::ProcessModel,
            utils::{all_indices, single_launcher},
        },
    },
};

pub mod emoji;
pub mod file;
pub mod home;
pub mod message;
pub mod pipeline;
pub mod process;
mod utils;

pub type ItemIdx = (LauncherId, LocalId);

/// Data structure that holds all item indices.
pub struct Model {
    /// The launcher data this model shows.
    /// Internally handled as: `Rc<HashMap<LauncherId, Launcher>>`.
    pub data: LauncherEntity,
    /// Indices of the visible items, in display order.
    pub filtered: Arc<[ItemIdx]>,
    /// The query that produced `filtered` if any.
    pub last_query: Option<SharedString>,
    /// In-flight filter task; dropping it, cancels it.
    pub pending: Option<Task<Option<()>>>,
    /// Where the items come from: local filtering or an external search.
    pub source: ModelSource,
    /// Defines which filter rules should apply at runtime.
    pub filter_policy: FilterPolicy,
}

// Constructors
impl Model {
    /// Shared constructor. Starts with no query and no pending task.
    pub fn new(
        data: LauncherEntity,
        filtered: Arc<[ItemIdx]>,
        source: ModelSource,
        filter_policy: FilterPolicy,
    ) -> Self {
        Self {
            data,
            filtered,
            last_query: None,
            pending: None,
            filter_policy,
            source,
        }
    }

    /// Local model over owned launcher data, with every item visible.
    pub fn standard(data: HashMap<LauncherId, Launcher>, cx: &mut App) -> Self {
        Self::standard_with_entity(cx.new(|_| Rc::new(data)), cx)
    }

    pub fn sub_view(data: HashMap<LauncherId, Launcher>, cx: &mut App) -> Self {
        let filtered = all_indices(&data);
        let entity = cx.new(|_| Rc::new(data));
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
impl Model {
    /// Handle to the launcher data this model filters.
    /// Cloning is cheap; it's a reference-counted entity handle.
    #[inline(always)]
    pub fn data(&self) -> LauncherEntity {
        self.data.clone()
    }

    /// Indices of the visible items, in display order.
    /// Cloning is cheap; it's a shared `Arc`.
    #[inline(always)]
    pub fn filtered_indices(&self) -> Arc<[ItemIdx]> {
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

use gpui::{AppContext, AsyncApp, Context, SharedString, WeakEntity};

use crate::app::{LauncherEntity, LauncherEntityInner, LauncherWeakEntity};
use crate::launcher::{Launcher, LauncherId, LauncherValues};
use crate::ui::launcher::views::NavigationViewType;
use crate::ui::launcher::{LauncherMode, LauncherView, ModeTransition};
use crate::ui::model::Model;
use crate::ui::traits::RenderableChildDelegate;
use crate::ui::utils::scoring::SortKey;
use crate::ui::utils::search::SherlockSearch;
use crate::utils::config::HomeType;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

struct FilterResult {
    index: (usize, usize),
    prio: SortKey,
    limiter: Option<(LauncherId, u16)>,
}

impl LauncherView {
    pub fn apply_results(
        &mut self,
        results: Arc<[(usize, usize)]>,
        query: impl Into<SharedString>,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.navigation.current().style.list_state() else {
            return;
        };

        let mut changed = force;
        let query: SharedString = query.into();

        let old_count = state.item_count();
        let new_count = results.len();
        if old_count != new_count {
            state.splice(0..old_count, new_count);
        }

        self.active_bar = 0;
        self.navigation.with_model_mut(cx, |mdl, _| match mdl {
            Model::Standard {
                filtered_indices: idx,
                last_query: q,
                ..
            }
            | Model::FileSearch {
                filtered_indices: idx,
                last_query: q,
                ..
            }
            | Model::Process {
                filtered_indices: idx,
                last_query: q,
                ..
            } => {
                if idx != &results {
                    changed = true;
                    *idx = results;
                }
                *q = Some(query.clone());
            }
        });

        self.update_sync(query, cx);

        if changed {
            self.focus_first(cx);
        } else if self.variable_input.is_empty() {
            self.update_vars(cx);
            cx.notify();
        }
    }
    pub fn filter_and_sort_sync(&mut self, cx: &mut Context<Self>) {
        self.filter_and_sort_internal(false, true, cx);
    }
    pub fn filter_and_sort(&mut self, cx: &mut Context<Self>) {
        self.filter_and_sort_internal(false, false, cx);
    }
    pub fn force_filter_and_sort(&mut self, cx: &mut Context<Self>) {
        self.filter_and_sort_internal(true, false, cx);
    }
    /// The main reactive entry point for query processing.
    ///
    /// 1. **Mode Detection**: Checks for prefix triggers to push new views or clear input.
    /// 2. **Dispatch**: Selects a search strategy based on the current `ModelKind`.
    ///    - `BufferedSearch`: Delegates to the specific model's async search.
    ///    - `Standard`: Spawns a background task to filter and sort local data.
    /// 3. **Processing**: Applies the 6-rule filtering engine and calculates priority
    ///    scores via `make_prio`.
    /// 4. **Concurrency**: Uses a `deferred_render_task` to debounce typing and release
    ///    data locks early, preventing UI stutters.
    ///
    /// # Arguments
    /// * `force` - Bypasses query-change optimization to re-run the search.
    fn filter_and_sort_internal(&mut self, force: bool, sync: bool, cx: &mut Context<Self>) {
        let mut query: SharedString = self.text_input.read(cx).content.to_lowercase().into();

        // handle mode change
        {
            let nav_mut = self.navigation.current_mut();
            match nav_mut.mode.transition_for_query(&query, &self.modes) {
                ModeTransition::None => {}
                ModeTransition::ClearInput => {
                    self.text_input.update(cx, |this, _cx| {
                        this.reset();
                    });
                    query = "".into();
                }
                ModeTransition::PushStack(launcher) => {
                    let Ok(view) = NavigationViewType::try_from(&launcher.launcher_type) else {
                        return;
                    };
                    self.text_input.update(cx, |this, _| this.reset());
                    self.navigation.push(view.create_view(launcher, cx));
                    query = "".into();
                }
            }
        }

        enum ModelKind {
            BufferedSearch {
                weak_data: LauncherWeakEntity,
                last_query: Option<SharedString>,
            },
            Standard {
                data: LauncherEntity,
            },
        }

        let kind = self.navigation.with_model(cx, |mdl| match mdl {
            Model::FileSearch {
                data, last_query, ..
            }
            | Model::Process {
                data, last_query, ..
            } => ModelKind::BufferedSearch {
                weak_data: data.downgrade(),
                last_query: last_query.clone(),
            },
            Model::Standard { data, .. } => ModelKind::Standard { data: data.clone() },
        });

        match kind {
            ModelKind::BufferedSearch {
                weak_data,
                last_query,
            } => {
                if !force && last_query.is_some_and(|s| s == query) {
                    return;
                }

                let weak_self = cx.entity().downgrade();
                self.navigation.with_model_mut(cx, |mdl, cx| match mdl {
                    Model::FileSearch { search, .. } => {
                        search.search(query.into(), weak_data, weak_self, cx);
                    }
                    Model::Process { search, .. } => {
                        search.search(query.into(), weak_data, weak_self, cx);
                    }
                    _ => {}
                });
            }
            ModelKind::Standard { data } => {
                // drop active tasks
                self.navigation.with_model_mut(cx, |mdl, _| {
                    if let Model::Standard {
                        deferred_render_task,
                        ..
                    } = mdl
                    {
                        *deferred_render_task = None;
                    }
                });

                if sync {
                    self.filter_update_sync(data, query, force, cx);
                } else {
                    self.filter_update_async(data, query, force, cx);
                }
            }
        }
    }
    fn filter_update_sync(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let (data_arc, mode, counter_cache) = self.prepare_filter_inputs(&data, cx);
        let results_arc =
            Self::compute_filtered_results(data_arc, mode.as_str(), &query, counter_cache, cx);
        self.apply_results(results_arc, query, force, cx);
    }
    fn filter_update_async(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let (data_arc, mode, counter_cache) = self.prepare_filter_inputs(&data, cx);
        let render_task = Some(cx.spawn(
            move |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let mode = mode.as_str();
                    let results_arc = Self::compute_filtered_results(
                        data_arc,
                        mode,
                        &query,
                        counter_cache,
                        &mut cx,
                    );

                    this.update(&mut cx, |this, cx| {
                        this.apply_results(results_arc, query, force, cx);
                    })
                    .ok();

                    Some(())
                }
            },
        ));

        // set active render task
        self.navigation.with_model_mut(cx, |mdl, _| {
            if let Model::Standard {
                deferred_render_task,
                ..
            } = mdl
            {
                *deferred_render_task = render_task;
            }
        })
    }
    fn compute_filtered_results<C: AppContext>(
        data_arc: LauncherEntityInner,
        mode: &str,
        query: &SharedString,
        mut limit_cache: HashMap<LauncherId, u16>,
        cx: &mut C,
    ) -> Arc<[(usize, usize)]> {
        let is_home = query.is_empty() && mode == "all";

        // collects Vec<(index, priority)>
        let mut results: Vec<FilterResult> = (0..data_arc.len())
            .map(|i| (i, &data_arc[i]))
            .filter(|(_, launcher)| {
                // [Rule 1]
                // Case 1: Early return if mode applies but item is not assigned to that mode
                // Case 2: Early return if current mode is not required mode for item
                if Some(mode) != launcher.config.alias.as_deref()
                    && (mode != "all" || launcher.config.priority < 1)
                {
                    return false;
                }

                // [Rule 2]
                // Early return if not home but item is assigned to only show on home
                if !is_home && launcher.config.home == HomeType::OnlyHome {
                    return false;
                }

                // [Rule 3]
                // Early return if item should only show on search but mode is home
                if is_home && launcher.config.home == HomeType::Search {
                    return false;
                }

                true
            })
            .flat_map(|(launcher_idx, launcher)| {
                launcher
                    .children
                    .iter()
                    .enumerate()
                    .map(move |(child_idx, child)| {
                        (launcher_idx, launcher.config.clone(), child_idx, child)
                    })
            })
            .filter(|(_, launcher, _, child)| {
                // [Rule 4]
                // Early return if item should always show (websearch for example)
                if launcher.home == HomeType::Persist {
                    return true;
                }

                // [Rule 5]
                // Early return if based show (calc for example) applies
                if let Some(based) = child.based_show(query, cx) {
                    return based;
                }
                // [Rule 6]
                // Check if query matches
                child.search().fuzzy_match(query)
            })
            .map(
                |(launcher_idx, launcher_config, child_idx, child)| FilterResult {
                    index: (launcher_idx, child_idx),
                    prio: child.priority().sort_key(query, child.search()),
                    limiter: launcher_config
                        .limit
                        .map(|limit| (LauncherId::from(launcher_config.as_ref()), limit)),
                },
            )
            .collect();

        // sort based on priority
        results.sort_unstable_by_key(|a| a.prio);

        // strip the priority from results
        results
            .into_iter()
            .filter_map(|r| {
                if let Some((id, limit)) = r.limiter {
                    let count = limit_cache.entry(id).or_insert(0);
                    *count += 1;
                    (*count <= limit).then_some(r.index)
                } else {
                    Some(r.index)
                }
            })
            .collect::<Vec<_>>()
            .into()
    }

    /// Shared setup for both sync and async filtering: snapshot the current
    /// mode, clone the data handle, and clone the limit cache. Pulled into
    /// one place so the two call sites can't drift.
    /// Returns: Data, Mode, LimitCache
    fn prepare_filter_inputs(
        &self,
        data: &LauncherEntity,
        cx: &Context<Self>,
    ) -> (LauncherEntityInner, LauncherMode, HashMap<LauncherId, u16>) {
        let mode = self.navigation.current().mode.clone();
        let data_rc = data.read(cx).clone();
        let limit_cache = HashMap::new();
        (data_rc, mode, limit_cache)
    }
}

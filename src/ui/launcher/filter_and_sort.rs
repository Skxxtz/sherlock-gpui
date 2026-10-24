use std::{collections::HashMap, sync::Arc};

use gpui::{AppContext, AsyncApp, Context, SharedString, WeakEntity};

use crate::{
    QUERY_MEMORY,
    app::{LauncherEntity, LauncherEntityInner},
    launcher::{LauncherConfig, LauncherId, LauncherValues},
    ui::{
        launcher::{LauncherMode, LauncherView, ModeTransition, navigator::handle::ViewKind},
        traits::RenderableChildDelegate,
        utils::{scoring::SortKey, search::SherlockSearch},
    },
    utils::config::HomeType,
};

struct FilterResult {
    index: (LauncherId, usize),
    prio: SortKey,
    limiter: Option<(LauncherId, u16)>,
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum FilterPolicy {
    /// Use all available filter rules
    #[default]
    All,
    /// Only match the search string
    QueryOnly,
}

impl FilterPolicy {
    #[inline(always)]
    pub fn keep_launchers(self, cfg: &LauncherConfig, mode: &str, is_home: bool) -> bool {
        if self == Self::QueryOnly {
            return true;
        }

        // [Rule 1]
        // Case 1: Early return if mode applies but item is not assigned to that mode
        // Case 2: Early return if current mode is not required mode for item
        if Some(mode) != cfg.alias.as_deref() && (mode != "all" || cfg.priority < 1) {
            return false;
        }

        // [Rule 2]
        // Not home but item is assigned to only show on home
        // [Rule 3]
        // Item should only show on search but mode is home
        match cfg.home {
            HomeType::Search => !is_home,
            HomeType::OnlyHome => is_home,
            _ => true,
        }
    }

    #[inline(always)]
    pub fn persists(self, cfg: &LauncherConfig) -> bool {
        // [Rule 4]
        // Early return if item should always show (websearch for example)
        self == Self::All && cfg.home == HomeType::Persist
    }

    #[inline(always)]
    pub fn uses_limit(self) -> bool {
        self == Self::All
    }
}

impl LauncherView {
    pub fn apply_results(
        &mut self,
        results: Arc<[(LauncherId, usize)]>,
        query: impl Into<SharedString>,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let state = &self.navigation.current().layout.list;
        let mut changed = force;
        let query: SharedString = query.into();

        let old_count = state.item_count();
        let new_count = results.len();
        if old_count != new_count {
            state.splice(0..old_count, new_count);
        }

        self.active_bar = 0;
        self.navigation.with_model_mut(cx, |mdl, _| {
            if mdl.filtered != results {
                changed = true;
                mdl.filtered = results;
            }
            mdl.last_query = Some(query.clone());
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
                    let Some(view) = ViewKind::try_from(&launcher.launcher_type)
                        .ok()
                        .and_then(|kind| kind.build(launcher, cx))
                    else {
                        return;
                    };
                    self.text_input.update(cx, |this, _| this.reset());
                    self.navigation.push(view);
                    query = "".into();
                }
            }
        }

        let (data, last_query, is_static) = self.navigation.with_model(cx, |m| {
            (m.data.clone(), m.last_query.clone(), m.source.is_static())
        });

        // Handle searches on non-static models (file, process).
        if !is_static {
            if !force && last_query.as_ref() == Some(&query) {
                return;
            }

            let view = cx.entity().downgrade();
            let weak_data = data.downgrade();
            self.navigation.with_model_mut(cx, |m, cx| {
                m.source.search(query.into(), weak_data, view, cx)
            });
            return;
        }

        crate::features::plugin::dispatch_query(data.read(cx), &query);

        // drop active tasks
        self.navigation.with_model_mut(cx, |m, _| m.pending = None);

        if sync {
            self.filter_update_sync(data, query, force, cx);
        } else {
            self.filter_update_async(data, query, force, cx);
        }
    }
    fn filter_update_sync(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let (data_arc, mode, policy, counter_cache) = self.prepare_filter_inputs(&data, cx);
        let results_arc = Self::compute_filtered_results(
            data_arc,
            policy,
            mode.as_str(),
            &query,
            counter_cache,
            cx,
        );
        self.apply_results(results_arc, query, force, cx);
    }
    fn filter_update_async(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let (data_arc, mode, policy, counter_cache) = self.prepare_filter_inputs(&data, cx);
        let render_task = Some(cx.spawn(
            move |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let mode = mode.as_str();
                    let results_arc = Self::compute_filtered_results(
                        data_arc,
                        policy,
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
        self.navigation
            .with_model_mut(cx, |m, _| m.pending = render_task);
    }
    fn compute_filtered_results<C: AppContext>(
        data_arc: LauncherEntityInner,
        policy: FilterPolicy,
        mode: &str,
        query: &SharedString,
        mut limit_cache: HashMap<LauncherId, u16>,
        cx: &mut C,
    ) -> Arc<[(LauncherId, usize)]> {
        let is_home = query.is_empty() && mode == "all";
        let boosts = QUERY_MEMORY
            .get()
            .and_then(|l| l.read().ok())
            .map(|m| m.boosts(query))
            .unwrap_or_default();

        // collects Vec<(index, priority)>
        let mut results: Vec<FilterResult> = data_arc
            .iter()
            .filter(|(_, launcher)| policy.keep_launchers(&launcher.config, mode, is_home))
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
                if policy.persists(launcher) {
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
            .map(|(launcher_idx, launcher_config, child_idx, child)| {
                let boost = child.item_key_ref().map(|id| boosts.get(id)).unwrap_or(0.);
                FilterResult {
                    index: (*launcher_idx, child_idx),
                    prio: child.priority().sort_key(query, child.search(), boost),
                    limiter: launcher_config
                        .limit
                        .map(|limit| (LauncherId::from(launcher_config.as_ref()), limit)),
                }
            })
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
    ) -> (
        LauncherEntityInner,
        LauncherMode,
        FilterPolicy,
        HashMap<LauncherId, u16>,
    ) {
        let mode = self.navigation.current().mode.clone();
        let policy = self.navigation.with_model(cx, |m| m.filter_policy);
        let data_rc = data.read(cx).clone();
        (data_rc, mode, policy, HashMap::new())
    }
}

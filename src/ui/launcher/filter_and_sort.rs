use std::sync::Arc;

use gpui::{AppContext, AsyncApp, Context, SharedString, WeakEntity};

pub use crate::ui::model::pipeline::FilterPolicy;
use crate::{
    QUERY_MEMORY,
    app::{LauncherEntity, LauncherEntityInner},
    launcher::LauncherValues,
    ui::{
        launcher::{LauncherMode, LauncherView, ModeTransition, navigator::handle::ViewKind},
        model::{
            ItemIdx,
            pipeline::{self, FilterCtx, GroupGate, Scored},
        },
        traits::RenderableChildDelegate,
        utils::search::SherlockSearch,
    },
};

impl LauncherView {
    pub fn apply_results(
        &mut self,
        results: Arc<[ItemIdx]>,
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
        let (data_arc, mode, policy) = self.prepare_filter_inputs(&data, cx);
        let results_arc =
            Self::compute_filtered_results(data_arc, policy, mode.as_str(), &query, cx);
        self.apply_results(results_arc, query, force, cx);
    }
    fn filter_update_async(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let (data_arc, mode, policy) = self.prepare_filter_inputs(&data, cx);
        let render_task = Some(cx.spawn(
            move |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let results_arc = Self::compute_filtered_results(
                        data_arc,
                        policy,
                        mode.as_str(),
                        &query,
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
    /// Runs the rule pipeline (see `model::pipeline`) over `data_arc`.
    /// Only the cx-dependent parts (`based_show`, fuzzy match, boosts) live here.
    fn compute_filtered_results<C: AppContext>(
        data_arc: LauncherEntityInner,
        policy: FilterPolicy,
        mode: &str,
        query: &SharedString,
        cx: &mut C,
    ) -> Arc<[ItemIdx]> {
        let fcx = FilterCtx::new(policy, mode, query);
        let boosts = QUERY_MEMORY
            .get()
            .and_then(|l| l.read().ok())
            .map(|m| m.boosts(query))
            .unwrap_or_default();

        let results: Vec<Scored<ItemIdx>> = data_arc
            .iter()
            // [Rules 1–4] once per launcher
            .filter_map(|(launcher_idx, launcher)| {
                let gate = fcx.gate(&launcher.config);
                (gate != GroupGate::Hidden).then_some((*launcher_idx, launcher, gate))
            })
            .flat_map(|(launcher_idx, launcher, gate)| {
                let limit = pipeline::limit_of(&launcher.config);
                launcher
                    .children
                    .iter_with_ids()
                    .map(move |(child_idx, child)| (launcher_idx, gate, limit, child_idx, child))
            })
            // [Rules 4–6] per item
            .filter(|(_, gate, _, _, child)| {
                pipeline::admit(
                    *gate,
                    || child.based_show(query, cx),
                    || child.search().fuzzy_match(query),
                )
            })
            .map(|(launcher_idx, _, limit, child_idx, child)| {
                let boost = child.item_key_ref().map(|id| boosts.get(id)).unwrap_or(0.);
                Scored {
                    index: (launcher_idx, child_idx),
                    prio: child.priority().sort_key(query, child.search(), boost),
                    limit,
                }
            })
            .collect();

        pipeline::sort_and_limit(results, policy)
    }

    /// Shared setup for both sync and async filtering: snapshot the current
    /// mode, policy and data handle. Pulled into one place so the two call
    /// sites can't drift.
    fn prepare_filter_inputs(
        &self,
        data: &LauncherEntity,
        cx: &Context<Self>,
    ) -> (LauncherEntityInner, LauncherMode, FilterPolicy) {
        let mode = self.navigation.current().mode.clone();
        let policy = self.navigation.with_model(cx, |m| m.filter_policy);
        let data_rc = data.read(cx).clone();
        (data_rc, mode, policy)
    }
}

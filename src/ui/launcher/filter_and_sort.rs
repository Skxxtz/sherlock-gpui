use std::sync::Arc;

use gpui::{AsyncApp, Context, SharedString, WeakEntity};

pub use crate::ui::model::pipeline::FilterPolicy;
use crate::{
    app::{LauncherEntity, LauncherEntityInner},
    ui::{
        launcher::{LauncherView, ModeTransition, navigator::handle::ViewKind},
        model::{ItemIdx, index::FilterJob},
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
        let change = self
            .navigation
            .with_model_mut(cx, |mdl, _| mdl.set_results(results, query.clone()));
        changed |= change.changed;

        self.update_sync(query, cx);

        if changed && change.restored {
            // same item still visible: keep it selected, just reveal it
            let n = self.navigation.selected_index(cx);
            self.focus_nth(n, cx);
        } else if changed {
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
        let job = self.prepare_filter_job(&data, query.clone(), cx);
        self.apply_results(job.run(), query, force, cx);
    }
    fn filter_update_async(
        &mut self,
        data: LauncherEntity,
        query: SharedString,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let job = self.prepare_filter_job(&data, query.clone(), cx);
        let render_task = Some(cx.spawn(
            move |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    // Pure items: scored off the main thread.
                    let results = cx
                        .background_executor()
                        .spawn(async move { job.run() })
                        .await;

                    this.update(&mut cx, |this, cx| {
                        this.apply_results(results, query, force, cx);
                    })
                    .ok();

                    Some(())
                }
            },
        ));

        // set active render task (replacing the old one cancels it)
        self.navigation
            .with_model_mut(cx, |m, _| m.pending = render_task);
    }

    /// Main-thread half of filtering, shared by the sync and async paths:
    ///
    /// 1. get the (cached) search index for `data`
    /// 2. evaluate rules 1–4 per launcher
    /// 3. score the contextual items (`based_show` needs `cx`)
    ///
    /// The returned job scores the pure items and sorts/limits; it is `Send`.
    fn prepare_filter_job(
        &mut self,
        data: &LauncherEntity,
        query: SharedString,
        cx: &mut Context<Self>,
    ) -> FilterJob<ItemIdx> {
        let mode: SharedString = self.navigation.current().mode.as_str().to_owned().into();
        let data: LauncherEntityInner = data.read(cx).clone();
        let (policy, (index, rebuilt)) = self
            .navigation
            .with_model_mut(cx, |m, _| (m.filter_policy, m.search_index(&data)));

        #[cfg(debug_assertions)]
        if rebuilt {
            debug_check_contextual(&index, &data, cx);
        }
        #[cfg(not(debug_assertions))]
        let _ = rebuilt;

        FilterJob::prepare(index, &data, policy, &mode, query, cx)
    }
}

/// Catches widgets that override `based_show` but forget `has_based_show`:
/// their `based_show` would silently never run. The default returns `None`
/// for any input, so anything else means an override.
#[cfg(debug_assertions)]
fn debug_check_contextual<D: crate::ui::model::data::ModelData>(
    index: &crate::ui::model::index::SearchIndex<D::Id>,
    data: &D,
    cx: &mut Context<LauncherView>,
) {
    for row in index.pure.iter() {
        // missing items report `Some(false)`; the index was just built, so all exist
        debug_assert!(
            data.based_show(row.id, "", cx).is_none(),
            "item {:?} overrides `based_show` but `has_based_show` returns false",
            row.text
        );
    }
}

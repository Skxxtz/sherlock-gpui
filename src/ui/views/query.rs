use std::sync::Arc;

use gpui::{App, AsyncApp, Context, WeakEntity};

use crate::{
    app::LauncherWeakEntity,
    ui::{
        launcher::LauncherView,
        model::{Model, data::ModelData, index::FilterJob},
        views::{Query, QueryResult},
    },
};

/// Filters static launcher data in `model` for `query`: notifies plugins, then `model_query`.
pub fn launcher_query<V: 'static>(
    model: &mut Model,
    query: &Query,
    cx: &mut Context<V>,
    model_of: fn(&mut V) -> &mut Model,
) -> QueryResult {
    crate::features::plugin::dispatch_query(model.data().read(cx), &query.text);
    model_query(model, query, cx, model_of)
}

/// Filters `model` for `query` through its search index: contextual items now, pure items now
/// (`sync`) or on the background executor.
///
/// Results are applied to the model (found again later via `model_of`); the shell is notified
/// through the return value (`Ready`) or `LauncherView::results_changed` (`Pending`).
pub fn model_query<V: 'static, D: ModelData>(
    model: &mut Model<D>,
    query: &Query,
    cx: &mut Context<V>,
    model_of: fn(&mut V) -> &mut Model<D>,
) -> QueryResult {
    // drop the in-flight task; its results are stale
    model.pending = None;

    let snapshot = model.data().read(cx).clone();
    let (index, rebuilt) = model.search_index(&snapshot);

    #[cfg(debug_assertions)]
    if rebuilt {
        debug_check_contextual(&index, &snapshot, cx);
    }
    #[cfg(not(debug_assertions))]
    let _ = rebuilt;

    let job = FilterJob::prepare(
        index,
        &snapshot,
        model.filter_policy,
        &query.mode,
        query.text.clone(),
        cx,
    );

    if query.sync {
        return QueryResult::Ready(model.set_results(job.run(), query.text.clone()));
    }

    let shell = query.shell.clone();
    let (text, force) = (query.text.clone(), query.force);
    model.pending = Some(cx.spawn(move |this: WeakEntity<V>, cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            // pure items: scored off the main thread
            let results = cx
                .background_executor()
                .spawn(async move { job.run() })
                .await;
            // two separate updates: view first, then shell
            let change = this
                .update(&mut cx, |view, _| {
                    model_of(view).set_results(results, text.clone())
                })
                .ok()?;
            shell
                .update(&mut cx, |shell, cx| {
                    shell.results_changed(change, text, force, cx)
                })
                .ok();
            Some(())
        }
    }));
    QueryResult::Pending
}

/// Runs an external search (files, processes) for `query`, unless the query is unchanged.
///
/// `search` writes its results into the model's data and reports them through
/// `LauncherView::apply_results`.
pub fn external_query(
    model: &Model,
    query: &Query,
    cx: &mut App,
    search: impl FnOnce(Arc<str>, LauncherWeakEntity, WeakEntity<LauncherView>, &mut App),
) -> QueryResult {
    if !query.force && model.last_query.as_ref() == Some(&query.text) {
        return QueryResult::Ignored;
    }
    search(
        query.text.as_ref().into(),
        model.data().downgrade(),
        query.shell.clone(),
        cx,
    );
    QueryResult::Pending
}

/// Catches widgets that override `based_show` but forget `has_based_show`: their `based_show`
/// would silently never run. The default returns `None` for any input.
#[cfg(debug_assertions)]
fn debug_check_contextual<D: ModelData, C: gpui::AppContext>(
    index: &crate::ui::model::index::SearchIndex<D::Id>,
    data: &D,
    cx: &mut C,
) {
    for row in index.pure.iter() {
        debug_assert!(
            data.based_show(row.id, "", cx).is_none(),
            "item {:?} overrides `based_show` but `has_based_show` returns false",
            row.text
        );
    }
}

use std::sync::Arc;

use gpui::{App, AsyncApp, Context, WeakEntity};

use crate::{
    app::LauncherWeakEntity,
    ui::{
        launcher::LauncherView,
        model::{Model, index::FilterJob},
        views::{Query, QueryResult},
    },
};

/// Filters static launcher data in `model` for `query`, through the search index:
/// contextual items now, pure items now (`sync`) or on the background executor.
pub fn launcher_query<V: 'static>(
    model: &mut Model,
    query: &Query,
    cx: &mut Context<V>,
) -> QueryResult {
    let data = model.data();

    crate::features::plugin::dispatch_query(data.read(cx), &query.text);

    // drop the in-flight task; its results are stale
    model.pending = None;

    let snapshot = data.read(cx).clone();
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
        return QueryResult::Ready(job.run());
    }

    let shell = query.shell.clone();
    let (text, force) = (query.text.clone(), query.force);
    model.pending = Some(cx.spawn(move |_: WeakEntity<V>, cx: &mut AsyncApp| {
        let mut cx = cx.clone();
        async move {
            // pure items: scored off the main thread
            let results = cx
                .background_executor()
                .spawn(async move { job.run() })
                .await;
            shell
                .update(&mut cx, |shell, cx| {
                    shell.apply_results(results, text, force, cx)
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
fn debug_check_contextual<D: crate::ui::model::data::ModelData, C: gpui::AppContext>(
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

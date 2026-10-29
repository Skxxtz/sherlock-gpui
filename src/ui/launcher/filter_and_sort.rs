use gpui::{Context, SharedString};

pub use crate::ui::model::pipeline::FilterPolicy;
use crate::ui::{
    launcher::{LauncherView, Route, navigator::handle::ViewKind},
    model::ResultsChange,
    views::{Query, QueryResult},
};

impl LauncherView {
    /// Called after the active view applied new results to its model: resizes the list, keeps or
    /// moves the selection and refreshes the selected item's vars.
    pub fn results_changed(
        &mut self,
        change: ResultsChange,
        query: impl Into<SharedString>,
        force: bool,
        cx: &mut Context<Self>,
    ) {
        let query: SharedString = query.into();

        let state = &self.navigation.current().layout.list;
        let old_count = state.item_count();
        let new_count = self.navigation.len(cx);
        if old_count != new_count {
            state.splice(0..old_count, new_count);
        }

        self.active_bar = 0;
        let changed = force || change.changed;

        self.navigation.current().handle.on_results(&query, cx);

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
    /// 1. **Routing**: prefix triggers switch the mode, clear the input or push a new view.
    /// 2. **Dispatch**: the active view filters (`SherlockView::on_query`). Results come back
    ///    right away (`Ready`) or later through `results_changed`.
    ///
    /// # Arguments
    /// * `force` - Bypasses query-change optimization to re-run the search.
    /// * `sync` - Compute results before returning instead of in the background.
    fn filter_and_sort_internal(&mut self, force: bool, sync: bool, cx: &mut Context<Self>) {
        let mut query: SharedString = self.text_input.read(cx).content.to_lowercase().into();

        // handle mode change
        match self.navigation.current().mode.route(&query, &self.modes) {
            Route::Stay => {}
            Route::Switch(mode) => {
                self.navigation.current_mut().mode = mode;
                self.text_input.update(cx, |this, _| this.reset());
                query = "".into()
            }
            Route::Push(mode) => {
                let Some(mut view) = mode
                    .launcher()
                    .and_then(|l| Some((ViewKind::try_from(&l.launcher_type).ok()?, l.clone())))
                    .and_then(|(kind, launcher)| kind.build(launcher, cx))
                else {
                    return;
                };
                view.mode = mode;
                self.text_input.update(cx, |this, _| this.reset());
                self.navigation.push(view);
                query = "".into()
            }
        }

        let query = Query {
            text: query,
            mode: self.navigation.current().mode.as_str().to_owned().into(),
            force,
            sync,
            shell: cx.entity().downgrade(),
        };
        match self.navigation.current().handle.on_query(&query, cx) {
            QueryResult::Ready(change) => self.results_changed(change, query.text, force, cx),
            QueryResult::Pending | QueryResult::Ignored => {}
        }
    }
}

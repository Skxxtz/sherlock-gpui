use std::sync::Arc;

use crate::launcher::children::RenderableChild;
use crate::launcher::children::{RenderableChildDelegate, SherlockSearch};
use crate::loader::utils::{ApplicationAction, ExecVariable};
use crate::utils::config::HomeType;
use gpui::{AppContext, WeakEntity};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, ListState, Subscription,
};
use gpui::{AsyncApp, Task};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use simd_json::prelude::Indexed;

use crate::ui::search_bar::TextInput;

pub mod actions;
pub mod render;

pub use actions::{
    Quit,
    FocusNext,
    FocusPrev,
    NextVar,
    PrevVar,
    Execute,
    OpenContext
};

pub struct SherlockMainWindow {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub list_state: ListState,
    pub _subs: Vec<Subscription>,
    pub selected_index: usize,

    // context menu
    pub context_idx: Option<usize>,
    pub context_actions: Arc<[Arc<ApplicationAction>]>,

    // variable input fields
    pub variable_input: Vec<Entity<TextInput>>,
    pub active_bar: usize,

    // Model
    pub deferred_render_task: Option<Task<Option<()>>>,
    pub data: Entity<Arc<Vec<RenderableChild>>>,
    pub filtered_indices: Arc<[usize]>,
    pub last_query: Option<String>,
}

impl Focusable for SherlockMainWindow {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl SherlockMainWindow {
    pub fn apply_results(&mut self, results: Arc<[usize]>, query: String, cx: &mut Context<Self>) {
        let old_count = self.list_state.item_count();
        let new_count = results.len();

        if let Some(&first_idx) = results.first() {
            let needed_vars: Option<Vec<ExecVariable>> = {
                let data_guard = self.data.read(cx);
                data_guard
                    .get(first_idx)
                    .and_then(|data| data.vars().map(|slice| slice.to_vec()))
            };

            if let Some(vars_to_create) = needed_vars {
                let current_top_idx = self.filtered_indices.get(self.selected_index).copied();
                if current_top_idx != Some(first_idx) {
                    self.variable_input = vars_to_create
                        .into_iter()
                        .map(|var| {
                            cx.new(|cx| TextInput {
                                focus_handle: cx.focus_handle(),
                                content: "".into(),
                                placeholder: var.placeholder(),
                                variable: Some(var),
                                selected_range: 0..0,
                                selection_reversed: false,
                                marked_range: None,
                                last_layout: None,
                                last_bounds: None,
                                is_selecting: false,
                            })
                        })
                        .collect();
                }
            } else {
                self.variable_input.clear();
            }
        }

        self.selected_index = 0;
        self.active_bar = 0;
        self.filtered_indices = results;
        self.last_query = Some(query);

        self.list_state.splice(0..old_count, new_count);

        cx.notify();
    }
    pub fn filter_and_sort(&mut self, cx: &mut Context<Self>) {
        let query = self.text_input.read(cx).content.to_lowercase();

        if Some(&query) == self.last_query.as_ref() {
            return;
        }

        if let Some(task) = self.deferred_render_task.take() {
            drop(task);
        }

        let data_arc = self.data.read(cx).clone();
        self.deferred_render_task = Some(cx.spawn(
            |this: WeakEntity<SherlockMainWindow>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                let mode = "all";
                async move {
                    let is_home = query.is_empty(); // && mode == "all";

                    // collects Vec<(index, priority)>
                    let mut results: Vec<(usize, f32)> = (0..data_arc.len())
                        .into_par_iter()
                        .map(|i| (i, &data_arc[i]))
                        .filter(|(_, data)| {
                            let home = data.home();

                            // [Rule 1]
                            // Early return if mode applies but item is not assigned to that mode
                            if mode != "all" && Some(mode) != data.alias() {
                                return false;
                            }

                            // [Rule 2]
                            // Early return if item should always show (websearch for example)
                            if home == HomeType::Persist {
                                return true;
                            }

                            // [Rule 3]
                            // Early return if based show (calc for example) applies
                            if let Some(based) = data.based_show(&query) {
                                return based;
                            }

                            // [Rule 4]
                            // Early return if not home but item is assigned to only show on home
                            if !is_home && home == HomeType::OnlyHome {
                                return false;
                            }

                            // [Rule 5]
                            // Early return if item should only show on search but mode is home
                            if is_home && home == HomeType::Search {
                                return false;
                            }

                            // [Rule 6]
                            // Check if query matches
                            data.search().fuzzy_match(&query)
                        })
                        .map(|(i, data)| (i, data.priority()))
                        .collect();

                    // drop here to release lock faster
                    drop(data_arc);

                    // sort based on priority
                    results.sort_unstable_by(|a, b| {
                        a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal)
                    });

                    // strip the priority from results
                    let results_arc: Arc<[usize]> = results
                        .into_iter()
                        .map(|(i, _)| i)
                        .collect::<Vec<_>>()
                        .into();

                    this.update(&mut cx, |this, cx| {
                        this.apply_results(results_arc, query, cx);
                    })
                    .ok();

                    Some(())
                }
            },
        ));
    }

    // pub fn get_icon_path(&self, icon_name: &str) -> Option<Arc<Path>> {
    //     // Check if we already have it
    //     if let Some(cached) = self.icon_cache.borrow().get(icon_name) {
    //         return cached.clone();
    //     }

    //     if let Ok(Some(icon)) = IconThemeGuard::lookup_icon(icon_name) {
    //         let path_arc: Arc<Path> = Arc::from(icon);
    //         self.icon_cache.borrow_mut().insert(icon_name.to_string(), Some(path_arc.clone()));
    //         return Some(path_arc)
    //     }

    //     let icon_size = if icon_name.ends_with(".svg") {
    //         256
    //     } else {
    //         64
    //     };

    //     let result = (|| {
    //         let icon_path = lookup_icon(icon_name)
    //             .with_size(icon_size)
    //             .with_search_paths(&["~/.local/share/icons/"])
    //             .ok()?
    //             .next()?
    //             .map(|i| i.path)
    //             .ok()?;

    //         Some(Arc::from(icon_path.into_boxed_path()))
    //     })();

    //     self.icon_cache
    //         .borrow_mut()
    //         .insert(icon_name.to_string(), result.clone());

    //     result
    // }

}

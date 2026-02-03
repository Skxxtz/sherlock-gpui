use std::sync::Arc;

use crate::launcher::children::{LauncherValues, RenderableChild};
use crate::launcher::children::{RenderableChildDelegate, SherlockSearch};
use crate::loader::utils::{ApplicationAction, ExecVariable};
use crate::utils::config::HomeType;
use gpui::{App, Context, Entity, FocusHandle, Focusable, ListState, SharedString, Subscription};
use gpui::{AppContext, WeakEntity};
use gpui::{AsyncApp, Task};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use simd_json::prelude::Indexed;

use crate::ui::search_bar::TextInput;

pub mod actions;
pub mod render;

pub use actions::{Execute, FocusNext, FocusPrev, NextVar, OpenContext, PrevVar, Quit};

pub struct SherlockMainWindow {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub list_state: ListState,
    pub _subs: Vec<Subscription>,
    pub selected_index: usize,

    // mode
    pub mode: LauncherMode,
    pub modes: Arc<[LauncherMode]>,

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
        self.list_state.scroll_to_reveal_item(self.selected_index);

        cx.notify();
    }
    pub fn filter_and_sort(&mut self, cx: &mut Context<Self>) {
        let mut query = self.text_input.read(cx).content.to_lowercase();

        if Some(&query) == self.last_query.as_ref() {
            return;
        }

        if let Some(task) = self.deferred_render_task.take() {
            drop(task);
        }

        // handle mode change
        if self.mode.transition_for_query(&query, &self.modes) {
            self.text_input.update(cx, |this, _cx| {
                this.reset();
            });
            query = "".into();
        }

        let data_arc = self.data.read(cx).clone();
        let mode = self.mode.clone();
        self.deferred_render_task = Some(cx.spawn(
            |this: WeakEntity<SherlockMainWindow>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let mode = mode.as_str();
                    let is_home = query.is_empty() && mode == "all";

                    // collects Vec<(index, priority)>
                    let mut results: Vec<(usize, f32)> = (0..data_arc.len())
                        .into_par_iter()
                        .map(|i| (i, &data_arc[i]))
                        .filter(|(_, data)| {
                            let home = data.home();

                            // [Rule 1]
                            // Case 1: Early return if mode applies but item is not assigned to that mode
                            // Case 2: Early return if current mode is not required mode for item
                            if Some(mode) != data.alias() {
                                if mode != "all" || data.priority() < 1.0 {
                                    return false;
                                }
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
}

#[derive(PartialEq, Eq, Clone)]
pub enum LauncherMode {
    Home,
    Search,
    Alias {
        short: SharedString,
        name: SharedString,
    },
}

impl LauncherMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Home | Self::Search => "all",
            Self::Alias { short, .. } => short.as_ref(),
        }
    }
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Alias { .. })
    }
    pub fn display_str(&self) -> SharedString {
        match self {
            // "".into() uses static literals (no allocation) → efficient
            Self::Home => "All".into(),
            Self::Search => "Search".into(),
            Self::Alias { name, .. } => name.clone(),
        }
    }
    pub fn transition_for_query(&mut self, query: &str, modes: &[Self]) -> bool {
        match (self, query.is_empty()) {
            (m @ Self::Search, true) => *m = Self::Home,
            (m @ Self::Home, false) => *m = Self::Search,
            (m @ Self::Search, false) | (m @ Self::Alias { .. }, false) => {
                if let Some(alias_input) = query.strip_suffix(' ') {
                    let found_mode = modes.iter().find(|mode| {
                        if let Self::Alias { short, .. } = mode {
                            short.eq_ignore_ascii_case(alias_input)
                        } else {
                            false
                        }
                    });

                    if let Some(new_mode) = found_mode {
                        *m = new_mode.clone();
                        // should clear search bar
                        return true;
                    }
                }
            }
            _ => {}
        }

        // only minor change
        false
    }
}

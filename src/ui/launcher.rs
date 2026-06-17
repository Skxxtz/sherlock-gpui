use crate::app::{RenderableChildEntity, RenderableChildWeak};
use crate::launcher::{Launcher, LauncherId, LauncherValues};
use crate::tokio_utils::SizedMessageObj;
use crate::ui::backdrop::Backdrop;
use crate::ui::choice::Choice;
use crate::ui::traits::RenderableChildDelegate;
use crate::ui::utils::scoring::SortKey;
use crate::ui::{
    launcher::context_menu::ContextMenuAction,
    launcher::views::{NavigationStack, NavigationViewType},
    model::Model,
    utils::search::SherlockSearch,
};
use crate::utils::config::HomeType;
use crate::utils::networking::ServerResponse;
use crate::utils::sized_message_sync::SizedMessage;
use gpui::{AnyElement, AsyncApp, IntoElement};
use gpui::{App, Context, Entity, FocusHandle, Focusable, SharedString, Subscription};
use gpui::{WeakEntity, WindowHandle};
use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::sync::Arc;

use crate::ui::search_bar::TextInput;

pub mod actions;
pub mod context_menu;
pub mod render;
pub mod views;

pub use actions::{
    Execute, NextVar, OpenContext, PrevVar, Quit, SelectionDown, SelectionLeft, SelectionRight,
    SelectionUp,
};

#[derive(Clone, Debug)]
pub enum VariableInput {
    Text(Entity<TextInput>),
    Choice(Entity<Choice>),
}
impl Focusable for VariableInput {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        match self {
            Self::Text(ent) => ent.read(cx).focus_handle.clone(),
            Self::Choice(ent) => ent.read(cx).focus_handle.clone(),
        }
    }
}

impl IntoElement for VariableInput {
    type Element = AnyElement;

    fn into_element(self) -> Self::Element {
        match self {
            Self::Text(ent) => ent.into_any_element(),
            Self::Choice(ent) => ent.into_any_element(),
        }
    }
}

pub struct LauncherView {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub _subs: Vec<Subscription>,

    // mode
    pub modes: Arc<[LauncherMode]>,

    // context menu
    pub context_idx: Option<usize>,
    pub context_actions: Arc<[Arc<ContextMenuAction>]>,
    pub has_actions: bool,

    // variable input fields
    pub variable_input: Vec<VariableInput>,
    pub active_bar: usize,

    // Model
    pub navigation: NavigationStack,
    pub limit_cache: Arc<HashMap<LauncherId, u16>>,

    // State
    pub config_initialized: bool,

    // Responses
    pub response_socket: Option<Arc<UnixStream>>,

    pub backdrop: Option<WindowHandle<Backdrop>>,
}

impl Focusable for LauncherView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Drop for LauncherView {
    fn drop(&mut self) {
        if let Some(socket) = self.response_socket.take() {
            let _ = socket.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl LauncherView {
    pub fn write_response(&self, what: String, cx: &mut App) {
        if let Some(stream) = self.response_socket.as_ref() {
            let mut stream = &**stream;
            let response = match SizedMessageObj::from_struct(&ServerResponse::Print(what)) {
                Ok(r) => r,
                Err(e) => {
                    self.navigation.push_message(e, cx);
                    return;
                }
            };

            let _ = stream.write_sized(response);
        }
    }

    pub fn apply_results(
        &mut self,
        results: Arc<[usize]>,
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
    pub fn filter_and_sort(&mut self, cx: &mut Context<Self>) {
        self.filter_and_sort_internal(false, cx);
    }
    pub fn force_filter_and_sort(&mut self, cx: &mut Context<Self>) {
        self.filter_and_sort_internal(true, cx);
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
    fn filter_and_sort_internal(&mut self, force: bool, cx: &mut Context<Self>) {
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
                weak_data: RenderableChildWeak,
                last_query: Option<SharedString>,
            },
            Standard {
                data: RenderableChildEntity,
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
                // reset cache
                Arc::make_mut(&mut self.limit_cache).clear();

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

                // filter result struct
                struct FilterResult {
                    index: usize,
                    prio: SortKey,
                    limiter: Option<(LauncherId, u16)>,
                }

                let mode = self.navigation.current().mode.clone();
                let data_arc = data.read(cx).clone();
                let mut limit_cache = self.limit_cache.clone();
                let render_task = Some(cx.spawn(
                    move |this: WeakEntity<LauncherView>, cx: &mut AsyncApp| {
                        let mut cx = cx.clone();
                        async move {
                            let mode = mode.as_str();
                            let is_home = query.is_empty() && mode == "all";
                            let counter_cache = Arc::make_mut(&mut limit_cache);

                            // collects Vec<(index, priority)>
                            let mut results: Vec<FilterResult> = (0..data_arc.len())
                                .map(|i| (i, &data_arc[i]))
                                .filter(|(_, data)| {
                                    let home = data.home();
                                    // [Rule 1]
                                    // Case 1: Early return if mode applies but item is not assigned to that mode
                                    // Case 2: Early return if current mode is not required mode for item
                                    if Some(mode) != data.alias()
                                        && (mode != "all" || data.priority().base < 1)
                                    {
                                        return false;
                                    }

                                    // [Rule 2]
                                    // Early return if item should always show (websearch for example)
                                    if home == HomeType::Persist {
                                        return true;
                                    }

                                    // [Rule 3]
                                    // Early return if not home but item is assigned to only show on home
                                    if !is_home && home == HomeType::OnlyHome {
                                        return false;
                                    }

                                    // [Rule 4]
                                    // Early return if based show (calc for example) applies
                                    if let Some(based) = data.based_show(&query, &mut cx) {
                                        return based;
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
                                .map(|(index, data)| FilterResult {
                                    index,
                                    prio: data.priority().sort_key(&query, data.search()),
                                    limiter: data.with_launcher(|l| {
                                        l.limit.map(|limit| (LauncherId::from(l.as_ref()), limit))
                                    }),
                                })
                                .collect();

                            // drop here to release lock faster
                            drop(data_arc);

                            // sort based on priority
                            results.sort_unstable_by_key(|a| a.prio);

                            // strip the priority from results
                            let results_arc: Arc<[usize]> = results
                                .into_iter()
                                .filter_map(|r| {
                                    if let Some((id, limit)) = r.limiter {
                                        let count = counter_cache.entry(id).or_insert(0);
                                        *count += 1;
                                        (*count <= limit).then_some(r.index)
                                    } else {
                                        Some(r.index)
                                    }
                                })
                                .collect::<Vec<_>>()
                                .into();

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
        }
    }
}

#[derive(Debug, PartialEq, Clone, Default)]
pub enum LauncherMode {
    #[default]
    Home,
    Search,
    Alias {
        short: SharedString,
        name: SharedString,
        launcher: Arc<Launcher>,
    },
}

pub enum ModeTransition {
    None,
    PushStack(Arc<Launcher>),
    ClearInput,
}

impl LauncherMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Home | Self::Search => "all",
            Self::Alias { short, .. } => short.as_ref(),
        }
    }
    pub fn display_str(&self) -> SharedString {
        match self {
            // "".into() uses static literals (no allocation) → efficient
            Self::Home => "All".into(),
            Self::Search => "Search".into(),
            Self::Alias { name, .. } => name.clone(),
        }
    }
    pub fn transition_for_query(&mut self, query: &str, modes: &[Self]) -> ModeTransition {
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
                        if let Self::Alias { launcher, .. } = new_mode
                            && launcher.needs_stack_push()
                        {
                            return ModeTransition::PushStack(launcher.clone());
                        }
                        // should clear search bar
                        return ModeTransition::ClearInput;
                    }
                }
            }
            _ => {}
        }

        // only minor change
        ModeTransition::None
    }
}

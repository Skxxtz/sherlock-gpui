use std::{cell::RefCell, collections::HashMap, path::Path, sync::Arc};

use crate::launcher::children::RenderableChild;
use crate::launcher::children::{RenderableChildDelegate, SherlockSearch};
use crate::utils::config::HomeType;
use gpui::{AnyElement, WeakEntity};
use gpui::{
    App, Context, Entity, FocusHandle, Focusable, ListState, Subscription, Window, actions, div,
    hsla, list, prelude::*, px, rgb,
};
use gpui::{AsyncApp, Task};
use linicon::lookup_icon;
use rayon::iter::{IntoParallelIterator, ParallelIterator};

use crate::ui::search_bar::TextInput;

actions!(example_input, [Quit, FocusNext, FocusPrev, Execute,]);

pub struct InputExample {
    pub text_input: Entity<TextInput>,
    pub focus_handle: FocusHandle,
    pub list_state: ListState,
    pub _subs: Vec<Subscription>,
    pub selected_index: usize,
    pub icon_cache: RefCell<HashMap<String, Option<Arc<Path>>>>,

    // Model
    pub deferred_render_task: Option<Task<Option<()>>>,
    pub data: Entity<Arc<Vec<RenderableChild>>>,
    pub filtered_indices: Arc<[usize]>,
    pub last_query: Option<String>,
}

impl Focusable for InputExample {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl InputExample {
    fn focus_next(&mut self, _: &FocusNext, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.data.read(cx).len();
        if count == 0 {
            return;
        }

        if self.selected_index < count - 1 {
            self.selected_index += 1;
            self.list_state.scroll_to_reveal_item(self.selected_index);
            cx.notify();
        }
    }
    fn focus_prev(&mut self, _: &FocusPrev, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.data.read(cx).len();
        if count == 0 {
            return;
        }

        if self.selected_index > 0 {
            self.selected_index -= 1;
            self.list_state.scroll_to_reveal_item(self.selected_index);
            cx.notify();
        }
    }
    fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) {
        let keyword = self.text_input.read(cx).content.as_str();
        if let Some(selected) = self
            .data
            .read(cx)
            .get(self.filtered_indices[self.selected_index])
        {
            match selected.execute(keyword) {
                Ok(exit) if exit => win.remove_window(),
                Err(e) => eprintln!("{e}"),
                _ => {}
            }
        }
    }
    fn quit(&mut self, _: &Quit, win: &mut Window, _: &mut Context<Self>) {
        win.remove_window();
    }
}

impl Render for InputExample {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let weak_self = cx.entity().downgrade();
        div()
            .id("sherlock")
            .track_focus(&self.focus_handle(cx))
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(0x0F0F0F))
            .border_2()
            .border_color(hsla(0., 0., 0.1882, 1.0))
            .rounded(px(5.))
            .shadow_xl()
            .overflow_hidden()
            .on_action(cx.listener(Self::focus_next))
            .on_action(cx.listener(Self::focus_prev))
            .on_action(cx.listener(Self::execute))
            .on_action(cx.listener(Self::quit))
            .child(
                // search bar
                div()
                    .flex()
                    .flex_row()
                    .w_full()
                    .items_center()
                    .px_4()
                    .py(px(4.))
                    .gap_3()
                    .child(div().text_color(rgb(0x888888)).child(""))
                    .child(div().w_auto().child(self.text_input.clone()))
                    // .children(iterator) TODO: implement the variable text fields here
                    .border_b_2()
                    .border_color(hsla(0., 0., 0.1882, 1.0)),
            )
            .child(
                div()
                    .id("results-container")
                    .flex_1()
                    .min_h_0()
                    .p_2()
                    .child(
                        list(self.list_state.clone(), move |idx, _win, cx| {
                            // 1. Upgrade and Read
                            let entity = weak_self.upgrade();
                            if entity.is_none() {
                                return div().into_any_element();
                            }
                            let state = entity.unwrap().read(cx);

                            // 2. Bounds Check - If this fails, we return an empty div to satisfy AnyElement
                            let data_idx = match state.filtered_indices.get(idx) {
                                Some(&i) => i,
                                None => return div().into_any_element(),
                            };

                            let data_guard = state.data.read(cx);
                            let child = match data_guard.get(data_idx) {
                                Some(c) => c,
                                None => return div().into_any_element(),
                            };

                            state.render_list_item(&child, idx)
                        })
                        .size_full(),
                    ),
            )
            .child(
                // statusbar
                div()
                    .h(px(30.))
                    .line_height(px(30.))
                    .w_full()
                    .bg(hsla(0., 0., 0.098, 1.0))
                    .border_t_1()
                    .border_color(hsla(0., 0., 0.1882, 1.0))
                    .px_5()
                    .text_size(px(13.))
                    .items_center()
                    .text_color(hsla(0.6, 0.0217, 0.3608, 1.0))
                    .child(String::from("Sherlock")),
            )
    }
}
impl InputExample {
    pub fn apply_results(&mut self, results: Arc<[usize]>, query: String, cx: &mut Context<Self>) {
        let old_count = self.list_state.item_count();
        let new_count = results.len();

        self.filtered_indices = results;
        self.selected_index = 0;
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
            |this: WeakEntity<InputExample>, cx: &mut AsyncApp| {
                let mut cx = cx.clone();
                let mode = "all";
                async move {
                    // collects Vec<(index, priority)>
                    //
                    let is_home = query.is_empty(); // && mode == "all";

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

    pub fn get_icon_path(&self, icon_name: &str) -> Option<Arc<Path>> {
        // Check if we already have it
        if let Some(cached) = self.icon_cache.borrow().get(icon_name) {
            return cached.clone();
        }

        let result = (|| {
            let icon_path = lookup_icon(icon_name)
                .with_size(24)
                .with_search_paths(&["~/.local/share/icons/"])
                .ok()?
                .next()?
                .map(|i| i.path)
                .ok()?;

            Some(Arc::from(icon_path.into_boxed_path()))
        })();

        self.icon_cache
            .borrow_mut()
            .insert(icon_name.to_string(), result.clone());

        result
    }

    fn render_list_item(&self, ad: &RenderableChild, idx: usize) -> AnyElement {
        let is_selected = self.selected_index == idx;
        let icon = ad.icon().and_then(|i| self.get_icon_path(&i));
        div()
            .id(("keystroke", idx))
            .w_full()
            .on_click(move |_, _, _| {
                println!("Clicked item {}", idx);
            })
            .child(
                div()
                    .group("")
                    .rounded_md()
                    .relative()
                    .mb(px(5.0))
                    .w_full()
                    .cursor_pointer()
                    .bg(if is_selected {
                        hsla(0., 0., 0.149, 1.0)
                    } else {
                        hsla(0., 0., 0., 0.)
                    })
                    .hover(|s| {
                        if is_selected {
                            s
                        } else {
                            s.bg(hsla(0., 0., 0.12, 1.0))
                        }
                    })
                    .child(ad.render(icon, is_selected)),
            )
            .into_any_element()
    }
}

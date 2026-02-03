use std::sync::Arc;

use gpui::{AppContext, Context, SharedString, Window, actions};
use smallvec::SmallVec;

use crate::{
    launcher::children::{RenderableChild, RenderableChildDelegate},
    loader::utils::ExecVariable,
    ui::{main_window::SherlockMainWindow, search_bar::TextInput},
};

actions!(
    example_input,
    [
        Quit,
        FocusNext,
        FocusPrev,
        NextVar,
        PrevVar,
        Execute,
        OpenContext,
        Backspace,
    ]
);

impl SherlockMainWindow {
    pub fn focus_nth(&mut self, n: usize, cx: &mut Context<Self>) {
        self.selected_index = n;
        self.list_state.scroll_to_reveal_item(n);

        // Handle variable inputs
        self.update_vars(cx);
        self.active_bar = 0;

        // Handle context menu entries
        self.context_actions = self
            .filtered_indices
            .get(n)
            .and_then(|i| self.data.read(cx).get(*i))
            .and_then(RenderableChild::actions)
            .unwrap_or_default();

        cx.notify()
    }
    pub(super) fn focus_next(&mut self, _: &FocusNext, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.filtered_indices.len();
        if count == 0 {
            return;
        }

        if let Some(idx) = self.context_idx {
            // handle context
            if idx < self.context_actions.len() - 1 {
                self.context_idx = Some(idx + 1);
                cx.notify();
            }
        } else {
            // handle normal view
            if self.selected_index < count - 1 {
                self.focus_nth(self.selected_index + 1, cx);
            }
        }
    }
    pub(super) fn focus_prev(&mut self, _: &FocusPrev, _: &mut Window, cx: &mut Context<Self>) {
        let count = self.data.read(cx).len();
        if count == 0 {
            return;
        }

        if let Some(idx) = self.context_idx {
            // handle context
            if idx > 0 {
                self.context_idx = Some(idx - 1);
                cx.notify();
            }
        } else {
            // handle normal view
            if self.selected_index > 0 {
                self.focus_nth(self.selected_index - 1, cx);
            }
        }
    }
    pub(super) fn next_var(&mut self, _: &NextVar, win: &mut Window, cx: &mut Context<Self>) {
        let total_inputs = 1 + self.variable_input.len();

        if self.active_bar < total_inputs - 1 {
            self.active_bar += 1;

            if self.active_bar == 0 {
                self.text_input.read(cx).focus_handle.focus(win);
            } else {
                let var_idx = self.active_bar - 1;
                let handle = self.variable_input[var_idx].read(cx).focus_handle.clone();
                handle.focus(win);
            }

            cx.notify();
        }
    }

    pub(super) fn prev_var(&mut self, _: &PrevVar, win: &mut Window, cx: &mut Context<Self>) {
        if self.active_bar > 0 {
            self.active_bar -= 1;

            if self.active_bar == 0 {
                self.text_input.read(cx).focus_handle.focus(win);
            } else {
                let var_idx = self.active_bar - 1;
                let handle = self.variable_input[var_idx].read(cx).focus_handle.clone();
                handle.focus(win);
            }

            cx.notify();
        }
    }
    pub(super) fn execute(&mut self, _: &Execute, win: &mut Window, cx: &mut Context<Self>) {
        if let Some(idx) = self.context_idx {
            if let Some(action) = self.context_actions.get(idx) {
                if let Some(selected) = self
                    .data
                    .read(cx)
                    .get(self.filtered_indices[self.selected_index])
                {
                    match selected.execute_action(action) {
                        Ok(exit) if exit => self.close_window(win, cx),
                        Err(e) => eprintln!("{e}"),
                        _ => {}
                    }
                }
            }
        } else {
            let keyword = self.text_input.read(cx).content.as_str();
            // collect variables
            let mut variables: SmallVec<[(SharedString, SharedString); 4]> = SmallVec::new();
            for s in &self.variable_input {
                let guard = s.read(cx);
                variables.push((guard.placeholder.clone(), guard.content.clone()));
            }

            if let Some(selected) = self
                .data
                .read(cx)
                .get(self.filtered_indices[self.selected_index])
            {
                match selected.execute(keyword, &variables) {
                    Ok(exit) if exit => self.close_window(win, cx),
                    Err(e) => eprintln!("{e}"),
                    _ => {}
                }
            }
        }
    }
    pub(super) fn open_context(
        &mut self,
        _: &OpenContext,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_actions.is_empty() {
            return;
        }

        // toggle logic
        if self.context_idx.take().is_none() {
            self.context_idx = Some(0);
        }

        cx.notify();
    }
    pub(super) fn close_context(&mut self, cx: &mut Context<Self>) {
        if let Some(_) = self.context_idx.take() {
            cx.notify();
        }
    }
    pub(super) fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) {
        if self.context_idx.is_some() {
            self.close_context(cx);
        } else {
            self.close_window(win, cx);
        }
    }
    pub(super) fn backspace(&mut self, _: &Backspace, win: &mut Window, cx: &mut Context<Self>) {
        println!("testing");
        cx.stop_propagation();
    }
    pub(super) fn close_window(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        // Cleanup
        self.variable_input.clear();
        self.filtered_indices = Arc::new([]);
        if let Some(task) = self.deferred_render_task.take() {
            drop(task)
        }

        // Close window
        win.remove_window();

        // Propagate state change
        cx.notify();
    }
    pub(super) fn update_vars(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.filtered_indices.get(self.selected_index).copied() else {
            return;
        };

        let needed_vars: Option<Vec<ExecVariable>> = {
            let data_guard = self.data.read(cx);
            data_guard
                .get(idx)
                .and_then(|data| data.vars().map(|slice| slice.to_vec()))
        };

        if let Some(vars_to_create) = needed_vars {
            self.variable_input = vars_to_create
                .into_iter()
                .map(|var| {
                    cx.new(|cx| TextInput {
                        focus_handle: cx.focus_handle(),
                        content: "".into(),
                        placeholder: var.placeholder(),
                        variable: Some(var),
                        // Initialize your other fields here...
                        selected_range: 0..0,
                        selection_reversed: false,
                        marked_range: None,
                        last_layout: None,
                        last_bounds: None,
                        is_selecting: false,
                    })
                })
                .collect();
        } else {
            self.variable_input.clear();
        }
    }
}

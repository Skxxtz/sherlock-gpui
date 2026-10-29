use std::{borrow::Cow, sync::Arc, time::Duration};

use gpui::{App, AppContext, Context, Focusable, KeyDownEvent, KeyUpEvent, SharedString, Window};
use simd_json::prelude::{ArrayTrait, Indexed};
use smallvec::SmallVec;

use super::execute;
use crate::{
    app::reset_generation,
    launcher::{LauncherValues, utils::exec_mode::ExecMode, variant_type::QuickActionCapabilities},
    sherlock_msg,
    ui::{
        actions::{
            launcher::*,
            search_bar::{Backspace as SearchBackspace, ShortcutAction},
        },
        components::{
            choice::Choice,
            search_bar::{EmptyBackspace, TextInput},
        },
        launcher::{
            LauncherView,
            VariableInput,
            context_menu::{ContextMenuAction, wrapper::Move},
            execute::{ExecRequest, ExecSource},
            mode::LauncherMode,
            navigator::views::MoveDirection,
        },
        runtime_arguments::RuntimeArgument,
        traits::{InputAction, RenderableChildDelegate},
        views::Propagation,
    },
    utils::errors::types::SherlockErrorType,
};

// Selection and focus
impl LauncherView {
    pub fn focus_first(&mut self, cx: &mut Context<Self>) {
        // Find the first focusable item
        let nav = &self.navigation;
        let first =
            (0..nav.len(cx)).find(|&i| nav.item_ref(i, cx).is_some_and(|item| item.spawn_focus()));
        if let Some(n) = first {
            self.focus_nth(n, cx);
        }
    }

    #[inline(always)]
    fn valid_selection_idx(&self, n: usize, cx: &mut Context<Self>) -> bool {
        let list_len = self.navigation.len(cx);
        n < list_len && list_len != 0
    }

    pub fn focus_nth(&mut self, n: usize, cx: &mut Context<Self>) {
        // early return on invalid index
        // or
        // early return if not a list view
        if !self.valid_selection_idx(n, cx) {
            return;
        }
        self.navigation.select(n, cx);

        // Handle variable inputs
        self.update_vars(cx);
        self.active_bar = 0;

        cx.notify()
    }

    /// Default for the arrow keys: the selected item moves inside itself first, otherwise the
    /// selection moves.
    fn move_selection(
        &mut self,
        direction: MoveDirection,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let handled = self
            .navigation
            .with_selected_item(cx, |item, cx| item.move_inner(&direction, cx))
            .unwrap_or(false);
        if handled {
            cx.notify();
            return;
        }

        let old = self.navigation.selected_index(cx);
        if let Some(target_idx) = self.navigation.current().layout.next_index(old, direction)
            && self.valid_selection_idx(target_idx, cx)
        {
            self.focus_nth(target_idx, cx);
            if old != target_idx {
                self.focus_search_bar(win, cx);
            }
        }
    }

    #[inline(always)]
    pub(super) fn focus_search_bar(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.active_bar == 0 {
            return;
        }

        self.active_bar = 0;
        self.text_input.update(cx, |this, cx| {
            this.focus_handle(cx).focus(win, cx);
        });
    }

    /// Tab: focus the next variable input.
    fn focus_next_var(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        let total_inputs = 1 + self.variable_input.len();

        if self.active_bar < total_inputs - 1 {
            self.active_bar += 1;

            if self.active_bar == 0 {
                self.text_input.focus_handle(cx).focus(win, cx);
            } else {
                // handle switching forward
                let var_idx = self.active_bar - 1;
                let Some(active_bar) = self.variable_input.get(var_idx) else {
                    return;
                };
                active_bar.focus_handle(cx).focus(win, cx);

                // handle switching back if variable input is empty
                match active_bar {
                    VariableInput::Text(ent) => {
                        let sub = Some(cx.subscribe(
                            ent,
                            |_this, _entity, _ev: &EmptyBackspace, cx| {
                                cx.dispatch_action(&PrevVar);
                            },
                        ));
                        ent.update(cx, |var_input, _| {
                            var_input._sub = sub;
                        });
                    }
                    VariableInput::Choice(ent) => {
                        let sub = Some(cx.subscribe(
                            ent,
                            |_this, _entity, _ev: &EmptyBackspace, cx| {
                                cx.dispatch_action(&PrevVar);
                            },
                        ));
                        ent.update(cx, |var_input, _| {
                            var_input._sub = sub;
                        });
                    }
                }
            }

            cx.notify();
        }
    }

    /// Shift-Tab: focus the previous variable input (or the search bar).
    fn focus_prev_var(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.active_bar > 0 {
            self.active_bar -= 1;

            if self.active_bar == 0 {
                self.text_input.focus_handle(cx).focus(win, cx);
            } else {
                let var_idx = self.active_bar - 1;
                self.variable_input[var_idx].focus_handle(cx).focus(win, cx);
            }

            cx.notify();
        }
    }
}

// Action listeners. Order for every action: context menu (where it applies), then the current
// view, then the launcher's default.
impl LauncherView {
    /// Offers `action` to the current view and runs `default` unless the view took it.
    fn dispatch<A: InputAction>(
        &mut self,
        action: &A,
        win: &mut Window,
        cx: &mut Context<Self>,
        default: impl FnOnce(&mut Self, &mut Window, &mut Context<Self>),
    ) {
        if !self.view_handled(action, win, cx) {
            default(self, win, cx);
        }
    }

    /// Offers `action` to the current view. `true`: the view handled it, skip the default.
    fn view_handled<A: InputAction>(
        &mut self,
        action: &A,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let handled = self.navigation.current().handle.route(action, win, cx) == Propagation::Stop;
        if handled {
            cx.notify();
        }
        handled
    }

    /// Arrow keys: an open context menu takes them, otherwise the view, otherwise the list.
    fn arrow<A: InputAction>(
        &mut self,
        action: &A,
        direction: MoveDirection,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.is_open() {
            self.context_menu.handle_move(direction);
            cx.notify();
            return;
        }
        self.dispatch(action, win, cx, |this, win, cx| {
            this.move_selection(direction, win, cx)
        });
    }

    pub(super) fn selection_down(
        &mut self,
        action: &ItemDown,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.arrow(action, MoveDirection::Down, win, cx);
    }

    pub(super) fn selection_up(
        &mut self,
        action: &ItemUp,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.arrow(action, MoveDirection::Up, win, cx);
    }

    pub(super) fn selection_left(
        &mut self,
        action: &ItemLeft,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.arrow(action, MoveDirection::Left, win, cx);
    }

    pub(super) fn selection_right(
        &mut self,
        action: &ItemRight,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.arrow(action, MoveDirection::Right, win, cx);
    }

    pub(super) fn next_var(&mut self, action: &NextVar, win: &mut Window, cx: &mut Context<Self>) {
        self.dispatch(action, win, cx, Self::focus_next_var);
    }

    pub(super) fn prev_var(&mut self, action: &PrevVar, win: &mut Window, cx: &mut Context<Self>) {
        self.dispatch(action, win, cx, Self::focus_prev_var);
    }

    /// Enter: an open context menu runs its action, otherwise the view, otherwise the selection.
    fn execute_current<A: InputAction>(
        &mut self,
        action: &A,
        exit_override: bool,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.is_open() {
            let Some(menu_action) = self.context_menu.get_current() else {
                return;
            };
            self.execute_context_action(menu_action.clone(), exit_override, win, cx);
            return;
        }
        self.dispatch(action, win, cx, |this, win, cx| {
            if let Some(req) = this.request(ExecSource::Selected, exit_override, cx) {
                this.run(req, win, cx);
            }
        });
    }

    #[inline(always)]
    pub(super) fn execute_listener(
        &mut self,
        action: &Execute,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.execute_current(action, true, win, cx);
    }

    #[inline(always)]
    pub(super) fn execute_inplace_listener(
        &mut self,
        action: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.execute_current(action, false, win, cx);
    }

    #[inline(always)]
    pub(super) fn shortcut_listener(
        &mut self,
        action: &ShortcutAction,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let index = action.index;
        self.dispatch(action, win, cx, |this, win, cx| {
            if let Some(req) = this.request(execute::ExecSource::Shortcut(index), true, cx) {
                this.run(req, win, cx);
            }
        });
    }

    /// ctrl-l: an open menu always closes; the view only gets to override opening it.
    pub(super) fn open_context(
        &mut self,
        action: &ToggleContext,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.is_open() {
            self.close_context(cx);
            return;
        }
        self.dispatch(action, win, cx, Self::open_context_menu);
    }

    pub(super) fn error_page(&mut self, _: &ErrorPage, win: &mut Window, cx: &mut Context<Self>) {
        self.run(
            ExecRequest {
                mode: ExecMode::ShowMessages,
                source: ExecSource::Bind,
                query: self.text_input.read(cx).content.clone(),
                keyword: "".into(),
                key: None,
                exit: false,
            },
            win,
            cx,
        );
    }

    pub(super) fn quit(&mut self, action: &Quit, win: &mut Window, cx: &mut Context<Self>) {
        if self.context_menu.is_open() {
            self.close_context(cx);
            return;
        }
        self.dispatch(action, win, cx, Self::close_window);
    }

    /// `Back`, from an empty search bar.
    pub(super) fn back(&mut self, action: &Back, win: &mut Window, cx: &mut Context<Self>) {
        self.dispatch(action, win, cx, Self::go_back);
    }

    /// Backspace that no input handled: a page without a search bar is focused.
    pub(super) fn backspace(
        &mut self,
        action: &SearchBackspace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dispatch(action, win, cx, Self::go_back);
    }

    pub(super) fn context_menu_binds_listener(
        &mut self,
        ev: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.context_menu.is_open() || self.context_menu.is_flashing() {
            return;
        }
        let Some(idx) = self.context_menu.action_index_for(&ev.keystroke) else {
            return;
        };
        let Some(action) = self.context_menu.get(idx).cloned() else {
            return;
        };

        cx.stop_propagation();
        self.context_menu.flash(idx); // visible right away
        cx.notify();

        cx.spawn_in(window, async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(120))
                .await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.context_menu.clear_flash();
                this.execute_context_action(action, false, window, cx);
                cx.notify();
            });
        })
        .detach();
    }

    pub(super) fn inner_bind_listener(
        &mut self,
        ev: &KeyUpEvent,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(binds) = self
            .navigation
            .with_selected_item(cx, |item, cx| item.binds(cx))
            .flatten()
        else {
            return;
        };
        let Some(pressed) = binds.iter().find(|bind| bind.matches(&ev.keystroke)) else {
            return;
        };

        if let Some(req) = self.request_with(ExecSource::Bind, true, cx, |item, cx| {
            ExecMode::from_bind(pressed, item.as_launcher_item()?, cx)
        }) {
            self.run(req, win, cx);
        }
    }
}

// Context menu
impl LauncherView {
    fn execute_context_action(
        &mut self,
        action: Arc<ContextMenuAction>,
        exit: bool,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(mut req) = self.request_with(ExecSource::ContextAction, exit, cx, |item, cx| {
            Some(item.action_exec(action, cx))
        }) else {
            // no selected item anymore: the menu may be stale
            self.refresh_context_actions(cx);
            cx.notify();
            return;
        };
        req.keyword = SharedString::default(); // `req.query` stays for learning

        self.run(req, win, cx);
    }

    pub(super) fn check_actions(&mut self, cx: &mut App) -> bool {
        let selected = self.navigation.selected_key(cx);
        self.context_menu.refresh_for(selected, || {
            self.navigation
                .with_selected_shell_item(cx, |itm, cx| (itm.quick_actions(), itm.has_actions(cx)))
                .unwrap_or((QuickActionCapabilities::empty(), false))
        })
    }

    pub(super) fn refresh_context_actions(&mut self, cx: &mut Context<Self>) {
        if self.check_actions(cx) {
            self.context_menu
                .set_actions(self.navigation.current_actions(cx).unwrap_or_default(), cx);
            self.context_menu.idx = self
                .context_menu
                .idx
                .map(|idx| idx.min(self.context_menu.num_actions().saturating_sub(1)));
        } else {
            self.context_menu.clear_actions();
            self.close_context(cx);
        }
    }

    /// Default for ctrl-l with no menu open: opens the selected item's context menu.
    fn open_context_menu(&mut self, _win: &mut Window, cx: &mut Context<Self>) {
        if !self.context_menu.can_open() {
            return;
        }

        self.context_menu
            .set_actions(self.navigation.current_actions(cx).unwrap_or_default(), cx);

        // should never be called
        if self.context_menu.num_actions() == 0 {
            let launcher_type = self
                .navigation
                .selected_item_ref(cx)
                .map(|i| i.launcher_type().to_owned());

            self.navigation.push_message(
                sherlock_msg!(
                    Error,
                    SherlockErrorType::Unreachable,
                    format!(
                        "Launcher {:?} configured incorrectly: context actions are empty but `has_actions` flag is set to true.",
                        launcher_type
                    )
                ),
                cx,
            );
            return;
        }

        self.context_menu.open(0);
        cx.notify();
    }

    pub(super) fn close_context(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.close() {
            cx.notify();
        }
    }
}

// Navigation, window and variables
impl LauncherView {
    /// Previous view, or back to home mode.
    fn go_back(&mut self, _win: &mut Window, cx: &mut Context<Self>) {
        if self.context_menu.is_open() {
            return;
        }
        if !self.navigation.is_home() {
            self.navigation.back(cx);
            let text = self.navigation.last_query(cx).unwrap_or_default();
            self.text_input.update(cx, |ipt, cx| {
                ipt.set_text(text);
                cx.notify();
            });
            self.force_filter_and_sort(cx);
        } else if *self.navigation.current_mode() != LauncherMode::Home {
            self.navigation.current_mut().mode = LauncherMode::Home;
            self.navigation.clear_last_query(cx);
            self.filter_and_sort(cx);
        }
        cx.notify();
    }

    pub(super) fn close_window(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        // Cleanup
        self.variable_input.clear();
        self.navigation.clear();

        // Close window
        win.remove_window();

        // Close backdrop
        if let Some(backdrop) = self.backdrop {
            let _ = backdrop.update(cx, |_, win, _| win.remove_window());
        }

        // Propagate state change
        reset_generation();
        cx.notify();
    }

    pub(super) fn get_variables(
        &self,
        cx: &mut App,
    ) -> SmallVec<[(SharedString, SharedString); 4]> {
        self.variable_input
            .iter()
            .filter_map(|input| match input {
                VariableInput::Choice(ent) => {
                    let c = ent.read(cx);
                    let value = c.selected.and_then(|i| c.options.get(i))?.value.clone();
                    Some((c.placeholder.text.clone(), value))
                }
                VariableInput::Text(ent) => {
                    let t = ent.read(cx);
                    let value = match &t.variable {
                        Some(var) => var.finalize(&t.content),
                        None => Cow::Borrowed(t.content.as_ref()),
                    };
                    Some((
                        t.placeholder.clone(),
                        SharedString::from(value.into_owned()),
                    ))
                }
            })
            .collect()
    }

    pub(super) fn update_vars(&mut self, cx: &mut Context<Self>) {
        let needed_vars: Option<Vec<RuntimeArgument>> = self
            .navigation
            .with_selected_item(cx, |item, cx| item.vars(cx).map(|slice| slice.to_vec()))
            .flatten();

        if let Some(vars_to_create) = needed_vars {
            self.variable_input = vars_to_create
                .into_iter()
                .map(|var| -> VariableInput {
                    let default = var.default_value();
                    match var {
                        RuntimeArgument::Choice(c) => {
                            let body = c.choice;
                            let selected = default
                                .as_ref()
                                .and_then(|d| body.choices.iter().position(|o| &o.value == d));
                            VariableInput::Choice(cx.new(|cx| {
                                Choice::builder()
                                    .scope("variable")
                                    .placeholder(body.name)
                                    .options(body.choices.to_vec())
                                    .selected(selected)
                                    .build(cx)
                            }))
                        }
                        var => VariableInput::Text(cx.new(|cx| {
                            TextInput::builder()
                                .scope("variable")
                                .placeholder(var.placeholder())
                                .content(default.unwrap_or_default())
                                .variable(var)
                                .build(cx)
                        })),
                    }
                })
                .collect();
        } else {
            self.variable_input.clear();
        }
    }

    /// Refreshes live items (weather, music, ...) of the active view.
    pub(crate) fn update_async(&mut self, cx: &mut Context<Self>) {
        self.navigation.current().handle.refresh(cx);
    }
}

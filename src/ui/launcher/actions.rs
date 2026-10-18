use std::{borrow::Cow, sync::Arc, time::Duration};

use gpui::{
    App, AppContext, ClipboardItem, Context, Focusable, KeyDownEvent, KeyUpEvent, SharedString,
    Window, actions,
};
use simd_json::prelude::{ArrayTrait, Indexed};
use smallvec::SmallVec;

use crate::{
    app::reset_generation,
    launcher::{
        ExecEffect, LauncherValues,
        utils::exec_mode::ExecMode,
        variant_type::{LauncherType, QuickActionCapabilities},
    },
    loader::utils::CounterReader,
    sherlock_msg,
    ui::{
        choice::Choice,
        launcher::{
            LauncherView, VariableInput,
            context_menu::{
                ContextMenuAction,
                wrapper::{ContextMenu, Move},
            },
            navigator::views::MoveDirection,
        },
        runtime_arguments::RuntimeArgument,
        search_bar::{EmptyBackspace, TextInput, actions::ShortcutAction},
        traits::RenderableChildDelegate,
    },
    utils::{
        command_launch::spawn_detached,
        config::{ConfigFileChange, ConfigGuard, SherlockAlias, SherlockIgnore, reload},
        errors::{SherlockMessage, types::SherlockErrorType},
        websearch::websearch,
    },
};

actions!(
    example_input,
    [
        Quit,
        SelectionDown,
        SelectionUp,
        SelectionLeft,
        SelectionRight,
        NextVar,
        PrevVar,
        Execute,
        ExecuteInplace,
        OpenContext,
        Backspace,
    ]
);

impl LauncherView {
    pub fn focus_first(&mut self, cx: &mut Context<Self>) {
        let Some((indices, data_entity)) = self.navigation.with_model(cx, |mdl| {
            let filtered_indices = mdl.filtered_indices();
            if filtered_indices.is_empty() {
                None
            } else {
                Some((filtered_indices, mdl.data()))
            }
        }) else {
            return;
        };

        if data_entity.read(cx).is_empty() {
            return;
        }

        // Find the first focusable item
        if let Some(n) = {
            let data_guard = data_entity.read(cx);
            indices.iter().position(|&idx| {
                data_guard
                    .get(&idx.0)
                    .is_some_and(|launcher| launcher.config.spawn_focus)
            })
        } {
            self.focus_nth(n, cx);
        }
    }

    #[inline(always)]
    fn valid_selection_idx(&self, n: usize, cx: &mut Context<Self>) -> bool {
        let list_len = self.navigation.with_model(cx, |mdl| mdl.len());
        n < list_len && list_len != 0
    }

    pub fn focus_nth(&mut self, n: usize, cx: &mut Context<Self>) {
        // early return on invalid index
        // or
        // early return if not a list view
        if !self.valid_selection_idx(n, cx) {
            return;
        }
        self.navigation.current_mut().layout.focus_nth(n);

        // Handle variable inputs
        self.update_vars(cx);
        self.active_bar = 0;

        cx.notify()
    }

    fn move_selection(
        &mut self,
        direction: MoveDirection,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.context_menu.is_open() {
            self.context_menu.handle_move(direction);
            cx.notify();
            return;
        }

        let handled = self
            .navigation
            .with_selected_item(cx, |item, cx| item.move_inner(&direction, cx))
            .unwrap_or(false);
        if handled {
            cx.notify();
            return;
        }

        if let Some((old, target_idx)) = self.navigation.current().layout.next_index(direction)
            && self.valid_selection_idx(target_idx, cx)
        {
            self.focus_nth(target_idx, cx);
            if old != target_idx {
                self.focus_search_bar(win, cx);
            }
        }
    }

    pub(super) fn selection_down(
        &mut self,
        _: &SelectionDown,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Down, win, cx);
    }

    pub(super) fn selection_up(
        &mut self,
        _: &SelectionUp,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Up, win, cx);
    }

    pub(super) fn selection_left(
        &mut self,
        _: &SelectionLeft,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Left, win, cx);
    }

    pub(super) fn selection_right(
        &mut self,
        _: &SelectionRight,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.move_selection(MoveDirection::Right, win, cx);
    }

    pub(super) fn next_var(&mut self, _: &NextVar, win: &mut Window, cx: &mut Context<Self>) {
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

    pub(super) fn prev_var(&mut self, _: &PrevVar, win: &mut Window, cx: &mut Context<Self>) {
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

    fn execute_context_action(
        &mut self,
        action: Arc<ContextMenuAction>,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(what) = self
            .navigation
            .with_selected_item(cx, |item, cx| item.build_action_exec(action, cx))
        else {
            return;
        };

        // deliberately no closing here
        if let Err(e) = self.execute_helper(what, "", false, cx) {
            self.navigation.push_message(e, cx)
        }

        self.refresh_context_actions(cx);
        self.focus_search_bar(win, cx);
        cx.notify();
    }

    fn execute_current(&mut self, exit_override: bool, win: &mut Window, cx: &mut Context<Self>) {
        if self.context_menu.is_open() {
            let Some(action) = self.context_menu.get_current() else {
                return;
            };
            let Some(what) = self
                .navigation
                .with_selected_item(cx, |item, cx| item.build_action_exec(action.clone(), cx))
            else {
                self.refresh_context_actions(cx);
                cx.notify();
                return;
            };

            match self.execute_helper(what, "", exit_override, cx) {
                Ok(exit) if exit => self.close_window(win, cx),
                Err(e) => self.navigation.push_message(e, cx),
                _ => {}
            }

            // update context menu actions in case of no-exit action and changed actions
            self.refresh_context_actions(cx);
            self.focus_search_bar(win, cx);
            cx.notify();
        } else {
            let keyword = self.text_input.read(cx).content.clone();

            let Some(what) = self
                .navigation
                .with_selected_item(cx, ExecMode::from_child)
                .flatten()
            else {
                return;
            };

            match self.execute_helper(what, keyword.as_ref(), exit_override, cx) {
                Ok(exit) if exit => self.close_window(win, cx),
                Err(e) => self.navigation.push_message(e, cx),
                _ => {}
            }

            self.refresh_context_actions(cx);
            self.focus_search_bar(win, cx);
            self.force_filter_and_sort(cx);
        }
    }

    pub(super) fn check_actions(&mut self, cx: &mut App) -> bool {
        let selected_data_idx = self.navigation.selected_data_idx(cx);
        self.context_menu.refresh_for(selected_data_idx, || {
            self.navigation
                .with_selected_item(cx, |itm, cx| {
                    (ContextMenu::availability_mask(itm), itm.has_actions(cx))
                })
                .unwrap_or((QuickActionCapabilities::empty(), false))
        })
    }

    fn refresh_context_actions(&mut self, cx: &mut Context<Self>) {
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

    #[inline(always)]
    fn focus_search_bar(&mut self, win: &mut Window, cx: &mut Context<Self>) {
        if self.active_bar == 0 {
            return;
        }

        self.active_bar = 0;
        self.text_input.update(cx, |this, cx| {
            this.focus_handle(cx).focus(win, cx);
        });
    }

    pub(super) fn execute_helper(
        &mut self,
        what: ExecMode,
        keyword: &str,
        exit_override: bool,
        cx: &mut Context<Self>,
    ) -> Result<bool, SherlockMessage> {
        let variables = &self.get_variables(cx);
        match what {
            ExecMode::Inner { func, exit } => {
                if let Some(res) = self.navigation.with_selected_item(cx, move |this, cx| {
                    this.execute_function(func, variables, cx)
                }) {
                    let effect = res?;
                    match effect {
                        ExecEffect::InsertMessages(msgs) => {
                            for msg in msgs {
                                self.navigation.push_message(msg, cx);
                            }
                        }
                        ExecEffect::ClearMessages => {
                            self.navigation.clear_messages(cx);
                        }
                        ExecEffect::UpdateAsync => {
                            self.update_async(cx);
                        }
                        ExecEffect::None => {}
                    }
                    return Ok(exit && exit_override);
                };
            }
            ExecMode::App { exec, terminal } => {
                let cmd = if terminal {
                    format!(r#"{{terminal}} {exec}"#)
                } else {
                    exec.to_string()
                };

                spawn_detached(&cmd, keyword, variables)?;
                self.navigation
                    .with_selected_item(cx, |item, _| item.increment_count());
                if let Err(e) = increment(&exec) {
                    self.navigation.push_message(e, cx);
                }
            }
            ExecMode::Category { category } => {
                self.navigation.current_mut().mode = category;
                self.text_input.update(cx, |this, _cx| {
                    this.reset();
                });
                self.filter_and_sort(cx);
                return Ok(false);
            }
            ExecMode::Command { exec } => {
                spawn_detached(&exec, keyword, variables)?;
                self.navigation
                    .with_selected_item(cx, |item, _| item.increment_count());
                if let Err(e) = increment(&exec) {
                    self.navigation.push_message(e, cx);
                }
            }
            ExecMode::Copy { content } => {
                cx.write_to_clipboard(ClipboardItem::new_string(content.to_string()));
            }
            ExecMode::Print { content } => {
                println!("{content}");
                self.write_response(content, cx);
            }
            ExecMode::Hide { exec } => {
                let config = ConfigGuard::read()?;
                let entry = SherlockAlias::find_app_by_name(&config.files.alias, exec)?;
                SherlockIgnore::add_entry(&config.files.ignore, &entry)?;

                if !exit_override {
                    self.navigation.with_model_mut(cx, |model, cx| {
                        let mut initial_messages = Vec::new();
                        reload(
                            cx,
                            &model.data,
                            &mut initial_messages,
                            ConfigFileChange::Ignore,
                        );
                    });
                    self.filter_and_sort_sync(cx);
                    self.context_menu.focus_first();
                    self.update_async(cx);
                }
            }
            ExecMode::Info => {
                if let Some(idx) = self.navigation.selected_data_idx(cx) {
                    self.sidebar_controller.set_for_item(idx);
                } else {
                    self.navigation.push_message(
                        sherlock_msg!(
                            Warning,
                            SherlockErrorType::InvalidAction,
                            "The current item doesn't support the info action."
                        ),
                        cx,
                    );
                }
                // dont close context_menu on `exec_inplace`, only in normal exec
                if exit_override {
                    self.context_menu.close();
                }
                return Ok(false);
            }
            ExecMode::CreateView { launcher, mode } => {
                let Some(view) = mode.build(launcher, cx) else {
                    return Ok(false);
                };
                self.text_input.update(cx, |this, _| this.reset());
                self.navigation.push(view);
                self.context_menu.invalidate();
                self.filter_and_sort(cx);
                return Ok(false);
            }
            ExecMode::DynamicContextMenuFunc { action } => {
                let ContextMenuAction::Fn(opts) = action.as_ref() else {
                    return Ok(false);
                };

                if let Some(func) = opts.func.as_ref() {
                    func(cx)
                }

                cx.notify();
                return Ok(opts.exit && exit_override);
            }
            ExecMode::ShowMessages => {
                self.navigation.show_messages();
                self.text_input.update(cx, |this, _| this.reset());
                self.filter_and_sort(cx);
                return Ok(false);
            }
            ExecMode::Web {
                engine,
                browser,
                exec,
            } => {
                let engine = engine.as_deref().unwrap_or("plain");
                let query = if let Some(query) = exec.as_deref() {
                    query
                } else {
                    keyword
                };
                websearch(engine, query, browser.as_deref(), variables)?;
            }
            ExecMode::None => {
                return Ok(false);
            }
        };

        Ok(exit_override)
    }

    pub(super) fn shortcut_listener(
        &mut self,
        action: &ShortcutAction,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(what) = self
            .navigation
            .with_nth_shortcut_item(action.index, cx, ExecMode::from_child)
            .flatten()
        {
            let keyword = self.text_input.read(cx).content.clone();
            match self.execute_helper(what, keyword.as_ref(), true, cx) {
                Ok(exit) if exit => {
                    self.close_window(win, cx);
                }
                Err(e) => {
                    self.navigation.push_message(e, cx);
                }
                _ => {}
            }
        }
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
                this.execute_context_action(action, window, cx);
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
        let selected_binds = self
            .navigation
            .with_selected_item(cx, |item, cx| item.binds(cx))
            .and_then(|b| b);

        if let Some(binds) = &selected_binds
            && let Some(pressed) = binds.iter().find(|bind| bind.matches(&ev.keystroke))
        {
            let what = self
                .navigation
                .with_selected_item(cx, move |child, cx| ExecMode::from_bind(pressed, child, cx))
                .and_then(|f| f);

            let query = self.text_input.read(cx).content.clone();

            if let Some(what) = what {
                match self.execute_helper(what, query.as_str(), true, cx) {
                    Ok(exit) if exit => self.close_window(win, cx),
                    Err(e) => self.navigation.push_message(e, cx),
                    _ => {}
                }
            }
            cx.notify();
        }
    }

    fn get_variables(&self, cx: &mut App) -> SmallVec<[(SharedString, SharedString); 4]> {
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

    #[inline(always)]
    pub(super) fn execute_inplace_listener(
        &mut self,
        _: &ExecuteInplace,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.execute_current(false, win, cx);
    }

    #[inline(always)]
    pub(super) fn execute_listener(
        &mut self,
        _: &Execute,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.execute_current(true, win, cx);
    }

    pub(super) fn open_context(
        &mut self,
        _: &OpenContext,
        _win: &mut Window,
        cx: &mut Context<Self>,
    ) {
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

        // toggle logic
        if !self.context_menu.close() {
            self.context_menu.open(0);
        }

        cx.notify();
    }

    pub(super) fn close_context(&mut self, cx: &mut Context<Self>) {
        if self.context_menu.close() {
            cx.notify();
        }
    }

    pub(super) fn quit(&mut self, _: &Quit, win: &mut Window, cx: &mut Context<Self>) {
        if self.context_menu.is_open() {
            self.close_context(cx);
        } else {
            self.close_window(win, cx);
        }
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

    pub(super) fn update_vars(&mut self, cx: &mut Context<Self>) {
        let Some(idx) = self.navigation.selected_data_idx(cx) else {
            self.variable_input.clear();
            return;
        };

        let needed_vars: Option<Vec<RuntimeArgument>> = {
            self.navigation.with_model_mut(cx, |mdl, cx| {
                let data_guard = mdl.data().read(cx).clone();
                data_guard
                    .get(&idx.0)
                    .and_then(|l| l.children.get(idx.1))
                    .and_then(|data| data.vars(cx).map(|slice| slice.to_vec()))
            })
        };

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

    pub(crate) fn update_async(&mut self, cx: &mut Context<Self>) {
        let data = self.navigation.with_model(cx, |mdl| mdl.data());
        let data_snapshot_rc = data.read(cx).clone();
        data_snapshot_rc
            .values()
            .filter_map(|launcher| {
                if matches!(
                    launcher.config.launcher_type,
                    LauncherType::Weather(_)
                        | LauncherType::MusicPlayer(_)
                        | LauncherType::Clipboard(_)
                        | LauncherType::Event(_)
                        | LauncherType::Plugin(_)
                ) {
                    Some(launcher.children.iter())
                } else {
                    None
                }
            })
            .flatten()
            .for_each(|item| item.update_async(cx));
    }

    pub(crate) fn update_sync(&mut self, query: SharedString, cx: &mut Context<Self>) {
        let data_entity = self.navigation.with_model(cx, |mdl| mdl.data().clone());

        let data_snapshot_arc = data_entity.read(cx).clone();
        let filtered_indices_arc = self.navigation.with_model(cx, |mdl| mdl.filtered_indices());
        filtered_indices_arc
            .iter()
            .filter_map(|i| {
                data_snapshot_arc
                    .get(&i.0)
                    .and_then(|l| l.children.get(i.1))
            })
            .for_each(|render_child| render_child.update_sync(query.clone(), cx));
    }
}

#[inline(always)]
fn increment(key: &str) -> Result<(), SherlockMessage> {
    let reader = CounterReader::new()?;
    reader.increment(key)
}

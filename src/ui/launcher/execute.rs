use gpui::{App, AppContext, ClipboardItem, Context, SharedString, Window};

use crate::{
    QUERY_MEMORY,
    app::LauncherEntityGlobal,
    launcher::{ExecEffect, utils::exec_mode::ExecMode},
    loader::utils::CounterReader,
    sherlock_msg,
    ui::{
        launcher::{LauncherView, context_menu::ContextMenuAction, navigator::views::NavView},
        model::item::ShellItem,
        traits::RenderableChildDelegate,
        views::credits::CreditView,
    },
    utils::{
        command_launch::spawn_detached,
        config::{ConfigGuard, SherlockAlias, SherlockIgnore, reload},
        errors::{SherlockMessage, types::SherlockErrorType},
        item_key::ItemKey,
        websearch,
    },
};

/// Where the execution came from: decides post-processing and bookkeeping.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecSource {
    /// <execute-bind> on the selected item.
    Selected,
    /// Context menu (click, key, or bind).
    ContextAction,
    /// <ctrl+1-9>.
    Shortcut(usize),
    /// Item definied keybind, like an inner action.
    Bind,
}

pub struct ExecRequest {
    pub mode: ExecMode,
    pub source: ExecSource,
    /// Identity of the item the request was started from
    pub key: Option<ItemKey>,
    /// Passed to the executed command (`{keyword}`); may be cleared, e.g. for context actions.
    pub keyword: SharedString,
    /// Query at the time of execution; picks are learned under it.
    pub query: SharedString,
    pub exit: bool,
}

impl Default for ExecRequest {
    fn default() -> Self {
        Self {
            mode: ExecMode::None,
            source: ExecSource::Bind,
            key: None,
            keyword: SharedString::default(),
            query: SharedString::default(),
            exit: false,
        }
    }
}

pub enum Outcome {
    Close,
    Stay,
    Navigated,
    NoOp,
}

impl Outcome {
    fn done(exit: bool) -> Self {
        if exit { Self::Close } else { Self::Stay }
    }
}

impl LauncherView {
    pub(super) fn request(
        &self,
        source: ExecSource,
        exit: bool,
        cx: &mut Context<Self>,
    ) -> Option<ExecRequest> {
        self.request_with(source, exit, cx, |item, cx| item.exec(cx))
    }

    pub(super) fn request_with(
        &self,
        source: ExecSource,
        exit: bool,
        cx: &mut App,
        mode_fn: impl FnOnce(&dyn ShellItem, &mut App) -> Option<ExecMode>,
    ) -> Option<ExecRequest> {
        let build = |item: &dyn ShellItem, cx: &mut App| {
            let mode = mode_fn(item, cx)?;
            Some((mode, item.pick_key()))
        };

        let (mode, key) = match source {
            ExecSource::Shortcut(n) => self.navigation.with_nth_shortcut_shell_item(n, cx, build),
            _ => self.navigation.with_selected_shell_item(cx, build),
        }??;

        let query = self.text_input.read(cx).content.clone();
        Some(ExecRequest {
            mode,
            source,
            key,
            keyword: query.clone(),
            query,
            exit,
        })
    }

    pub(super) fn run(&mut self, req: ExecRequest, win: &mut Window, cx: &mut Context<Self>) {
        let picked = req.mode.is_pick();
        let (key, source) = (req.key.clone(), req.source);
        let query = req.query.clone();

        let (outcome, ok) = match self.perform(req, win, cx) {
            Ok(o) => (o, true),
            Err(e) => {
                self.navigation.push_message(e, cx);
                (Outcome::Stay, false)
            }
        };

        // bookkeeping
        if ok
            && picked
            && !matches!(outcome, Outcome::Navigated)
            && let Some(key) = &key
        {
            if let Err(e) = record_pick(&query, key) {
                self.navigation.push_message(e, cx);
            }

            let target = match source {
                ExecSource::Shortcut(n) => self.navigation.shortcut_item_ref(n, cx),
                _ => self.navigation.selected_shell_item_ref(cx),
            };
            if let Some(item) = target {
                item.refresh_popularity();
            }
        }

        // post-processing: controlled by outcome and source
        match outcome {
            Outcome::Close => return self.close_window(win, cx),
            Outcome::Navigated => {
                self.text_input.update(cx, |t, _| t.reset());
                self.context_menu.invalidate();
                self.filter_and_sort(cx);
            }
            Outcome::Stay if matches!(source, ExecSource::Selected) => {
                self.force_filter_and_sort(cx)
            }
            Outcome::NoOp => return,
            Outcome::Stay => {}
        }
        self.refresh_context_actions(cx);
        self.focus_search_bar(win, cx);
        cx.notify();
    }

    fn perform(
        &mut self,
        req: ExecRequest,
        win: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<Outcome, SherlockMessage> {
        let done = Outcome::done(req.exit);
        let vars = &self.get_variables(cx);

        Ok(match req.mode {
            ExecMode::App { exec, terminal } => {
                let cmd = if terminal {
                    format!("{{terminal}} {exec}")
                } else {
                    exec
                };
                spawn_detached(&cmd, &req.keyword, vars)?;
                done
            }
            ExecMode::Command { exec } => {
                spawn_detached(&exec, &req.keyword, vars)?;
                done
            }
            ExecMode::Web {
                engine,
                browser,
                exec,
            } => {
                websearch::websearch(
                    engine.as_deref().unwrap_or("plain"),
                    exec.as_deref().unwrap_or(&req.keyword),
                    browser.as_deref(),
                    vars,
                )?;
                done
            }
            ExecMode::Copy { content } => {
                cx.write_to_clipboard(ClipboardItem::new_string(content));
                done
            }
            ExecMode::Print { content } => {
                println!("{content}");
                self.write_response(content, cx);
                done
            }
            ExecMode::Category { category } => {
                self.navigation.current_mut().mode = category;
                Outcome::Navigated
            }
            ExecMode::CreateView { mode, launcher } => match mode.build(launcher, cx) {
                Some(view) => {
                    self.navigation.push(view);
                    Outcome::Navigated
                }
                None => Outcome::Stay,
            },
            ExecMode::ShowMessages => {
                self.navigation.show_messages();
                Outcome::Navigated
            }
            ExecMode::ShowCredits => {
                self.navigation
                    .push(NavView::new(cx.new(|_| CreditView), cx));
                Outcome::Navigated
            }
            ExecMode::OpenContextMenu => {
                self.open_context(&super::ToggleContext, win, cx);
                Outcome::NoOp
            }
            ExecMode::Hide { exec } => {
                let config = ConfigGuard::read()?;
                let entry = SherlockAlias::find_app_by_name(&config.files.alias, exec)?;
                SherlockIgnore::add_entry(&config.files.ignore, &entry)?;

                if !req.exit {
                    // the hidden app lives in the global launcher data, whatever view is open
                    if let Some(data) = cx.global::<LauncherEntityGlobal>().0.upgrade() {
                        reload(
                            cx,
                            &data,
                            &mut Vec::new(),
                            crate::utils::config::ConfigFileChange::Ignore,
                        );
                    }
                    self.filter_and_sort_sync(cx);
                    self.context_menu.focus_first();
                    self.update_async(cx);
                }

                done
            }
            ExecMode::Info => {
                if let Some(key) = self.navigation.selected_key(cx) {
                    self.sidebar_controller.set_for_item(key);
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
                if req.exit {
                    self.context_menu.close();
                }
                Outcome::Stay
            }
            ExecMode::Inner { func, exit } => {
                // we can use selected item here since inner functions cant be run in combination
                // with ExecSource::Shortcut
                let Some(res) = self
                    .navigation
                    .with_selected_item(cx, move |this, cx| this.execute_function(func, vars, cx))
                else {
                    return Ok(Outcome::Stay);
                };
                self.apply_effect(res?, cx);
                Outcome::done(exit && req.exit)
            }
            ExecMode::DynamicContextMenuFunc { action } => {
                let ContextMenuAction::Fn(opts) = action.as_ref() else {
                    return Ok(Outcome::Stay);
                };

                if let Some(func) = opts.func.as_ref() {
                    func(cx);
                }

                if opts.exit && req.exit {
                    Outcome::Close
                } else {
                    Outcome::Stay
                }
            }
            ExecMode::None => Outcome::NoOp,
        })
    }

    fn apply_effect(&mut self, effect: ExecEffect, cx: &mut Context<Self>) {
        match effect {
            ExecEffect::InsertMessages(msgs) => {
                for msg in msgs {
                    self.navigation.push_message(msg, cx);
                }
            }
            ExecEffect::ClearMessages => self.navigation.clear_messages(cx),
            ExecEffect::UpdateAsync => self.update_async(cx),
            ExecEffect::None => {}
        }
    }
}

fn record_pick(query: &str, key: &ItemKey) -> Result<(), SherlockMessage> {
    CounterReader::new()?.increment(key.as_str())?;
    if let Some(lock) = QUERY_MEMORY.get()
        && let Ok(mut mem) = lock.write()
    {
        mem.record(query, key)?;
    }
    Ok(())
}

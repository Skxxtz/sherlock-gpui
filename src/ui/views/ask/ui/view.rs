mod asking;
mod chats;
mod feedback;
mod loading;
mod running;
mod shell;
mod suggestions;
mod thread;

use std::{rc::Rc, sync::Arc, time::Duration};

use gpui::{
    AnyElement,
    App,
    AsyncApp,
    ClipboardItem,
    Context,
    IntoElement,
    ListAlignment,
    ListState,
    ParentElement,
    ScrollHandle,
    SharedString,
    Styled,
    Task,
    WeakEntity,
    Window,
    div,
    point,
    prelude::*,
    px,
};

use super::{
    poll::poll_every,
    render::{self, EmptyState, Ui},
    selection::TextSelection,
};
use crate::{
    app::theme::ThemeData,
    features::ask::AskLauncher,
    launcher::{LauncherProvider, utils::binds::Bind},
    ui::{
        launcher::{ExecRequest, LauncherView},
        model::data::NoData,
        views::{
            Background,
            BusyStatus,
            Chrome,
            ShellCtx,
            SherlockView,
            ask::{
                content::prompt::{SystemContext, fill_context, system_prompt},
                exec::{
                    runner::{CommandRun, RunDrain, RunHandle, RunState},
                    targets::Targets,
                    terminal::run_in_terminal,
                },
                model::{
                    conversation::Conversation,
                    exchange::{Exchange, Phase},
                    focus::{self, Focus},
                    items::Item,
                },
                provider::{
                    AnswerStream,
                    Drain,
                    Message,
                    Provider,
                    key::{KeySource, read_key},
                },
                store::history::{ChatSummary, Entry, History, Stats},
            },
        },
    },
};

/// Drain interval while streaming or running.
const FRAME: Duration = Duration::from_millis(33);
/// History entries kept on disk.
const MAX_HISTORY: usize = 500;
/// Finished exchanges kept in memory and sent as context, unless set (`with_max_turns`).
/// Older ones live only in the history.
pub const DEFAULT_MAX_TURNS: usize = 6;
/// Recent chats read from the history.
const MAX_LOADED_CHATS: usize = 20;
/// How long a toast stays in the status line.
const TOAST: Duration = Duration::from_millis(1400);

/// The API key, when a command reads it.
enum KeyState {
    Ready,
    /// The command is running; questions wait for it.
    Pending,
    Failed(SharedString),
}

pub struct AskView {
    pub(super) provider: Provider,
    key: KeyState,
    /// A question asked while the key was still being read.
    queued: Option<Vec<Message>>,
    system_prompt: SharedString,
    history: Option<History>,
    /// Questions and tokens in the current quota day; `None` until loaded.
    pub(super) today: Option<Stats>,
    /// Latest distinct questions from the history, newest first.
    pub(super) recent: Vec<ChatSummary>,
    /// How many of `recent` the empty view shows.
    num_recent_chats: usize,
    pub(super) conv: Conversation,
    stream: Option<AnswerStream>,
    /// Drains `stream`. Dropping it stops polling.
    pump: Option<Task<()>>,
    pub(super) focus: Focus,
    /// Mouse text selection.
    pub(super) text_sel: Option<TextSelection>,
    pub(super) toast: Option<SharedString>,
    toast_task: Option<Task<()>>,
    /// Installed apps and web search settings.
    targets: Rc<Targets>,
    /// The chat's exchanges.
    list: ListState,
    /// The shell's theme, cached on render for the list's item callbacks.
    theme: Option<Arc<ThemeData>>,
    /// Raw search bar text, cached on render.
    pub(super) input: SharedString,
    shell: Option<WeakEntity<LauncherView>>,
    /// The command running inside Sherlock, and its drain task.
    run: Option<RunHandle>,
    run_at: Option<usize>,
    run_pump: Option<Task<()>>,
    output_scroll: ScrollHandle,
    /// Whether the user wants to retain this view.
    retain: bool,
    focused: usize,
    pub(super) binds: Option<Arc<[Bind]>>,
}

impl AskView {
    /// `provider` as configured in the Ask launcher; a missing model or key is filled in.
    pub fn new(launcher: &AskLauncher, cx: &mut Context<Self>) -> Self {
        let (provider, key) = launcher.provider.resolved();
        let key = match key {
            KeySource::Ready => KeyState::Ready,
            KeySource::Command(cmd) => {
                Self::load_key(cmd, cx);
                KeyState::Pending
            }
        };
        let history = History::default_path().map(|p| History::new(p, MAX_HISTORY));
        if let Some(h) = history.clone() {
            let since = provider.reset_zone().last_reset(chrono::Utc::now()).max(0) as u64;
            Self::load_history(h, since, cx);
        }
        // Apps by name right away, their kinds once the desktop files are read.
        let (targets, desktop_files) = Targets::load(cx);
        Self::load_app_tags(desktop_files, cx);

        let list = ListState::new(0, ListAlignment::Top, px(600.));
        Self::track_focus(&list, cx);

        Self {
            provider,
            key,
            queued: None,
            system_prompt: match &launcher.system_prompt {
                Some(template) => fill_context(template, &SystemContext::detect()),
                None => system_prompt(&SystemContext::detect()),
            }
            .into(),
            history,
            today: None,
            recent: Vec::new(),
            num_recent_chats: launcher.num_recent,
            conv: Conversation::new(launcher.max_turns.unwrap_or(DEFAULT_MAX_TURNS)),
            stream: None,
            pump: None,
            focus: Focus::None,
            text_sel: None,
            toast: None,
            toast_task: None,
            targets: Rc::new(targets),
            list,
            theme: None,
            input: SharedString::default(),
            shell: None,
            run: None,
            run_at: None,
            run_pump: None,
            output_scroll: ScrollHandle::new(),
            retain: launcher.retain,
            focused: 0,
            binds: launcher.binds(),
        }
    }

    /// Replaces the built-in system prompt; `{context}` becomes the OS, shell, desktop and date.
    pub fn with_system_prompt(mut self, template: &str) -> Self {
        self.system_prompt = fill_context(template, &SystemContext::detect()).into();
        self
    }

    /// How many recent chats the empty view offers.
    pub fn with_recent_chats(mut self, n: usize) -> Self {
        self.num_recent_chats = n.min(MAX_LOADED_CHATS);
        self
    }

    /// How many earlier exchanges go along with each question.
    pub fn with_max_turns(mut self, max_turns: usize) -> Self {
        self.conv = Conversation::new(max_turns);
        self
    }

    pub(super) fn is_running(&self) -> bool {
        self.run.is_some()
    }

    /// The shown answer's runnable items, once it is complete (cached on the exchange).
    pub(super) fn items(&self) -> Rc<[Item]> {
        match self.conv.shown().filter(|e| !e.phase.is_live()) {
            Some(ex) => ex.items(&self.targets.apps, self.targets.version),
            None => Rc::from([]),
        }
    }

    /// The body is drawn by the shell, so it has to repaint too.
    pub(super) fn repaint(&self, cx: &mut Context<Self>) {
        cx.notify();
        if let Some(shell) = self.shell.clone() {
            cx.defer(move |cx| {
                let _ = shell.update(cx, |_, cx| cx.notify());
            });
        }
    }

    pub(super) fn clear_input(&self, cx: &mut Context<Self>) {
        if let Some(shell) = self.shell.clone() {
            cx.defer(move |cx| {
                let _ = shell.update(cx, |shell, cx| {
                    shell.text_input.update(cx, |input, cx| {
                        input.reset();
                        cx.notify();
                    });
                    cx.notify();
                });
            });
        }
    }
}

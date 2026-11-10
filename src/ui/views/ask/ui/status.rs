use gpui::{Action, App, SharedString, Window};

use crate::ui::{
    actions::{Back, Copy, Execute, ExecuteInplace, NextVar},
    components::status_bar::{KeySpec, StatusBar, StatusItem},
    views::{
        BusyStatus,
        ask::{
            model::exchange::{Exchange, Phase, RemoteLimits, Usage},
            provider::Provider,
            store::quota::local_clock,
            ui::view::AskView,
        },
    },
};

/// Dispatches `A` like its key binding would.
fn dispatch<A: Action + Default>(win: &mut Window, cx: &mut App) {
    win.dispatch_action(Box::new(A::default()), cx);
}

const STOP_COMMAND: StatusItem = StatusItem::keys(KeySpec::Bind("stop"), "stop command");
const STOP: StatusItem = StatusItem::keys(KeySpec::Bind("stop"), "stop");
const NEW: StatusItem = StatusItem::keys(KeySpec::Bind("new_chat"), "new");
const COPY_SELECTION: StatusItem = hint::<Copy>("copy");
const COPY_OUTPUT: StatusItem = hint::<Execute>("copy");
const CLEAR_OUTPUT: StatusItem = hint::<Back>("clear");
const FIX: StatusItem = hint::<Execute>("fix it");
const OPEN_TERMINAL: StatusItem = hint::<Execute>("open in terminal");
const RUN: StatusItem = hint::<Execute>("run");
const RUN_IN_TERMINAL: StatusItem = hint::<ExecuteInplace>("in terminal");
const COPY_ALL: StatusItem = hint::<ExecuteInplace>("copy all");
const NEXT: StatusItem = hint::<NextVar>("next");
const OPEN_CHAT: StatusItem = hint::<Execute>("open");
const DELETE_CHAT: StatusItem = hint::<Back>("delete");
const ASK: StatusItem = hint::<Execute>("ask");
const EDIT: StatusItem = hint::<NextVar>("edit");
const SELECT: StatusItem = hint::<NextVar>("select");

/// A hint for action `A`: shows its bound keys, clicking dispatches it.
const fn hint<A: Action + Default>(label: &'static str) -> StatusItem {
    StatusItem::keys(KeySpec::action::<A>(), label).on_click(dispatch::<A>)
}

// Hint sets (statics, so the bar borrows them instead of allocating).
static WHILE_RUNNING: [StatusItem; 1] = [STOP_COMMAND];
static WHILE_ANSWERING: [StatusItem; 1] = [STOP];
static ON_SELECTION: [StatusItem; 1] = [COPY_SELECTION];
static ON_FAILED_OUTPUT: [StatusItem; 3] = [FIX, CLEAR_OUTPUT, NEXT];
static ON_TERMINAL_OUTPUT: [StatusItem; 3] = [OPEN_TERMINAL, CLEAR_OUTPUT, NEXT];
static ON_OUTPUT: [StatusItem; 3] = [COPY_OUTPUT, CLEAR_OUTPUT, NEXT];
static ON_ITEM: [StatusItem; 3] = [RUN, RUN_IN_TERMINAL, NEXT];
static ON_ONLY_ITEM: [StatusItem; 3] = [COPY_SELECTION, RUN, RUN_IN_TERMINAL];
static ON_ANSWER: [StatusItem; 2] = [COPY_ALL, NEW];
static ON_RUNNABLE_ANSWER: [StatusItem; 3] = [SELECT, COPY_ALL, NEW];
static ON_SUGGESTION: [StatusItem; 2] = [ASK, EDIT];
static ON_CHAT: [StatusItem; 2] = [OPEN_CHAT, DELETE_CHAT];

impl AskView {
    /// Requests/tokens left, as text, and whether it's running low (under 10%).
    pub(super) fn quota_left(&self, limits: Option<RemoteLimits>) -> Option<(String, bool)> {
        // Exact numbers from the provider win over the local estimate.
        if let Some(n) = limits.and_then(|l| l.requests) {
            return Some((format!("{n} requests left"), false));
        }
        let today = self.today?;
        if let Some(max) = self.provider.daily_requests {
            let left = max.saturating_sub(today.questions);
            return Some((format!("{left} of {max} left"), left * 10 <= max));
        }
        if let Some(max) = self.provider.daily_tokens {
            let left = max.saturating_sub(today.tokens);
            let text = format!(
                "{} of {} tokens left",
                compact_count(left),
                compact_count(max)
            );
            return Some((text, left * 10 <= max));
        }
        None
    }

    /// The shell's status bar.
    pub(super) fn bar(&self) -> Option<StatusBar> {
        let Some(ex) = self.conv.shown() else {
            // Empty view.
            return self
                .focus
                .recent()
                .map(|_| StatusBar::shell().with_right(&ON_CHAT[..]));
        };
        let (label, status): (SharedString, _) = match &ex.phase {
            Phase::Waiting => ("Thinking".into(), BusyStatus::Busy),
            Phase::Streaming => ("Answering".into(), BusyStatus::Busy),
            Phase::Done { elapsed } => (
                format!("{:.1} s", elapsed.as_secs_f32()).into(),
                BusyStatus::Ok,
            ),
            Phase::Stopped => ("Stopped".into(), BusyStatus::Ok),
            Phase::Failed(_) => ("Failed".into(), BusyStatus::Err),
            Phase::Loaded => ("Saved chat".into(), BusyStatus::Ok),
        };
        // Left
        let mut bar = StatusBar::empty().left(StatusItem::status(label, status));
        let meta: Vec<String> = usage_label(&self.provider, ex.usage)
            .into_iter()
            .chain(self.conv.position().map(|p| p.to_string()))
            .chain(self.quota_left(ex.limits).map(|(q, _)| q))
            .collect();
        if !meta.is_empty() {
            bar = bar.left(StatusItem::text(meta.join("  ·  ")));
        }
        // Right
        Some(match &self.toast {
            Some(toast) => bar.right(StatusItem::status(toast.clone(), BusyStatus::Ok)),
            None => bar.with_right(self.hints(ex)),
        })
    }

    /// The keys that apply now, most relevant first.
    fn hints(&self, ex: &Exchange) -> &'static [StatusItem] {
        if self.is_running() {
            &WHILE_RUNNING
        } else if ex.phase.is_live() {
            &WHILE_ANSWERING
        } else if self.text_sel.as_ref().is_some_and(|s| !s.is_empty()) {
            &ON_SELECTION
        } else if self.focus.suggestion().is_some() {
            &ON_SUGGESTION
        } else if let Some(run) = self.focused_output().filter(|r| r.failed() && !r.sent) {
            if run.wants_terminal() {
                &ON_TERMINAL_OUTPUT
            } else {
                &ON_FAILED_OUTPUT
            }
        } else if self.focus.output().is_some() {
            &ON_OUTPUT
        } else if self.focus.selected_item().is_some() {
            if self.items().len() > 1 {
                &ON_ITEM
            } else {
                &ON_ONLY_ITEM
            }
        } else if self.items().is_empty() && self.suggestions().is_empty() {
            &ON_ANSWER
        } else {
            &ON_RUNNABLE_ANSWER
        }
    }

    /// Today's use and the quota, under the recent chats.
    pub(super) fn empty_footer(&self) -> Option<(String, bool)> {
        let mut parts: Vec<String> = Vec::new();
        if let Some(t) = self.today.filter(|t| t.questions > 0) {
            let q = if t.questions == 1 {
                "question"
            } else {
                "questions"
            };
            parts.push(format!(
                "Today: {} {q} · {} tokens",
                t.questions,
                compact_count(t.tokens)
            ));
        }
        let quota = self.quota_left(None);
        let low = quota.as_ref().is_some_and(|(_, low)| *low);
        if let Some((text, _)) = quota {
            let reset = self.provider.reset_zone().next_reset(chrono::Utc::now());
            parts.push(format!("{text} · resets {}", local_clock(reset)));
        }
        (!parts.is_empty()).then(|| (parts.join("  ·  "), low))
    }
}

/// `612 → 143 tokens · $0.0004`; the cost only when prices are configured.
fn usage_label(provider: &Provider, usage: Option<Usage>) -> Option<String> {
    let u = usage?;
    let mut s = format!("{} → {} tokens", u.input, u.output);
    if let Some(cost) = provider.cost(u) {
        s.push_str(&format!(" · ${cost:.4}"));
    }
    Some(s)
}

/// `1.2k`, `9.8M`.
fn compact_count(n: u64) -> String {
    match n {
        0..1_000 => n.to_string(),
        1_000..1_000_000 => format!("{:.1}k", n as f64 / 1e3),
        _ => format!("{:.1}M", n as f64 / 1e6),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn compact_counts() {
        assert_eq!(super::compact_count(999), "999");
        assert_eq!(super::compact_count(9_200), "9.2k");
        assert_eq!(super::compact_count(1_250_000), "1.2M");
    }
}

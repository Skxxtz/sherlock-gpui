use gpui::SharedString;

use crate::ui::{
    components::status_bar::{StatusBar, StatusItem},
    views::{
        BusyStatus,
        ask::{
            model::exchange::{Phase, RemoteLimits, Usage},
            provider::Provider,
            store::quota::local_clock,
            ui::view::AskView,
        },
    },
};

/// The right side: what the keys do now (`AskView::COMMANDS`).
static HINTS: [StatusItem; 1] = [StatusItem::HINTS];

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
            return Some(StatusBar::shell().with_right(&HINTS[..]));
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
            None => bar.with_right(&HINTS[..]),
        })
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

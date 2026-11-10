use std::{cell::RefCell, rc::Rc, time::Duration};

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::views::ask::{
    content::{
        actions::AppEntry,
        markdown::{self, Block},
    },
    exec::runner::CommandRun,
    model::items::{self, Item},
};

/// Runnable items, with the text length and apps version they were found at.
type ItemCache = Option<(usize, u64, Rc<[Item]>)>;

/// Tokens one exchange consumed, as reported by the provider.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    pub input: u32,
    pub output: u32,
}

impl Usage {
    pub fn total(self) -> u64 {
        self.input as u64 + self.output as u64
    }
}

/// What the provider says is left in the current rate-limit window (OpenAI-style headers).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RemoteLimits {
    pub requests: Option<u32>,
    pub tokens: Option<u64>,
}

/// Lifecycle of an exchange.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Sent, no token yet.
    Waiting,
    Streaming,
    Done {
        elapsed: Duration,
    },
    Stopped,
    Failed(SharedString),
    /// Reopened from the history: complete, already saved.
    Loaded,
}

impl Phase {
    pub fn is_live(&self) -> bool {
        matches!(self, Self::Waiting | Self::Streaming)
    }
}

#[derive(Debug)]
pub struct Exchange {
    pub question: SharedString,
    pub phase: Phase,
    /// Set when the provider reports it, at the end of the stream.
    pub usage: Option<Usage>,
    /// Remaining requests/tokens, when the provider reports them.
    pub limits: Option<RemoteLimits>,
    /// Commands from this answer that were run inside Sherlock, with their output.
    pub runs: Vec<CommandRun>,
    /// Output of earlier commands sent along with this question.
    pub attached: Option<String>,
    /// First lines of the commands whose output is attached, for display.
    pub attached_cmds: Vec<String>,
    text: String,
    /// `text` parsed, with the length it was parsed at: reused until the text grows.
    parsed: RefCell<(usize, Rc<[Block]>)>,
    /// `items()`, reused until the text or the installed apps change.
    items: RefCell<ItemCache>,
}

impl Exchange {
    pub fn new(question: impl Into<SharedString>) -> Self {
        Self {
            question: question.into(),
            phase: Phase::Waiting,
            usage: None,
            limits: None,
            runs: Vec::new(),
            attached: None,
            attached_cmds: Vec::new(),
            text: String::new(),
            parsed: RefCell::new((0, Rc::from([]))),
            items: RefCell::new(None),
        }
    }

    /// A saved exchange, reopened from the history.
    pub fn loaded(
        question: impl Into<SharedString>,
        answer: impl Into<String>,
        usage: Option<Usage>,
    ) -> Self {
        let mut ex = Self::new(question);
        ex.text = answer.into();
        ex.usage = usage;
        ex.phase = Phase::Loaded;
        ex
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The answer as markdown blocks. Parsed once per change of the text.
    pub fn blocks(&self) -> Rc<[Block]> {
        let mut parsed = self.parsed.borrow_mut();
        if parsed.0 != self.text.len() {
            *parsed = (self.text.len(), markdown::parse(&self.text).into());
        }
        parsed.1.clone()
    }

    /// The runnable items.
    pub fn items(&self, apps: &[AppEntry], apps_version: u64) -> Rc<[Item]> {
        let len = self.text.len();
        let mut cache = self.items.borrow_mut();
        match &*cache {
            Some((l, v, items)) if *l == len && *v == apps_version => items.clone(),
            _ => {
                let found: Rc<[Item]> = items::items(&self.blocks(), apps).into();
                *cache = Some((len, apps_version, found.clone()));
                found
            }
        }
    }

    /// Appends streamed text. Ignored once the exchange stopped.
    pub fn push(&mut self, delta: &str) {
        if !self.phase.is_live() || delta.is_empty() {
            return;
        }
        self.phase = Phase::Streaming;
        self.text.push_str(delta);
    }

    pub fn finish(&mut self, elapsed: Duration) {
        if self.phase.is_live() {
            self.phase = Phase::Done { elapsed };
        }
    }

    pub fn stop(&mut self) {
        if self.phase.is_live() {
            self.phase = Phase::Stopped;
        }
    }

    pub fn fail(&mut self, msg: impl Into<SharedString>) {
        self.phase = Phase::Failed(msg.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn streams_then_finishes() {
        let mut ex = Exchange::new("q");
        assert_eq!(ex.phase, Phase::Waiting);
        ex.push("Run `ss`. ");
        ex.push("Done");
        assert_eq!(ex.phase, Phase::Streaming);
        assert_eq!(ex.text(), "Run `ss`. Done");
        ex.finish(Duration::from_secs(1));
        assert!(matches!(ex.phase, Phase::Done { .. }));
        assert_eq!(ex.blocks().len(), 1);
    }

    #[test]
    fn no_push_after_stop() {
        let mut ex = Exchange::new("q");
        ex.push("a");
        ex.stop();
        ex.push("b");
        assert_eq!(ex.text(), "a");
        assert_eq!(ex.phase, Phase::Stopped);
    }

    #[test]
    fn blocks_are_parsed_once_per_change() {
        let mut ex = Exchange::new("q");
        ex.push("Hello **world**.");
        let a = ex.blocks();
        assert!(Rc::ptr_eq(&a, &ex.blocks()));
        ex.push(" More.");
        let b = ex.blocks();
        assert!(!Rc::ptr_eq(&a, &b));
        assert_eq!(b.len(), 1);
    }

    #[test]
    fn loaded_exchanges_are_complete() {
        let ex = Exchange::loaded("q", "**a**", None);
        assert!(!ex.phase.is_live());
        assert_eq!(ex.text(), "**a**");
        assert_eq!(ex.blocks().len(), 1);
    }
}

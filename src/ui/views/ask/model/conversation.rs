use std::collections::VecDeque;

use gpui::SharedString;

use crate::ui::views::ask::{
    content::compact,
    model::exchange::Exchange,
    provider::{Message, Role},
};

pub struct Conversation {
    /// Written to the history; follow-ups share it.
    pub id: u64,
    /// Finished exchanges, oldest first. Only the newest `max_turns` are kept (and sent).
    turns: VecDeque<Exchange>,
    /// Compacted forms of `turns`, computed once when an exchange becomes a turn.
    compacted: VecDeque<Compacted>,
    /// The exchange being streamed or last answered.
    pub current: Option<Exchange>,
    /// Browsing earlier exchanges: index into `turns`; `None` shows `current`.
    pub viewing: Option<usize>,
    max_turns: usize,
}

/// What an earlier turn sends along with later questions.
struct Compacted {
    /// `compact::answer` of the reply, shared with every message it's sent in.
    answer: SharedString,
    /// `compact::old_attachment` of the attached output, used once the turn isn't the latest.
    attached: Option<String>,
}

impl Conversation {
    pub fn new(max_turns: usize) -> Self {
        let max_turns = max_turns.max(1);
        Self {
            id: new_id(),
            turns: VecDeque::with_capacity(max_turns + 1),
            compacted: VecDeque::with_capacity(max_turns + 1),
            current: None,
            viewing: None,
            max_turns,
        }
    }

    /// A conversation reopened from the history. `exchanges` oldest first.
    pub fn restore(id: u64, mut exchanges: Vec<Exchange>, max_turns: usize) -> Self {
        let mut c = Self::new(max_turns);
        c.id = id;
        c.current = exchanges.pop();
        let skip = exchanges.len().saturating_sub(c.max_turns);
        for ex in exchanges.into_iter().skip(skip) {
            c.push_turn(ex);
        }
        c
    }

    /// How many earlier exchanges are kept.
    pub fn max_turns(&self) -> usize {
        self.max_turns
    }

    /// Finished exchanges, oldest first.
    pub fn turns(&self) -> &VecDeque<Exchange> {
        &self.turns
    }

    /// The exchange on screen.
    pub fn shown(&self) -> Option<&Exchange> {
        match self.viewing {
            Some(i) => self.turns.get(i),
            None => self.current.as_ref(),
        }
    }

    /// The exchange at `at`: an earlier turn, or the current one for `None`.
    pub fn get_mut(&mut self, at: Option<usize>) -> Option<&mut Exchange> {
        match at {
            Some(i) => self.turns.get_mut(i),
            None => self.current.as_mut(),
        }
    }

    pub fn shown_mut(&mut self) -> Option<&mut Exchange> {
        self.get_mut(self.viewing)
    }

    pub fn is_empty(&self) -> bool {
        self.current.is_none() && self.turns.is_empty()
    }

    pub fn is_live(&self) -> bool {
        self.current.as_ref().is_some_and(|e| e.phase.is_live())
    }

    /// Whether finished command output waits to go along with the next question.
    pub fn has_unsent_output(&self) -> bool {
        self.turns
            .iter()
            .chain(&self.current)
            .flat_map(|ex| &ex.runs)
            .any(|r| !r.sent && !r.is_running())
    }

    /// Starts a new exchange for `question`. The current one becomes a turn, unsent command
    /// output is attached, and the messages for the model are built.
    pub fn begin(&mut self, question: SharedString, system_prompt: &SharedString) -> Vec<Message> {
        let (attached, attached_cmds) = self.take_unsent_output();

        if let Some(prev) = self.current.take()
            && !prev.text().trim().is_empty()
        {
            self.push_turn(prev);
        }
        self.viewing = None;

        let mut messages = Vec::with_capacity(2 + self.turns.len() * 2);
        messages.push(Message {
            role: Role::System,
            content: system_prompt.clone(),
        });
        // Earlier turns go back compacted (see `content::compact`); only the latest turn's
        // output is sent whole, since that's what a follow-up is usually about.
        let last = self.turns.len().saturating_sub(1);
        for (i, (t, c)) in self.turns.iter().zip(&self.compacted).enumerate() {
            let attached = if i == last {
                t.attached.as_deref()
            } else {
                c.attached.as_deref()
            };
            messages.push(Message {
                role: Role::User,
                content: with_attached(attached, &t.question),
            });
            messages.push(Message {
                role: Role::Assistant,
                content: c.answer.clone(),
            });
        }
        messages.push(Message {
            role: Role::User,
            content: with_attached(attached.as_deref(), &question),
        });

        let mut ex = Exchange::new(question);
        ex.attached = attached;
        ex.attached_cmds = attached_cmds;
        self.current = Some(ex);
        messages
    }

    /// Marks finished, unsent command output as sent and
    /// returns it joined for the prompt, plus each command's first line.
    fn take_unsent_output(&mut self) -> (Option<String>, Vec<String>) {
        let mut text = String::new();
        let mut cmds = Vec::new();
        let runs = self
            .turns
            .iter_mut()
            .chain(self.current.iter_mut())
            .flat_map(|ex| ex.runs.iter_mut())
            .filter(|r| !r.sent && !r.is_running());
        for r in runs {
            r.sent = true;
            if !cmds.is_empty() {
                text.push_str("\n\n");
            }
            text.push_str(&r.for_prompt());
            cmds.push(r.script.lines().next().unwrap_or("").to_owned());
        }
        ((!cmds.is_empty()).then_some(text), cmds)
    }

    /// Appends a finished exchange as a turn and drops the oldest
    /// beyond `max_turns`.
    fn push_turn(&mut self, ex: Exchange) {
        self.compacted.push_back(Compacted {
            answer: compact::answer(ex.text()).into(),
            attached: ex.attached.as_deref().map(compact::old_attachment),
        });
        self.turns.push_back(ex);
        while self.turns.len() > self.max_turns {
            self.turns.pop_front();
            self.compacted.pop_front();
        }
    }

    /// Forgets everything; the next question starts a new conversation.
    pub fn reset(&mut self) {
        *self = Self::new(self.max_turns);
    }

    /// Toward an earlier exchange (down the chat). Returns whether it moved.
    pub fn view_earlier(&mut self) -> bool {
        if self.turns.is_empty() {
            return false;
        }
        let next = match self.viewing {
            None => self.turns.len() - 1,
            Some(i) => i.saturating_sub(1),
        };
        let moved = self.viewing != Some(next);
        self.viewing = Some(next);
        moved
    }

    /// Toward a later exchange, then the current one. Returns whether it moved.
    pub fn view_later(&mut self) -> bool {
        match self.viewing {
            Some(i) => {
                self.viewing = (i + 1 < self.turns.len()).then_some(i + 1);
                true
            }
            None => false,
        }
    }

    /// Where the shown exchange sits in the chat, for the status bar; formatted there.
    pub fn position(&self) -> Option<Position> {
        match self.viewing {
            Some(i) => Some(Position::Of {
                n: i + 1,
                total: self.turns.len() + 1,
            }),
            None if !self.turns.is_empty() => Some(Position::Earlier(self.turns.len())),
            None => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Position {
    /// Browsing: exchange `n` of `total`.
    Of { n: usize, total: usize },
    /// On the newest; this many earlier ones are below.
    Earlier(usize),
}

impl std::fmt::Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Of { n, total } => write!(f, "{n} of {total}"),
            Self::Earlier(k) => write!(f, "↓ {k} earlier"),
        }
    }
}

/// The user message. Attached command output, then the question. Without output, it shares
/// the question instead of copying it.
pub fn with_attached(attached: Option<&str>, question: &SharedString) -> SharedString {
    match attached {
        Some(a) => {
            let mut s = String::with_capacity(a.len() + 2 + question.len());
            s.push_str(a);
            s.push_str("\n\n");
            s.push_str(question);
            s.into()
        }
        None => question.clone(),
    }
}

fn new_id() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64)
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;
    use crate::ui::views::ask::exec::runner::{CommandRun, RunState};

    fn answered(c: &mut Conversation, q: &str, a: &str) {
        c.begin(q.to_owned().into(), &"sys".into());
        let ex = c.current.as_mut().unwrap();
        ex.push(a);
        ex.finish(Duration::from_secs(1));
    }

    #[test]
    fn messages_carry_turns_and_attached_output() {
        let mut c = Conversation::new(6);
        answered(&mut c, "q1", "a1");
        let mut run = CommandRun::new(0, "ls -la".into());
        run.state = RunState::Exited(0);
        c.current.as_mut().unwrap().runs.push(run);

        let msgs = c.begin("why?".into(), &"sys".into());
        let roles: Vec<_> = msgs.iter().map(|m| format!("{:?}", m.role)).collect();
        assert_eq!(roles, ["System", "User", "Assistant", "User"]);
        assert!(msgs[3].content.contains("ls -la") && msgs[3].content.ends_with("why?"));
        assert_eq!(c.current.as_ref().unwrap().attached_cmds, ["ls -la"]);
        // sent once only
        assert!(c.turns[0].runs[0].sent);
    }

    #[test]
    fn only_the_latest_turns_output_goes_whole() {
        let mut c = Conversation::new(6);
        let long: Vec<String> = (0..200).map(|i| format!("line {i}")).collect();
        for q in ["q0", "q1", "q2"] {
            answered(&mut c, q, "a");
            let mut run = CommandRun::new(0, "seq 200".into());
            run.output = long.join("\n");
            run.state = RunState::Exited(0);
            c.current.as_mut().unwrap().runs.push(run);
        }
        let msgs = c.begin("q3".into(), &"sys".into());
        // system, q0+a, q1+a (with q0's output), q2+a (with q1's output, the latest turn), q3
        let (older, latest) = (&msgs[3].content, &msgs[5].content);
        assert!(
            older.contains("lines left out"),
            "an older turn's output is cropped"
        );
        assert!(
            latest.contains("line 100"),
            "the newest question's output goes whole"
        );
        assert!(msgs[3].content.len() < msgs[5].content.len());
    }

    #[test]
    fn unsent_output_from_any_answer_counts() {
        let mut c = Conversation::new(6);
        answered(&mut c, "q0", "a");
        answered(&mut c, "q1", "a");
        let mut run = CommandRun::new(0, "ls".into());
        run.state = RunState::Exited(0);
        c.turns[0].runs.push(run);
        assert!(c.has_unsent_output());
        c.begin("q2".into(), &"sys".into());
        assert!(!c.has_unsent_output());
    }

    #[test]
    fn keeps_only_max_turns() {
        let mut c = Conversation::new(2);
        for i in 0..4 {
            answered(&mut c, &format!("q{i}"), "a");
        }
        c.begin("last".into(), &"sys".into());
        assert_eq!(c.turns.len(), 2);
        assert_eq!(c.compacted.len(), 2, "cache trimmed with the turns");
        assert_eq!(&*c.turns[0].question, "q2");
    }

    #[test]
    fn empty_answers_are_not_kept_as_turns() {
        let mut c = Conversation::new(6);
        c.begin("q".into(), &"sys".into()); // stopped before any text
        c.begin("again".into(), &"sys".into());
        assert!(c.turns.is_empty());
        assert!(c.compacted.is_empty());
    }

    #[test]
    fn browsing() {
        let mut c = Conversation::new(6);
        answered(&mut c, "q0", "a");
        answered(&mut c, "q1", "a");
        answered(&mut c, "q2", "a");
        assert_eq!(c.position(), Some(Position::Earlier(2)));
        assert_eq!(c.position().unwrap().to_string(), "↓ 2 earlier");
        assert!(c.view_earlier());
        assert_eq!(&*c.shown().unwrap().question, "q1");
        assert!(c.view_earlier());
        assert!(!c.view_earlier(), "stays on the oldest");
        assert_eq!(c.position(), Some(Position::Of { n: 1, total: 3 }));
        assert_eq!(c.position().unwrap().to_string(), "1 of 3");
        assert!(c.view_later() && c.view_later());
        assert_eq!(&*c.shown().unwrap().question, "q2");
        assert!(!c.view_later());
    }

    #[test]
    fn restore_shows_newest_and_keeps_context() {
        let ex = |q: &str| Exchange::loaded(q.to_owned(), "a", None);
        let c = Conversation::restore(42, vec![ex("q0"), ex("q1"), ex("q2"), ex("q3")], 2);
        assert_eq!(c.id, 42);
        assert_eq!(&*c.shown().unwrap().question, "q3");
        let kept: Vec<_> = c.turns.iter().map(|t| t.question.to_string()).collect();
        assert_eq!(kept, ["q1", "q2"]);
        assert_eq!(c.compacted.len(), 2);

        // a follow-up sends the kept turns
        let mut c = c;
        let msgs = c.begin("next".into(), &"sys".into());
        assert_eq!(
            msgs.len(),
            1 + 2 * 2 + 1,
            "system + the 2 kept turns + question"
        );
        assert_eq!(c.id, 42);
    }
}

use std::{
    io::Read,
    os::unix::process::CommandExt,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tokio::sync::mpsc;

use crate::ui::views::ask::content::compact;

/// Output kept per run.
const MAX_KEPT: usize = 400 * 1024;
/// Lines the panel draws.
pub const MAX_SHOWN_LINES: usize = 500;
/// What goes to the model with the next question.
const PROMPT_LINES: usize = 150;
const PROMPT_BYTES: usize = 8 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RunState {
    Running,
    Exited(i32),
    /// Killed by a signal (also when the user stopped it).
    Killed,
    Failed(String),
}

/// One command run and its captured output.
#[derive(Clone, Debug)]
pub struct CommandRun {
    /// Index of the runnable item in the answer.
    pub item: usize,
    pub script: String,
    pub shell: Shell,
    pub output: String,
    /// Bytes dropped from the front to stay under `MAX_KEPT`.
    pub dropped: usize,
    pub state: RunState,
    pub started: Instant,
    pub elapsed: Duration,
    /// Already attached to a question.
    pub sent: bool,
}

impl CommandRun {
    pub fn new(item: usize, script: String) -> Self {
        Self {
            item,
            script,
            shell: Shell::Sh,
            output: String::new(),
            dropped: 0,
            state: RunState::Running,
            started: Instant::now(),
            elapsed: Duration::ZERO,
            sent: false,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == RunState::Running
    }

    pub fn with_shell(mut self, shell: Shell) -> Self {
        self.shell = shell;
        self
    }

    /// Failed because it wanted a terminal: the
    /// error output says so. Such a run is offered in a terminal instead of a fix.
    pub fn wants_terminal(&self) -> bool {
        const SIGNS: [&str; 9] = [
            "terminal is required",
            "not a terminal",
            "not a tty",
            "no tty",
            "inappropriate ioctl for device",
            "cannot open terminal",
            "error opening terminal",
            "must be run from a terminal",
            "askpass",
        ];
        if !self.failed() {
            return false;
        }
        let mut from = self.output.len().saturating_sub(2048);
        while !self.output.is_char_boundary(from) {
            from += 1;
        }
        let tail = self.output[from..].to_ascii_lowercase();
        SIGNS.iter().any(|s| tail.contains(s))
    }

    /// Ended with an error or couldn't start (stopping it isn't failing).
    pub fn failed(&self) -> bool {
        matches!(self.state, RunState::Exited(c) if c != 0)
            || matches!(self.state, RunState::Failed(_))
    }

    /// The follow-up that asks the model to fix this run; the output goes along with it.
    pub fn fix_question(&self) -> String {
        let cmd = self
            .script
            .lines()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("")
            .trim();
        let how = match &self.state {
            RunState::Exited(code) => format!("exit {code}"),
            RunState::Failed(_) => "couldn't start".to_owned(),
            _ => "failed".to_owned(),
        };
        format!("`{cmd}` failed ({how}). Fix it.")
    }

    fn push(&mut self, chunk: &str) {
        self.output.push_str(&strip_ansi(chunk));
        if self.output.len() > MAX_KEPT {
            // Drop whole lines from the front.
            let excess = self.output.len() - MAX_KEPT;
            let cut = self.output[excess..]
                .find('\n')
                .map_or(excess, |i| excess + i + 1);
            let cut = (cut..=self.output.len())
                .find(|&i| self.output.is_char_boundary(i))
                .unwrap_or(self.output.len());
            self.output.drain(..cut);
            self.dropped += cut;
        }
    }

    pub fn line_count(&self) -> usize {
        self.output.lines().count()
    }

    /// The last `n` lines, and how many lines are hidden before them.
    pub fn tail(&self, n: usize) -> (&str, usize) {
        let total = self.line_count();
        if total <= n {
            return (self.output.trim_end_matches('\n'), 0);
        }
        let skip = total - n;
        let start = self
            .output
            .match_indices('\n')
            .nth(skip - 1)
            .map_or(0, |(i, _)| i + 1);
        (self.output[start..].trim_end_matches('\n'), skip)
    }

    /// The run as context for the next question.
    pub fn for_prompt(&self) -> String {
        let (mut tail, mut hidden) = self.tail(PROMPT_LINES);
        if tail.len() > PROMPT_BYTES {
            let mut start = tail.len() - PROMPT_BYTES;
            while !tail.is_char_boundary(start) {
                start += 1;
            }
            hidden += tail[..start].lines().count();
            tail = &tail[start..];
        }
        let status = match &self.state {
            RunState::Exited(code) => format!("exit code {code}"),
            RunState::Killed => "stopped".into(),
            RunState::Failed(e) => format!("failed to start: {e}"),
            RunState::Running => "still running".into(),
        };
        let cut = if hidden > 0 || self.dropped > 0 {
            "\n(earlier output omitted)"
        } else {
            ""
        };
        let compact = compact::output(tail);
        let body = if compact.is_empty() {
            "(no output)"
        } else {
            &compact
        };
        format!(
            "I ran:\n```{}\n{}\n```\n{status}, output:{cut}\n```text\n{body}\n```",
            self.shell.program(),
            self.script
        )
    }
}

/// The shell a code block runs in, from its fence. Unknown or empty fences use `sh`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shell {
    #[default]
    Sh,
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn from_lang(lang: &str) -> Self {
        match lang.to_ascii_lowercase().as_str() {
            "bash" => Self::Bash,
            "zsh" => Self::Zsh,
            "fish" => Self::Fish,
            _ => Self::Sh,
        }
    }

    pub fn program(self) -> &'static str {
        match self {
            Self::Sh => "sh",
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
        }
    }
}

/// Whether a script must run in a real terminal: a well-known interactive program (password
/// prompt, full-screen) at a command position. A safety net: the model marks most such blocks
/// itself, and a run that still needs one fails fast and says so (`wants_terminal`).
pub fn needs_terminal(script: &str) -> bool {
    const INTERACTIVE: &[&str] = &[
        "sudo", "su", "doas", "pkexec", "ssh", "passwd", "top", "htop", "btop", "less", "more",
        "man", "vim", "vi", "nvim", "nano", "emacs", "watch", "tmux", "screen", "fzf", "ncdu",
        "ranger", "nmtui", "mc", "lazygit", "tig",
    ];
    // Wrappers that run the next word as the command.
    const PREFIXES: &[&str] = &["env", "time", "nohup", "exec", "command", "nice"];
    script
        .split(['\n', ';', '|', '&', '(', ')', '`'])
        .filter_map(|cmd| {
            cmd.split_whitespace()
                .find(|w| !w.contains('=') && !w.starts_with('-') && !PREFIXES.contains(w))
        })
        .map(|prog| prog.rsplit('/').next().unwrap_or(prog))
        .any(|prog| INTERACTIVE.contains(&prog))
}

enum Event {
    Output(String),
    Done(RunState),
}

/// Result of one `drain_into`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunDrain {
    Idle,
    Changed,
    Ended,
}

pub struct RunHandle {
    rx: mpsc::UnboundedReceiver<Event>,
    child: Arc<Mutex<Option<Child>>>,
}

impl Drop for RunHandle {
    fn drop(&mut self) {
        self.kill();
    }
}

impl RunHandle {
    pub fn spawn(script: &str, shell: Shell) -> Result<Self, String> {
        let mut cmd = Command::new(shell.program());
        cmd.arg("-c")
            .arg(script)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("NO_COLOR", "1")
            .env("TERM", "dumb")
            .env("PAGER", "cat")
            .env("GIT_PAGER", "cat")
            .process_group(0);
        let mut child = cmd.spawn().map_err(|e| e.to_string())?;
        let (tx, rx) = mpsc::unbounded_channel();

        let readers: Vec<_> = [
            child
                .stdout
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
            child
                .stderr
                .take()
                .map(|s| Box::new(s) as Box<dyn Read + Send>),
        ]
        .into_iter()
        .flatten()
        .map(|mut pipe| {
            let tx = tx.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 8192];
                // Bytes of a UTF-8 sequence split across reads.
                let mut carry: Vec<u8> = Vec::new();
                while let Ok(n) = pipe.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    carry.extend_from_slice(&buf[..n]);
                    let valid = match std::str::from_utf8(&carry) {
                        Ok(_) => carry.len(),
                        Err(e) if e.error_len().is_none() => e.valid_up_to(),
                        Err(_) => carry.len(), // invalid bytes: let lossy handle them
                    };
                    let text = String::from_utf8_lossy(&carry[..valid]).into_owned();
                    carry.drain(..valid);
                    if tx.send(Event::Output(text)).is_err() {
                        break;
                    }
                }
            })
        })
        .collect();

        let child = Arc::new(Mutex::new(Some(child)));
        let waiter = child.clone();
        std::thread::spawn(move || {
            for r in readers {
                let _ = r.join();
            }
            let status = waiter
                .lock()
                .ok()
                .and_then(|mut c| c.take())
                .map(|mut c| c.wait());
            let state = match status {
                Some(Ok(s)) => match s.code() {
                    Some(code) => RunState::Exited(code),
                    None => RunState::Killed,
                },
                Some(Err(e)) => RunState::Failed(e.to_string()),
                None => RunState::Killed,
            };
            let _ = tx.send(Event::Done(state));
        });
        Ok(Self { rx, child })
    }

    /// Kills the command and everything it started.
    pub fn kill(&self) {
        if let Ok(guard) = self.child.lock()
            && let Some(child) = guard.as_ref()
        {
            // The process group id is the child's pid (`process_group(0)`).
            unsafe {
                libc::kill(-(child.id() as i32), libc::SIGKILL);
            }
        }
    }

    /// Moves everything received so far into `run`. Call once per frame.
    pub fn drain_into(&mut self, run: &mut CommandRun) -> RunDrain {
        let mut state = RunDrain::Idle;
        loop {
            match self.rx.try_recv() {
                Ok(Event::Output(text)) => {
                    run.push(&text);
                    state = RunDrain::Changed;
                }
                Ok(Event::Done(s)) => {
                    run.state = s;
                    run.elapsed = run.started.elapsed();
                    return RunDrain::Ended;
                }
                Err(mpsc::error::TryRecvError::Empty) => {
                    run.elapsed = run.started.elapsed();
                    return state;
                }
                Err(mpsc::error::TryRecvError::Disconnected) => {
                    if run.is_running() {
                        run.state = RunState::Killed;
                    }
                    return RunDrain::Ended;
                }
            }
        }
    }
}

/// Removes ANSI escape sequences (colors, cursor moves) and carriage-return progress redraws.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => match chars.peek() {
                Some('[') => {
                    chars.next();
                    // CSI: parameters until a final byte in @..~
                    for c in chars.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC: until BEL or ESC \
                    chars.next();
                    while let Some(c) = chars.next() {
                        if c == '\u{7}' || (c == '\u{1b}' && chars.peek() == Some(&'\\')) {
                            chars.next_if_eq(&'\\');
                            break;
                        }
                    }
                }
                _ => {
                    chars.next();
                }
            },
            // "\r" without "\n" is a progress bar redrawing its line: keep only the last state.
            '\r' if chars.peek() != Some(&'\n') => {
                if let Some(start) = out.rfind('\n') {
                    out.truncate(start + 1);
                } else {
                    out.clear();
                }
            }
            '\r' => {}
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_to_end(script: &str) -> CommandRun {
        let mut h = RunHandle::spawn(script, Shell::Sh).unwrap();
        let mut run = CommandRun::new(0, script.into());
        let deadline = Instant::now() + Duration::from_secs(5);
        while h.drain_into(&mut run) != RunDrain::Ended {
            assert!(Instant::now() < deadline, "timed out");
            std::thread::sleep(Duration::from_millis(5));
        }
        run
    }

    #[test]
    fn captures_stdout_stderr_and_exit() {
        let run = run_to_end("echo out; echo err >&2; exit 3");
        assert!(run.output.contains("out\n") && run.output.contains("err\n"));
        assert_eq!(run.state, RunState::Exited(3));
        assert!(run.failed());
        assert!(run.fix_question().ends_with("failed (exit 3). Fix it."));
    }

    #[test]
    fn kill_stops_the_group() {
        let mut h = RunHandle::spawn("sleep 30 & sleep 30", Shell::Sh).unwrap();
        let mut run = CommandRun::new(0, String::new());
        h.kill();
        let deadline = Instant::now() + Duration::from_secs(5);
        while h.drain_into(&mut run) != RunDrain::Ended {
            assert!(Instant::now() < deadline, "kill didn't end the run");
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(run.state, RunState::Killed);
    }

    #[test]
    fn tail_and_prompt() {
        let mut run = CommandRun::new(0, "seq 1 400".into());
        run.push(&(1..=400).map(|i| format!("{i}\n")).collect::<String>());
        run.state = RunState::Exited(0);
        let (tail, hidden) = run.tail(3);
        assert_eq!((tail, hidden), ("398\n399\n400", 397));
        let p = run.for_prompt();
        assert!(p.contains("exit code 0") && p.contains("(earlier output omitted)"));
        assert!(p.contains("\n400\n```") && !p.contains("\n1\n"));
    }

    #[test]
    fn memory_is_bounded() {
        let mut run = CommandRun::new(0, String::new());
        let line = "x".repeat(99) + "\n";
        for _ in 0..10_000 {
            run.push(&line);
        }
        assert!(run.output.len() <= MAX_KEPT);
        assert!(run.dropped > 0 && run.output.starts_with('x'));
    }

    #[test]
    fn ansi_and_progress_bars() {
        assert_eq!(strip_ansi("\u{1b}[1;31mred\u{1b}[0m ok"), "red ok");
        assert_eq!(strip_ansi("10%\r50%\r100%\ndone\n"), "100%\ndone\n");
        assert_eq!(strip_ansi("a\r\nb"), "a\nb");
        assert_eq!(strip_ansi("\u{1b}]0;title\u{7}text"), "text");
    }

    #[test]
    fn terminal_detection() {
        assert!(needs_terminal("sudo ss -ltnp"));
        assert!(needs_terminal("journalctl -u x | less"));
        assert!(!needs_terminal("ss -ltnp | grep 8080"));
        assert!(!needs_terminal("echo sudoers"));
        assert!(
            !needs_terminal("grep man /etc/passwd"),
            "only command positions count"
        );
        assert!(!needs_terminal("echo more | wc -c"));
        assert!(needs_terminal("env LANG=C ncdu ~"));
        assert!(needs_terminal("ls\n/usr/bin/htop"));
    }

    #[test]
    fn recognizes_runs_that_wanted_a_terminal() {
        let mut run = CommandRun::new(0, "sudo x".into());
        run.output = "sudo: a terminal is required to read the password".into();
        run.state = RunState::Exited(1);
        assert!(run.wants_terminal());
        run.state = RunState::Exited(0);
        assert!(!run.wants_terminal(), "only failed runs");
        run.output = "permission denied".into();
        run.state = RunState::Exited(1);
        assert!(!run.wants_terminal());
    }

    #[test]
    fn shells_from_fences() {
        assert_eq!(Shell::from_lang("FISH"), Shell::Fish);
        assert_eq!(Shell::from_lang(""), Shell::Sh);
        assert_eq!(Shell::from_lang("console"), Shell::Sh);
    }
}

#![allow(dead_code)]

use std::{
    borrow::Cow,
    fs::{self, File, OpenOptions},
    io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::views::ask::model::exchange::{Exchange, Phase, Usage};

/// One finished exchange.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry<'a> {
    /// Unix seconds.
    pub ts: u64,
    /// Conversation id; follow-ups share it.
    pub conv: u64,
    pub model: Cow<'a, str>,
    pub question: Cow<'a, str>,
    pub answer: Cow<'a, str>,
    /// Stopped by the user before the model finished.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stopped: bool,
    /// Missing in older lines and for providers that don't report it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// A conversation in the history, for the list of recent chats.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatSummary {
    pub conv: u64,
    /// The first question (as far back as the scan reached).
    pub title: SharedString,
    /// Unix seconds of the newest exchange.
    pub last: u64,
    /// Exchanges found.
    pub exchanges: u32,
}

/// Totals over a time range, for the usage line.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub questions: u32,
    pub tokens: u64,
}

impl Stats {
    pub fn add(&mut self, usage: Option<Usage>) {
        self.questions += 1;
        self.tokens += usage.map_or(0, Usage::total);
    }
}

impl<'a> Entry<'a> {
    /// Borrowing entry for a finished exchange; `None` while it's live or if it failed.
    pub fn from_exchange(ex: &'a Exchange, model: &'a str, conv: u64) -> Option<Self> {
        let stopped = match ex.phase {
            Phase::Done { .. } => false,
            Phase::Stopped => true,
            _ => return None,
        };
        if ex.text().trim().is_empty() {
            return None;
        }
        Some(Self {
            ts: now(),
            conv,
            model: model.into(),
            question: Cow::Borrowed(&*ex.question),
            answer: ex.text().into(),
            stopped,
            usage: ex.usage,
        })
    }
}

pub type OwnedEntry = Entry<'static>;

impl Entry<'_> {
    /// Detaches the entry from the exchange it borrows, e.g. to save it on another thread.
    pub fn into_owned(self) -> OwnedEntry {
        Entry {
            ts: self.ts,
            conv: self.conv,
            model: Cow::Owned(self.model.into_owned()),
            question: Cow::Owned(self.question.into_owned()),
            answer: Cow::Owned(self.answer.into_owned()),
            stopped: self.stopped,
            usage: self.usage,
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

#[derive(Clone, Debug)]
pub struct History {
    path: PathBuf,
    /// Entries kept by `compact`.
    max_entries: usize,
}

impl History {
    pub fn new(path: impl Into<PathBuf>, max_entries: usize) -> Self {
        Self {
            path: path.into(),
            max_entries: max_entries.max(1),
        }
    }

    /// `$XDG_STATE_HOME/sherlock/ask/history.jsonl`, falling back to `~/.local/state/...`.
    pub fn default_path() -> Option<PathBuf> {
        let state = crate::utils::paths::get_xdg_dirs().get_state_home()?;
        Some(state.join("ask").join("history.jsonl"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Appends one entry. Blocking file IO: call off the UI thread.
    pub fn append(&self, entry: &Entry<'_>) -> io::Result<()> {
        let mut line = serde_json::to_vec(entry)?;
        line.push(b'\n');
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        // One write of a whole line with O_APPEND: concurrent instances can't interleave lines.
        open_private(&self.path, OpenOptions::new().append(true).create(true))?.write_all(&line)
    }

    /// The last `n` entries, oldest first. Reads backwards from the end, so the cost depends on
    /// `n`, not on the file size. Unparsable lines are skipped.
    pub fn tail(&self, n: usize) -> io::Result<Vec<OwnedEntry>> {
        let mut file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let bytes = read_tail_lines(&mut file, n)?;
        let mut out: Vec<OwnedEntry> = bytes
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .filter_map(|l| {
                serde_json::from_slice::<Entry>(l)
                    .ok()
                    .map(Entry::into_owned)
            })
            .collect();
        let skip = out.len().saturating_sub(n);
        out.drain(..skip);
        Ok(out)
    }

    /// Entries of one conversation, oldest first, searching the last `scan` entries.
    pub fn conversation(&self, conv: u64, scan: usize) -> io::Result<Vec<OwnedEntry>> {
        let mut entries = self.tail(scan)?;
        entries.retain(|e| e.conv == conv);
        Ok(entries)
    }

    /// The newest `n` conversations among the last `scan` entries, most recent first.
    pub fn recent_chats(&self, n: usize, scan: usize) -> io::Result<Vec<ChatSummary>> {
        let mut chats: Vec<ChatSummary> = Vec::new();
        // Newest first: the first entry of a conversation seen is its latest, the last its first.
        for e in self.tail(scan)?.into_iter().rev() {
            match chats.iter().position(|c| c.conv == e.conv) {
                Some(i) => {
                    chats[i].title = e.question.into_owned().into();
                    chats[i].exchanges += 1;
                }
                None if chats.len() < n => chats.push(ChatSummary {
                    conv: e.conv,
                    title: e.question.into_owned().into(),
                    last: e.ts,
                    exchanges: 1,
                }),
                None => {}
            }
        }
        Ok(chats)
    }

    /// Totals of the entries written at or after `since` (unix seconds), among the last `scan`.
    pub fn stats_since(&self, since: u64, scan: usize) -> io::Result<Stats> {
        let mut stats = Stats::default();
        for e in self.tail(scan)?.into_iter().filter(|e| e.ts >= since) {
            stats.add(e.usage);
        }
        Ok(stats)
    }

    /// Keeps only the newest `max_entries`. Runs only when the file is 25% over the limit, so
    /// the rewrite is rare. Atomic: writes a temp file, then renames it over the history.
    pub fn compact(&self) -> io::Result<bool> {
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(false),
            Err(e) => return Err(e),
        };
        let mut lines = 0usize;
        for line in BufReader::new(&file).split(b'\n') {
            if !line?.is_empty() {
                lines += 1;
            }
        }
        if lines <= self.max_entries + self.max_entries / 4 {
            return Ok(false);
        }

        let tmp = self.path.with_extension("jsonl.tmp");
        let mut out = BufWriter::new(open_private(
            &tmp,
            OpenOptions::new().write(true).create(true).truncate(true),
        )?);
        let mut skip = lines - self.max_entries;
        for line in BufReader::new(File::open(&self.path)?).split(b'\n') {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            if skip > 0 {
                skip -= 1;
                continue;
            }
            out.write_all(&line)?;
            out.write_all(b"\n")?;
        }
        out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        fs::rename(&tmp, &self.path)?;
        Ok(true)
    }

    /// Removes every entry of conversation `conv`. Rewrites the file atomically (temp file,
    /// then rename); lines that don't parse are kept. Returns how many entries were removed.
    pub fn delete_conversation(&self, conv: u64) -> io::Result<usize> {
        /// Only the id is needed to decide; the rest of the line is skipped, not parsed.
        #[derive(Deserialize)]
        struct Conv {
            conv: u64,
        }
        let file = match File::open(&self.path) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
            Err(e) => return Err(e),
        };
        let tmp = self.path.with_extension("jsonl.tmp");
        let mut out = BufWriter::new(open_private(
            &tmp,
            OpenOptions::new().write(true).create(true).truncate(true),
        )?);
        let mut removed = 0;
        for line in BufReader::new(file).split(b'\n') {
            let line = line?;
            if line.is_empty() {
                continue;
            }
            if serde_json::from_slice::<Conv>(&line).is_ok_and(|c| c.conv == conv) {
                removed += 1;
                continue;
            }
            out.write_all(&line)?;
            out.write_all(b"\n")?;
        }
        out.into_inner().map_err(|e| e.into_error())?.sync_all()?;
        if removed == 0 {
            fs::remove_file(&tmp)?;
        } else {
            fs::rename(&tmp, &self.path)?;
        }
        Ok(removed)
    }

    /// Deletes the whole history.
    pub fn clear(&self) -> io::Result<()> {
        match fs::remove_file(&self.path) {
            Err(e) if e.kind() != io::ErrorKind::NotFound => Err(e),
            _ => Ok(()),
        }
    }
}

fn open_private(path: &Path, opts: &mut OpenOptions) -> io::Result<File> {
    opts.mode(0o600).open(path)
}

/// Bytes holding at least the last `n` complete lines (or the whole file).
fn read_tail_lines(file: &mut File, n: usize) -> io::Result<Vec<u8>> {
    const BLOCK: u64 = 16 * 1024;
    let len = file.seek(SeekFrom::End(0))?;
    let mut pos = len;
    let mut buf: Vec<u8> = Vec::new();
    // n lines need n + 1 newlines before them (the last line ends with one).
    while pos > 0 && bytecount(&buf, b'\n') <= n {
        let step = BLOCK.min(pos);
        pos -= step;
        file.seek(SeekFrom::Start(pos))?;
        let mut block = vec![0; step as usize];
        file.read_exact(&mut block)?;
        block.extend_from_slice(&buf);
        buf = block;
    }
    // Drop a partial first line unless we reached the start of the file.
    if pos > 0
        && let Some(nl) = buf.iter().position(|&b| b == b'\n')
    {
        buf.drain(..=nl);
    }
    Ok(buf)
}

fn bytecount(buf: &[u8], b: u8) -> usize {
    buf.iter().filter(|&&x| x == b).count()
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sherlock-ask-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir.join("ask").join("history.jsonl")
    }

    fn entry(i: usize) -> OwnedEntry {
        Entry {
            ts: i as u64,
            conv: (i / 2) as u64,
            model: "m".into(),
            question: format!("q{i}").into(),
            answer: format!("answer {i} with \"quotes\"\nand a newline").into(),
            stopped: false,
            usage: (i % 2 == 0).then_some(Usage {
                input: 10,
                output: 5,
            }),
        }
    }

    #[test]
    fn append_and_tail() {
        let h = History::new(tmp("tail"), 100);
        for i in 0..10 {
            h.append(&entry(i)).unwrap();
        }
        let t = h.tail(3).unwrap();
        assert_eq!(t, vec![entry(7), entry(8), entry(9)]);
        assert_eq!(h.tail(50).unwrap().len(), 10);
        let mode = fs::metadata(h.path()).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn tail_across_blocks() {
        let h = History::new(tmp("blocks"), 10_000);
        let mut big = entry(0);
        big.answer = "x".repeat(20_000).into();
        h.append(&big).unwrap();
        for i in 1..5 {
            h.append(&entry(i)).unwrap();
        }
        let t = h.tail(5).unwrap();
        assert_eq!(t.len(), 5);
        assert_eq!(t[0].answer.len(), 20_000);
    }

    #[test]
    fn missing_file_is_empty() {
        let h = History::new(tmp("missing"), 10);
        assert!(h.tail(5).unwrap().is_empty());
        assert!(!h.compact().unwrap());
    }

    #[test]
    fn compact_keeps_newest() {
        let h = History::new(tmp("compact"), 4);
        for i in 0..5 {
            h.append(&entry(i)).unwrap();
        }
        assert!(!h.compact().unwrap(), "within the 25% slack");
        h.append(&entry(5)).unwrap();
        assert!(h.compact().unwrap());
        assert_eq!(h.tail(10).unwrap(), (2..6).map(entry).collect::<Vec<_>>());
    }

    #[test]
    fn stats_since_counts_recent_entries() {
        let h = History::new(tmp("stats"), 100);
        for i in 0..6 {
            h.append(&entry(i)).unwrap();
        }
        // ts = i; entries 2..6 are "today"; usage on even ones (2, 4).
        assert_eq!(
            h.stats_since(2, 100).unwrap(),
            Stats {
                questions: 4,
                tokens: 30
            }
        );
    }

    #[test]
    fn old_lines_without_usage_still_parse() {
        let h = History::new(tmp("compat"), 100);
        fs::create_dir_all(h.path().parent().unwrap()).unwrap();
        fs::write(
            h.path(),
            "{\"ts\":1,\"conv\":0,\"model\":\"m\",\"question\":\"q\",\"answer\":\"a\"}\n",
        )
        .unwrap();
        assert_eq!(h.tail(5).unwrap()[0].usage, None);
    }

    #[test]
    fn conversation_filter() {
        let h = History::new(tmp("conv"), 100);
        for i in 0..6 {
            h.append(&entry(i)).unwrap();
        }
        let c = h.conversation(1, 100).unwrap();
        assert_eq!(c, vec![entry(2), entry(3)]);
    }

    #[test]
    fn only_finished_exchanges_are_saved() {
        let mut ex = Exchange::new("q");
        ex.push("partial");
        assert!(Entry::from_exchange(&ex, "m", 0).is_none());
        ex.stop();
        let e = Entry::from_exchange(&ex, "m", 0).unwrap();
        assert!(e.stopped);
        assert_eq!(e.answer, "partial");
    }

    #[test]
    fn recent_chats_group_by_conversation() {
        let p = tmp("chats");
        let h = History::new(&p, 100);
        let e = |conv: u64, ts: u64, q: &str| Entry {
            ts,
            conv,
            model: "m".into(),
            question: q.to_owned().into(),
            answer: "a".into(),
            stopped: false,
            usage: None,
        };
        for entry in [
            e(1, 1, "first"),
            e(2, 2, "other"),
            e(1, 3, "follow-up"),
            e(3, 4, "third"),
        ] {
            h.append(&entry).unwrap();
        }
        let chats = h.recent_chats(2, 100).unwrap();
        assert_eq!(chats.len(), 2);
        assert_eq!(
            (chats[0].conv, &*chats[0].title, chats[0].exchanges),
            (3, "third", 1)
        );
        assert_eq!(
            (
                chats[1].conv,
                &*chats[1].title,
                chats[1].exchanges,
                chats[1].last
            ),
            (1, "first", 2, 3)
        );
        let _ = fs::remove_file(p);
    }

    #[test]
    fn delete_conversation_keeps_the_others() {
        let p = tmp("delete");
        let h = History::new(&p, 100);
        for (conv, q) in [(1, "a"), (2, "b"), (1, "c")] {
            let mut e = entry(0);
            e.conv = conv;
            e.question = q.to_owned().into();
            h.append(&e).unwrap();
        }
        assert_eq!(h.delete_conversation(1).unwrap(), 2);
        let left: Vec<_> = h
            .tail(10)
            .unwrap()
            .into_iter()
            .map(|e| e.question.into_owned())
            .collect();
        assert_eq!(left, ["b"]);
        assert_eq!(h.delete_conversation(7).unwrap(), 0);
        assert_eq!(h.tail(10).unwrap().len(), 1);
        let _ = fs::remove_file(p);
    }
}

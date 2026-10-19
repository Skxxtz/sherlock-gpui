//! Turns words into hashed feature ids, the model's only input.
//!
//! A query is analysed once into a [`Query`] (lowercase words, a coarse class
//! per word, and spans found by small duration/time/percent recognisers).
//! Every feature is then a short string like `w=timer` or `w-1|w=a|timer`,
//! hashed straight into `0..BUCKETS` without building the string. Used by
//! `parse()` at runtime and by the trainer, so both see the same features.

use std::collections::HashSet;

use crate::{Intent, SlotKind, Tag};

/// Hash table size as a power of two.
pub const BITS: u32 = 14;
/// Number of weight slots per model row.
pub const BUCKETS: usize = 1 << BITS;

/// Words the caller knows about, e.g. installed app names. Optional:
/// the model works without, but recognises unseen app names much better with.
#[derive(Debug, Clone, Default)]
pub struct Lexicon {
    apps: HashSet<String>,
}

impl Lexicon {
    /// From app names like `"Visual Studio Code"`; each word is added lowercase.
    pub fn with_apps<I, S>(apps: I) -> Lexicon
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let apps = apps
            .into_iter()
            .flat_map(|a| {
                a.as_ref()
                    .split_whitespace()
                    .map(str::to_lowercase)
                    .collect::<Vec<_>>()
            })
            .collect();
        Lexicon { apps }
    }

    /// No words at all (then app names can't be checked).
    pub fn is_empty(&self) -> bool {
        self.apps.is_empty()
    }

    /// Is `word` (lowercase) part of a known app name?
    pub fn is_app(&self, word: &str) -> bool {
        self.apps.contains(word)
    }
}

/// Split and lowercase a query.
pub fn words(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// A query analysed once: words, a class per word, recognised spans.
pub struct Query<'a> {
    /// Lowercase words.
    pub words: &'a [String],
    class: Vec<u8>,
    /// Per word: 0 = no span, else `Tag::index()` of a B-/I- tag of the
    /// recognised slot kind (duration, time or percent).
    span: Vec<u8>,
}

impl<'a> Query<'a> {
    /// Analyse lowercase `words`.
    pub fn new(words: &'a [String], lex: &Lexicon) -> Query<'a> {
        Query {
            words,
            class: words.iter().map(|w| class(w, lex)).collect(),
            span: recognise(words),
        }
    }

    /// Number of words.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// No words at all.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    fn word(&self, i: usize, d: isize) -> &str {
        let j = i as isize + d;
        if j < 0 || j as usize >= self.words.len() {
            "<pad>"
        } else {
            &self.words[j as usize]
        }
    }

    fn class_at(&self, i: usize, d: isize) -> u64 {
        let j = i as isize + d;
        if j < 0 || j as usize >= self.words.len() {
            0
        } else {
            self.class[j as usize] as u64
        }
    }

    fn span_at(&self, i: usize, d: isize) -> u64 {
        let j = i as isize + d;
        if j < 0 || j as usize >= self.words.len() {
            0
        } else {
            self.span[j as usize] as u64
        }
    }
}

/// Sentence features for the intent classifier.
pub fn sentence(q: &Query, out: &mut Vec<usize>) {
    out.clear();
    out.push(H::new(b'b').done());
    out.push(H::new(b'L').num(q.len().min(6) as u64).done());
    for (i, w) in q.words.iter().enumerate() {
        let c = q.class[i] as u64;
        out.push(H::new(b'u').s(w).done());
        out.push(H::new(b's').s(&shape(w)).done());
        out.push(H::new(b'c').num(c).done());
        if i == 0 {
            out.push(H::new(b'f').s(w).done());
        }
        if let Some(next) = q.words.get(i + 1) {
            out.push(H::new(b'2').s(w).s(next).done());
            out.push(H::new(b'C').num(c).num(q.class_at(i, 1)).done());
        }
        // which kinds of value the recognisers found (a duration hints timer/remind)
        if let Tag::B(k) = Tag::from_index(q.span[i] as usize)
            && q.span[i] != 0
        {
            out.push(H::new(b'r').num(k.index() as u64).done());
        }
    }
    if let Some(last) = q.words.last() {
        out.push(H::new(b'l').s(last).done());
    }
}

/// Word features for the tagger. = `token_static` + `token_prev`.
pub fn token(q: &Query, i: usize, prev: Tag, intent: Option<Intent>, out: &mut Vec<usize>) {
    token_static(q, i, intent, out);
    let mut p = Vec::with_capacity(7);
    token_prev(q, i, prev, intent, &mut p);
    out.extend_from_slice(&p);
}

/// The part of `token` that doesn't depend on the previous tag
/// (computed once per word during Viterbi decoding).
pub fn token_static(q: &Query, i: usize, intent: Option<Intent>, out: &mut Vec<usize>) {
    out.clear();
    let w = q.words[i].as_str();
    let it = intent_id(intent);
    let c = q.class[i] as u64;
    let sp = q.span[i] as u64;
    let pre: String = w.chars().take(3).collect();
    let suf: String = w.chars().rev().take(3).collect();

    out.push(H::new(b'b').done());
    out.push(H::new(b'w').s(w).done());
    out.push(H::new(b'p').s(&pre).done());
    out.push(H::new(b'x').s(&suf).done());
    out.push(H::new(b's').s(&shape(w)).done());
    out.push(H::new(b'c').num(c).done());
    for (k, d) in [(b'1', -1), (b'2', 1), (b'3', -2), (b'4', 2)] {
        out.push(H::new(k).s(q.word(i, d)).done());
    }
    out.push(H::new(b'5').s(q.word(i, -1)).s(w).done());
    out.push(H::new(b'6').s(w).s(q.word(i, 1)).done());
    out.push(H::new(b'7').num(q.class_at(i, -1)).done());
    out.push(H::new(b'8').num(q.class_at(i, 1)).done());
    out.push(H::new(b'i').num(it).done());
    out.push(H::new(b'I').num(it).s(w).done());
    out.push(H::new(b'J').num(it).num(c).done());
    // recognised spans: this word's and its neighbours'
    out.push(H::new(b'R').num(sp).done());
    out.push(H::new(b'U').num(it).num(sp).done());
    out.push(
        H::new(b'V')
            .num(q.span_at(i, -1))
            .num(sp)
            .num(q.span_at(i, 1))
            .done(),
    );
}

/// The part of `token` that depends on the previous tag.
pub fn token_prev(q: &Query, i: usize, prev: Tag, intent: Option<Intent>, out: &mut Vec<usize>) {
    out.clear();
    let w = q.words[i].as_str();
    let pt = prev.index() as u64;
    out.push(H::new(b't').num(pt).done());
    out.push(H::new(b'T').num(pt).s(w).done());
    out.push(H::new(b'S').num(pt).s(&shape(w)).done());
    out.push(H::new(b'K').num(pt).num(q.class[i] as u64).done());
    out.push(H::new(b'Q').num(intent_id(intent)).num(pt).done());
    out.push(H::new(b'P').num(pt).num(q.span[i] as u64).done());
}

fn intent_id(intent: Option<Intent>) -> u64 {
    intent.map_or(99, |x| Intent::ALL.iter().position(|y| *y == x).unwrap()) as u64
}

const DUR_UNITS: &[&str] = &[
    "s", "sec", "secs", "second", "seconds", "m", "min", "mins", "minute", "minutes", "h", "hr",
    "hrs", "hour", "hours",
];
const TIME_WORDS: &[&str] = &[
    "noon",
    "midnight",
    "tomorrow",
    "tonight",
    "morning",
    "evening",
    "afternoon",
];
/// Words that stand for a level: `volume max`.
const PCT_WORDS: &[&str] = &["max", "min", "mute", "half", "full", "loud", "quiet"];
/// Spelled-out numbers.
const NUM_WORDS: &[&str] = &[
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "ten",
    "eleven",
    "twelve",
    "thirteen",
    "fourteen",
    "fifteen",
    "sixteen",
    "seventeen",
    "eighteen",
    "nineteen",
    "twenty",
    "thirty",
    "forty",
    "fifty",
    "sixty",
    "seventy",
    "eighty",
    "ninety",
    "hundred",
];
/// Amount words that can stand before a unit: `a minute`, `half an hour`.
const AMOUNTS: &[&str] = &["a", "an", "half", "quarter", "couple", "few", "several"];

fn digits(w: &str) -> usize {
    w.find(|c: char| !c.is_ascii_digit()).unwrap_or(w.len())
}

fn is_number(w: &str) -> bool {
    (!w.is_empty() && digits(w) == w.len()) || NUM_WORDS.contains(&w)
}

/// `20m`, `3hrs`
fn is_num_unit(w: &str) -> bool {
    let d = digits(w);
    d > 0 && DUR_UNITS.contains(&&w[d..])
}

/// `5pm`, `7:30`, `7:30pm`
fn is_clock(w: &str) -> bool {
    let d = digits(w);
    let rest = &w[d..];
    d > 0 && (rest == "am" || rest == "pm" || rest.starts_with(':'))
}

/// Coarse word class from fixed word lists and the lexicon.
fn class(w: &str, lex: &Lexicon) -> u8 {
    let d = digits(w);
    let rest = &w[d..];
    match () {
        _ if is_number(w) => 1,                                // 20, twenty
        _ if is_num_unit(w) => 2,                              // 20m
        _ if is_clock(w) => 3,                                 // 5pm, 7:30
        _ if d > 0 && (rest == "%" || rest == "percent") => 4, // 30%
        _ if w == "%" || w == "percent" || PCT_WORDS.contains(&w) => 4,
        _ if DUR_UNITS.contains(&w) => 5, // minutes
        _ if TIME_WORDS.contains(&w) || w == "am" || w == "pm" || w == "o'clock" => 6,
        _ if AMOUNTS.contains(&w) => 10, // a, half, quarter
        _ if lex.is_app(w) => 7,         // firefox
        _ if d > 0 => 8,                 // 3rd, 2x
        _ => 9,
    }
}

/// Small rule-based recognisers for durations, times and percentages.
/// They mark candidate spans; the model learns how far to trust them.
fn recognise(words: &[String]) -> Vec<u8> {
    let n = words.len();
    let w = |i: usize| words.get(i).map_or("", String::as_str);
    let mut out = vec![0u8; n];
    let mark = |out: &mut Vec<u8>, from: usize, to: usize, kind: SlotKind| {
        for (j, o) in out.iter_mut().enumerate().take(to).skip(from) {
            *o = if j == from {
                Tag::B(kind)
            } else {
                Tag::I(kind)
            }
            .index() as u8;
        }
    };

    let mut i = 0;
    while i < n {
        // duration: `20m`, `1h 30m`
        if is_num_unit(w(i)) {
            let mut end = i + 1;
            while end < n && is_num_unit(w(end)) {
                end += 1;
            }
            mark(&mut out, i, end, SlotKind::Duration);
            i = end;
            continue;
        }
        // duration: amount/number words … unit: `ten minutes`, `half an hour`,
        // `quarter of an hour`, `2 and a half hours`, `a couple of minutes`
        if is_number(w(i)) || AMOUNTS.contains(&w(i)) {
            let mut j = i + 1;
            while j < n
                && j < i + 5
                && (is_number(w(j)) || AMOUNTS.contains(&w(j)) || ["of", "and"].contains(&w(j)))
            {
                j += 1;
            }
            if j < n && DUR_UNITS.contains(&w(j)) {
                let mut end = j + 1;
                if w(end) == "and" && w(end + 1) == "a" && w(end + 2) == "half" {
                    end += 3;
                }
                mark(&mut out, i, end, SlotKind::Duration);
                i = end;
                continue;
            }
        }
        // time: `5pm`, `7:30`, `5 pm`, `5 o'clock`, `half past 5`, `quarter to 7`,
        // `noon`, `tomorrow`, each optionally followed by more time words
        let time_len = if is_clock(w(i)) {
            1
        } else if is_number(w(i)) && ["am", "pm", "o'clock"].contains(&w(i + 1)) {
            2
        } else if ["half", "quarter"].contains(&w(i))
            && ["past", "to"].contains(&w(i + 1))
            && is_number(w(i + 2))
        {
            3
        } else if TIME_WORDS.contains(&w(i)) {
            1
        } else {
            0
        };
        if time_len > 0 {
            let mut end = i + time_len;
            while end < n
                && (TIME_WORDS.contains(&w(end))
                    || ["am", "pm"].contains(&w(end))
                    || is_clock(w(end)))
            {
                end += 1;
            }
            mark(&mut out, i, end, SlotKind::Time);
            i = end;
            continue;
        }
        // percent: `30%`, `30 %`, `30 percent`, `30 per cent`, `max`
        let d = digits(w(i));
        let pct_len = if d > 0 && (&w(i)[d..] == "%" || &w(i)[d..] == "percent") {
            1
        } else if is_number(w(i)) && (w(i + 1) == "%" || w(i + 1) == "percent") {
            2
        } else if is_number(w(i)) && w(i + 1) == "per" && w(i + 2) == "cent" {
            3
        } else if PCT_WORDS.contains(&w(i)) {
            1
        } else {
            0
        };
        if pct_len > 0 {
            mark(&mut out, i, i + pct_len, SlotKind::Percent);
            i += pct_len;
            continue;
        }
        i += 1;
    }
    out
}

/// Character classes with repeats collapsed: `20m` → `9a`, `7:30pm` → `9:9a`.
fn shape(w: &str) -> String {
    let mut s = String::new();
    let mut last = '\0';
    for c in w.chars() {
        let k = if c.is_ascii_digit() {
            '9'
        } else if c.is_alphabetic() {
            'a'
        } else {
            c
        };
        if k != last {
            s.push(k);
            last = k;
        }
    }
    s
}

/// FNV-1a over feature parts, without building a string.
struct H(u64);

impl H {
    fn new(kind: u8) -> H {
        let mut h = H(0xcbf2_9ce4_8422_2325);
        h.byte(kind);
        h
    }
    fn byte(&mut self, b: u8) {
        self.0 ^= b as u64;
        self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
    }
    fn s(mut self, s: &str) -> H {
        self.byte(0x1f);
        for b in s.bytes() {
            self.byte(b);
        }
        self
    }
    fn num(mut self, n: u64) -> H {
        self.byte(0x1e);
        for b in n.to_le_bytes() {
            self.byte(b);
        }
        self
    }
    fn done(self) -> usize {
        (self.0 as usize) & (BUCKETS - 1)
    }
}

/// The recogniser's spans as `(kind, words)`, for tests and debugging.
pub fn recognised(query: &str) -> Vec<(SlotKind, String)> {
    let w = words(query);
    let marks = recognise(&w);
    let mut out: Vec<(SlotKind, String)> = Vec::new();
    for (word, m) in w.iter().zip(marks) {
        match Tag::from_index(m as usize) {
            Tag::B(k) if m != 0 => out.push((k, word.clone())),
            Tag::I(_) => {
                let last = out.last_mut().unwrap();
                last.1.push(' ');
                last.1.push_str(word);
            }
            _ => {}
        }
    }
    out
}

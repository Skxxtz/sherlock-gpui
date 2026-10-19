//! Turns words into hashed feature ids, the model's only input.
//!
//! Every feature is a short string like `w=timer` or `w-1|w=a|timer`, hashed
//! straight into `0..BUCKETS` without building the string. Used by `parse()`
//! at runtime and by the trainer, so both always see the same features.

use std::collections::HashSet;

use crate::{Intent, Tag};

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

    /// Is `word` (lowercase) part of a known app name?
    pub fn is_app(&self, word: &str) -> bool {
        self.apps.contains(word)
    }
}

/// Split and lowercase a query.
pub fn words(query: &str) -> Vec<String> {
    query.split_whitespace().map(str::to_lowercase).collect()
}

/// Sentence features for the intent classifier.
pub fn sentence(words: &[String], lex: &Lexicon, out: &mut Vec<usize>) {
    out.clear();
    out.push(H::new(b'b').done());
    out.push(H::new(b'L').num(words.len().min(6) as u64).done());
    for (i, w) in words.iter().enumerate() {
        let class = class(w, lex);
        out.push(H::new(b'u').s(w).done());
        out.push(H::new(b's').s(&shape(w)).done());
        out.push(H::new(b'c').num(class as u64).done());
        if i == 0 {
            out.push(H::new(b'f').s(w).done());
        }
        if let Some(next) = words.get(i + 1) {
            out.push(H::new(b'2').s(w).s(next).done());
            out.push(
                H::new(b'C')
                    .num(class as u64)
                    .num(self::class(next, lex) as u64)
                    .done(),
            );
        }
    }
    if let Some(last) = words.last() {
        out.push(H::new(b'l').s(last).done());
    }
}

/// Word features for the tagger: word `i`, its neighbours, the previous tag
/// and the (predicted) intent. = `token_static` + `token_prev`.
pub fn token(
    words: &[String],
    i: usize,
    prev: Tag,
    intent: Option<Intent>,
    lex: &Lexicon,
    out: &mut Vec<usize>,
) {
    token_static(words, i, intent, lex, out);
    let mut p = Vec::with_capacity(5);
    token_prev(words, i, prev, intent, lex, &mut p);
    out.extend_from_slice(&p);
}

/// The part of `token` that doesn't depend on the previous tag
/// (computed once per word during Viterbi decoding).
pub fn token_static(
    words: &[String],
    i: usize,
    intent: Option<Intent>,
    lex: &Lexicon,
    out: &mut Vec<usize>,
) {
    out.clear();
    let w = words[i].as_str();
    let at = |d: isize| at(words, i, d);
    let it = intent_id(intent);
    let c = class(w, lex) as u64;
    let pre: String = w.chars().take(3).collect();
    let suf: String = w.chars().rev().take(3).collect();

    out.push(H::new(b'b').done());
    out.push(H::new(b'w').s(w).done());
    out.push(H::new(b'p').s(&pre).done());
    out.push(H::new(b'x').s(&suf).done());
    out.push(H::new(b's').s(&shape(w)).done());
    out.push(H::new(b'c').num(c).done());
    for (k, d) in [(b'1', -1), (b'2', 1), (b'3', -2), (b'4', 2)] {
        out.push(H::new(k).s(at(d)).done());
    }
    out.push(H::new(b'5').s(at(-1)).s(w).done());
    out.push(H::new(b'6').s(w).s(at(1)).done());
    out.push(H::new(b'7').num(class(at(-1), lex) as u64).done());
    out.push(H::new(b'8').num(class(at(1), lex) as u64).done());
    out.push(H::new(b'i').num(it).done());
    out.push(H::new(b'I').num(it).s(w).done());
    out.push(H::new(b'J').num(it).num(c).done());
}

/// The part of `token` that depends on the previous tag.
pub fn token_prev(
    words: &[String],
    i: usize,
    prev: Tag,
    intent: Option<Intent>,
    lex: &Lexicon,
    out: &mut Vec<usize>,
) {
    out.clear();
    let w = words[i].as_str();
    let pt = prev.index() as u64;
    out.push(H::new(b't').num(pt).done());
    out.push(H::new(b'T').num(pt).s(w).done());
    out.push(H::new(b'S').num(pt).s(&shape(w)).done());
    out.push(H::new(b'K').num(pt).num(class(w, lex) as u64).done());
    out.push(H::new(b'Q').num(intent_id(intent)).num(pt).done());
}

/// Word at offset `d` from `i`, or `<pad>` outside the query.
fn at(words: &[String], i: usize, d: isize) -> &str {
    let j = i as isize + d;
    if j < 0 || j as usize >= words.len() {
        "<pad>"
    } else {
        &words[j as usize]
    }
}

fn intent_id(intent: Option<Intent>) -> u64 {
    intent.map_or(99, |x| Intent::ALL.iter().position(|y| *y == x).unwrap()) as u64
}

/// Coarse word class from fixed word lists and the lexicon.
fn class(w: &str, lex: &Lexicon) -> u8 {
    const DUR_UNITS: &[&str] = &[
        "s", "sec", "secs", "second", "seconds", "m", "min", "mins", "minute", "minutes", "h",
        "hr", "hrs", "hour", "hours",
    ];
    const TIME_WORDS: &[&str] = &[
        "am", "pm", "noon", "midnight", "tomorrow", "tonight", "morning", "evening", "o'clock",
    ];
    // words that stand for a level: `volume max`
    const PCT_WORDS: &[&str] = &[
        "%", "percent", "max", "min", "mute", "half", "full", "loud", "quiet",
    ];
    let digits = w.find(|c: char| !c.is_ascii_digit()).unwrap_or(w.len());
    let rest = &w[digits..];
    match () {
        _ if w == "<pad>" => 0,
        _ if digits > 0 && rest.is_empty() => 1, // 20
        _ if digits > 0 && DUR_UNITS.contains(&rest) => 2, // 20m
        _ if digits > 0 && (rest == "am" || rest == "pm") => 3, // 5pm
        _ if digits > 0 && rest.starts_with(':') => 3, // 7:30
        _ if digits > 0 && rest == "%" => 4,     // 30%
        _ if PCT_WORDS.contains(&w) => 4,
        _ if DUR_UNITS.contains(&w) => 5,  // minutes
        _ if TIME_WORDS.contains(&w) => 6, // tomorrow
        _ if lex.is_app(w) => 7,           // firefox
        _ if digits > 0 => 8,              // 3rd, 2x
        _ => 9,
    }
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

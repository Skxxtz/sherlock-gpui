//! Synthetic training data from `data/templates.txt` and `data/slots.txt`.
//!
//! Templates are expanded with random list choices, optional parts and slot
//! values into labelled examples: words, one `Tag` per word, and the intent.

use std::collections::HashMap;

use super::Rng;
use crate::{Intent, SlotKind, Tag};

/// The bundled template file.
pub const TEMPLATES: &str = include_str!("../../data/templates.txt");
/// The bundled slot value file.
pub const SLOTS: &str = include_str!("../../data/slots.txt");

/// One labelled sentence.
#[derive(Debug, Clone, PartialEq)]
pub struct Example {
    /// The words of the query.
    pub words: Vec<String>,
    /// One tag per word.
    pub tags: Vec<Tag>,
    /// The intent; `None` = not a command.
    pub intent: Option<Intent>,
}

impl Example {
    /// The query as typed.
    pub fn text(&self) -> String {
        self.words.join(" ")
    }

    /// The phrasing without slot values: `timer {DUR} then {ACTION}`.
    /// Used to check that test phrasings never appear in training.
    pub fn skeleton(&self) -> String {
        let mut out: Vec<String> = Vec::new();
        for (w, t) in self.words.iter().zip(&self.tags) {
            match t {
                Tag::O => out.push(w.to_lowercase()),
                Tag::B(s) => out.push(format!("{{{}}}", s.name())),
                Tag::I(_) => {}
            }
        }
        out.join(" ")
    }
}

/// Which part of the data to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// Training phrasings / values.
    Train,
    /// Held-out phrasings / values, for evaluation only.
    Test,
}

/// One template token.
#[derive(Debug, Clone)]
enum Tok {
    Word(String),
    List(String),
    Slot(SlotKind),
    /// `[ … ]`: all of it or nothing.
    Opt(Vec<Tok>),
}

#[derive(Debug, Clone)]
struct Template {
    intent: Option<Intent>,
    toks: Vec<Tok>,
}

/// Parsed templates, word lists and slot values.
#[derive(Clone)]
pub struct Dataset {
    lists: HashMap<String, Vec<Vec<String>>>,
    /// per split: templates grouped by intent, so every intent is sampled
    /// equally often however many templates it has
    templates: [Vec<Vec<Template>>; 2],
    values: [HashMap<SlotKind, Vec<Vec<String>>>; 2],
}

fn split_choices(s: &str) -> Vec<Vec<String>> {
    s.split('|')
        .map(|c| c.split_whitespace().map(String::from).collect())
        .collect()
}

fn idx(split: Split) -> usize {
    match split {
        Split::Train => 0,
        Split::Test => 1,
    }
}

impl Dataset {
    /// The bundled data files.
    pub fn bundled() -> Result<Dataset, String> {
        Dataset::parse(TEMPLATES, SLOTS)
    }

    /// Parse template and slot files; errors name the line and problem.
    pub fn parse(templates: &str, slots: &str) -> Result<Dataset, String> {
        let mut ds = Dataset {
            lists: HashMap::new(),
            templates: [Vec::new(), Vec::new()],
            values: [HashMap::new(), HashMap::new()],
        };

        // slot values
        for (n, line) in slots.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (name, vals) = line
                .split_once('=')
                .ok_or(format!("slots.txt:{}: expected `SLOT = values`", n + 1))?;
            let name = name.trim();
            let (name, split) = match name.strip_suffix('!') {
                Some(n) => (n, Split::Test),
                None => (name, Split::Train),
            };
            let kind = SlotKind::from_name(name)
                .ok_or(format!("slots.txt:{}: unknown slot `{name}`", n + 1))?;
            ds.values[idx(split)].insert(kind, split_choices(vals));
        }

        // lists and templates
        let mut split = Split::Train;
        for (n, line) in templates.lines().enumerate() {
            let at = |msg: String| format!("templates.txt:{}: {msg}", n + 1);
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "== test ==" {
                split = Split::Test;
                continue;
            }
            if let Some(rest) = line.strip_prefix('@') {
                let (name, choices) = rest
                    .split_once('=')
                    .ok_or(at("expected `@name = a | b`".into()))?;
                ds.lists
                    .insert(name.trim().to_string(), split_choices(choices));
                continue;
            }
            let (intent, body) = line
                .split_once(':')
                .ok_or(at("expected `intent: words`".into()))?;
            let intent = match intent.trim() {
                "none" => None,
                name => {
                    Some(Intent::from_name(name).ok_or(at(format!("unknown intent `{name}`")))?)
                }
            };
            let toks = parse_body(body, intent, &ds.lists).map_err(at)?;
            let groups = &mut ds.templates[idx(split)];
            match groups.iter_mut().find(|g| g[0].intent == intent) {
                Some(g) => g.push(Template { intent, toks }),
                None => groups.push(vec![Template { intent, toks }]),
            }
        }

        // every slot used must have values for both splits
        let mut used = Vec::new();
        for t in ds.templates.iter().flatten().flatten() {
            collect_slots(&t.toks, &mut used);
        }
        for kind in used {
            for sp in [Split::Train, Split::Test] {
                if !ds.values[idx(sp)].contains_key(&kind) {
                    return Err(format!(
                        "slots.txt: no {sp:?} values for slot {}",
                        kind.name()
                    ));
                }
            }
        }
        Ok(ds)
    }

    /// Split the training templates into `k` folds for cross-validation:
    /// returns `(rest, fold)` datasets, where `fold`'s training templates are
    /// every `k`-th template of each intent and `rest` has the others.
    pub fn fold(&self, k: usize, i: usize) -> (Dataset, Dataset) {
        let (mut rest, mut fold) = (self.clone(), self.clone());
        for (r, f) in rest.templates[0]
            .iter_mut()
            .zip(fold.templates[0].iter_mut())
        {
            let all = std::mem::take(r);
            f.clear();
            for (j, t) in all.into_iter().enumerate() {
                if j % k == i { f.push(t) } else { r.push(t) }
            }
        }
        rest.templates[0].retain(|g| !g.is_empty());
        fold.templates[0].retain(|g| !g.is_empty());
        (rest, fold)
    }

    /// All values of a slot (both splits), words joined with spaces.
    /// E.g. the app names, to build a `Lexicon` for training.
    pub fn values(&self, kind: SlotKind) -> Vec<String> {
        self.values
            .iter()
            .filter_map(|m| m.get(&kind))
            .flatten()
            .map(|v| v.join(" "))
            .collect()
    }

    /// Number of templates in a split.
    pub fn template_count(&self, split: Split) -> usize {
        self.templates[idx(split)].iter().map(Vec::len).sum()
    }

    /// Generate `n` examples from `phrasings` templates with `values` slot values.
    /// `noise` adds occasional typos to non-slot words (for training).
    pub fn generate(
        &self,
        phrasings: Split,
        values: Split,
        n: usize,
        rng: &mut Rng,
        noise: bool,
    ) -> Vec<Example> {
        let groups = &self.templates[idx(phrasings)];
        (0..n)
            .map(|_| {
                let group = &groups[rng.below(groups.len())];
                let t = rng.pick(group);
                let mut ex = Example {
                    words: Vec::new(),
                    tags: Vec::new(),
                    intent: t.intent,
                };
                for tok in &t.toks {
                    self.expand(tok, values, rng, noise, &mut ex);
                }
                ex
            })
            .collect()
    }

    fn expand(&self, tok: &Tok, values: Split, rng: &mut Rng, noise: bool, ex: &mut Example) {
        match tok {
            Tok::Word(w) => push_word(w, noise, rng, ex),
            Tok::List(name) => {
                for w in rng.pick(&self.lists[name]) {
                    push_word(w, noise, rng, ex);
                }
            }
            Tok::Slot(kind) => {
                let value = rng.pick(&self.values[idx(values)][kind]).clone();
                // `[a] {DUR}` + `a minute` would give `a a minute`: drop the template's article
                let article = |w: &str| w == "a" || w == "an";
                if value.first().is_some_and(|w| article(w))
                    && ex.words.last().is_some_and(|w| article(w))
                    && ex.tags.last() == Some(&Tag::O)
                {
                    ex.words.pop();
                    ex.tags.pop();
                }
                for (i, w) in value.iter().enumerate() {
                    // word dropout: in training, sometimes swap a free-text word for a
                    // made-up one, so the model learns the structure, not the vocabulary
                    let free = matches!(kind, SlotKind::Text | SlotKind::Query | SlotKind::App);
                    let w = if noise && free && rng.chance(0.05) {
                        pseudo_word(rng)
                    } else {
                        w.clone()
                    };
                    ex.words.push(fill_number(&w, rng));
                    ex.tags
                        .push(if i == 0 { Tag::B(*kind) } else { Tag::I(*kind) });
                }
            }
            Tok::Opt(inner) => {
                if rng.chance(0.5) {
                    for t in inner {
                        self.expand(t, values, rng, noise, ex);
                    }
                }
            }
        }
    }
}

/// Parse a template body into tokens; `[ … ]` may span several words.
fn parse_body(
    body: &str,
    intent: Option<Intent>,
    lists: &HashMap<String, Vec<Vec<String>>>,
) -> Result<Vec<Tok>, String> {
    let mut out = Vec::new();
    let mut group: Option<Vec<Tok>> = None;
    for word in body.split_whitespace() {
        let (word, opens) = match word.strip_prefix('[') {
            Some(w) => (w, true),
            None => (word, false),
        };
        let (word, closes) = match word.strip_suffix(']') {
            Some(w) => (w, true),
            None => (word, false),
        };
        if opens {
            if group.is_some() {
                return Err("nested `[` is not supported".into());
            }
            group = Some(Vec::new());
        }
        let tok = parse_word(word, intent, lists)?;
        match group.as_mut() {
            Some(g) => g.push(tok),
            None => out.push(tok),
        }
        if closes {
            let g = group.take().ok_or("`]` without `[`")?;
            out.push(Tok::Opt(g));
        }
    }
    if group.is_some() {
        return Err("`[` without `]`".into());
    }
    Ok(out)
}

fn parse_word(
    word: &str,
    intent: Option<Intent>,
    lists: &HashMap<String, Vec<Vec<String>>>,
) -> Result<Tok, String> {
    if let Some(name) = word.strip_prefix('@') {
        if !lists.contains_key(name) {
            return Err(format!("unknown list `@{name}` (define it above)"));
        }
        return Ok(Tok::List(name.to_string()));
    }
    if let Some(name) = word.strip_prefix('{').and_then(|w| w.strip_suffix('}')) {
        let kind = SlotKind::from_name(name).ok_or(format!("unknown slot `{{{name}}}`"))?;
        return match intent {
            None => Err("`none` templates can't have slots".into()),
            Some(i) if !i.slots().contains(&kind) => {
                Err(format!("slot {name} is not allowed for `{}`", i.name()))
            }
            _ => Ok(Tok::Slot(kind)),
        };
    }
    if word.contains(['[', ']', '{', '}', '@']) {
        return Err(format!("stray syntax character in `{word}`"));
    }
    Ok(Tok::Word(word.to_string()))
}

fn collect_slots(toks: &[Tok], out: &mut Vec<SlotKind>) {
    for t in toks {
        match t {
            Tok::Slot(k) if !out.contains(k) => out.push(*k),
            Tok::Opt(inner) => collect_slots(inner, out),
            _ => {}
        }
    }
}

/// A non-slot word, with an occasional swapped-letters typo.
fn push_word(w: &str, noise: bool, rng: &mut Rng, ex: &mut Example) {
    let mut word = w.to_string();
    if noise && word.len() > 3 && word.is_ascii() && rng.chance(0.04) {
        let mut b = word.into_bytes();
        let i = rng.below(b.len() - 1);
        b.swap(i, i + 1);
        word = String::from_utf8(b).unwrap();
    }
    ex.words.push(word);
    ex.tags.push(Tag::O);
}

/// A random lowercase pseudo-word of 3–8 letters.
fn pseudo_word(rng: &mut Rng) -> String {
    let len = 3 + rng.below(6);
    (0..len)
        .map(|_| (b'a' + rng.below(26) as u8) as char)
        .collect()
}

/// Replace `#` with a random number 1–90: `#m` → `25m`.
fn fill_number(w: &str, rng: &mut Rng) -> String {
    if w.contains('#') {
        w.replace('#', &(1 + rng.below(90)).to_string())
    } else {
        w.to_string()
    }
}

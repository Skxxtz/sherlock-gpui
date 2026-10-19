//! The model: one weight row per intent class and per tag, scoring,
//! constrained tagging, and the weights file format.

use crate::features::{self, BITS, BUCKETS, Lexicon, Query};
use crate::{Intent, SlotKind, Tag};

/// Intent classes: every `Intent`, plus "not a command" as the last class.
pub const CLASSES: usize = Intent::ALL.len() + 1;

/// Class index for an intent (`None` = not a command).
pub fn class_of(intent: Option<Intent>) -> usize {
    match intent {
        Some(i) => Intent::ALL.iter().position(|x| *x == i).unwrap(),
        None => CLASSES - 1,
    }
}

/// Inverse of `class_of`.
pub fn intent_of(class: usize) -> Option<Intent> {
    Intent::ALL.get(class).copied()
}

/// What the model predicts for one query.
#[derive(Debug, Clone, PartialEq)]
pub struct Prediction {
    /// Best intent; `None` = not a command.
    pub intent: Option<Intent>,
    /// Softmax probability of that intent, 0..1.
    pub confidence: f32,
    /// One tag per word.
    pub tags: Vec<Tag>,
}

/// Weight tables, `rows × BUCKETS` each.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// Intent weights, `CLASSES` rows.
    pub intent: Vec<f32>,
    /// Tag weights, `Tag::COUNT` rows.
    pub tags: Vec<f32>,
    /// Softmax temperature for `confidence`, fitted during training so
    /// that e.g. 0.8 means "right about 80% of the time".
    pub temperature: f32,
}

impl Default for Model {
    fn default() -> Self {
        Model::new()
    }
}

impl Model {
    /// All weights zero.
    pub fn new() -> Model {
        Model {
            intent: vec![0.0; CLASSES * BUCKETS],
            tags: vec![0.0; Tag::COUNT * BUCKETS],
            temperature: 1.0,
        }
    }

    /// Score of every intent class for sentence features `f`.
    pub fn intent_scores(&self, f: &[usize]) -> [f32; CLASSES] {
        let mut s = [0.0; CLASSES];
        for (c, score) in s.iter_mut().enumerate() {
            let row = &self.intent[c * BUCKETS..][..BUCKETS];
            *score = f.iter().map(|&i| row[i]).sum();
        }
        s
    }

    /// Score of `tag` for word features `f`.
    pub fn tag_score(&self, tag: Tag, f: &[usize]) -> f32 {
        let row = &self.tags[tag.index() * BUCKETS..][..BUCKETS];
        f.iter().map(|&i| row[i]).sum()
    }

    /// Best tag after `prev` that `intent` allows.
    pub fn best_tag(&self, prev: Tag, intent: Option<Intent>, f: &[usize]) -> Tag {
        (0..Tag::COUNT)
            .map(Tag::from_index)
            .filter(|t| allowed(prev, *t, intent))
            .map(|t| (self.tag_score(t, f), t))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .map_or(Tag::O, |(_, t)| t)
    }

    /// Intent first, then the best tag sequence under the intent's constraints.
    pub fn predict(&self, words: &[String], lex: &Lexicon) -> Prediction {
        let q = Query::new(words, lex);
        let mut f = Vec::with_capacity(32);
        features::sentence(&q, &mut f);
        let scores = self.intent_scores(&f);
        let (best, confidence) = softmax_best(&scores, self.temperature);
        let intent = intent_of(best);
        Prediction {
            intent,
            confidence,
            tags: self.extract_query(&q, words, intent),
        }
    }

    /// Slot tags for a given intent (skips intent classification).
    /// Used to measure extraction on its own, with the correct intent.
    pub fn extract(&self, words: &[String], lex: &Lexicon, intent: Option<Intent>) -> Vec<Tag> {
        self.extract_query(&Query::new(words, lex), words, intent)
    }

    fn extract_query(&self, q: &Query, words: &[String], intent: Option<Intent>) -> Vec<Tag> {
        let mut tags = self.viterbi(q, intent);
        trim_connectors(words, &mut tags);
        tags
    }

    /// Highest-scoring tag sequence for `words` (Viterbi), instead of
    /// deciding each word greedily: a later word can change an earlier
    /// choice, e.g. where a slot ends.
    pub fn viterbi(&self, q: &Query, intent: Option<Intent>) -> Vec<Tag> {
        self.viterbi_scored(q, intent).0
    }

    /// `viterbi` plus the score of the returned sequence.
    pub fn viterbi_scored(&self, q: &Query, intent: Option<Intent>) -> (Vec<Tag>, f32) {
        let n = q.len();
        if n == 0 {
            return (Vec::new(), 0.0);
        }
        const T: usize = Tag::COUNT;
        let mut best = vec![[f32::NEG_INFINITY; T]; n]; // best score ending in tag t at word i
        let mut back = vec![[0u8; T]; n]; // predecessor on that best path
        let (mut fs, mut fp) = (Vec::with_capacity(24), Vec::with_capacity(5));

        for i in 0..n {
            features::token_static(q, i, intent, &mut fs);
            let base: [f32; T] = std::array::from_fn(|t| self.tag_score(Tag::from_index(t), &fs));
            let prevs = if i == 0 { 0..1 } else { 0..T };
            for p in prevs {
                let from = if i == 0 { 0.0 } else { best[i - 1][p] };
                if from == f32::NEG_INFINITY {
                    continue;
                }
                let prev = Tag::from_index(p);
                features::token_prev(q, i, prev, intent, &mut fp);
                let punct = features::is_punct(&q.words[i]);
                for (c, &b) in base.iter().enumerate() {
                    let cand = Tag::from_index(c);
                    // punctuation is never part of a slot
                    if !allowed(prev, cand, intent) || (punct && cand != Tag::O) {
                        continue;
                    }
                    let s = from + b + self.tag_score(cand, &fp);
                    if s > best[i][c] {
                        best[i][c] = s;
                        back[i][c] = p as u8;
                    }
                }
            }
        }

        // follow the back-pointers from the best final tag
        let mut t = (0..T)
            .max_by(|a, b| best[n - 1][*a].total_cmp(&best[n - 1][*b]))
            .unwrap();
        let score = best[n - 1][t];
        let mut tags = vec![Tag::O; n];
        for i in (0..n).rev() {
            tags[i] = Tag::from_index(t);
            t = back[i][t] as usize;
        }
        (tags, score)
    }

    /// Serialise: header (magic, sizes, schema names, temperature), then the non-zero
    /// weights of each table as `(index: u32, value: f32)` pairs.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(BITS as u8);
        let schema = schema_string();
        out.extend_from_slice(&(schema.len() as u16).to_le_bytes());
        out.extend_from_slice(schema.as_bytes());
        out.extend_from_slice(&self.temperature.to_le_bytes());
        for table in [&self.intent, &self.tags] {
            let nz: Vec<(u32, f32)> = table
                .iter()
                .enumerate()
                .filter(|(_, v)| **v != 0.0)
                .map(|(i, v)| (i as u32, *v))
                .collect();
            out.extend_from_slice(&(nz.len() as u32).to_le_bytes());
            for (i, v) in nz {
                out.extend_from_slice(&i.to_le_bytes());
                out.extend_from_slice(&v.to_le_bytes());
            }
        }
        out
    }

    /// Inverse of `to_bytes`. Fails if the file was trained for a
    /// different schema or feature size (then: retrain).
    pub fn from_bytes(b: &[u8]) -> Result<Model, String> {
        if b.is_empty() {
            return Err("model file is empty; run parley-train".into());
        }
        let mut r = Reader { b, at: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a parley model".into());
        }
        if r.u8()? != VERSION {
            return Err("model file is from another parley version; retrain".into());
        }
        if r.u8()? as u32 != BITS {
            return Err("model was trained with a different feature size; retrain".into());
        }
        let n = r.u16()? as usize;
        if r.take(n)? != schema_string().as_bytes() {
            return Err("model was trained for different intents/slots; retrain".into());
        }
        let mut m = Model::new();
        m.temperature = f32::from_le_bytes(r.take(4)?.try_into().unwrap());
        for table in [&mut m.intent, &mut m.tags] {
            for _ in 0..r.u32()? {
                let i = r.u32()? as usize;
                let v = f32::from_le_bytes(r.take(4)?.try_into().unwrap());
                *table.get_mut(i).ok_or("weight index out of range")? = v;
            }
        }
        Ok(m)
    }
}

/// Connector words never start a slot: `and then ring` → `ring`.
const CONNECTORS: &[&str] = &["then", "and", "to", "so", "also"];
/// Polite filler never ends a slot: `ring please` → `ring`.
const FILLERS: &[&str] = &["please", "now", "thanks", "pls"];

/// Cleanup rules the model gets wrong when a slot value is unfamiliar:
/// no leading connectors, adjacent same-kind slots merge, no trailing fillers.
pub fn trim_connectors(words: &[String], tags: &mut [Tag]) {
    for i in 0..tags.len() {
        if let Tag::B(k) = tags[i]
            && CONNECTORS.contains(&words[i].as_str())
            && tags.get(i + 1) == Some(&Tag::I(k))
        {
            tags[i] = Tag::O;
            tags[i + 1] = Tag::B(k);
        }
    }
    // two slots of the same kind side by side are one slot: `30 %`
    for i in 1..tags.len() {
        if let (Tag::B(k), Tag::B(p) | Tag::I(p)) = (tags[i], tags[i - 1])
            && k == p
        {
            tags[i] = Tag::I(k);
        }
    }
    for i in (1..tags.len()).rev() {
        let ends_slot = tags.get(i + 1).is_none_or(|t| !matches!(t, Tag::I(_)));
        if matches!(tags[i], Tag::I(_)) && ends_slot && FILLERS.contains(&words[i].as_str()) {
            tags[i] = Tag::O;
        }
    }
}

/// BIO + intent constraints: `I-X` only after `B-X`/`I-X`, and only slots
/// the intent may have. Non-commands get no slots at all.
pub fn allowed(prev: Tag, cand: Tag, intent: Option<Intent>) -> bool {
    let slot_ok = |k: SlotKind| intent.is_some_and(|i| i.slots().contains(&k));
    match cand {
        Tag::O => true,
        Tag::B(k) => slot_ok(k),
        Tag::I(k) => slot_ok(k) && matches!(prev, Tag::B(p) | Tag::I(p) if p == k),
    }
}

/// Index and softmax probability (at temperature `t`) of the best score.
pub fn softmax_best(s: &[f32; CLASSES], t: f32) -> (usize, f32) {
    let (best, max) = s
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, v)| (i, *v))
        .unwrap();
    let sum: f32 = s.iter().map(|v| ((v - max) / t).exp()).sum();
    (best, 1.0 / sum)
}

const MAGIC: &[u8] = b"PRLY";
const VERSION: u8 = 2;

/// Intent and slot names in model row order; must match to load a file.
fn schema_string() -> String {
    let i: Vec<_> = Intent::ALL.iter().map(|x| x.name()).collect();
    let s: Vec<_> = SlotKind::ALL.iter().map(|x| x.name()).collect();
    format!("{};{}", i.join(","), s.join(","))
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        let s = self
            .b
            .get(self.at..self.at + n)
            .ok_or("model file is truncated")?;
        self.at += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, String> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, String> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
}

//! The model: one weight row per intent class and per tag, scoring,
//! constrained tagging, and the weights file format.

use crate::features::{self, BITS, BUCKETS, Lexicon};
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

    /// Intent first, then tags left to right under the intent's constraints.
    pub fn predict(&self, words: &[String], lex: &Lexicon) -> Prediction {
        let mut f = Vec::with_capacity(32);
        features::sentence(words, lex, &mut f);
        let scores = self.intent_scores(&f);
        let (best, confidence) = softmax_best(&scores);
        let intent = intent_of(best);

        let mut tags = Vec::with_capacity(words.len());
        let mut prev = Tag::O;
        for i in 0..words.len() {
            features::token(words, i, prev, intent, lex, &mut f);
            let t = self.best_tag(prev, intent, &f);
            tags.push(t);
            prev = t;
        }
        Prediction {
            intent,
            confidence,
            tags,
        }
    }

    /// Serialise: header (magic, sizes, schema names), then the non-zero
    /// weights of each table as `(index: u32, value: f32)` pairs.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(MAGIC);
        out.push(VERSION);
        out.push(BITS as u8);
        let schema = schema_string();
        out.extend_from_slice(&(schema.len() as u16).to_le_bytes());
        out.extend_from_slice(schema.as_bytes());
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
        let mut r = Reader { b, at: 0 };
        if r.take(4)? != MAGIC {
            return Err("not a parley model".into());
        }
        if r.u8()? != VERSION {
            return Err("unsupported model version".into());
        }
        if r.u8()? as u32 != BITS {
            return Err("model was trained with a different feature size; retrain".into());
        }
        let n = r.u16()? as usize;
        if r.take(n)? != schema_string().as_bytes() {
            return Err("model was trained for different intents/slots; retrain".into());
        }
        let mut m = Model::new();
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

/// Index and softmax probability of the best score.
fn softmax_best(s: &[f32; CLASSES]) -> (usize, f32) {
    let (best, max) = s
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.total_cmp(b.1))
        .map(|(i, v)| (i, *v))
        .unwrap();
    let sum: f32 = s.iter().map(|v| (v - max).exp()).sum();
    (best, 1.0 / sum)
}

const MAGIC: &[u8] = b"PRLY";
const VERSION: u8 = 1;

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

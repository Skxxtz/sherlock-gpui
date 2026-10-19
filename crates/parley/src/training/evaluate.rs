//! Accuracy of a model on labelled examples.

use super::corpus::Example;
use crate::features::Lexicon;
use crate::model::Model;
use crate::{Intent, Tag};

/// Accuracy figures, all 0..1.
#[derive(Debug, Clone, Copy)]
pub struct Report {
    /// Share of examples with the right intent.
    pub intent: f64,
    /// F1 over slots (kind and exact word span must match).
    pub slot_f1: f64,
    /// Share of examples where intent and all slots are right.
    pub exact: f64,
}

/// Slots as `(start, end, tag index of B)`.
fn spans(tags: &[Tag]) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < tags.len() {
        if let Tag::B(k) = tags[i] {
            let mut j = i + 1;
            while j < tags.len() && tags[j] == Tag::I(k) {
                j += 1;
            }
            out.push((i, j, tags[i].index()));
            i = j;
        } else {
            i += 1;
        }
    }
    out
}

/// Evaluate `model` on `examples`.
pub fn evaluate(model: &Model, examples: &[Example], lex: &Lexicon) -> Report {
    let (mut intent_ok, mut exact, mut tp, mut fp, mut fn_) = (0, 0, 0, 0, 0);
    for ex in examples {
        let p = model.predict(&ex.words, lex);
        let (gold, pred) = (spans(&ex.tags), spans(&p.tags));
        let intent_right = p.intent == ex.intent;
        intent_ok += intent_right as usize;
        exact += (intent_right && gold == pred) as usize;
        tp += pred.iter().filter(|s| gold.contains(s)).count();
        fp += pred.iter().filter(|s| !gold.contains(s)).count();
        fn_ += gold.iter().filter(|s| !pred.contains(s)).count();
    }
    let n = examples.len().max(1) as f64;
    let denom = (2 * tp + fp + fn_).max(1) as f64;
    Report {
        intent: intent_ok as f64 / n,
        slot_f1: 2.0 * tp as f64 / denom,
        exact: exact as f64 / n,
    }
}

/// `timer DUR="20m" ACTION="notify me"` – for printing predictions.
pub fn render(words: &[String], tags: &[Tag], intent: Option<Intent>) -> String {
    let mut out = intent.map_or("none", |i| i.name()).to_string();
    for (s, e, _) in spans(tags) {
        let Tag::B(k) = tags[s] else { unreachable!() };
        out.push_str(&format!(" {}=\"{}\"", k.name(), words[s..e].join(" ")));
    }
    out
}

/// Examples the model gets wrong, as `(query, expected, predicted)`.
pub fn errors(model: &Model, examples: &[Example], lex: &Lexicon) -> Vec<(String, String, String)> {
    examples
        .iter()
        .filter_map(|ex| {
            let p = model.predict(&ex.words, lex);
            let gold = render(&ex.words, &ex.tags, ex.intent);
            let pred = render(&ex.words, &p.tags, p.intent);
            (gold != pred).then(|| (ex.words.join(" "), gold, pred))
        })
        .collect()
}

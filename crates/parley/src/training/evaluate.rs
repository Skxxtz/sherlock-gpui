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

/// `evaluate` per gold intent (`None` = not a command), in schema order.
pub fn by_intent(
    model: &Model,
    examples: &[Example],
    lex: &Lexicon,
) -> Vec<(Option<Intent>, usize, Report)> {
    Intent::ALL
        .iter()
        .map(|i| Some(*i))
        .chain([None])
        .map(|i| {
            let subset: Vec<Example> = examples.iter().filter(|e| e.intent == i).cloned().collect();
            (i, subset.len(), evaluate(model, &subset, lex))
        })
        .collect()
}

/// What matters in a launcher, all 0..1.
#[derive(Debug, Clone, Copy)]
pub struct Launcher {
    /// Of the commands shown, how many were fully right.
    pub precision: f64,
    /// Of the real commands, how many were shown fully right.
    pub coverage: f64,
    /// Of the non-commands, how many wrongly got a command shown.
    pub false_positive: f64,
}

/// Measure a show/don't-show `gate` on `examples` (with gold `none`s mixed in).
pub fn launcher(
    model: &Model,
    examples: &[Example],
    lex: &Lexicon,
    gate: impl Fn(&crate::model::Prediction, &[String]) -> bool,
) -> Launcher {
    let (mut shown, mut right, mut commands, mut non, mut fp) = (0, 0, 0, 0, 0);
    for ex in examples {
        let p = model.predict(&ex.words, lex);
        let show = gate(&p, &ex.words);
        match ex.intent {
            Some(_) => commands += 1,
            None => {
                non += 1;
                fp += show as usize;
            }
        }
        if show {
            shown += 1;
            right += (render(&ex.words, &ex.tags, ex.intent)
                == render(&ex.words, &p.tags, p.intent)) as usize;
        }
    }
    let r = |a: usize, b: usize| a as f64 / b.max(1) as f64;
    Launcher {
        precision: r(right, shown),
        coverage: r(right, commands),
        false_positive: r(fp, non),
    }
}

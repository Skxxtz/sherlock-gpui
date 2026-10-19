//! The gate between "the model thinks this is a command" and "show it":
//! confidence, plus a check that the intent's essential slot is present and
//! plausible. Most false positives on ordinary input (`terminal`, `sleep`)
//! have no such slot, so they never reach the user.

use crate::features::{Lexicon, recognised};
use crate::model::Prediction;
use crate::{Intent, MIN_CONFIDENCE, SlotKind, Tag};

/// Slots of a tag sequence as `(kind, words joined by spaces)`.
pub fn slots(words: &[String], tags: &[Tag]) -> Vec<(SlotKind, String)> {
    let mut out: Vec<(SlotKind, String)> = Vec::new();
    for (w, t) in words.iter().zip(tags) {
        match t {
            Tag::O => {}
            Tag::B(k) => out.push((*k, w.clone())),
            Tag::I(_) => {
                if let Some(last) = out.last_mut() {
                    last.1.push(' ');
                    last.1.push_str(w);
                }
            }
        }
    }
    out
}

/// Does the intent have the slot it can't do without, and does it look real?
///
/// | intent | needs |
/// |---|---|
/// | timer  | a duration the recogniser also accepts |
/// | remind | text, plus a duration or time the recogniser accepts |
/// | volume | a level: recognised percent or a number 0–100 |
/// | open   | an app; every word known, if the caller gave a lexicon |
/// | search | a non-empty query |
pub fn plausible(intent: Intent, words: &[String], tags: &[Tag], lex: &Lexicon) -> bool {
    let slots = slots(words, tags);
    let any =
        |kind: SlotKind, ok: &dyn Fn(&str) -> bool| slots.iter().any(|(k, t)| *k == kind && ok(t));
    let recognised_as =
        |kind: SlotKind| move |t: &str| recognised(t).iter().any(|(k, _)| *k == kind);
    let level = |t: &str| t.parse::<u32>().is_ok_and(|n| n <= 100);
    match intent {
        Intent::Timer => any(SlotKind::Duration, &recognised_as(SlotKind::Duration)),
        Intent::Remind => {
            any(SlotKind::Text, &|t| !t.is_empty())
                && (any(SlotKind::Duration, &recognised_as(SlotKind::Duration))
                    || any(SlotKind::Time, &recognised_as(SlotKind::Time)))
        }
        Intent::Volume => any(SlotKind::Percent, &|t| {
            recognised_as(SlotKind::Percent)(t) || level(t)
        }),
        Intent::Open => any(SlotKind::App, &|t| {
            lex.is_empty() || t.split_whitespace().all(|w| lex.is_app(w))
        }),
        Intent::Search => any(SlotKind::Query, &|t| !t.is_empty()),
    }
}

/// Should this prediction be shown as a command?
pub fn accept(p: &Prediction, words: &[String], lex: &Lexicon) -> bool {
    match p.intent {
        Some(i) => p.confidence >= MIN_CONFIDENCE && plausible(i, words, &p.tags, lex),
        None => false,
    }
}

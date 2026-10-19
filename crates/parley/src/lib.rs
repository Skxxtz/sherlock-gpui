//! `parley` – turns short commands into an intent plus typed slots.
//!
//! `"new 20m timer then notify me"` → `Timer { duration: "20m", action: "notify me" }`
//!
//! A small averaged-perceptron model, trained offline on synthetic data
//! (`cargo run -p parley --features training --bin parley-train`) and
//! embedded into the library.

use std::sync::OnceLock;

pub mod features;
pub mod model;
mod schema;
#[cfg(feature = "training")]
pub mod training;

pub use features::Lexicon;
pub use schema::{Intent, SlotKind, Tag};

use model::Model;

/// Below this confidence, `parse` returns `None` (treat as normal search).
pub const MIN_CONFIDENCE: f32 = 0.6;

/// One extracted slot: its kind and the words it covers.
#[derive(Debug, Clone, PartialEq)]
pub struct Slot {
    /// What the words mean, e.g. `SlotKind::Duration`.
    pub kind: SlotKind,
    /// The words as typed, e.g. `"20m"`.
    pub text: String,
}

/// Result of parsing a query.
#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    /// What the user wants to do.
    pub intent: Intent,
    /// The details, in input order.
    pub slots: Vec<Slot>,
    /// 0..1; how sure the model is about the intent.
    pub confidence: f32,
}

impl Parsed {
    /// First slot of `kind`, e.g. the duration of a timer.
    pub fn slot(&self, kind: SlotKind) -> Option<&str> {
        self.slots
            .iter()
            .find(|s| s.kind == kind)
            .map(|s| s.text.as_str())
    }
}

/// The embedded model; `None` if the file is missing or stale.
fn model() -> Option<&'static Model> {
    static MODEL: OnceLock<Option<Model>> = OnceLock::new();
    MODEL
        .get_or_init(|| Model::from_bytes(include_bytes!("../model/parley.bin")).ok())
        .as_ref()
}

/// Parse a query. `None` if it isn't a command or the model is unsure.
pub fn parse(query: &str) -> Option<Parsed> {
    parse_with(query, &Lexicon::default())
}

/// Like `parse`, with known words (e.g. installed apps) to help the model.
pub fn parse_with(query: &str, lex: &Lexicon) -> Option<Parsed> {
    let raw: Vec<&str> = query.split_whitespace().collect();
    if raw.is_empty() {
        return None;
    }
    let p = model()?.predict(&features::words(query), lex);
    let intent = p.intent?;
    if p.confidence < MIN_CONFIDENCE {
        return None;
    }

    let mut slots: Vec<Slot> = Vec::new();
    for (word, tag) in raw.iter().zip(&p.tags) {
        match tag {
            Tag::O => {}
            Tag::B(kind) => slots.push(Slot {
                kind: *kind,
                text: word.to_string(),
            }),
            Tag::I(_) => {
                let last = slots.last_mut().expect("I-tag always follows B-tag");
                last.text.push(' ');
                last.text.push_str(word);
            }
        }
    }
    Some(Parsed {
        intent,
        slots,
        confidence: p.confidence,
    })
}

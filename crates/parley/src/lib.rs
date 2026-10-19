//! `parley` – turns short commands into an intent plus typed slots.
//!
//! `"new 20m timer then notify me"` → `Timer { duration: "20m", action: "notify me" }`
//!
//! A small averaged-perceptron model, trained offline on synthetic data
//! (`cargo run -p parley --features training --bin parley-train`) and embedded into the library.

mod schema;
#[cfg(feature = "training")]
pub mod training;

pub use schema::{Intent, SlotKind, Tag};

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
    /// 0..1; how sure the model is. Act only above a threshold.
    pub confidence: f32,
}

/// Parse a query. `None` if it isn't a command (or the model is unsure).
pub fn parse(query: &str) -> Option<Parsed> {
    let _ = query;
    None // model comes in a later step
}

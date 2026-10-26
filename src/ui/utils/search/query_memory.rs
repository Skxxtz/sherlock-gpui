mod memory;
mod persistent;
mod report;
#[cfg(test)]
mod tests;

pub use memory::{Boosts, today};
pub use persistent::{PersistentQueryMemory, QueryMemoryGuard};

/// Weight halves after this many days without use.
const HALF_LIFE_DAYS: f32 = 30.0;
/// Remembered queries; the weakest are evicted beyond this.
const MAX_QUERIES: usize = 500;
/// Remembered items per query.
const MAX_HITS: usize = 4;

/// Below this, a learned pick is treated as noise and ignored.
const MIN_P: f32 = 0.01;
/// Pseudo-count for unseen alternatives (additive smoothing).
const ALPHA: f32 = 1.0;

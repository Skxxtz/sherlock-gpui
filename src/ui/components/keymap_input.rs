mod input;
mod state;
#[cfg(test)]
mod tests;
mod utils;

pub use input::KeymapInput;

/// Event emitted when the chords changed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeymapEvent {
    Changed,
}

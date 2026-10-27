use gpui::{Keystroke, SharedString};

/// Modifier keys that alone dont finish a chord.
const MODIFIERS: [&str; 7] = [
    "control", "shift", "alt", "platform", "function", "super", "fn",
];

/// Editing state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct KeymapState {
    /// The collection of chords
    pub chords: Vec<SharedString>,
    /// Keyboard selection. `chords.len()` is the add button.
    pub cursor: usize,
    /// The line is waiting for keys.
    /// `Some(i)` replaces the chord `i`
    /// `Some(chords.len())` adds a chord.
    pub recording: Option<usize>,
}

impl KeymapState {
    /// Create a new state from binds
    #[inline(always)]
    pub fn new(chords: Vec<SharedString>) -> Self {
        Self {
            chords,
            ..Self::default()
        }
    }

    /// The position of the "Add binding" line.
    #[inline(always)]
    pub fn add_line(&self) -> usize {
        self.chords.len()
    }

    /// Sets the cursor to the given position.
    /// Safe for exceeding cursors, since it's clamped to `self.add_line`.
    #[inline(always)]
    pub fn set_cursor(&mut self, cursor: usize) {
        self.cursor = cursor.min(self.add_line());
    }

    /// Start a recording for `line`
    #[inline]
    pub fn record_at(&mut self, line: usize) {
        self.set_cursor(line);
        self.recording = Some(self.cursor);
    }

    /// Removes chord `i`. Returns whether anything changed.
    pub fn remove(&mut self, i: usize) -> bool {
        self.recording = None;
        if i >= self.chords.len() {
            return false;
        }
        self.chords.remove(i);
        self.set_cursor(self.cursor);
        true
    }

    /// One keystroke while recording. Lone modifiers wait for the rest of the chord, Esc cancels,
    /// backspace removes the chord being changed.
    /// Returns whether the chords changed.
    pub fn record(&mut self, keystroke: &Keystroke) -> bool {
        let Some(line) = self.recording else {
            return false;
        };

        if MODIFIERS.contains(&keystroke.key.as_str()) {
            return false;
        }
        self.recording = None;

        let plain = !keystroke.modifiers.modified();
        match keystroke.key.as_str() {
            "escape" if plain => false,
            "backspace" if plain => line < self.chords.len() && self.remove(line),
            _ => {
                let chord = SharedString::from(keystroke.unparse());
                if let Some(existing) = self.chords.iter().position(|c| *c == chord) {
                    self.set_cursor(existing);
                    return false;
                }
                if line < self.chords.len() {
                    self.chords[line] = chord; // change
                } else {
                    self.chords.push(chord); // add
                }
                self.set_cursor(line);
                true
            }
        }
    }
}

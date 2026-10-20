mod handle;
mod protocol;
mod watcher;

use std::sync::{Arc, RwLock};

use xkbcommon::xkb;

pub use handle::LayoutHandle;
pub use watcher::Keyboard;

/// Physical ISO-105 rows: (first evdev keycode, last, x offset of first key).
/// Units are key widths; row index is y.
const ROWS: [&[(u32, u32, f32)]; 4] = [
    &[(41, 41, 0.0), (2, 13, 1.0)],     // ^ 1 … 0 ß ´
    &[(16, 27, 1.5)],                   // q … p ü +
    &[(30, 40, 1.75), (43, 43, 12.75)], // a … l ö ä #
    &[(86, 86, 1.25), (44, 53, 2.25)],  // < y … -
];

/// Covers ASCII + Latin-1 (äöüß, é, …).
const TABLE: usize = 256;

pub type Pos = (f32, f32);

/// Character → physical key position for one layout group.
pub struct Layout([Option<Pos>; TABLE]);

impl Layout {
    #[inline]
    pub fn pos(&self, c: char) -> Option<Pos> {
        self.0.get(c as usize).copied().flatten()
    }

    fn build(km: &xkb::Keymap, group: xkb::LayoutIndex) -> Self {
        let mut t = [None; TABLE];
        for (y, row) in ROWS.iter().enumerate() {
            for &(first, last, x0) in *row {
                for code in first..=last {
                    let pos = (x0 + (code - first) as f32, y as f32);
                    // level 0 = unshifted; level 1 fills symbols only reachable via shift
                    for level in 0..2 {
                        for &sym in km.key_get_syms_by_level((code + 8).into(), group, level) {
                            let Some(c) = char::from_u32(xkb::keysym_to_utf32(sym)) else {
                                continue;
                            };
                            let c = c.to_lowercase().next().unwrap_or(c);
                            if let Some(slot) = t.get_mut(c as usize)
                                && slot.is_none()
                            {
                                *slot = Some(pos);
                            }
                        }
                    }
                }
            }
        }
        Self(t)
    }
}

type Shared = Arc<RwLock<Option<Arc<Layout>>>>;

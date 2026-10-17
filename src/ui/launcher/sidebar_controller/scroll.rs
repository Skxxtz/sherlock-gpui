use gpui::{Pixels, ScrollHandle, point, px};

/// Scroll state for the sidebar that remembers the user's position actoss items. 
/// Each item is scrolled as far toward that position as its content allows
#[derive(Default)]
pub struct SidebarScroll {
    pub handle: ScrollHandle,
    /// Desired y offset (< 0, GPUI scrolls with negative offsets)
    want: Pixels,
}

impl SidebarScroll {
    /// Keeps the sidebar scrolled to the remembered position, as far as the content allows.
    pub fn sync(&mut self) {
        let h = &self.handle;
        let cur = h.offset().y;
        let max = -h.max_offset().y;

        // What GPUI should have clamped our last request to. Anything else is a user scroll.
        let expected = self.want.max(max);
        if (cur - expected).abs() > px(0.5) {
            self.want = cur;
        }

        // Ask for the full position; GPUI clamps it to this frame's content during layout.
        h.set_offset(point(px(0.), self.want));
    }
}

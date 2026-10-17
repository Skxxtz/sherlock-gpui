use std::{
    collections::{HashSet, VecDeque},
    sync::Arc,
};

use gpui::{Global, RenderImage, SharedString, Window};

use crate::utils::preview::Preview;

const CAPACITY: usize = 32;

/// Small LRU of finished previews. `None` = failed
#[derive(Default)]
pub struct PreviewCache {
    /// Oldest at the front, most recent used at the back.
    entries: VecDeque<(SharedString, Option<Preview>)>,
    /// Paths currently being rendered in the background.
    pub pending: HashSet<SharedString>,
    /// Evicted images waiting for a `Window` to free their atlas slots.
    to_free: Vec<Arc<RenderImage>>,
}
impl Global for PreviewCache {}

impl PreviewCache {
    /// Cached result for `path`, marking it as recently used.
    /// `Some(None)` means the render failed; `None` means not cached.
    pub fn get(&mut self, path: &SharedString) -> Option<Option<Preview>> {
        let pos = self.entries.iter().position(|(p, _)| p == path)?;
        let entry = self.entries.remove(pos)?;
        let hit = entry.1.clone();
        self.entries.push_back(entry);
        Some(hit)
    }

    /// Stores a finished render and evicts the oldest entries past `CAPACITY`.
    pub fn insert(&mut self, path: SharedString, preview: Option<Preview>) {
        self.pending.remove(&path);
        if let Some(pos) = self.entries.iter().position(|(p, _)| *p == path)
            && let Some((_, old)) = self.entries.remove(pos)
        {
            self.retire(old);
        }
        self.entries.push_back((path, preview));
        while self.entries.len() > CAPACITY {
            if let Some((_, old)) = self.entries.pop_front() {
                self.retire(old);
            }
        }
    }

    /// Frees GPU atlas slots for evicted images. Call where a `Window` is available.
    pub fn flush(&mut self, window: &mut Window) {
        for img in self.to_free.drain(..) {
            let _ = window.drop_image(img);
        }
    }

    /// Drops everything. Call when the launcher hides.
    pub fn clear(&mut self, window: &mut Window) {
        self.pending.clear();
        for (_, old) in std::mem::take(&mut self.entries) {
            self.retire(old);
        }
        self.flush(window);
        #[cfg(target_os = "linux")]
        unsafe {
            libc::malloc_trim(0);
        }
    }

    /// Queues an evicted image for atlas cleanup. Text needs none.
    fn retire(&mut self, preview: Option<Preview>) {
        if let Some(Preview::Image(img)) = preview {
            self.to_free.push(img);
        }
    }
}

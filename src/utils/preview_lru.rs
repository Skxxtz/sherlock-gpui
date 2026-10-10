use std::{collections::{HashSet, VecDeque}, sync::Arc};

use gpui::{Global, RenderImage, SharedString, Window};

/// Small LRU of finished previews. `None` = failed
#[derive(Default)]
pub struct PreviewCache {
    entries: VecDeque<(SharedString, Option<Arc<RenderImage>>)>,
    pub pending: HashSet<SharedString>,
    to_free: Vec<Arc<RenderImage>>, // evicted, waiting for a Window to free atlas slots
}
impl Global for PreviewCache {}

const CAP: usize = 8;

impl PreviewCache {
    pub fn get(&self, path: &SharedString) -> Option<Option<Arc<RenderImage>>> {
        self.entries.iter().find(|(p, _)| p == path).map(|(_, i)| i.clone())
    }

    pub fn insert(&mut self, path: SharedString, img: Option<Arc<RenderImage>>) {
        self.pending.remove(&path);
        self.entries.push_back((path, img));
        while self.entries.len() > CAP {
            if let Some((_, Some(old))) = self.entries.pop_front() { self.to_free.push(old); }
        }
    }

    /// Free GPU atlas slots for evicted images. Call where a Window is available.
    pub fn flush(&mut self, window: &mut Window) {
        for img in self.to_free.drain(..) { let _ = window.drop_image(img); }
    }

    /// Call when the launcher hides.
    pub fn clear(&mut self, window: &mut Window) {
        self.pending.clear();
        self.to_free.extend(self.entries.drain(..).filter_map(|(_, i)| i));
        self.flush(window);
    }
}


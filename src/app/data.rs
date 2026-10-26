use std::{
    collections::HashMap,
    ops::Deref,
    rc::Rc,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    launcher::{Launcher, LauncherId},
    ui::{model::ItemIdx, widgets::RenderableChild},
};

/// Global so revisions are unique across *all* data entites.
static NEXT_REVISION: AtomicU64 = AtomicU64::new(1);

/// Next unique data revision. Use for every `ModelData` implementation.
#[inline]
pub fn next_revision() -> u64 {
    NEXT_REVISION.fetch_add(1, Ordering::Relaxed)
}

/// Launcher daata shown by a model: `Rc<HashMap<LauncherId, Launcher>>` plus a revision that
/// changes on every mutation.
///
/// Read access defers to the map. All writes go through `Self::make_mut`, so cachges built from
/// this data (e.g. `SeachIndex`) can tell when they are stale by comparing revisions.
#[derive(Clone, Debug)]
pub struct LauncherData {
    map: Rc<HashMap<LauncherId, Launcher>>,
    revision: u64,
}

impl LauncherData {
    pub fn new(map: HashMap<LauncherId, Launcher>) -> Self {
        Self {
            map: Rc::new(map),
            revision: next_revision(),
        }
    }

    /// Identifies the exact state of the data. Changes on every `make_mut`
    #[inline(always)]
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// Mutable access to the map (clone-on-write, like `Rc::make_mut`).
    /// Bumps the revision, so dependent caches rebuild.
    pub fn make_mut(&mut self) -> &mut HashMap<LauncherId, Launcher> {
        self.revision = next_revision();
        Rc::make_mut(&mut self.map)
    }

    /// The item at `idx`, if it still exists.
    #[inline]
    pub fn child(&self, idx: ItemIdx) -> Option<&RenderableChild> {
        self.map.get(&idx.0).and_then(|l| l.children.get(idx.1))
    }
}

impl Default for LauncherData {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}

impl From<HashMap<LauncherId, Launcher>> for LauncherData {
    fn from(map: HashMap<LauncherId, Launcher>) -> Self {
        Self::new(map)
    }
}

impl Deref for LauncherData {
    type Target = HashMap<LauncherId, Launcher>;
    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_changes_on_mutation_only() {
        let mut d = LauncherData::default();
        let r0 = d.revision();
        let clone = d.clone();
        assert_eq!(clone.revision(), r0);

        d.make_mut();
        assert_ne!(d.revision(), r0);
        // clones keep their own revision
        assert_eq!(clone.revision(), r0);
    }

    #[test]
    fn revisions_unique_across_instances() {
        assert_ne!(
            LauncherData::default().revision(),
            LauncherData::default().revision()
        );
    }
}

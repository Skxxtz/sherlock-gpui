use std::sync::{Arc, PoisonError};

use super::{Output, Shared};
use crate::listeners::Subscription;

#[derive(Clone)]
pub struct OutputsHandle(pub(super) Arc<Shared>);

impl OutputsHandle {
    /// Current monitors. Populated once `WaylandWatcher::spawn()` returns.
    pub fn load(&self) -> Arc<[Output]> {
        self.0
            .outputs
            .read()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }

    /// Notify on hotplug / mode / scale changes until the `Subscription` drops.
    /// Runs on the Wayland thread; keep it cheap.
    pub fn subscribe(&self, f: impl Fn(&Self) + Send + Sync + 'static) -> Subscription {
        let h = self.clone();
        self.0.listeners.subscribe(move |()| f(&h))
    }
}

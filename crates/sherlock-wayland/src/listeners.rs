//! Drop-scoped listeners: `subscribe` returns a guard, dropping the guard
//! unregisters the callback. The registry only holds `Weak`s, so dropping
//! needs no lock and dead entries are pruned on the next notify.

use std::sync::{Arc, Mutex, Weak};

use smallvec::SmallVec;

type Callback<T> = dyn Fn(T) + Send + Sync;

pub struct Listeners<T>(Mutex<Vec<Weak<Callback<T>>>>);

impl<T> Default for Listeners<T> {
    fn default() -> Self {
        Self(Mutex::new(Vec::new()))
    }
}

/// Keeps a listener alive. Store it in the owner; dropping it unsubscribes.
#[must_use = "the listener is removed as soon as the Subscription is dropped"]
pub struct Subscription(#[allow(dead_code)] Box<dyn Send + Sync>);

impl std::fmt::Debug for Subscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Wayland subsctiption")
    }
}

impl<T: Copy + 'static> Listeners<T> {
    pub fn subscribe(&self, f: impl Fn(T) + Send + Sync + 'static) -> Subscription {
        let f: Arc<Callback<T>> = Arc::new(f);
        if let Ok(mut v) = self.0.lock() {
            v.push(Arc::downgrade(&f));
        }
        Subscription(Box::new(f))
    }

    pub fn notify(&self, value: T) {
        // Upgrade under the lock, call outside it, so a callback may
        // subscribe/unsubscribe without deadlocking.
        let alive: SmallVec<[Arc<Callback<T>>; 4]> = match self.0.lock() {
            Ok(mut v) => {
                v.retain(|w| w.strong_count() > 0);
                v.iter().filter_map(Weak::upgrade).collect()
            }
            Err(_) => return,
        };
        for f in alive {
            f(value);
        }
    }
}

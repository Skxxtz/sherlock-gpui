use std::io::Read;
use std::os::fd::AsFd;
use std::sync::Arc;

use wayland_client::Connection;

use super::{ClipboardChange, Selection, Shared};
use crate::listeners::Subscription;

#[derive(Clone)]
pub struct ClipboardHandle {
    pub(super) shared: Arc<Shared>,
    pub(super) conn: Connection,
}

impl ClipboardHandle {
    /// Notify on every selection change until the returned `Subscription` is
    /// dropped — store it in the owner so both die together.
    ///
    /// Runs on the Wayland thread: keep it cheap and don't call `read()` in
    /// it (that stalls the event loop until the source app delivers).
    pub fn subscribe(
        &self,
        f: impl Fn(&Self, ClipboardChange) + Send + Sync + 'static,
    ) -> Subscription {
        let h = self.clone();
        self.shared.listeners.subscribe(move |c| f(&h, c))
    }

    /// Like `subscribe`, but delivers the text itself — starting with the
    /// current clipboard. Reads happen in order on one worker thread (never the
    /// Wayland thread); bursts of changes are coalesced to the latest.
    /// Dropping the `Subscription` stops the worker.
    pub fn subscribe_text(&self, f: impl Fn(Option<Arc<str>>) + Send + 'static) -> Subscription {
        let (tx, rx) = std::sync::mpsc::channel::<ClipboardChange>();
        let handle = self.clone();

        let _ = std::thread::Builder::new()
            .name("clipboard-reader".into())
            .spawn(move || {
                // Deliver the current state first, so new subscribers aren't empty
                // until the next copy.
                f(handle.read());

                // Ends when the Subscription drops → its closure (and `tx`) drop.
                while let Ok(first) = rx.recv() {
                    let latest = rx.try_iter().last().unwrap_or(first);
                    f(match latest {
                        ClipboardChange::Text => handle.read(),
                        ClipboardChange::Empty => None,
                    });
                }
            });

        self.subscribe(move |_, c| {
            let _ = tx.send(c);
        })
    }

    /// Current clipboard text. First call per selection blocks until the
    /// source app has written the data; later calls return the cached `Arc`.
    pub fn read(&self) -> Option<Arc<str>> {
        let sel = self.shared.current.read().ok()?.clone()?;
        sel.text.get_or_init(|| self.fetch(&sel)).clone()
    }

    fn fetch(&self, sel: &Selection) -> Option<Arc<str>> {
        let (mut rx, tx) = std::io::pipe().ok()?;
        sel.offer.receive(sel.mime.clone(), tx.as_fd());
        drop(tx); // keep only the source's write end open → EOF when it's done
        self.conn.flush().ok()?;

        let mut buf = Vec::new();
        rx.read_to_end(&mut buf).ok()?;
        let text = String::from_utf8(buf).ok()?;
        Some(if text.contains('\r') {
            text.replace("\r\n", "\n").into()
        } else {
            text.into()
        })
    }
}

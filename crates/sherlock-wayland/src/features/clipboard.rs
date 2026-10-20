mod handle;
mod protocol;
mod watcher;

pub use handle::ClipboardHandle;
pub use watcher::Clipboard;

use std::sync::{Arc, Mutex, OnceLock, RwLock};

use wayland_protocols_wlr::data_control::v1::client::zwlr_data_control_offer_v1::ZwlrDataControlOfferV1;

use crate::listeners::Listeners;

/// What a change notification reports. Data itself is still fetched lazily.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardChange {
    /// New selection with text available via `ClipboardHandle::read()`.
    Text,
    /// Cleared, or the new selection has no text.
    Empty,
}

/// State shared between the Wayland thread and all handles.
#[derive(Default)]
struct Shared {
    current: RwLock<Option<Arc<Selection>>>,
    listeners: Listeners<ClipboardChange>,
}

/// Current selection. Dropping the last reference destroys the offer.
struct Selection {
    offer: ZwlrDataControlOfferV1,
    mime: String,
    text: OnceLock<Option<Arc<str>>>,
}

impl Drop for Selection {
    fn drop(&mut self) {
        self.offer.destroy();
    }
}

/// Per-offer user data: best text MIME type seen so far.
#[derive(Default)]
struct OfferMime(Mutex<Option<(u8, String)>>);

/// Preferred MIME types, best first. Other `text/*` (non-html) rank after these.
const PREFERRED: [&str; 3] = ["text/plain;charset=utf-8", "UTF8_STRING", "text/plain"];

fn rank(mime: &str) -> Option<u8> {
    match PREFERRED.iter().position(|p| *p == mime) {
        Some(i) => Some(i as u8),
        None if mime.starts_with("text/") && !mime.contains("html") => Some(PREFERRED.len() as u8),
        None => None,
    }
}

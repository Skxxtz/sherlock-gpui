mod handle;
mod protocol;
mod watcher;

pub use handle::OutputsHandle;
pub use watcher::Outputs;

use std::sync::{Arc, RwLock};

use wayland_client::protocol::wl_output::WlOutput;
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_v1::ZxdgOutputV1;

use crate::listeners::Listeners;

/// One monitor in the compositor's logical coordinate space.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Output {
    /// Connector name, e.g. `"DP-1"` (wl_output v4+).
    pub name: Option<Arc<str>>,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// Integer buffer scale; logical size already accounts for (fractional) scaling.
    pub scale: i32,
}

#[derive(Default)]
struct Shared {
    outputs: RwLock<Arc<[Output]>>,
    listeners: Listeners<()>,
}

/// Raw per-output state, assembled from several events until `done`.
struct Entry {
    global: u32,
    wl: WlOutput,
    xdg: Option<ZxdgOutputV1>,
    name: Option<Arc<str>>,
    position: Option<(i32, i32)>,
    mode: Option<(i32, i32)>,
    scale: i32,
    logical_pos: Option<(i32, i32)>,
    logical_size: Option<(i32, i32)>,
}

impl Entry {
    fn resolve(&self) -> Option<Output> {
        let (x, y) = self.logical_pos.or(self.position)?;
        // Without xdg-output, fall back to physical mode / integer scale.
        let (width, height) = self.logical_size.or_else(|| {
            self.mode
                .map(|(w, h)| (w / self.scale.max(1), h / self.scale.max(1)))
        })?;
        Some(Output {
            name: self.name.clone(),
            x,
            y,
            width,
            height,
            scale: self.scale,
        })
    }
}

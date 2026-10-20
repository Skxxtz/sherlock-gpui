#[cfg(feature = "clipboard")]
pub mod clipboard;
#[cfg(feature = "keyboard")]
pub mod keyboard;
#[cfg(feature = "outputs")]
pub mod outputs;

use enum_dispatch::enum_dispatch;
use wayland_client::{
    Connection, QueueHandle,
    protocol::{
        wl_registry::WlRegistry,
        wl_seat::{Capability, WlSeat},
    },
};

use super::state::WaylandState;

pub type Qh = QueueHandle<WaylandState>;

/// Lifecycle hooks; all optional.
#[enum_dispatch]
pub trait Hooks {
    /// Every global the compositor advertises; bind what you need.
    fn on_global(&mut self, _reg: &WlRegistry, _name: u32, _iface: &str, _ver: u32, _qh: &Qh) {}
    /// Seat capabilities known; all globals are bound by now.
    fn on_seat(&mut self, _seat: &WlSeat, _caps: Capability, _qh: &Qh) {}
    /// A global disappeared (e.g. monitor unplugged).
    fn on_global_remove(&mut self, _name: u32) {}
}

#[enum_dispatch(Hooks)]
pub enum Feature {
    #[cfg(feature = "clipboard")]
    Clipboard(clipboard::Clipboard),
    #[cfg(feature = "keyboard")]
    Keyboard(keyboard::Keyboard),
    #[cfg(feature = "outputs")]
    Outputs(outputs::Outputs),
}

pub trait FeatureKind: Into<Feature> {
    /// Cheap, thread-safe consumer handle.
    type Handle: Clone + Send + Sync;
    fn handle(&self, conn: &Connection) -> Self::Handle;
    /// Typed access for the feature's own `Dispatch` impls.
    fn get(f: &mut Feature) -> Option<&mut Self>;
}

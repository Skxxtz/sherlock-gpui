use std::sync::Arc;

use wayland_client::Connection;
use wayland_client::protocol::wl_keyboard::WlKeyboard;
use wayland_client::protocol::wl_seat::{Capability, WlSeat};

use super::handle::LayoutHandle;
use super::{Layout, Shared};
use crate::features::{Feature, FeatureKind, Hooks, Qh};

#[derive(Default)]
pub struct Keyboard {
    pub(super) active: Shared,
    pub(super) keyboard: Option<WlKeyboard>,
    pub(super) groups: Vec<Arc<Layout>>,
    pub(super) group: u32,
}

impl Keyboard {
    pub(super) fn publish(&self) {
        let l = self.groups.get(self.group as usize).or(self.groups.first());
        *self.active.write().unwrap() = l.cloned();
    }
}

impl Hooks for Keyboard {
    fn on_seat(&mut self, seat: &WlSeat, caps: Capability, qh: &Qh) {
        if caps.contains(Capability::Keyboard) && self.keyboard.is_none() {
            self.keyboard = Some(seat.get_keyboard(qh, ()));
        }
    }
}

impl FeatureKind for Keyboard {
    type Handle = LayoutHandle;

    fn handle(&self, _: &Connection) -> LayoutHandle {
        LayoutHandle(self.active.clone())
    }

    fn get(f: &mut Feature) -> Option<&mut Self> {
        match f {
            Feature::Keyboard(k) => Some(k),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

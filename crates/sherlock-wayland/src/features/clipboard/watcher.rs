use std::sync::Arc;

use wayland_client::{
    Connection,
    Proxy,
    protocol::{
        wl_registry::WlRegistry,
        wl_seat::{Capability, WlSeat},
    },
};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::ZwlrDataControlDeviceV1,
    zwlr_data_control_manager_v1::ZwlrDataControlManagerV1,
};

use super::{ClipboardChange, ClipboardHandle, Selection, Shared};
use crate::features::{Feature, FeatureKind, Hooks, Qh};

#[derive(Default)]
pub struct Clipboard {
    pub(super) shared: Arc<Shared>,
    pub(super) manager: Option<ZwlrDataControlManagerV1>,
    pub(super) device: Option<ZwlrDataControlDeviceV1>,
}

impl Clipboard {
    pub(super) fn set(&self, sel: Option<Arc<Selection>>) {
        let change = if sel.is_some() {
            ClipboardChange::Text
        } else {
            ClipboardChange::Empty
        };
        *self.shared.current.write().unwrap() = sel;
        self.shared.listeners.notify(change);
    }
}

impl Hooks for Clipboard {
    fn on_global(&mut self, reg: &WlRegistry, name: u32, iface: &str, ver: u32, qh: &Qh) {
        if iface == ZwlrDataControlManagerV1::interface().name {
            self.manager = Some(reg.bind(name, ver.min(2), qh, ()));
        }
    }

    fn on_seat(&mut self, seat: &WlSeat, _: Capability, qh: &Qh) {
        if self.device.is_none()
            && let Some(m) = &self.manager
        {
            self.device = Some(m.get_data_device(seat, qh, ()));
        }
    }
}

impl FeatureKind for Clipboard {
    type Handle = ClipboardHandle;

    fn handle(&self, conn: &Connection) -> ClipboardHandle {
        ClipboardHandle {
            shared: self.shared.clone(),
            conn: conn.clone(),
        }
    }

    fn get(f: &mut Feature) -> Option<&mut Self> {
        match f {
            Feature::Clipboard(c) => Some(c),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

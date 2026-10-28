use smallvec::SmallVec;
use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
    WEnum,
    protocol::{
        wl_registry::{self, WlRegistry},
        wl_seat::{self, WlSeat},
    },
};

use super::features::{Feature, FeatureKind, Hooks};

#[derive(Default)]
pub struct WaylandState {
    seat: Option<WlSeat>,
    pub(super) features: SmallVec<[Feature; 4]>,
}

impl WaylandState {
    pub fn get<T: FeatureKind>(&mut self) -> Option<&mut T> {
        self.features.iter_mut().find_map(T::get)
    }
}

impl Dispatch<WlRegistry, ()> for WaylandState {
    fn event(
        s: &mut Self,
        reg: &WlRegistry,
        ev: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        match ev {
            wl_registry::Event::Global {
                name,
                interface,
                version,
            } => {
                if interface == "wl_seat" && s.seat.is_none() {
                    s.seat = Some(reg.bind(name, version.min(7), qh, ()));
                }
                for f in &mut s.features {
                    f.on_global(reg, name, &interface, version, qh);
                }
            }
            wl_registry::Event::GlobalRemove { name } => {
                for f in &mut s.features {
                    f.on_global_remove(name);
                }
            }
            _ => {}
        }
    }
}

impl Dispatch<WlSeat, ()> for WaylandState {
    fn event(
        s: &mut Self,
        seat: &WlSeat,
        ev: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_seat::Event::Capabilities {
            capabilities: WEnum::Value(caps),
        } = ev
        {
            for f in &mut s.features {
                f.on_seat(seat, caps, qh);
            }
        }
    }
}

use std::sync::{Arc, OnceLock};

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, event_created_child};
use wayland_protocols_wlr::data_control::v1::client::{
    zwlr_data_control_device_v1::{self, ZwlrDataControlDeviceV1},
    zwlr_data_control_manager_v1::{self, ZwlrDataControlManagerV1},
    zwlr_data_control_offer_v1::{self, ZwlrDataControlOfferV1},
};

use super::{Clipboard, OfferMime, Selection, rank};
use crate::state::WaylandState;

impl Dispatch<ZwlrDataControlManagerV1, ()> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrDataControlManagerV1,
        _: zwlr_data_control_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<ZwlrDataControlDeviceV1, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ZwlrDataControlDeviceV1,
        ev: zwlr_data_control_device_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        use zwlr_data_control_device_v1::Event;
        let Some(clip) = state.get::<Clipboard>() else {
            return;
        };

        match ev {
            Event::Selection { id } => {
                let sel = id.and_then(|offer| {
                    let mime = offer.data::<OfferMime>()?.0.lock().ok()?.take();
                    match mime {
                        Some((_, mime)) => Some(Arc::new(Selection {
                            offer,
                            mime,
                            text: OnceLock::new(),
                        })),
                        None => {
                            offer.destroy(); // not text
                            None
                        }
                    }
                });
                clip.set(sel);
            }
            // Primary selection is unused; don't leak its offers.
            Event::PrimarySelection { id: Some(offer) } => offer.destroy(),
            Event::Finished => {
                clip.device = None;
                clip.set(None);
            }
            _ => {}
        }
    }

    event_created_child!(WaylandState, ZwlrDataControlDeviceV1, [
        zwlr_data_control_device_v1::EVT_DATA_OFFER_OPCODE => (ZwlrDataControlOfferV1, OfferMime::default())
    ]);
}

impl Dispatch<ZwlrDataControlOfferV1, OfferMime> for WaylandState {
    fn event(
        _: &mut Self,
        _: &ZwlrDataControlOfferV1,
        ev: zwlr_data_control_offer_v1::Event,
        data: &OfferMime,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let zwlr_data_control_offer_v1::Event::Offer { mime_type } = ev else {
            return;
        };
        let Some(r) = rank(&mime_type) else { return };
        if let Ok(mut best) = data.0.lock()
            && best.as_ref().is_none_or(|(b, _)| r < *b)
        {
            *best = Some((r, mime_type));
        }
    }
}

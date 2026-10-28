use std::sync::Arc;

use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
    WEnum,
    protocol::wl_keyboard::{self, KeymapFormat, WlKeyboard},
};
use xkbcommon::xkb;

use super::{Layout, watcher::Keyboard};
use crate::state::WaylandState;

impl Dispatch<WlKeyboard, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlKeyboard,
        ev: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(kb) = state.get::<Keyboard>() else {
            return;
        };

        match ev {
            wl_keyboard::Event::Keymap {
                format: WEnum::Value(KeymapFormat::XkbV1),
                fd,
                size,
            } => {
                let ctx = xkb::Context::new(xkb::CONTEXT_NO_FLAGS);
                // SAFETY: fd/size come straight from the compositor's keymap event.
                let km = unsafe {
                    xkb::Keymap::new_from_fd(
                        &ctx,
                        fd,
                        size as usize,
                        xkb::KEYMAP_FORMAT_TEXT_V1,
                        xkb::KEYMAP_COMPILE_NO_FLAGS,
                    )
                };
                let Ok(Some(km)) = km else { return };
                kb.groups = (0..km.num_layouts())
                    .map(|g| Arc::new(Layout::build(&km, g)))
                    .collect();
                kb.publish();
            }
            // Only sent while this client has keyboard focus (see note in wayland.rs).
            wl_keyboard::Event::Modifiers { group, .. } if group != kb.group => {
                kb.group = group;
                kb.publish();
            }
            _ => {}
        }
    }
}

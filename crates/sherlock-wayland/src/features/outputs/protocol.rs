use std::sync::Arc;

use wayland_client::{
    Connection,
    Dispatch,
    QueueHandle,
    delegate_noop,
    protocol::wl_output::{self, WlOutput},
};
use wayland_protocols::xdg::xdg_output::zv1::client::{
    zxdg_output_manager_v1::ZxdgOutputManagerV1,
    zxdg_output_v1::{self, ZxdgOutputV1},
};

use super::Outputs;
use crate::state::WaylandState;

delegate_noop!(WaylandState: ignore ZxdgOutputManagerV1);

/// User data = the output's registry global name.
impl Dispatch<WlOutput, u32> for WaylandState {
    fn event(
        state: &mut Self,
        _: &WlOutput,
        ev: wl_output::Event,
        global: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(outs) = state.get::<Outputs>() else {
            return;
        };

        if let wl_output::Event::Done = ev {
            outs.publish(); // atomic end of an update batch
            return;
        }
        let Some(e) = outs.entry(*global) else { return };
        match ev {
            wl_output::Event::Geometry { x, y, .. } => e.position = Some((x, y)),
            wl_output::Event::Mode {
                flags,
                width,
                height,
                ..
            } if flags
                .into_result()
                .is_ok_and(|f| f.contains(wl_output::Mode::Current)) =>
            {
                e.mode = Some((width, height));
            }
            wl_output::Event::Scale { factor } => e.scale = factor,
            wl_output::Event::Name { name } => e.name = Some(Arc::from(name)),
            _ => {}
        }
    }
}

impl Dispatch<ZxdgOutputV1, u32> for WaylandState {
    fn event(
        state: &mut Self,
        _: &ZxdgOutputV1,
        ev: zxdg_output_v1::Event,
        global: &u32,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let Some(outs) = state.get::<Outputs>() else {
            return;
        };

        // Pre-v3 managers end xdg updates with their own `done`.
        if let zxdg_output_v1::Event::Done = ev {
            outs.publish();
            return;
        }
        let Some(e) = outs.entry(*global) else { return };
        match ev {
            zxdg_output_v1::Event::LogicalPosition { x, y } => e.logical_pos = Some((x, y)),
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                e.logical_size = Some((width, height));
            }
            _ => {}
        }
    }
}

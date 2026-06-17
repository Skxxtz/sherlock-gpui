use gpui::{Bounds, Pixels, Point, Size, px};
use wayland_client::{
    Connection, Dispatch, QueueHandle, delegate_noop,
    protocol::{wl_compositor, wl_output, wl_registry, wl_surface},
};
use wayland_protocols::{
    wp::fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    xdg::xdg_output::zv1::client::{zxdg_output_manager_v1, zxdg_output_v1},
};

use std::sync::OnceLock;

static DISPLAY_BOUNDS: OnceLock<Vec<Bounds<Pixels>>> = OnceLock::new();

pub fn display_bounds() -> &'static [Bounds<Pixels>] {
    DISPLAY_BOUNDS.get_or_init(probe_display_bounds)
}

pub fn get_max_display_bounds() -> Option<Bounds<Pixels>> {
    let bounds = display_bounds();
    bounds
        .iter()
        .max_by(|a, b| {
            let area_a = f32::from(a.size.width) * f32::from(a.size.height);
            let area_b = f32::from(b.size.width) * f32::from(b.size.height);
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .copied()
}

#[derive(Debug, Default)]
struct OutputInfo {
    // width, height, hz
    mode: Option<(i32, i32, i32)>,
    scale: i32,
    fractional_scale: Option<f32>,
    position: Option<(i32, i32)>,
    logical_pos: Option<(i32, i32)>,
    logical_size: Option<(i32, i32)>,
}

struct AppState {
    outputs: Vec<(wl_output::WlOutput, OutputInfo)>,
    xdg_output_manager: Option<zxdg_output_manager_v1::ZxdgOutputManagerV1>,
    xdg_outputs: Vec<(zxdg_output_v1::ZxdgOutputV1, usize)>,
    pending_xdg_binds: bool,

    compositor: Option<wl_compositor::WlCompositor>,
    fractional_scale_manager: Option<WpFractionalScaleManagerV1>,

    probe_surfaces: Vec<wl_surface::WlSurface>,
}

impl AppState {
    fn new() -> Self {
        Self {
            outputs: Vec::new(),
            xdg_output_manager: None,
            xdg_outputs: Vec::new(),
            pending_xdg_binds: false,
            compositor: None,
            fractional_scale_manager: None,
            probe_surfaces: Vec::new(),
        }
    }
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _conn: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_output" => {
                    let output: wl_output::WlOutput = registry.bind(name, version.min(4), qh, ());
                    state.outputs.push((
                        output,
                        OutputInfo {
                            scale: 1,
                            ..Default::default()
                        },
                    ));
                }
                "zxdg_output_manager_v1" => {
                    let mgr: zxdg_output_manager_v1::ZxdgOutputManagerV1 =
                        registry.bind(name, version.min(3), qh, ());
                    state.xdg_output_manager = Some(mgr);
                    state.pending_xdg_binds = true;
                }
                "wl_compositor" => {
                    let comp: wl_compositor::WlCompositor =
                        registry.bind(name, version.min(5), qh, ());
                    state.compositor = Some(comp);
                }
                "wp_fractional_scale_manager_v1" => {
                    let mgr: WpFractionalScaleManagerV1 =
                        registry.bind(name, version.min(1), qh, ());
                    state.fractional_scale_manager = Some(mgr);
                }
                _ => {}
            }
        }
    }
}

impl Dispatch<wl_output::WlOutput, ()> for AppState {
    fn event(
        state: &mut Self,
        output: &wl_output::WlOutput,
        event: wl_output::Event,
        _: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let Some((_, info)) = state.outputs.iter_mut().find(|(o, _)| o == output) else {
            return;
        };

        match event {
            wl_output::Event::Geometry { x, y, .. } => {
                info.position = Some((x, y));
            }
            wl_output::Event::Mode {
                flags,
                width,
                height,
                refresh,
            } => {
                if flags
                    .into_result()
                    .map(|f| f.contains(wl_output::Mode::Current))
                    .unwrap_or(false)
                {
                    info.mode = Some((width, height, refresh));
                }
            }
            wl_output::Event::Scale { factor } => {
                info.scale = factor;
            }
            _ => {}
        }
    }
}

delegate_noop!(AppState: ignore wl_compositor::WlCompositor);
impl Dispatch<wl_surface::WlSurface, usize> for AppState {
    fn event(
        state: &mut Self,
        _surface: &wl_surface::WlSurface,
        event: wl_surface::Event,
        output_index: &usize,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_surface::Event::Enter { output } = event
            && let Some((wl_out, _)) = state.outputs.get(*output_index)
        {
            debug_assert_eq!(
                wl_out, &output,
                "probe surface entered an unexpected output"
            );
        }
    }
}

delegate_noop!(AppState: ignore zxdg_output_manager_v1::ZxdgOutputManagerV1);
impl Dispatch<zxdg_output_v1::ZxdgOutputV1, usize> for AppState {
    fn event(
        state: &mut Self,
        xdg_out: &zxdg_output_v1::ZxdgOutputV1,
        event: zxdg_output_v1::Event,
        _idx: &usize,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        let output_idx = match state.xdg_outputs.iter().find(|(xo, _)| xo == xdg_out) {
            Some((_, i)) => *i,
            None => return,
        };
        let Some((_, info)) = state.outputs.get_mut(output_idx) else {
            return;
        };

        match event {
            zxdg_output_v1::Event::LogicalPosition { x, y } => {
                info.logical_pos = Some((x, y));
            }
            zxdg_output_v1::Event::LogicalSize { width, height } => {
                info.logical_size = Some((width, height));
            }
            _ => {}
        }
    }
}

delegate_noop!(AppState: ignore WpFractionalScaleManagerV1);
impl Dispatch<WpFractionalScaleV1, usize> for AppState {
    fn event(
        state: &mut Self,
        _proxy: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        output_index: &usize,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event
            && let Some((_, info)) = state.outputs.get_mut(*output_index)
        {
            info.fractional_scale = Some(scale as f32 / 120.0);
        }
    }
}

fn probe_display_bounds() -> Vec<Bounds<Pixels>> {
    let conn = match Connection::connect_to_env() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: could not connect to Wayland display: {}", e);
            eprintln!("       Is $WAYLAND_DISPLAY set and the compositor running?");
            return Vec::new();
        }
    };

    let mut event_queue = conn.new_event_queue::<AppState>();
    let qh = event_queue.handle();
    conn.display().get_registry(&qh, ());

    let mut state = AppState::new();

    if event_queue.roundtrip(&mut state).is_err() {
        return Vec::new();
    }

    if state.pending_xdg_binds {
        if let Some(mgr) = &state.xdg_output_manager {
            let pairs: Vec<_> = state
                .outputs
                .iter()
                .enumerate()
                .map(|(i, (wl_out, _))| (i, mgr.get_xdg_output(wl_out, &qh, i)))
                .collect();
            state.xdg_outputs.reserve(pairs.len());
            for (idx, xdg_out) in pairs {
                state.xdg_outputs.push((xdg_out, idx));
            }
        }
        state.pending_xdg_binds = false;
    }

    if let (Some(compositor), Some(fs_mgr)) = (
        state.compositor.clone(),
        state.fractional_scale_manager.clone(),
    ) {
        let count = state.outputs.len();
        state.probe_surfaces.reserve(count);
        for idx in 0..count {
            let surface = compositor.create_surface(&qh, idx);
            fs_mgr.get_fractional_scale(&surface, &qh, idx);
            surface.commit();
            state.probe_surfaces.push(surface);
        }
    }

    for _ in 0..4 {
        if event_queue.roundtrip(&mut state).is_err() {
            break;
        }
    }

    let mut result = Vec::with_capacity(state.outputs.len());
    result.extend(state.outputs.iter().filter_map(|(_, info)| {
        let logical_pos = info.logical_pos.or(info.position).map(|(x, y)| Point {
            x: px(x as f32),
            y: px(y as f32),
        })?;
        let logical_size = info
            .logical_size
            .map(|(w, h)| Size::new(px(w as f32), px(h as f32)))?;
        Some(Bounds {
            origin: logical_pos,
            size: logical_size,
        })
    }));
    result
}

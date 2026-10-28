use std::sync::{Arc, PoisonError};

use wayland_client::{
    Connection,
    Proxy,
    protocol::{wl_output::WlOutput, wl_registry::WlRegistry},
};
use wayland_protocols::xdg::xdg_output::zv1::client::zxdg_output_manager_v1::ZxdgOutputManagerV1;

use super::{Entry, Output, OutputsHandle, Shared};
use crate::features::{Feature, FeatureKind, Hooks, Qh};

#[derive(Default)]
pub struct Outputs {
    pub(super) shared: Arc<Shared>,
    pub(super) manager: Option<ZxdgOutputManagerV1>,
    pub(super) entries: Vec<Entry>,
}

impl Outputs {
    pub(super) fn entry(&mut self, global: u32) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|e| e.global == global)
    }

    pub(super) fn publish(&self) {
        let list: Arc<[Output]> = self.entries.iter().filter_map(Entry::resolve).collect();
        let mut cur = self
            .shared
            .outputs
            .write()
            .unwrap_or_else(PoisonError::into_inner);
        if **cur == *list {
            return; // nothing visible changed
        }
        *cur = list;
        drop(cur);
        self.shared.listeners.notify(());
    }
}

impl Hooks for Outputs {
    fn on_global(&mut self, reg: &WlRegistry, name: u32, iface: &str, ver: u32, qh: &Qh) {
        match iface {
            "wl_output" => {
                let wl: WlOutput = reg.bind(name, ver.min(4), qh, name);
                let xdg = self
                    .manager
                    .as_ref()
                    .map(|m| m.get_xdg_output(&wl, qh, name));
                self.entries.push(Entry {
                    global: name,
                    wl,
                    xdg,
                    name: None,
                    position: None,
                    mode: None,
                    scale: 1,
                    logical_pos: None,
                    logical_size: None,
                });
            }
            "zxdg_output_manager_v1" => {
                let m: ZxdgOutputManagerV1 = reg.bind(name, ver.min(3), qh, ());
                // Outputs may have been announced before the manager.
                for e in self.entries.iter_mut().filter(|e| e.xdg.is_none()) {
                    e.xdg = Some(m.get_xdg_output(&e.wl, qh, e.global));
                }
                self.manager = Some(m);
            }
            _ => {}
        }
    }

    fn on_global_remove(&mut self, name: u32) {
        let Some(i) = self.entries.iter().position(|e| e.global == name) else {
            return;
        };
        let e = self.entries.swap_remove(i);
        if let Some(xdg) = e.xdg {
            xdg.destroy();
        }
        if e.wl.version() >= 3 {
            e.wl.release();
        }
        self.publish();
    }
}

impl FeatureKind for Outputs {
    type Handle = OutputsHandle;

    fn handle(&self, _: &Connection) -> OutputsHandle {
        OutputsHandle(self.shared.clone())
    }

    fn get(f: &mut Feature) -> Option<&mut Self> {
        match f {
            Feature::Outputs(o) => Some(o),
            #[allow(unreachable_patterns)]
            _ => None,
        }
    }
}

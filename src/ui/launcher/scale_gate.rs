use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use gpui::{Context, Task, Window};

use super::LauncherView;

/// Upper bound for the wait, for compositors that never send a scale.
const FALLBACK: Duration = Duration::from_millis(250);

/// Scale of the last settled window (`f32` bits); 0 = none yet.
static LAST_SCALE: AtomicU32 = AtomicU32::new(0);

#[derive(Default)]
pub struct ScaleGate {
    settled: bool,
    fallback: Option<Task<()>>,
}

impl ScaleGate {
    /// For a new window showing the same view.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl LauncherView {
    /// True once the window's scale is final.
    pub(super) fn scale_settled(&mut self, win: &Window, cx: &mut Context<Self>) -> bool {
        if self.scale_gate.settled {
            return true;
        }

        let scale = win.scale_factor();
        let last = f32::from_bits(LAST_SCALE.load(Ordering::Relaxed));
        if scale != 1.0 || last == 1.0 {
            LAST_SCALE.store(scale.to_bits(), Ordering::Relaxed);
            self.scale_gate.settled = true;
            self.scale_gate.fallback = None;
            return true;
        }

        if self.scale_gate.fallback.is_none() {
            self.scale_gate.fallback = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(FALLBACK).await;
                let _ = this.update(cx, |this, cx| {
                    if !this.scale_gate.settled {
                        LAST_SCALE.store(1.0f32.to_bits(), Ordering::Relaxed);
                        this.scale_gate.settled = true;
                        cx.notify();
                    }
                });
            }));
        }
        false
    }
}

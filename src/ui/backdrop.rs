use std::time::Duration;

use gpui::{
    Animation, AnimationExt, Bounds, IntoElement, Pixels, Render, Styled, WindowBounds, WindowKind,
    WindowOptions, black, div,
    layer_shell::{Layer, LayerShellOptions},
    point, px, size,
};

use crate::OUTPUTS;

pub struct Backdrop {
    alpha: f32,
    animation_duration: Option<u64>,
}

impl Backdrop {
    pub fn new(alpha: f32, animation_duration: Option<u64>) -> Self {
        Self {
            alpha,
            animation_duration,
        }
    }
    pub fn window_options() -> WindowOptions {
        WindowOptions {
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "sherlock".to_string(),
                layer: Layer::Overlay,
                exclusive_zone: Some(px(-1.)),
                ..Default::default()
            }),
            window_bounds: get_max_display_bounds().map(WindowBounds::Windowed),
            window_background: gpui::WindowBackgroundAppearance::Blurred,
            ..Default::default()
        }
    }
}

impl Render for Backdrop {
    fn render(
        &mut self,
        _: &mut gpui::Window,
        _: &mut gpui::Context<Self>,
    ) -> impl gpui::IntoElement {
        if let Some(animation_duration) = self.animation_duration {
            div()
                .size_full()
                .with_animation(
                    "backdrop-fade",
                    Animation::new(Duration::from_millis(animation_duration)),
                    {
                        let alpha = self.alpha;
                        move |this, t| this.bg(black().alpha(alpha * t))
                    },
                )
                .into_any_element()
        } else {
            div()
                .size_full()
                .bg(black().alpha(self.alpha))
                .into_any_element()
        }
    }
}

/// Largest monitor by area, in logical pixels
fn get_max_display_bounds() -> Option<Bounds<Pixels>> {
    OUTPUTS
        .get()?
        .load()
        .iter()
        .max_by_key(|o| o.width * o.height)
        .map(|o| Bounds {
            origin: point(px(o.x as f32), px(o.y as f32)),
            size: size(px(o.width as f32), px(o.height as f32)),
        })
}

use std::time::Duration;

use gpui::{
    Animation, AnimationExt, IntoElement, Render, Styled, WindowBounds, WindowKind, WindowOptions,
    black, div,
    layer_shell::{Layer, LayerShellOptions},
    px,
};

use crate::wayland::display::get_max_display_bounds;

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

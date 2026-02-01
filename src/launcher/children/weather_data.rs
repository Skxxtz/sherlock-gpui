use std::sync::Arc;

use gpui::{
    AnyElement, Image, ImageSource, IntoElement, ParentElement, Styled, div, img, linear_gradient,
    px,
};

use crate::{
    launcher::{Launcher, children::RenderableChildImpl, weather_launcher::WeatherData},
    utils::errors::SherlockError,
};

impl RenderableChildImpl for WeatherData {
    fn execute(&self, _launcher: &Arc<Launcher>, _keyword: &str) -> Result<bool, SherlockError> {
        Ok(false)
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        launcher.priority as f32
    }
    fn search(&self, _launcher: &Arc<Launcher>) -> String {
        String::new()
    }
    fn render(&self, _launcher: &Arc<Launcher>, _is_selected: bool) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .rounded_md()
            .bg({
                let (p1, p2) = self.css.background();
                linear_gradient(90., p1, p2)
            })
            .flex_col()
            .gap_5()
            .items_center()
            .text_size(px(12.0))
            .child(self.format_str.clone())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_5()
                    .child(if let Some(icon) = self.icon.as_ref() {
                        img(Arc::clone(&icon)).size(px(48.))
                    } else {
                        img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
                    })
                    .child(div().text_size(px(40.0)).child(self.temperature.clone())),
            )
            .into_any_element()
    }
}

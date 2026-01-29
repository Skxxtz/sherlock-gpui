use std::sync::Arc;

use gpui::{AnyElement, Image, ImageSource, IntoElement, ParentElement, Resource, Styled, div, img, linear_gradient, px};

use crate::{launcher::{children::RenderableChildImpl, weather_launcher::WeatherData}, utils::errors::SherlockError};

impl RenderableChildImpl for WeatherData {
    fn execute(&self, _keyword: &str) -> Result<bool, SherlockError> {
        Ok(false)
    }
    fn priority(&self) -> f32 {
        0.0
    }
    fn search(&self) -> String {
        String::new()
    }
    fn icon(&self) -> Option<String> {
        Some(self.icon.clone())
    }
    fn render(&self, icon: Option<Arc<std::path::Path>>, _is_selected: bool) -> AnyElement {
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
                    .gap_5()
                    .child(if let Some(icon) = icon {
                        img(ImageSource::Resource(Resource::Path(icon))).size(px(24.))
                    } else {
                        img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
                    })
                    .child(div().text_size(px(40.0)).child(self.temperature.clone())),
            )
            .into_any_element()
    }
}

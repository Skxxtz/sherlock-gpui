use std::sync::Arc;

use gpui::{AnyElement, Image, ImageSource, IntoElement, ParentElement, Resource, Styled, div, img, px, rgb};

use crate::{launcher::{ExecAttrs, children::RenderableChildImpl}, loader::utils::AppData, utils::errors::SherlockError};

impl RenderableChildImpl for AppData {
    fn render(&self, icon: Option<Arc<std::path::Path>>, is_selected: bool) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = icon {
                img(ImageSource::Resource(Resource::Path(icon))).size(px(24.))
            } else {
                img(ImageSource::Image(Arc::new(Image::empty()))).size(px(24.))
            })
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(if is_selected {
                                rgb(0xffffff)
                            } else {
                                rgb(0xcccccc)
                            })
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(self.name.clone()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_selected {
                                rgb(0x999999)
                            } else {
                                rgb(0x666666)
                            })
                            .children(
                                self.launcher
                                    .name
                                    .as_ref()
                                    .map(|name| div().child(name.clone())),
                            ),
                    ),
            )
            .into_any_element()
    }
    fn execute(&self, keyword: &str) -> Result<bool, SherlockError> {
        let attrs = ExecAttrs::from(self);
        self.launcher.execute(&attrs, keyword)
    }
    fn priority(&self) -> f32 {
        self.priority
    }
    fn search(&self) -> String {
        self.search_string.clone()
    }
    fn icon(&self) -> Option<String> {
        self.icon.clone()
    }
}

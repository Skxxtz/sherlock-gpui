use std::sync::Arc;

use gpui::{
    AnyElement, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div, img, px,
    rgb,
};

use crate::{
    launcher::{ExecMode, Launcher, children::RenderableChildImpl},
    loader::utils::AppData,
    utils::errors::SherlockError,
};

impl RenderableChildImpl for AppData {
    fn render(&self, launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement {
        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .child(if let Some(icon) = self.icon.as_ref() {
                img(Arc::clone(&icon)).size(px(24.)).into_any_element()
            } else {
                img(ImageSource::Image(Arc::new(Image::empty())))
                    .size(px(24.))
                    .into_any_element()
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
                            .children(
                                self.name
                                    .as_ref()
                                    .or(launcher.display_name.as_ref())
                                    .map(|name| div().child(name.clone())),
                            ),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(if is_selected {
                                rgb(0x999999)
                            } else {
                                rgb(0x666666)
                            })
                            .children(launcher.name.as_ref().map(|name| div().child(name.clone()))),
                    ),
            )
            .into_any_element()
    }
    fn execute(
        &self,
        launcher: &Arc<Launcher>,
        keyword: &str,
        variables: &[(SharedString, SharedString)],
    ) -> Result<bool, SherlockError> {
        let attrs = ExecMode::from_appdata(self, launcher);
        launcher.execute(&attrs, keyword, variables)
    }
    fn priority(&self, launcher: &Arc<Launcher>) -> f32 {
        self.priority.unwrap_or(launcher.priority as f32)
    }
    fn search(&self, _launcher: &Arc<Launcher>) -> String {
        self.search_string.clone()
    }
}

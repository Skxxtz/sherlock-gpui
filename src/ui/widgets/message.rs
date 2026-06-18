use std::sync::Arc;

use gpui::{
    AnyElement, App, FontWeight, InteractiveElement, IntoElement, MouseButton, ParentElement,
    Styled, div, prelude::FluentBuilder, px, relative,
};

use crate::{
    app::theme::ThemeData,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::utils::Priority,
    ui::{traits::RenderableChildImpl, widgets::Selection},
    utils::errors::{SherlockMessage, SherlockMessageLevel},
};

type DismissFunction = Arc<dyn Fn(&mut gpui::App, (usize, usize)) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct MessageChild {
    pub message: SherlockMessage,
    pub on_dismiss: Option<DismissFunction>,
    pub count: usize,
}

impl MessageChild {
    pub fn new(message: SherlockMessage) -> Self {
        Self {
            message,
            on_dismiss: None,
            count: 1,
        }
    }
    pub fn on_dismiss(
        mut self,
        f: impl Fn(&mut gpui::App, (usize, usize)) + Send + Sync + 'static,
    ) -> Self {
        self.on_dismiss = Some(std::sync::Arc::new(f));
        self
    }
}

impl<'a> RenderableChildImpl<'a> for MessageChild {
    const HANDLES_BORDERS: bool = true;
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        let text = match self.message.level {
            SherlockMessageLevel::Error => theme.color_err,
            SherlockMessageLevel::Warning => theme.color_warn,
            SherlockMessageLevel::Info => theme.color_info,
        };
        let bg = text.alpha(if selection.is_selected { 0.15 } else { 0.08 });
        let border = text.alpha(if selection.is_selected { 0.8 } else { 0.4 });

        let dismiss_btn = self.on_dismiss.as_ref().map(|f| {
            let f = f.clone();
            div()
                .id("dismiss")
                .absolute()
                .top(px(1.))
                .right(px(1.))
                .px(px(4.))
                .py(px(1.))
                .rounded_sm()
                .text_size(px(10.))
                .font_family(theme.font_family.clone())
                .text_color(text)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, move |_, _, cx| f(cx, selection.data_idx))
                .child("✕")
        });

        let count_badge = (self.count > 1).then(|| {
            div()
                .flex()
                .p_0p5()
                .items_center()
                .justify_center()
                .rounded_sm()
                .border_1()
                .text_color(text)
                .bg(border.opacity(0.5))
                .text_size(px(8.))
                .line_height(relative(1.))
                .font_weight(FontWeight::BOLD)
                .child(self.count.to_string())
        });

        div()
            .id("error-box")
            .group("error-box")
            .w_full()
            .px_4()
            .py_3()
            .rounded_md()
            .bg(bg)
            .border_1()
            .border_color(border)
            .text_size(px(12.0))
            .text_color(text)
            .font_family(theme.font_family.clone())
            .relative()
            .child(
                div()
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .items_center()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .text_size(px(13.))
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(text)
                                    .child(self.message.error_type.to_string())
                                    .when_some(count_badge, ParentElement::child),
                            )
                            .when_some(dismiss_btn, ParentElement::child),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(11.))
                            .line_height(px(12.))
                            .font_family(theme.font_family.clone())
                            .text_color(text)
                            .opacity(0.8)
                            .child(self.message.location.clone()),
                    )
                    .child(
                        div()
                            .w(relative(0.9))
                            .h(px(1.5))
                            .rounded_full()
                            .bg(text)
                            .opacity(0.25)
                            .mt(px(10.))
                            .mb(px(10.)),
                    )
                    .child(
                        div()
                            .mt_1()
                            .text_size(px(12.))
                            .line_height(px(16.))
                            .font_family(theme.monospace.clone())
                            .child(self.message.traceback.clone()),
                    ),
            )
            .into_any_element()
    }
    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        self.get_content(launcher, cx)
            .map(|content| ExecMode::Copy { content })
    }
    #[inline(always)]
    fn priority(&self, _launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new(1, 0)
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.message.traceback
    }
    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        Some(self.message.to_string())
    }
}

use crate::{
    app::theme::ThemeData,
    launcher::{
        LauncherConfig, theme_launcher::ThemePickerFunctions, utils::exec_mode::ExecMode,
        variant_type::InnerFunction,
    },
    loader::utils::Priority,
    ui::{
        utils::render::ListItemBorder,
        widgets::{RenderableChildImpl, Selection},
    },
};
use gpui::{
    AnyElement, App, FontWeight, IntoElement, ParentElement, SharedString, Styled, div,
    prelude::FluentBuilder, px,
};
use std::sync::Arc;

#[derive(Copy, Clone, Default, Debug)]
pub enum ThemeType {
    #[default]
    BuiltIn,
    Custom,
}
impl ThemeType {
    fn label(&self) -> &'static str {
        match self {
            ThemeType::Custom => "Custom Theme",
            ThemeType::BuiltIn => "Built-in Theme",
        }
    }
}

#[derive(Clone, Default, Debug)]
pub struct ThemeWidget {
    r#type: ThemeType,
    name: SharedString,
    theme: Arc<ThemeData>,
}

impl ThemeWidget {
    pub fn new<N: Into<SharedString>>(name: N, theme: Arc<ThemeData>, built_in: bool) -> Self {
        let t = if built_in {
            ThemeType::BuiltIn
        } else {
            ThemeType::Custom
        };

        Self {
            r#type: t,
            name: name.into(),
            theme,
        }
    }
}

impl<'a> RenderableChildImpl<'a> for ThemeWidget {
    const HANDLES_BORDERS: bool = true;
    fn render(
        &self,
        _launcher: &Arc<LauncherConfig>,
        selection: Selection,
        _query: &str,
        global_theme: Arc<ThemeData>,
        _cx: &mut App,
    ) -> AnyElement {
        let theme = self.theme.clone();
        let active = theme == global_theme;
        div()
            .rounded_md()
            .bg(theme.bg_app)
            .child(
                div()
                    .list_item_border(&theme, &selection)
                    .px_4()
                    .py_2()
                    .w_full()
                    .flex()
                    .gap_5()
                    .items_center()
                    .justify_start()
                    .child(
                        div()
                            .flex_col()
                            .justify_between()
                            .items_center()
                            .min_w_0()
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .font_family(theme.font_family.clone())
                                    .text_sm()
                                    .w_full()
                                    .text_color(theme.secondary_text)
                                    .when(selection.is_selected, |this| {
                                        this.text_color(theme.primary_text)
                                    })
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(self.name.clone())
                                    .when(active, |this| {
                                        this.child(
                                            div()
                                                .h(px(15.))
                                                .px(px(5.))
                                                .rounded_full()
                                                .bg(theme.bg_badge)
                                                .flex()
                                                .items_center()
                                                .justify_center()
                                                .child(
                                                    div()
                                                        .text_size(px(10.0))
                                                        .font_weight(FontWeight::MEDIUM)
                                                        .text_color(theme.secondary_text)
                                                        .child("active"),
                                                ),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .font_family(theme.font_family.clone())
                                    .text_xs()
                                    .w_full()
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .text_color(theme.secondary_text)
                                    .child(self.r#type.label()),
                            ),
                    ),
            )
            .into_any_element()
    }

    #[inline(always)]
    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<ExecMode> {
        Some(ExecMode::Inner {
            func: InnerFunction::Theme(ThemePickerFunctions::Pick {
                theme: self.theme.clone(),
            }),
            exit: launcher.exit,
        })
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut App) -> Option<String> {
        Some(self.name.to_string())
    }

    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }

    #[inline(always)]
    fn search(&'a self, _launcher: &Arc<LauncherConfig>) -> &'a str {
        &self.name
    }
}

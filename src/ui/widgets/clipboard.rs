use std::{rc::Rc, sync::Arc};

use gpui::{
    App, AppContext, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled, div,
    img, prelude::FluentBuilder, px, rgb, svg,
};

use crate::{
    app::theme::ThemeData,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode, variant_type::LauncherType},
    loader::{
        resolve_icon_path,
        utils::{ApplicationAction, Priority},
    },
    ui::{
        launcher::context_menu::ContextMenuAction,
        traits::RenderableChildImpl,
        utils::{
            async_update::{AsyncUpdate, AsyncUpdateEntity, Fetchable},
            selection::Selection,
        },
    },
    utils::{
        clipboard::get_clipboard,
        errors::SherlockMessage,
        intent::{Intent, IntentResult},
    },
};

#[derive(Clone)]
pub struct ClipData {
    pub actions: Option<Arc<[Arc<ContextMenuAction>]>>,
    result: Option<(Intent, IntentResult)>,
}

impl Fetchable for ClipData {
    type Error = SherlockMessage;
    async fn fetch(
        launcher: &Arc<LauncherConfig>,
        old: Option<Rc<Self>>,
    ) -> Result<Option<Rc<Self>>, Self::Error> {
        let LauncherType::Clipboard(clp) = &launcher.launcher_type else {
            unreachable!()
        };

        let content = match get_clipboard() {
            Some(c) if c.len() < 2048 => c,
            _ => return Ok(None),
        };

        let intent = Intent::parse(&content, &clp.capabilities);

        // early return if intents are the same
        if let Some(old_intent) = old.as_ref().and_then(|o| o.result.as_ref()).map(|(i, _)| i)
            && old_intent == &intent
        {
            return Ok(old);
        }

        let mut actions: Option<Arc<[Arc<ContextMenuAction>]>> = None;
        let r = match &intent {
            Intent::ColorConvert { .. } => intent.execute(),
            Intent::Conversion { .. } => intent.execute(),
            Intent::ColorDisplay { .. } => intent.execute(),
            Intent::Url { url } => {
                actions = Some(Arc::new([Arc::from(
                    ApplicationAction::new("create_bookmark", "Create Bookmark")
                        .icon_name("sherlock-bookmark"),
                )]));
                Some(IntentResult::String(url.into()))
            }
            _ => None,
        };

        let Some(r) = r else {
            return Ok(None);
        };

        let new = Self {
            result: Some((intent, r)),
            actions,
        };

        Ok(Some(Rc::new(new)))
    }
}

#[derive(Clone)]
pub struct ClipWidget {
    entity: AsyncUpdateEntity<ClipData>,
}

impl ClipWidget {
    pub fn new(cx: &mut impl AppContext) -> Self {
        Self {
            entity: AsyncUpdateEntity::<ClipData>::new(cx),
        }
    }
}

impl<'a> RenderableChildImpl<'a> for ClipWidget {
    fn render(
        &self,
        _launcher: &std::sync::Arc<crate::launcher::LauncherConfig>,
        selection: Selection,
        _query: &str,
        theme: Arc<ThemeData>,
        cx: &mut App,
    ) -> gpui::AnyElement {
        let Some((intent, result)) = self
            .entity
            .read(cx)
            .as_ref()
            .ok()
            .and_then(|data| data.as_ref())
            .and_then(|d| d.result.as_ref())
        else {
            return div().into_any_element();
        };

        match (intent, result) {
            (Intent::Url { url }, _) => url_show(url.clone(), selection, theme),
            (Intent::Conversion { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), selection, theme)
            }
            (Intent::ColorConvert { .. }, IntentResult::String(s)) => {
                calc_tile(s.clone(), selection, theme)
            }
            (Intent::ColorDisplay { .. }, IntentResult::Color(c)) => {
                color_show(*c, selection, theme)
            }
            _ => div().into_any_element(),
        }
    }
    #[inline(always)]
    fn search(&'a self, _launcher: &std::sync::Arc<crate::launcher::LauncherConfig>) -> &'a str {
        ""
    }
    #[inline(always)]
    fn build_exec(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<ExecMode> {
        if let Some((intent, res)) = self
            .entity
            .read(cx)
            .as_ref()
            .ok()
            .and_then(|data| data.as_ref())
            .and_then(|d| d.result.as_ref())
        {
            return match intent {
                Intent::Url { url } => Some(ExecMode::Web {
                    engine: None,
                    browser: None,
                    exec: Some(url.to_string()),
                }),
                _ => Some(ExecMode::Copy {
                    content: res.to_string(),
                }),
            };
        }

        None
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String> {
        let (intent, result) = self
            .entity
            .read(cx)
            .as_ref()
            .ok()
            .and_then(|data| data.as_ref())
            .and_then(|d| d.result.as_ref())?;

        match (intent, result) {
            (Intent::Url { url }, _) => Some(url.to_string()),
            (Intent::Conversion { .. }, IntentResult::String(s)) => Some(s.to_string()),
            (Intent::ColorConvert { .. }, IntentResult::String(s)) => Some(s.to_string()),
            (Intent::ColorDisplay { .. }, IntentResult::Color(c)) => Some(format!("#{:06X}", c)),
            _ => None,
        }
    }

    #[inline(always)]
    fn priority(&self, launcher: &Arc<LauncherConfig>) -> Priority {
        Priority::new_with_launcher(launcher, 0)
    }
    #[inline(always)]
    fn actions(
        &self,
        launcher: &Arc<LauncherConfig>,
        cx: &mut App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        if let Some(own_actions) = self
            .entity
            .read(cx)
            .as_ref()
            .ok()
            .and_then(|e| e.as_ref())
            .and_then(|d| d.actions.clone())
        {
            if let Some(extra_actions) = launcher.add_actions.as_ref() {
                if extra_actions.is_empty() {
                    return Some(own_actions);
                }

                let mut combined = Vec::with_capacity(own_actions.len() + extra_actions.len());

                combined.extend(own_actions.iter().cloned());
                combined.extend(extra_actions.iter().cloned());

                return Some(combined.into());
            }
            return Some(own_actions);
        }
        None
    }
    #[inline(always)]
    fn has_actions(&self, cx: &mut App) -> bool {
        self.entity.read(cx).as_ref().is_ok_and(|e| {
            e.as_ref()
                .and_then(|d| d.actions.as_ref())
                .is_some_and(|a| !a.is_empty())
        })
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, cx: &mut C) -> Option<bool> {
        Some(self.entity.read_with(cx, |this, _| {
            if let Ok(Some(clip)) = this.as_ref() {
                clip.result.is_some()
            } else {
                false
            }
        }))
    }
    #[inline(always)]
    fn update_async<C: AppContext>(&self, launcher: Arc<LauncherConfig>, cx: &mut C) {
        self.entity.update_async(launcher, cx);
    }
}

fn calc_tile(
    result: SharedString,
    selection: Selection,
    theme: Arc<ThemeData>,
) -> gpui::AnyElement {
    div()
        .px_4()
        .py_7()
        .size_full()
        .flex()
        .gap_5()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(px(24.0))
                .text_color(theme.secondary_text)
                .when(selection.is_selected, |this| {
                    this.text_color(theme.primary_text)
                })
                .overflow_hidden()
                .text_ellipsis()
                .whitespace_nowrap()
                .child(result),
        )
        .into_any_element()
}

fn color_show(result: u32, selection: Selection, theme: Arc<ThemeData>) -> gpui::AnyElement {
    div()
        .px_4()
        .py_2()
        .w_full()
        .flex()
        .gap_5()
        .items_center()
        .border_1()
        .rounded_md()
        .when(!selection.is_selected, |this| {
            this.border_color(theme.border_idle)
        })
        .child(div().size(px(24.)).rounded_full().bg(rgb(result)))
        .child(
            div()
                .flex_col()
                .justify_between()
                .items_center()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(format!("#{:06X}", result)),
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.secondary_text)
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}

fn url_show(url: SharedString, selection: Selection, theme: Arc<ThemeData>) -> gpui::AnyElement {
    div()
        .px_4()
        .py_2()
        .w_full()
        .flex()
        .gap_5()
        .items_center()
        .child(if let Some(icon) = resolve_icon_path("sherlock-link") {
            match icon {
                crate::loader::IconType::Png(png) => img(png).size(px(24.)).into_any_element(),
                crate::loader::IconType::Symbolic(sym) => svg()
                    .path(sym.to_string_lossy().into_owned())
                    .text_color(theme.secondary_text)
                    .size(px(24.))
                    .into_any_element(),
            }
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
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .when(selection.is_selected, |this| {
                            this.text_color(theme.primary_text)
                        })
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(url),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .child("From Clipboard"),
                ),
        )
        .into_any_element()
}

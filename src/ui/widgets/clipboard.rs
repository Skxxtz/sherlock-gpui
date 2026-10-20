use std::sync::Arc;

use futures::{StreamExt, channel::mpsc};
use gpui::{
    App, AppContext, Entity, Image, ImageSource, IntoElement, ParentElement, SharedString, Styled,
    div, img, prelude::FluentBuilder, px, rgb, svg,
};
use sherlock_derive::Hideable;

use crate::{
    CLIPBOARD,
    app::theme::ThemeData,
    launcher::{LauncherConfig, utils::exec_mode::ExecMode, variant_type::LauncherType},
    loader::{
        resolve_icon_path,
        utils::{ApplicationAction, Priority},
    },
    ui::{
        launcher::context_menu::ContextMenuAction, traits::RenderableChildImpl,
        utils::selection::Selection,
    },
    utils::intent::{Intent, IntentResult},
};

#[derive(Clone, Hideable)]
pub struct ClipWidget {
    entity: Entity<ClipboardContent>,
}

#[derive(Default, Debug)]
struct ClipboardContent {
    /// Subscription to wayland clipboard events. Provides text, starting with current one.
    _sub: Option<sherlock_wayland::Subscription>,
    /// Last copied text – used to debounce intent parsing.
    content: Option<Arc<str>>,
    /// Intent parsed for the copied text.
    result: Option<(Intent, IntentResult)>,
    /// Context menu actions connected to an intent.
    actions: Option<Arc<[Arc<ContextMenuAction>]>>,
}

impl ClipboardContent {
    /// Called on every new copied text. Parses new intent for the given string.
    fn with_content(&mut self, launcher: &Arc<LauncherConfig>, text: Option<Arc<str>>) {
        let LauncherType::Clipboard(clp) = &launcher.launcher_type else {
            unreachable!()
        };

        // early return if None or blank
        let Some(text) = text.filter(|t| !t.trim().is_empty()) else {
            self.content = None;
            self.actions = None;
            self.result = None;
            return;
        };

        // skip if unchanged
        if self.content.as_ref() == Some(&text) {
            return;
        }

        let intent = Intent::parse(&text, &clp.capabilities);
        self.content = Some(text);

        let (actions, result) = match &intent {
            Intent::ColorConvert { .. }
            | Intent::Conversion { .. }
            | Intent::ColorDisplay { .. } => (None, intent.execute()),
            Intent::Url { url } => (
                Some(Arc::new([Arc::from(
                    ApplicationAction::new("create_bookmark", "Create Bookmark")
                        .icon_name("sherlock-bookmark"),
                )]) as Arc<[_]>),
                Some(IntentResult::String(url.into())),
            ),
            _ => (None, None),
        };

        self.actions = actions;
        self.result = result.map(|r| (intent, r));
    }
}

impl ClipWidget {
    pub fn new(launcher: Arc<LauncherConfig>, cx: &mut App) -> Self {
        let entity = cx.new(|cx| {
            let (tx, mut rx) = mpsc::unbounded::<Option<Arc<str>>>();

            cx.spawn(async move |weak, cx| {
                while let Some(text) = rx.next().await {
                    let alive = weak.update(cx, |this: &mut ClipboardContent, cx| {
                        this.with_content(&launcher, text);
                        cx.notify();
                    });

                    if alive.is_err() {
                        break;
                    }
                }
            })
            // safe here since it drops onece tx drops, which drops when subscription drops.
            .detach();

            let _sub = CLIPBOARD.get().map(|clip| {
                clip.subscribe_text(move |text| {
                    let _ = tx.unbounded_send(text);
                })
            });

            ClipboardContent {
                _sub,
                ..Default::default()
            }
        });

        Self { entity }
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
        let Some((ref intent, ref result)) = self.entity.read(cx).result else {
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
        let (intent, result) = self.entity.read(cx).result.as_ref()?;
        match intent {
            Intent::Url { url } => Some(ExecMode::Web {
                engine: None,
                browser: None,
                exec: Some(url.to_string()),
            }),
            _ => Some(ExecMode::Copy {
                content: result.to_string(),
            }),
        }
    }

    #[inline(always)]
    fn get_content(&self, _launcher: &Arc<LauncherConfig>, cx: &mut App) -> Option<String> {
        let (intent, result) = self.entity.read(cx).result.as_ref()?;
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
        let own = self
            .entity
            .read(cx)
            .actions
            .as_ref()
            .filter(|a| !a.is_empty());
        let extra = launcher.add_actions.as_ref().filter(|a| !a.is_empty());

        match (own, extra) {
            (Some(own), Some(extra)) => Some(own.iter().chain(extra.iter()).cloned().collect()),
            (Some(a), None) | (None, Some(a)) => Some(a.clone()),
            (None, None) => None,
        }
    }
    #[inline(always)]
    fn has_actions(&self, cx: &mut App) -> bool {
        self.entity
            .read(cx)
            .actions
            .as_ref()
            .is_some_and(|a| !a.is_empty())
    }
    #[inline(always)]
    fn based_show<C: AppContext>(&self, _keyword: &str, cx: &mut C) -> Option<bool> {
        Some(
            self.entity
                .read_with(cx, |e, _| e.result.as_ref().is_some()),
        )
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

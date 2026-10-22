use std::sync::Arc;

use gpui::{
    AnyElement, IntoElement, ParentElement, SharedString, Styled, div, img, prelude::FluentBuilder,
    px,
};
use serde::{Deserialize, Serialize};

use crate::{
    launcher::{LauncherConfig, utils::exec_mode::ExecMode},
    loader::{
        IconType, resolve_icon_path,
        utils::{ApplicationAction, PriorityGuard, construct_search},
    },
    ui::{
        launcher::context_menu::{ContextMenuAction, quick_actions::Hideable},
        runtime_arguments::RuntimeArgument,
        traits::RenderableChildImpl,
        utils::render::substitute,
    },
    utils::config::{ConfigGuard, SherlockAlias},
};

pub mod app;
pub mod category;
pub mod command;
pub mod fixed;
pub mod web;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemMeta {
    pub name: SharedString,
    /// Prebuilt (and pre-processed: lowercase) once at loadtime.
    pub search: SharedString,
    pub icon: Option<IconType>,
    pub priority: PriorityGuard,
    pub actions: Arc<[Arc<ContextMenuAction>]>,
    pub vars: Arc<[RuntimeArgument]>,
}

impl ItemMeta {
    pub fn new(name: impl Into<SharedString>, launcher: &LauncherConfig) -> Self {
        let name: SharedString = name.into();
        Self {
            search: name.to_lowercase().into(),
            name,
            icon: None,
            priority: PriorityGuard::new_with_launcher(launcher, 0),
            actions: Arc::new([]),
            vars: Arc::new([]),
        }
    }
}

pub trait Payload: Clone + 'static {
    const HANDLES_BORDERS: bool = false;
    const HAS_SIDEBAR: bool = false;
    const HIDEABLE: bool = false;

    /// Fallback to print if nothing is configured on Payload implementer
    fn exec(&self, meta: &ItemMeta, _launcher: &Arc<LauncherConfig>) -> Option<ExecMode> {
        let config = ConfigGuard::read().ok();
        let field = config.as_ref().and_then(|c| c.runtime.field.as_deref());
        self.content(meta, field)
            .map(|content| ExecMode::Print { content })
    }

    /// Alias `exec` override. No-op for payloads without an exec (e.g. `FixedPayload`).
    fn set_exec(&mut self, _exec: SharedString) {}

    /// Second line under the name. Defaults to the launcher name.
    fn subtitle(&self, _meta: &ItemMeta, launcher: &LauncherConfig) -> Option<SharedString> {
        launcher.name.clone()
    }

    /// Value for the `--field <x>` output. Defaults to the name.
    fn content(&self, meta: &ItemMeta, _field: Option<&str>) -> Option<String> {
        Some(meta.name.to_string())
    }

    fn sidebar(&self, _meta: &ItemMeta, _launcher: &Arc<LauncherConfig>) -> Option<AnyElement> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListItem<P> {
    pub meta: ItemMeta,
    pub payload: P,
}

impl<P: Payload> RenderableChildImpl for ListItem<P> {
    const HANDLES_BORDERS: bool = P::HANDLES_BORDERS;

    fn render(
        &self,
        launcher: &Arc<LauncherConfig>,
        selection: crate::ui::utils::selection::Selection,
        query: &str,
        theme: Arc<crate::app::theme::ThemeData>,
        _cx: &mut gpui::App,
    ) -> AnyElement {
        const ICON: f32 = 24.;

        let icon = match self.meta.icon.as_ref().or(launcher.icon.as_ref()) {
            Some(icon) => match icon.svg() {
                Some(svg) => svg
                    .size(px(ICON))
                    .text_color(theme.primary_text)
                    .into_any_element(),
                None => img(icon.clone()).size(px(ICON)).into_any_element(),
            },
            None => div().size(px(ICON)).into_any_element(),
        };

        let name_color = if selection.is_selected {
            theme.primary_text
        } else {
            theme.secondary_text
        };

        div()
            .px_4()
            .py_2()
            .w_full()
            .flex()
            .gap_5()
            .items_center()
            .font_family(theme.font_family.clone())
            .child(icon)
            .child(
                div()
                    .flex_col()
                    .justify_between()
                    .min_w_0()
                    .child(
                        div()
                            .text_sm()
                            .text_color(name_color)
                            .overflow_hidden()
                            .text_ellipsis()
                            .whitespace_nowrap()
                            .child(substitute(self.meta.name.clone(), "keyword", query)),
                    )
                    .when_some(self.payload.subtitle(&self.meta, launcher), |this, sub| {
                        this.child(div().text_sm().text_color(theme.secondary_text).child(sub))
                    }),
            )
            .into_any_element()
    }

    fn build_exec(&self, launcher: &Arc<LauncherConfig>, _cx: &mut gpui::App) -> Option<ExecMode> {
        self.payload.exec(&self.meta, launcher)
    }

    fn get_content(&self, _launcher: &Arc<LauncherConfig>, _cx: &mut gpui::App) -> Option<String> {
        let config = ConfigGuard::read().ok();
        let field = config.as_ref().and_then(|c| c.runtime.field.as_deref());
        self.payload.content(&self.meta, field)
    }

    fn priority(&self, _launcher: &Arc<LauncherConfig>) -> crate::loader::utils::Priority {
        self.meta.priority.get()
    }

    fn search(&self, _launcher: &Arc<LauncherConfig>) -> &str {
        &self.meta.search
    }

    fn actions(
        &self,
        _launcher: &Arc<LauncherConfig>,
        _cx: &mut gpui::App,
    ) -> Option<Arc<[Arc<ContextMenuAction>]>> {
        (!self.meta.actions.is_empty()).then(|| self.meta.actions.clone())
    }

    fn has_actions(&self, _cx: &mut gpui::App) -> bool {
        !self.meta.actions.is_empty()
    }

    fn vars(&self, _cx: &mut gpui::App) -> Option<&[RuntimeArgument]> {
        Some(&self.meta.vars)
    }

    fn increment_count(&self) {
        self.meta.priority.increment_count();
    }

    fn sidebar(&self, launcher: &Arc<LauncherConfig>, _cx: &mut gpui::App) -> Option<AnyElement> {
        self.payload.sidebar(&self.meta, launcher)
    }

    fn has_sidebar(&self) -> bool {
        P::HAS_SIDEBAR
    }
}

impl<P: Payload> ListItem<P> {
    /// Applies an optional alias, then (re)builds the search string.
    /// `keywords` = the item's own keywords (desktop file / config), used if the alias has none.
    /// Set `meta.actions` to the item's own actions (e.g. desktop actions) before calling.
    pub fn apply_alias(
        &mut self,
        alias: Option<SherlockAlias>,
        keywords: Option<&str>,
        use_keywords: bool,
    ) {
        let mut alias_keywords = None;

        if let Some(alias) = alias {
            let m = &mut self.meta;
            if let Some(name) = alias.name {
                m.name = name.into();
            }
            if let Some(icon) = alias.icon {
                m.icon = resolve_icon_path(&icon);
            }
            if let Some(exec) = alias.exec {
                self.payload.set_exec(exec.into());
            }
            if let Some(vars) = alias.variables {
                m.vars = m.vars.iter().cloned().chain(vars).collect();
            }

            let icon = m.icon.clone();
            let fill = |mut a: ApplicationAction| -> Arc<ContextMenuAction> {
                if a.icon.is_none() {
                    a.icon = icon.clone();
                }
                a.into()
            };
            // `actions` replaces, otherwise `add_actions` appends.
            if let Some(actions) = alias.actions {
                m.actions = actions.into_iter().map(fill).collect();
            } else if let Some(add) = alias.add_actions {
                m.actions = m
                    .actions
                    .iter()
                    .cloned()
                    .chain(add.into_iter().map(fill))
                    .collect();
            }

            alias_keywords = alias.keywords;
        }

        let kw = alias_keywords.as_deref().or(keywords).unwrap_or("");
        self.meta.search = construct_search(Some(&self.meta.name), kw, use_keywords).into();
    }

    /// Item icon falls back to `fallback`; actions without an icon fall back to the item icon.
    /// Only reallocates the action slice if an action actually needs filling.
    pub fn inherit_icons(&mut self, fallback: Option<&IconType>) {
        if self.meta.icon.is_none() {
            self.meta.icon = fallback.cloned();
        }
        let Some(icon) = &self.meta.icon else { return };

        let needs_fill = self
            .meta
            .actions
            .iter()
            .any(|a| matches!(a.as_ref(), ContextMenuAction::App(act) if act.icon.is_none()));
        if !needs_fill {
            return;
        }

        self.meta.actions = self
            .meta
            .actions
            .iter()
            .map(|a| match a.as_ref() {
                ContextMenuAction::App(act) if act.icon.is_none() => {
                    Arc::new(ContextMenuAction::App(ApplicationAction {
                        icon: Some(icon.clone()),
                        ..act.clone()
                    }))
                }
                _ => a.clone(),
            })
            .collect();
    }
}

impl<P: Payload> Hideable for ListItem<P> {
    fn hide_id(&self) -> Option<String> {
        P::HIDEABLE.then(|| self.meta.name.to_string())
    }

    fn is_hideable(&self) -> bool {
        P::HIDEABLE
    }
}

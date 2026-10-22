use std::sync::Arc;

use gpui::{IntoElement, RenderOnce, SharedString, prelude::FluentBuilder};

use crate::{
    app::theme::ActiveTheme,
    launcher::LauncherConfig,
    ui::{
        launcher::{context_menu::ContextMenuAction, sidebar::wrapper::Sidebar},
        widgets::list_item::{ItemMeta, app::AppPayload},
    },
};

/// All fields are Arc-backed: constructing this is a few refcount bumps.
#[derive(IntoElement)]
pub struct AppDataSidebar {
    meta: ItemMeta,
    payload: AppPayload,
    launcher: Arc<LauncherConfig>,
}

impl AppDataSidebar {
    pub fn new(meta: &ItemMeta, payload: &AppPayload, launcher: &Arc<LauncherConfig>) -> Self {
        Self {
            meta: meta.clone(),
            payload: payload.clone(),
            launcher: Arc::clone(launcher),
        }
    }
}

impl RenderOnce for AppDataSidebar {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let Self {
            meta,
            payload,
            launcher,
        } = self;

        let badges = std::iter::once(SharedString::from("Application"))
            .chain(payload.terminal.then(|| "Terminal".into()));

        let actions = meta.actions.iter().filter_map(|a| match a.as_ref() {
            ContextMenuAction::App(act) => Some(act.name.clone()),
            _ => None,
        });

        Sidebar::new(theme)
            .header(meta.icon.as_ref(), meta.name.clone(), badges)
            .location(payload.desktop_file.to_string_lossy().into_owned())
            .separator()
            .section("Info")
            .row("Command", payload.exec.clone())
            .when(payload.original_name != meta.name, |s| {
                s.row("Original name", payload.original_name.clone())
            })
            .row(
                "Runs in",
                if payload.terminal {
                    "Terminal"
                } else {
                    "Window"
                },
            )
            .when(!meta.actions.is_empty(), |s| {
                actions.fold(s.separator().section("Actions"), |s, name| s.line(name))
            })
            .launcher_info(&launcher)
    }
}

/// Shared "Launcher" section — reuse in every payload's sidebar.
pub trait SidebarLauncherExt {
    fn launcher_info(self, cfg: &LauncherConfig) -> Self;
}

impl SidebarLauncherExt for Sidebar {
    fn launcher_info(self, cfg: &LauncherConfig) -> Self {
        let yes_no = |b: bool| if b { "yes" } else { "no" };
        let dash = |v: Option<String>| v.unwrap_or_else(|| "—".into());

        self.separator()
            .section("Launcher")
            .row("Name", dash(cfg.name.as_ref().map(ToString::to_string)))
            .row("Type", format!("{:?}", cfg.launcher_type.variant()))
            .row("Alias", dash(cfg.alias.clone()))
            .row("Priority", cfg.priority.to_string())
            .row("Limit", dash(cfg.limit.map(|l| l.to_string())))
            .row("Home", format!("{:?}", cfg.home))
            .row("On return", dash(cfg.on_return.clone()))
            .row("Exit after run", yes_no(cfg.exit))
            .row("Shortcut", yes_no(cfg.shortcut))
            .row("Spawn focus", yes_no(cfg.spawn_focus))
            .row("Sidebar default", yes_no(cfg.sidebar_default_on))
            .row(
                "Actions",
                format!(
                    "{} + {} added",
                    cfg.actions.as_ref().map_or(0, |a| a.len()),
                    cfg.add_actions.as_ref().map_or(0, |a| a.len()),
                ),
            )
    }
}

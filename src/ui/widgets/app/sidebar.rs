use std::sync::Arc;

use gpui::{IntoElement, RenderOnce, SharedString, Styled, prelude::FluentBuilder};

use crate::{
    app::theme::ActiveTheme,
    launcher::{LauncherConfig, app_launcher::app_data::AppData},
    loader::IconType,
    ui::launcher::{context_menu::ContextMenuAction, sidebar::wrapper::Sidebar},
};

#[derive(IntoElement)]
pub struct AppDataSidebar {
    name: SharedString,
    original_name: Option<SharedString>,
    icon: Option<IconType>,
    exec: Option<SharedString>,
    desktop_file: Option<SharedString>,
    terminal: bool,
    actions: Vec<SharedString>,
    launcher: Arc<LauncherConfig>,
}

impl AppDataSidebar {
    pub fn new(d: &AppData, launcher: &Arc<LauncherConfig>) -> Self {
        let name = d.name.clone().unwrap_or_else(|| "Unnamed".into());
        Self {
            // Only show the original name if an alias changed it
            original_name: d
                .original_name
                .as_deref()
                .filter(|o| *o != name.as_ref())
                .map(SharedString::from),
            name,
            icon: d.icon.clone(),
            exec: d.exec.as_deref().map(SharedString::from),
            desktop_file: d
                .desktop_file
                .as_ref()
                .map(|p| p.to_string_lossy().into_owned().into()),
            terminal: d.terminal,
            actions: d
                .actions
                .iter()
                .filter_map(|a| match a.as_ref() {
                    ContextMenuAction::App(act) => Some(SharedString::from(act.name.clone())),
                    _ => None,
                })
                .collect(),
            launcher: Arc::clone(launcher),
        }
    }
}

impl RenderOnce for AppDataSidebar {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let theme = cx.global::<ActiveTheme>().0.clone();
        let cfg = &self.launcher;

        let yes_no = |b: bool| if b { "yes" } else { "no" };
        let or_dash = |v: Option<String>| v.unwrap_or_else(|| "—".into());

        let badges = std::iter::once(SharedString::from("Application"))
            .chain(self.terminal.then(|| "Terminal".into()));

        Sidebar::new(theme)
            .min_h_full()
            .header(self.icon.as_ref(), self.name.clone(), badges)
            .when_some(self.desktop_file.clone(), |s, path| s.location(path))
            .separator()
            .section("Info")
            .when_some(self.exec.clone(), |s, exec| s.row("Command", exec))
            .when_some(self.original_name.clone(), |s, orig| {
                s.row("Original name", orig)
            })
            .row("Runs in", if self.terminal { "Terminal" } else { "Window" })
            .when(!self.actions.is_empty(), |s| {
                self.actions
                    .iter()
                    .fold(s.separator().section("Actions"), |s, name| {
                        s.line(name.clone())
                    })
            })
            .separator()
            .section("Launcher")
            .row("Name", or_dash(cfg.name.as_ref().map(|n| n.to_string())))
            .row("Type", format!("{:?}", cfg.launcher_type.variant()))
            .row("Alias", or_dash(cfg.alias.clone()))
            .row("Priority", cfg.priority.to_string())
            .row("Limit", or_dash(cfg.limit.map(|l| l.to_string())))
            .row("Home", format!("{:?}", cfg.home))
            .row("On return", or_dash(cfg.on_return.clone()))
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

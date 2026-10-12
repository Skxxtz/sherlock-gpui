use std::sync::Arc;

use gpui::{
    IntoElement, ParentElement, RenderOnce, SharedString, Styled, div, img, prelude::FluentBuilder,
    px,
};

use crate::{
    app::theme::ActiveTheme,
    launcher::{LauncherConfig, app_launcher::app_data::AppData},
    loader::IconType,
    ui::launcher::context_menu::ContextMenuAction,
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

        let row = |label: &'static str, value: SharedString| {
            div()
                .flex()
                .items_start()
                .justify_between()
                .gap_4()
                .py(px(4.))
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .text_color(theme.secondary_text)
                        .flex_shrink_0()
                        .child(label),
                )
                .child(
                    div()
                        .text_xs()
                        .font_family(theme.font_family.clone())
                        .font_weight(gpui::FontWeight::MEDIUM)
                        .text_color(theme.primary_text)
                        .text_right()
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(value),
                )
        };

        let section_label = |text: &'static str| {
            div()
                .text_xs()
                .font_family(theme.font_family.clone())
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.secondary_text)
                .pt_3()
                .pb_1()
                .child(SharedString::from(text))
        };

        let badge = |text: &'static str| {
            div()
                .px(px(6.))
                .py(px(1.))
                .rounded(px(4.))
                .bg(theme.bg_muted)
                .text_xs()
                .font_family(theme.font_family.clone())
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.secondary_text)
                .child(SharedString::from(text))
        };

        let separator = || div().h(px(1.)).w_full().bg(theme.border_selected).my_1();

        let cfg = &self.launcher;
        let yes_no = |b: bool| SharedString::from(if b { "yes" } else { "no" });
        let opt = |v: Option<String>| SharedString::from(v.unwrap_or_else(|| "—".into()));

        let launcher_section = div()
            .flex_col()
            .child(section_label("Launcher"))
            .child(row("Name", opt(cfg.name.as_ref().map(|n| n.to_string()))))
            .child(row(
                "Type",
                format!("{:?}", cfg.launcher_type.variant()).into(),
            ))
            .child(row("Alias", opt(cfg.alias.clone())))
            .child(row("Priority", cfg.priority.to_string().into()))
            .child(row("Limit", opt(cfg.limit.map(|l| l.to_string()))))
            .child(row("Home", format!("{:?}", cfg.home).into()))
            .child(row("On return", opt(cfg.on_return.clone())))
            .child(row("Exit after run", yes_no(cfg.exit)))
            .child(row("Shortcut", yes_no(cfg.shortcut)))
            .child(row("Spawn focus", yes_no(cfg.spawn_focus)))
            .child(row("Sidebar default", yes_no(cfg.sidebar_default_on)))
            .child(row(
                "Actions",
                format!(
                    "{} + {} added",
                    cfg.actions.as_ref().map_or(0, |a| a.len()),
                    cfg.add_actions.as_ref().map_or(0, |a| a.len()),
                )
                .into(),
            ));

        div()
            .min_h_full()
            // Header: icon, name, badges
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_3()
                    .pb(px(12.))
                    .child(
                        div()
                            .size(px(48.))
                            .rounded_lg()
                            .bg(theme.bg_muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .child(match &self.icon {
                                Some(icon) => match icon.svg() {
                                    Some(svg) => svg
                                        .size(px(32.))
                                        .text_color(theme.tertiary_text)
                                        .into_any_element(),
                                    None => img(icon.clone()).size(px(32.)).into_any_element(),
                                },
                                None => div().into_any_element(),
                            }),
                    )
                    .child(
                        div()
                            .flex_col()
                            .gap_1()
                            .min_w_0()
                            .child(
                                div()
                                    .text_sm()
                                    .font_family(theme.font_family.clone())
                                    .px(px(3.))
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .text_color(theme.primary_text)
                                    .overflow_hidden()
                                    .text_ellipsis()
                                    .whitespace_nowrap()
                                    .child(self.name.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_1()
                                    .child(badge("Application"))
                                    .when(self.terminal, |this| this.child(badge("Terminal"))),
                            ),
                    ),
            )
            // Desktop file path, like the file sidebar's location line
            .when_some(self.desktop_file.clone(), |this, path| {
                this.child(
                    div()
                        .mt_auto()
                        .pt(px(5.))
                        .text_xs()
                        .text_color(theme.secondary_text)
                        .overflow_hidden()
                        .text_ellipsis()
                        .whitespace_nowrap()
                        .child(path),
                )
            })
            .child(separator())
            .child(
                div()
                    .flex_col()
                    .child(section_label("Info"))
                    .when_some(self.exec.clone(), |this, exec| {
                        this.child(row("Command", exec))
                    })
                    .when_some(self.original_name.clone(), |this, orig| {
                        this.child(row("Original name", orig))
                    })
                    .child(row(
                        "Runs in",
                        if self.terminal { "Terminal" } else { "Window" }.into(),
                    )),
            )
            .when(!self.actions.is_empty(), |this| {
                this.child(separator()).child(
                    div().flex_col().child(section_label("Actions")).children(
                        self.actions.iter().map(|name| {
                            div()
                                .py(px(4.))
                                .text_xs()
                                .font_family(theme.font_family.clone())
                                .text_color(theme.primary_text)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(name.clone())
                        }),
                    ),
                )
            })
            .child(separator())
            .child(launcher_section)
    }
}

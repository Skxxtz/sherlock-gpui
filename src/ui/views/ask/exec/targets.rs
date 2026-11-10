use std::{path::Path, sync::Arc};

use gpui::{App, SharedString};

use crate::{
    app::LauncherEntityGlobal,
    launcher::{utils::exec_mode::ExecMode, variant_type::LauncherType},
    ui::{
        views::ask::content::actions::{AppEntry, Resolved},
        widgets::ChildKind,
    },
};

#[derive(Clone, Default)]
pub struct Targets {
    pub apps: Vec<AppEntry>,
    /// Changes whenever `apps` does, so what was resolved against them can be cached.
    pub version: u64,
    pub engine: Option<SharedString>,
    pub browser: Option<SharedString>,
}

impl Targets {
    /// Apps (without tags) and web settings, plus each app's desktop file, so the tags can be
    /// read in the background (`with_tags`).
    pub fn load(cx: &App) -> (Self, Vec<Arc<Path>>) {
        let mut out = Targets::default();
        let mut files = Vec::new();
        let Some(data) = cx
            .try_global::<LauncherEntityGlobal>()
            .and_then(|g| g.0.upgrade())
        else {
            return (out, files);
        };
        for launcher in data.read(cx).values() {
            if let LauncherType::Web(web) = &launcher.config.launcher_type
                && out.engine.is_none()
            {
                out.engine = Some(web.engine.clone());
                out.browser = web.browser.clone().map(SharedString::from);
            }
            for child in launcher.children.iter() {
                if let ChildKind::App(app) = &child.kind {
                    out.apps.push(AppEntry {
                        name: app.meta.name.to_string(),
                        exec: app.payload.exec.to_string(),
                        terminal: app.payload.terminal,
                        tags: Vec::new(),
                    });
                    files.push(app.payload.desktop_file.clone());
                }
            }
        }
        (out, files)
    }

    /// Reads each desktop file's tags (kind of app). Blocking file IO: run off the UI thread.
    pub fn read_tags(files: &[Arc<Path>]) -> Vec<Vec<String>> {
        use crate::ui::views::ask::content::actions::desktop_tags;
        files
            .iter()
            .map(|p| {
                std::fs::read_to_string(p)
                    .map(|c| desktop_tags(&c))
                    .unwrap_or_default()
            })
            .collect()
    }

    /// A copy with the tags filled in, in `load` order.
    pub fn with_tags(&self, tags: Vec<Vec<String>>) -> Self {
        let mut out = self.clone();
        for (app, tags) in out.apps.iter_mut().zip(tags) {
            app.tags = tags;
        }
        out.version += 1;
        out
    }

    /// How the launcher runs `action`.
    pub fn exec_mode(&self, action: Resolved) -> Option<ExecMode> {
        Some(match action {
            Resolved::App(app) => ExecMode::App {
                exec: app.exec,
                terminal: app.terminal,
            },
            Resolved::Web(query) => ExecMode::Web {
                engine: Some(self.engine.clone().unwrap_or_else(|| "duckduckgo".into())),
                browser: self.browser.clone(),
                exec: Some(query.into()),
            },
            Resolved::Url(url) => ExecMode::Web {
                engine: Some("plain".into()),
                browser: self.browser.clone(),
                exec: Some(url.into()),
            },
            Resolved::Copy(content) => ExecMode::Copy { content },
            Resolved::Invalid(_) => return None,
        })
    }
}

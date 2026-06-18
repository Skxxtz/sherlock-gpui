use std::sync::Arc;

use gpui::SharedString;
use indoc::indoc;
use serde::Deserialize;

use crate::{
    display_name,
    docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
    launcher::{
        LauncherConfig, LauncherProvider, app_launcher::app_data::AppData,
        variant_type::LauncherType,
    },
    loader::{
        resolve_icon_path,
        utils::{PriorityGuard, RawLauncher},
    },
    ui::{model::file::FileSearchBackend, widgets::RenderableChild},
    utils::errors::SherlockMessage,
    variant_name,
};

/// The following arguments are available to users:
/// - `backend`: The backend used for the filesearch, `rg`, `fd`, `walkdir`
/// - `poll_interval`: Time between backend calls
/// - `max_results`: The maximum number of search results, displayed,
/// - `path`: The root path for the file search
#[derive(Clone, Debug, Deserialize)]
pub struct FileLauncher {
    pub loc: SharedString,
    pub max_results: usize,
    pub poll_interval: u64,
    pub backend: FileSearchBackend,
}

impl LauncherProvider for FileLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        let backend = raw
            .args
            .get("backend")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let poll_interval = raw
            .args
            .get("poll_interval")
            .and_then(|v| v.as_u64())
            .unwrap_or(50);

        let max_results = raw
            .args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);

        let loc = raw
            .args
            .get("path")
            .and_then(|v| v.as_str())
            .unwrap_or("~/")
            .to_string()
            .into();

        LauncherType::Files(Self {
            backend,
            loc,
            poll_interval,
            max_results,
        })
    }

    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let inner = AppData {
            name: launcher.name.as_ref().map(Into::into),
            search_string: "file;file search".into(),
            icon: launcher.icon.clone().or(resolve_icon_path("folder")),
            priority: PriorityGuard::new_with_launcher(&launcher, 0),
            ..AppData::new()
        };

        let child = RenderableChild::App { launcher, inner };

        Ok(vec![child])
    }
}

// DOCS
impl LauncherDoc for FileLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(FileLauncher),
            variant_name: variant_name!(Files),
            description: "A file search. Allows you to search for files and directories from within Sherlock.",
            args: &[
                FieldDoc {
                    name: "backend",
                    ty: "string",
                    required: false,
                    default: Some("fd"),
                    description: "The backend to be used by the file search. Can be either of: `Fd`, `Rg`, or `WalkDir`",
                },
                FieldDoc {
                    name: "poll_interval",
                    ty: "u64",
                    required: false,
                    default: Some("50"),
                    description: "The time in milliseconds between backend calls.",
                },
                FieldDoc {
                    name: "max_results",
                    ty: "usize",
                    required: false,
                    default: Some("50"),
                    description: "The maximum number of results to show in the file search.",
                },
                FieldDoc {
                    name: "path",
                    ty: "path",
                    required: false,
                    default: Some("~/"),
                    description: "The root path from which to start the file search.",
                },
            ],
            examples: &[Example {
                description: "Basic event launcher",
                json: indoc! {
                    r#"{
                        "name": "File Search",
                        "type": "files",
                        "alias": "fs",
                        "args": {
                            "max_results": 50,
                            "poll_interval": 50,
                            "backend": "fd",
                            "path": "~/"
                        },
                        "priority": 5,
                        "home": "Home"
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

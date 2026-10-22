use std::sync::Arc;

use gpui::SharedString;
use serde::Deserialize;

use crate::{
    launcher::{
        LauncherConfig, LauncherProvider, utils::exec_mode::ExecMode, variant_type::LauncherType,
    },
    loader::{resolve_icon_path, utils::RawLauncher},
    sherlock_msg,
    ui::{
        launcher::navigator::handle::ViewKind,
        model::file::FileSearchBackend,
        widgets::{
            RenderableChild,
            list_item::{ItemMeta, ListItem, fixed::FixedPayload},
        },
    },
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        files::binary_exists,
    },
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
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let backend: FileSearchBackend = raw
            .args
            .get("backend")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        if let Some(bin) = backend.provider().binary()
            && !binary_exists(bin)
        {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, bin.into()),
                format!("`{bin}` not found in $PATH")
            ));
        }

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

        Ok(LauncherType::Files(Self {
            backend,
            loc,
            poll_interval,
            max_results,
        }))
    }

    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: std::sync::Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        let item = ListItem {
            meta: ItemMeta {
                search: "file;file search".into(),
                icon: launcher.icon.clone().or(resolve_icon_path("folder")),
                ..ItemMeta::new(launcher.name.as_deref().unwrap_or("File Search"), &launcher)
            },
            payload: FixedPayload(|launcher| ExecMode::CreateView {
                mode: ViewKind::Files { dir: None },
                launcher: launcher.clone(),
            }),
        };

        Ok(vec![RenderableChild::new(launcher, item)])
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::FileLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };
    use indoc::indoc;

    impl LauncherDoc for FileLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(FileLauncher),
                variant_name: variant_name!(Files),
                description: "Search for files and directories from within Sherlock. \
                    Narrow results with filters: `t:<type>` (e.g. `t:d`, `t:img`, `t:doc`) \
                    and `ext:<ext,...>` (e.g. `ext:pdf,png`). Example: `invoice t:doc`.",
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
}

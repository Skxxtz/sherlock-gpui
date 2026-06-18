use std::sync::Arc;

use gpui::App;
use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    define_inner_functions, display_name,
    docs::launcher::{Example, FieldDoc, InnerFunctionDoc, LauncherDoc, LauncherDocEntry},
    ensure_func,
    launcher::{
        ExecEffect, LauncherProvider, LauncherType, LoadContext, app_launcher::app_data::AppData,
        variant_type::InnerFunction,
    },
    loader::{
        resolve_icon_path,
        utils::{PriorityGuard, RawLauncher},
    },
    sherlock_msg, skip_func_if_nav,
    ui::widgets::RenderableChild,
    utils::errors::{SherlockMessage, types::SherlockErrorType},
    variant_name,
};

use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;

define_inner_functions! {
    pub enum ProcessLauncherFunctions {
        Quit { pid: i32 }, // TODO strip the pid to make compatible with `inner.` notation
    }
}

/// The following arguments are available to users:
/// - `max_results`: Maximum number of search results displayed in the view
/// - `show_tile`: Wheather a tile should be displayed or the user only wants to use the alias
///
/// The following inner functions are available:
/// - `Quit`: Only internal for now.
#[derive(Clone, Debug, Deserialize)]
pub struct ProcessLauncher {
    pub max_results: usize,
}

impl LauncherProvider for ProcessLauncher {
    fn parse(raw: &RawLauncher) -> LauncherType {
        let max_results = raw
            .args
            .get("max_results")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(50);

        LauncherType::Process(ProcessLauncher { max_results })
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        _ctx: &LoadContext,
        opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        if opts
            .get("show_tile")
            .is_some_and(|s| s.as_bool().unwrap_or_default())
        {
            Ok(vec![RenderableChild::App {
                inner: AppData {
                    name: launcher.name.clone(),
                    icon: launcher
                        .icon
                        .clone()
                        .or(resolve_icon_path("sherlock-process")),
                    priority: PriorityGuard::new_with_launcher(&launcher, 0),
                    ..AppData::new()
                },
                launcher,
            }])
        } else {
            Ok(vec![])
        }
    }
    fn execute_function(
        &self,
        func: super::variant_type::InnerFunction,
        _child: &RenderableChild,
        _variables: &[(gpui::SharedString, gpui::SharedString)],
        _cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        skip_func_if_nav!(func);
        let func = ensure_func!(func, InnerFunction::Process);

        match func {
            ProcessLauncherFunctions::Quit { pid } => kill_process(pid)?,
        }

        Ok(ExecEffect::None)
    }
}

fn kill_process(pid: i32) -> Result<(), SherlockMessage> {
    let child = Pid::from_raw(pid);
    kill(child, Signal::SIGKILL).map_err(|e| sherlock_msg!(Warning, SherlockErrorType::IO, e))
}

// DOCS
impl LauncherDoc for ProcessLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(ProcessLauncher),
            variant_name: variant_name!(Process),
            description: "Searches and terminates processes from within Sherlock.",
            args: &[
                FieldDoc {
                    name: "max_results",
                    ty: "usize",
                    required: false,
                    default: Some("50"),
                    description: "The maximum number of results to show in the process search.",
                },
                FieldDoc {
                    name: "show_tile",
                    ty: "bool",
                    required: false,
                    default: Some("false"),
                    description: "Wheather a tile should be displayed of the user only wants the alias-based execution.",
                },
            ],
            inner_functions: &[InnerFunctionDoc {
                name: "Quit",
                identifier: "inner.quit",
                description: "Quit the current process",
                user_facing: true,
            }],
            examples: &[Example {
                description: "Basic process terminator",
                json: indoc! {
                    r#"{
                        "name": "Processes",
                        "type": "process",
                        "alias": "kill",
                        "args": {},
                        "priority": 1,
                        "home": "Home",
                        "shortcut": false,
                        "exit": false
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

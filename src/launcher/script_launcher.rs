use gpui::{App, AppContext, SharedString};
use indoc::indoc;
use serde_json::Value;
use std::sync::Arc;

use crate::{
    define_inner_functions, display_name,
    docs::launcher::{Example, FieldDoc, InnerFunctionDoc, LauncherDoc, LauncherDocEntry},
    ensure_func,
    launcher::{
        Bind, ExecEffect, LauncherProvider, LauncherType, LoadContext, variant_type::InnerFunction,
    },
    loader::utils::RawLauncher,
    sherlock_msg, skip_func_if_nav,
    ui::{
        traits::RenderableChildImpl,
        widgets::{
            RenderableChild,
            script::{ScriptData, ScriptDataUpdateEntity},
        },
    },
    utils::errors::{SherlockMessage, types::SherlockErrorType},
    variant_name,
};

define_inner_functions! {
    pub enum ScriptFunctions {
        Run,
    }
}

/// The following arguments are available to users:
/// - `exec`: The script to be executed
/// - `exec-args`: The arguments to the command
/// - `async`: Whether to wait for execution of the `inner.run` command or to run on keypress.
///
/// The following inner functions are available:
/// - `Run`: Runs the current script (if not async)
#[derive(Clone, Debug)]
pub struct ScriptLauncher {
    pub r#async: bool,
    binds: Option<Arc<Vec<Bind>>>,
}

impl LauncherProvider for ScriptLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let binds = raw
            .binds
            .as_ref()
            .map(|vec| Arc::new(vec.iter().filter_map(|b| Bind::try_from(b).ok()).collect()));

        let r#async = raw
            .args
            .get("async")
            .and_then(Value::as_bool)
            .unwrap_or(true);

        Ok(LauncherType::Script(ScriptLauncher { r#async, binds }))
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        _ctx: &LoadContext,
        opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let command: SharedString = opts
            .get("exec")
            .and_then(|v| v.as_str())
            .map(|s| SharedString::from(s.to_owned()))
            .ok_or(sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError(format!(
                    "Failed to parse command from launcher configuration of launcher: {launcher}"
                )),
                format!("`exec` key is required. Received arguments: {:?}", opts)
            ))?;

        let args: SharedString = opts
            .get("exec-args")
            .and_then(|v| v.as_str())
            .map(|s| SharedString::from(s.to_owned()))
            .unwrap_or_default();

        Ok(vec![RenderableChild::Script {
            launcher,
            inner: ScriptData {
                command,
                args,
                update_entity: cx.new(|_| ScriptDataUpdateEntity::default()),
            },
        }])
    }
    fn execute_function(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
        _variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        skip_func_if_nav!(func);
        let func = ensure_func!(func, InnerFunction::Script);
        match func {
            ScriptFunctions::Run => {
                if let RenderableChild::Script { inner, launcher } = child {
                    inner.update_async(launcher.clone(), cx);
                }
            }
        }
        Ok(ExecEffect::None)
    }
    fn binds(&self) -> Option<Arc<Vec<Bind>>> {
        self.binds.clone()
    }
}

// DOCS
impl LauncherDoc for ScriptLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(ScriptLauncher),
            variant_name: variant_name!(Script),
            description: "Executes commands either on keypress (async) or on return. The results will be displayed within Sherlock.",
            args: &[
                FieldDoc {
                    name: "async",
                    ty: "bool",
                    required: false,
                    default: Some("true"),
                    description: "If set to true, will run the script on every keypress. If set to false, will wait for the execution of the `inner.run` command.",
                },
                FieldDoc {
                    name: "exec",
                    ty: "command",
                    required: false,
                    default: Some("false"),
                    description: "Wheather a tile should be displayed of the user only wants the alias-based execution.",
                },
                FieldDoc {
                    name: "exec",
                    ty: "string",
                    required: false,
                    default: Some(""),
                    description: "The arguments to the command. Will replace `{keyword}` with the actual contents of the search bar.",
                },
            ],
            inner_functions: &[InnerFunctionDoc {
                name: "Run",
                identifier: "inner.run",
                description: "Run the current script. (Required if `async = false`)",
                user_facing: true,
            }],
            examples: &[Example {
                description: "Basic process terminator",
                json: indoc! {
                    r#"{
                        "name": "Wikipedia Search",
                        "alias": "wiki",
                        "type": "script",
                        "args": {
                            "icon": "wikipedia",
                            "exec": "sherlock-wiki",
                            "exec-args": "'{keyword}'"
                        },
                        "priority": 0,
                        "shortcut": false
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

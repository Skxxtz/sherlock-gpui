use std::{sync::Arc, time::Duration};

use gpui::{App, SharedString};
use indoc::indoc;
use serde::Deserialize;
use serde_json::Value;

use crate::{
    define_inner_functions, display_name,
    docs::launcher::{Example, FieldDoc, InnerFunctionDoc, LauncherDoc, LauncherDocEntry},
    ensure_func,
    launcher::{
        ExecEffect, LauncherConfig, LauncherProvider, LauncherType, LoadContext,
        variant_type::InnerFunction,
    },
    loader::utils::RawLauncher,
    sherlock_msg, skip_func_if_nav,
    ui::widgets::{RenderableChild, timer::TimerChild},
    utils::errors::{SherlockMessage, types::SherlockErrorType},
    variant_name,
};

define_inner_functions! {
    pub enum TimerLauncherFunctions {
        Toggle,
        Reset,
        NewTimer { duration: Duration },
    }
}

/// The following arguments are available to users:
/// - `exec`: Default command to execute on timer end
#[derive(Clone, Debug, Deserialize)]
pub struct TimerLauncher {
    #[serde(default)]
    command: Option<SharedString>,
}

impl LauncherProvider for TimerLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        serde_json::from_value::<TimerLauncher>(raw.args.as_ref().clone())
            .map(LauncherType::Timer)
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))
    }
    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        Ok(vec![RenderableChild::Timer {
            launcher,
            inner: TimerChild::new(cx),
        }])
    }
    fn execute_function(
        &self,
        func: super::variant_type::InnerFunction,
        child: &RenderableChild,
        variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, crate::utils::errors::SherlockMessage> {
        skip_func_if_nav!(func);
        let func = ensure_func!(func, InnerFunction::Timer);

        let RenderableChild::Timer { inner, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack music tile but received: {:?}", child)
            ));
        };

        let command = match variables.first() {
            Some(v) if v.0.as_str() == "command" && !v.1.is_empty() => Some(v.1.clone()),
            _ => self.command.clone(),
        };

        match func {
            TimerLauncherFunctions::Toggle => inner.toggle(cx),
            TimerLauncherFunctions::NewTimer { duration } => inner.new_timer(duration, command, cx),
            TimerLauncherFunctions::Reset => {
                unimplemented!()
            }
        }

        Ok(ExecEffect::None)
    }
}

// DOCS
impl LauncherDoc for TimerLauncher {
    fn doc() -> LauncherDocEntry {
        LauncherDocEntry {
            name: display_name!(TimerLauncher),
            variant_name: variant_name!(Timer),
            description: "Start and run up to four timers concurrently. Each timer can have a unique action to be run at completion.",
            args: &[FieldDoc {
                name: "exec",
                ty: "command",
                required: false,
                default: Some(""),
                description: "The command to execute on timer completion.",
            }],
            inner_functions: &[
                InnerFunctionDoc {
                    name: "Toggle",
                    identifier: "inner.toggle",
                    description: "Toggle all timers",
                    user_facing: true,
                },
                InnerFunctionDoc {
                    name: "Reset",
                    identifier: "inner.reset",
                    description: "Reset all timers",
                    user_facing: true,
                },
                InnerFunctionDoc {
                    name: "New Timer",
                    identifier: "",
                    description: "Create new timer",
                    user_facing: false,
                },
            ],
            examples: &[Example {
                description: "Basic process terminator",
                json: indoc! {
                    r#"{
                        "name": "Timer",
                        "type": "timer",
                        "args": {
                            "exec": "notify-send \"hello\""
                        },
                        "priority": 1,
                        "shortcut": false
                    }"#
                },
            }],
            ..LauncherDocEntry::new()
        }
    }
}

use std::sync::Arc;

use serde::Deserialize;
use serde_json::Value;

use crate::{
    launcher::{
        LauncherConfig,
        LauncherProvider,
        LauncherType,
        utils::{binds::Bind, exec_mode::ExecMode},
    },
    loader::{LoadContext, utils::RawLauncher},
    sherlock_msg,
    ui::{
        launcher::navigator::handle::ViewKind,
        views::ask::provider::Provider,
        widgets::{
            RenderableChild,
            list_item::{ItemMeta, ListItem, fixed::FixedPayload},
        },
    },
    utils::errors::{SherlockMessage, types::SherlockErrorType},
};

#[inline(always)]
fn default_chats() -> usize {
    5
}

/// The following arguments are available to users:
/// - `use_keywords`: Whether the search should use the keywords or only the app name
#[derive(Clone, Debug, Deserialize)]
pub struct AskLauncher {
    #[serde(flatten)]
    pub provider: Provider,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub max_turns: Option<usize>,
    #[serde(default)]
    pub retain: bool,
    #[serde(default = "default_chats")]
    pub num_recent: usize,
    #[serde(skip)]
    pub binds: Option<Arc<[Bind]>>,
}

impl LauncherProvider for AskLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let mut launcher = serde_json::from_value::<AskLauncher>(raw.args.as_ref().clone())
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::InvalidData, e))?;
        launcher.binds = raw
            .binds
            .as_ref()
            .map(|vec| {
                vec.iter()
                    .map(|b| {
                        Bind::try_from(b).map_err(|e| {
                            sherlock_msg!(Error, SherlockErrorType::InvalidLauncher, e)
                        })
                    })
                    .collect::<Result<Arc<[Bind]>, _>>()
            })
            .transpose()?;
        Ok(LauncherType::Ask(launcher))
    }
    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, crate::utils::errors::SherlockMessage> {
        let item = ListItem {
            meta: ItemMeta::new("Ask AI").icon_name("ai"),
            payload: FixedPayload(|launcher| ExecMode::CreateView {
                mode: ViewKind::Ask,
                launcher: launcher.clone(),
            }),
        };
        Ok(vec![RenderableChild::new(launcher, item)])
    }
    fn binds(&self) -> Option<Arc<[Bind]>> {
        self.binds.clone()
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use indoc::indoc;

    use super::AskLauncher;
    use crate::{
        display_name,
        docs::{
            Documentation,
            launcher::{Example, FieldDoc, LauncherDocEntry},
        },
        variant_name,
    };

    impl Documentation for AskLauncher {
        type Docs = LauncherDocEntry;
        fn docs() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(AskLauncher),
                feature: Some("ai"),
                variant_name: variant_name!(Ask),
                description: "Opens Sherlock's custom AI view. Data travels only between you and \
                  your chosen server, never through Sherlock's servers or any third party. \
                  The launcher's AI code can be omitted entirely by disabling the `ai` feature flag.",
                args: &[
                    FieldDoc {
                        name: "kind",
                        description: "The backend to talk to: `ollama`, `open_ai` or `gemini`.",
                        ty: "string",
                        required: false,
                        default: Some("ollama"),
                    },
                    FieldDoc {
                        name: "model",
                        description: "The model name, e.g. `gemini-3.5-flash-lite` or `llama3.2`.",
                        ty: "string",
                        required: false,
                        default: Some("The provider's default."),
                    },
                    FieldDoc {
                        name: "endpoint",
                        description: "Base URL of the API, e.g. `http://192.168.1.5:11434` for Ollama on another machine.",
                        ty: "string",
                        required: false,
                        default: Some("The provider's default."),
                    },
                    FieldDoc {
                        name: "api_key_cmd",
                        description: "Shell command whose first output line is the API key, e.g. `pass show ai/gemini` or `cat /run/secrets/gemini`. Runs once when the view opens.",
                        ty: "string",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "api_key_env",
                        description: "Environment variable holding the API key.",
                        ty: "string",
                        required: false,
                        default: Some("`GEMINI_API_KEY` or `OPENAI_API_KEY`, by kind."),
                    },
                    FieldDoc {
                        name: "api_key",
                        description: "The API key itself. Prefer `api_key_cmd` or `api_key_env`: a key here ends up in your dotfiles.",
                        ty: "string",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "temperature",
                        description: "Sampling temperature. Lower gives more predictable answers.",
                        ty: "f32",
                        required: false,
                        default: Some("0.2"),
                    },
                    FieldDoc {
                        name: "daily_requests",
                        description: "Requests allowed per day, shown as \"x left\" (e.g. `500` for Gemini Flash-Lite's free tier).",
                        ty: "u32",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "daily_tokens",
                        description: "Tokens allowed per day, shown as \"x left\" when `daily_requests` is not set.",
                        ty: "u64",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "quota_reset",
                        description: "When the daily quota resets: `local` (midnight) or `pacific` (midnight Pacific time).",
                        ty: "string",
                        required: false,
                        default: Some("`pacific` for Gemini, `local` otherwise."),
                    },
                    FieldDoc {
                        name: "price_input",
                        description: "USD per 1M input tokens. Together with `price_output`, shows each answer's cost.",
                        ty: "f64",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "price_output",
                        description: "USD per 1M output tokens.",
                        ty: "f64",
                        required: false,
                        default: None,
                    },
                    FieldDoc {
                        name: "system_prompt",
                        description: "Replaces the built-in system prompt. `{context}` is replaced with the OS, shell, desktop and today's date.",
                        ty: "string",
                        required: false,
                        default: Some("The built-in prompt."),
                    },
                    FieldDoc {
                        name: "max_turns",
                        description: "How many earlier questions and answers are sent along with each question.",
                        ty: "usize",
                        required: false,
                        default: Some("6"),
                    },
                ],
                examples: &[Example {
                    description: "Basic ask launcher",
                    json: indoc! {
                        r#"{
                            "type": "ask",
                            "name": "Ask AI",
                            "alias": "ask",
                            "priority": 1,
                            "args": {
                              "kind": "gemini",
                              "model": "gemini-3.5-flash-lite",
                              "api_key_cmd": "cat /run/agenix/gemini",
                              "daily_requests": 500,
                              "retain": true
                            },
                            "binds": [
                              {
                                "bind": "ctrl-n",
                                "callback": "new_chat"
                              },
                              {
                                "bind": "ctrl-c",
                                "callback": "stop;continue"
                              }
                            ],
                            "home": "Home"
                        }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

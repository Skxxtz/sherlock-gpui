//! Model backends: which API to talk to, with what settings, and the streaming client.
//!
//! - `Provider`: the configuration (kind, endpoint, model, key, prices, quota, temperature)
//! - `api`: one `Api` impl per backend's wire format (url, request body, stream lines)
//! - `stream`: `AnswerStream`, which posts the request and streams the answer into an exchange
//! - `key`: where the API key comes from
//! - `errors`: HTTP and connection failures in words
//!
//! Adding a backend: a `ProviderKind` variant and an `Api` impl.

use gpui::SharedString;
use serde::{Deserialize, Serialize};

use crate::ui::views::ask::{model::exchange::Usage, store::quota::ResetZone};

pub mod api;
pub mod errors;
pub mod key;
pub mod stream;

pub use stream::{AnswerStream, Drain};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    /// `POST {endpoint}/api/chat`, NDJSON stream.
    #[default]
    Ollama,
    /// `POST {endpoint}/v1/chat/completions`, SSE stream.
    OpenAi,
    /// Google AI Studio through its OpenAI-compatible API:
    /// `POST {endpoint}/chat/completions`, SSE stream, key as bearer token.
    Gemini,
}

impl ProviderKind {
    /// The wire format of this backend.
    pub fn api(self) -> &'static dyn api::Api {
        match self {
            Self::Ollama => &api::Ollama,
            Self::OpenAi => &api::OpenAi,
            Self::Gemini => &api::Gemini,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Provider {
    #[serde(default)]
    pub kind: ProviderKind,
    /// Base url, e.g. `http://localhost:11434`. Empty: the kind's default.
    #[serde(default)]
    pub endpoint: String,
    /// Empty: the kind's default (see `Api::default_model`).
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key: Option<String>,
    /// Shell command printing the API key (`pass show ai/gemini`, `cat /run/secrets/gemini`);
    /// runs once per view, off the UI thread. See `key`.
    #[serde(default)]
    pub api_key_cmd: Option<String>,
    /// Environment variable holding the API key. Default: the kind's (`GEMINI_API_KEY`, …).
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// USD per 1M input tokens; with `price_output`, enables cost display.
    #[serde(default)]
    pub price_input: Option<f64>,
    /// USD per 1M output tokens.
    #[serde(default)]
    pub price_output: Option<f64>,
    /// Requests allowed per quota day (e.g. 500 for Gemini Flash-Lite's free tier).
    #[serde(default)]
    pub daily_requests: Option<u32>,
    /// Tokens allowed per quota day.
    #[serde(default)]
    pub daily_tokens: Option<u64>,
    /// When the quota day starts. Default: Pacific for Gemini, local midnight otherwise.
    #[serde(default)]
    pub quota_reset: Option<ResetZone>,
    /// Sampling temperature. Default `DEFAULT_TEMPERATURE`: launcher answers should be
    /// predictable, not creative.
    #[serde(default)]
    pub temperature: Option<f32>,
}

pub const DEFAULT_TEMPERATURE: f32 = 0.2;

impl Default for Provider {
    fn default() -> Self {
        Self {
            kind: ProviderKind::Ollama,
            endpoint: String::new(),
            model: String::new(),
            api_key: None,
            api_key_cmd: None,
            api_key_env: None,
            price_input: None,
            price_output: None,
            daily_requests: None,
            daily_tokens: None,
            quota_reset: None,
            temperature: None,
        }
    }
}

impl Provider {
    /// Google AI Studio with an API key from aistudio.google.com.
    pub fn gemini(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            kind: ProviderKind::Gemini,
            endpoint: String::new(),
            model: model.into(),
            api_key: Some(api_key.into()),
            api_key_cmd: None,
            api_key_env: None,
            price_input: None,
            price_output: None,
            daily_requests: None,
            daily_tokens: None,
            quota_reset: None,
            temperature: None,
        }
    }

    pub fn reset_zone(&self) -> ResetZone {
        self.quota_reset
            .unwrap_or_else(|| self.kind.api().reset_zone())
    }

    /// Whether the provider has any daily limit configured.
    pub fn has_quota(&self) -> bool {
        self.daily_requests.is_some() || self.daily_tokens.is_some()
    }

    /// Cost of `usage` in USD, if both prices are configured.
    pub fn cost(&self, usage: Usage) -> Option<f64> {
        let (pi, po) = (self.price_input?, self.price_output?);
        Some((usage.input as f64 * pi + usage.output as f64 * po) / 1_000_000.0)
    }

    /// The base url, without a trailing slash: the configured endpoint or the backend's default.
    pub(crate) fn base(&self) -> &str {
        let ep = self.endpoint.trim();
        let ep = if ep.is_empty() {
            self.kind.api().default_endpoint()
        } else {
            ep
        };
        ep.trim_end_matches('/')
    }
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

/// One chat message. The content is shared (`Arc`-backed), so resending earlier turns and the
/// system prompt with every question doesn't copy them.
#[derive(Clone, Debug, Serialize)]
pub struct Message {
    pub role: Role,
    pub content: SharedString,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gemini_resets_in_pacific_time() {
        assert_eq!(Provider::gemini("k", "m").reset_zone(), ResetZone::Pacific);
        assert_eq!(Provider::default().reset_zone(), ResetZone::Local);
    }

    #[test]
    fn cost_needs_both_prices() {
        let mut p = Provider::default();
        let u = Usage {
            input: 1_000_000,
            output: 500_000,
        };
        assert_eq!(p.cost(u), None);
        p.price_input = Some(0.1);
        p.price_output = Some(0.4);
        assert!((p.cost(u).unwrap() - 0.3).abs() < 1e-9);
    }

    #[test]
    fn custom_endpoint_trailing_slash() {
        let p = Provider {
            endpoint: "http://box:11434/".into(),
            ..Default::default()
        };
        assert_eq!(p.kind.api().url(p.base()), "http://box:11434/api/chat");
    }

    #[test]
    fn gemini_url() {
        let p = Provider::gemini("key", "gemini-3.8-flash");
        assert_eq!(
            p.kind.api().url(p.base()),
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions"
        );
    }
}

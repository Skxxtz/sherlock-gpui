use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::ui::views::ask::{model::exchange::Usage, provider::Message, store::quota::ResetZone};

/// One backend's wire format.
pub trait Api: Sync {
    /// Base url when the config leaves `endpoint` empty.
    fn default_endpoint(&self) -> &'static str;

    /// Model when the config leaves `model` empty.
    fn default_model(&self) -> &'static str;

    /// Environment variable read for the API key when the config names none.
    fn key_env(&self) -> Option<&'static str> {
        None
    }

    /// The chat endpoint under `base` (no trailing slash).
    fn url(&self, base: &str) -> String;

    /// The streaming request body.
    fn body(&self, model: &str, messages: &[Message], temperature: f32) -> Result<Vec<u8>, String>;

    /// One line of the streamed response. Blank lines, comments and unknown shapes parse to an
    /// empty `Line`; an error reported by the API is an `Err`.
    fn parse_line<'a>(&self, line: &'a [u8]) -> Result<Line<'a>, String>;

    /// When this backend's daily quota resets, unless configured.
    fn reset_zone(&self) -> ResetZone {
        ResetZone::Local
    }
}

/// Ollama: `/api/chat`, NDJSON stream, temperature in `options`, usage on the last line.
pub struct Ollama;

/// OpenAI and compatible servers: `/v1/chat/completions`, SSE stream.
pub struct OpenAi;

/// Google AI Studio through its OpenAI-compatible endpoint; quota resets in Pacific time.
pub struct Gemini;

impl Api for Ollama {
    fn default_endpoint(&self) -> &'static str {
        "http://localhost:11434"
    }

    fn default_model(&self) -> &'static str {
        "llama3.2"
    }

    fn url(&self, base: &str) -> String {
        format!("{base}/api/chat")
    }

    fn body(&self, model: &str, messages: &[Message], temperature: f32) -> Result<Vec<u8>, String> {
        #[derive(Serialize)]
        struct Options {
            temperature: f32,
        }
        #[derive(Serialize)]
        struct Body<'a> {
            model: &'a str,
            messages: &'a [Message],
            stream: bool,
            options: Options,
        }
        serde_json::to_vec(&Body {
            model,
            messages,
            stream: true,
            options: Options { temperature },
        })
        .map_err(|e| e.to_string())
    }

    fn parse_line<'a>(&self, line: &'a [u8]) -> Result<Line<'a>, String> {
        let line = line.trim_ascii();
        if line.is_empty() {
            return Ok(Line::default());
        }
        parse_ollama(line)
    }
}

impl Api for OpenAi {
    fn default_endpoint(&self) -> &'static str {
        "https://api.openai.com"
    }

    fn default_model(&self) -> &'static str {
        "gpt-4.1-mini"
    }

    fn key_env(&self) -> Option<&'static str> {
        Some("OPENAI_API_KEY")
    }

    fn url(&self, base: &str) -> String {
        format!("{base}/v1/chat/completions")
    }

    fn body(&self, model: &str, messages: &[Message], temperature: f32) -> Result<Vec<u8>, String> {
        openai_body(model, messages, temperature)
    }

    fn parse_line<'a>(&self, line: &'a [u8]) -> Result<Line<'a>, String> {
        parse_sse(line)
    }
}

impl Api for Gemini {
    fn default_endpoint(&self) -> &'static str {
        "https://generativelanguage.googleapis.com/v1beta/openai"
    }

    fn default_model(&self) -> &'static str {
        "gemini-2.5-flash-lite"
    }

    fn key_env(&self) -> Option<&'static str> {
        Some("GEMINI_API_KEY")
    }

    fn url(&self, base: &str) -> String {
        format!("{base}/chat/completions")
    }

    fn body(&self, model: &str, messages: &[Message], temperature: f32) -> Result<Vec<u8>, String> {
        openai_body(model, messages, temperature)
    }

    fn parse_line<'a>(&self, line: &'a [u8]) -> Result<Line<'a>, String> {
        parse_sse(line)
    }

    fn reset_zone(&self) -> ResetZone {
        ResetZone::Pacific
    }
}

/// OpenAI-style body; `include_usage` asks for a final chunk with token usage.
fn openai_body(model: &str, messages: &[Message], temperature: f32) -> Result<Vec<u8>, String> {
    #[derive(Serialize)]
    struct StreamOptions {
        include_usage: bool,
    }
    #[derive(Serialize)]
    struct Body<'a> {
        model: &'a str,
        messages: &'a [Message],
        stream: bool,
        stream_options: StreamOptions,
        temperature: f32,
    }
    serde_json::to_vec(&Body {
        model,
        messages,
        stream: true,
        stream_options: StreamOptions {
            include_usage: true,
        },
        temperature,
    })
    .map_err(|e| e.to_string())
}

/// An SSE line: `data: <json>` or `data: [DONE]`; everything else is skipped.
fn parse_sse(line: &[u8]) -> Result<Line<'_>, String> {
    match line.trim_ascii().strip_prefix(b"data:") {
        Some(data) => parse_openai(data.trim_ascii()),
        None => Ok(Line::default()),
    }
}

#[derive(Debug, Default, PartialEq)]
pub struct Line<'a> {
    pub delta: Cow<'a, str>,
    pub usage: Option<Usage>,
    pub end: bool,
}

#[cfg(test)]
impl<'a> Line<'a> {
    fn delta(text: Cow<'a, str>) -> Self {
        Self {
            delta: text,
            ..Default::default()
        }
    }
}

fn parse_ollama(line: &[u8]) -> Result<Line<'_>, String> {
    #[derive(Deserialize)]
    struct Chunk<'a> {
        #[serde(borrow, default)]
        message: Option<Msg<'a>>,
        #[serde(default)]
        done: bool,
        #[serde(default)]
        error: Option<String>,
        #[serde(default)]
        prompt_eval_count: Option<u32>,
        #[serde(default)]
        eval_count: Option<u32>,
    }
    #[derive(Deserialize)]
    struct Msg<'a> {
        #[serde(borrow, default)]
        content: Cow<'a, str>,
    }
    let c: Chunk = serde_json::from_slice(line).map_err(|e| e.to_string())?;
    if let Some(e) = c.error {
        return Err(e);
    }
    // Ollama omits prompt_eval_count when the whole prompt came from its cache.
    let usage = (c.prompt_eval_count.is_some() || c.eval_count.is_some()).then(|| Usage {
        input: c.prompt_eval_count.unwrap_or(0),
        output: c.eval_count.unwrap_or(0),
    });
    Ok(Line {
        delta: c.message.map(|m| m.content).unwrap_or_default(),
        usage,
        end: c.done,
    })
}

fn parse_openai(data: &[u8]) -> Result<Line<'_>, String> {
    #[derive(Deserialize)]
    struct Chunk<'a> {
        #[serde(borrow, default)]
        choices: Vec<Choice<'a>>,
        #[serde(default)]
        error: Option<serde_json::Value>,
        #[serde(default)]
        usage: Option<ApiUsage>,
    }
    #[derive(Deserialize)]
    struct Choice<'a> {
        #[serde(borrow, default)]
        delta: Option<Delta<'a>>,
    }
    #[derive(Deserialize)]
    struct Delta<'a> {
        #[serde(borrow, default)]
        content: Option<Cow<'a, str>>,
    }
    #[derive(Deserialize)]
    struct ApiUsage {
        #[serde(default)]
        prompt_tokens: u32,
        #[serde(default)]
        completion_tokens: u32,
    }
    if data == b"[DONE]" {
        return Ok(Line {
            end: true,
            ..Default::default()
        });
    }
    let mut c: Chunk = serde_json::from_slice(data).map_err(|e| e.to_string())?;
    if let Some(e) = c.error {
        return Err(e.to_string());
    }
    Ok(Line {
        delta: c
            .choices
            .pop()
            .and_then(|c| c.delta?.content)
            .unwrap_or_default(),
        usage: c.usage.map(|u| Usage {
            input: u.prompt_tokens,
            output: u.completion_tokens,
        }),
        end: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::views::ask::provider::Role;

    #[test]
    fn ollama_delta_and_done() {
        let l =
            br#"{"model":"x","message":{"role":"assistant","content":"Hi \"you\""},"done":false}"#;
        assert_eq!(Ollama.parse_line(l), Ok(Line::delta("Hi \"you\"".into())));
        let end = br#"{"model":"x","message":{"role":"assistant","content":""},"done":true,"prompt_eval_count":612,"eval_count":143}"#;
        let line = Ollama.parse_line(end).unwrap();
        assert!(line.end);
        assert_eq!(
            line.usage,
            Some(Usage {
                input: 612,
                output: 143
            })
        );
    }

    #[test]
    fn ollama_error() {
        let l = br#"{"error":"model not found"}"#;
        assert_eq!(Ollama.parse_line(l), Err("model not found".into()));
    }

    #[test]
    fn openai_sse() {
        let l = br#"data: {"choices":[{"delta":{"content":"ss"}}]}"#;
        assert_eq!(OpenAi.parse_line(l), Ok(Line::delta("ss".into())));
        assert!(OpenAi.parse_line(b"data: [DONE]").unwrap().end);
        assert_eq!(OpenAi.parse_line(b": keep-alive"), Ok(Line::default()));
        let role_only = br#"data: {"choices":[{"delta":{"role":"assistant"}}]}"#;
        assert_eq!(OpenAi.parse_line(role_only), Ok(Line::default()));
    }

    #[test]
    fn openai_usage_chunk() {
        let l = br#"data: {"choices":[],"usage":{"prompt_tokens":530,"completion_tokens":88,"total_tokens":618}}"#;
        let line = OpenAi.parse_line(l).unwrap();
        assert_eq!(
            line.usage,
            Some(Usage {
                input: 530,
                output: 88
            })
        );
        assert!(line.delta.is_empty());
    }

    #[test]
    fn gemini_sse() {
        let l = br#"data: {"choices":[{"delta":{"content":"Hi","role":"assistant"},"index":0}],"model":"gemini-3.8-flash","object":"chat.completion.chunk"}"#;
        assert_eq!(Gemini.parse_line(l), Ok(Line::delta("Hi".into())));
    }

    #[test]
    fn temperature_goes_where_each_api_expects_it() {
        let msgs = [Message {
            role: Role::User,
            content: "q".into(),
        }];
        let g: serde_json::Value =
            serde_json::from_slice(&Gemini.body("m", &msgs, 0.2).unwrap()).unwrap();
        assert!((g["temperature"].as_f64().unwrap() - 0.2).abs() < 1e-6);
        assert_eq!(g["stream_options"]["include_usage"], true);
        assert!(g.get("options").is_none());
        let o: serde_json::Value =
            serde_json::from_slice(&Ollama.body("m", &msgs, 0.2).unwrap()).unwrap();
        assert!(o.get("temperature").is_none());
        assert!((o["options"]["temperature"].as_f64().unwrap() - 0.2).abs() < 1e-6);
        assert_eq!(o["messages"][0]["role"], "user");
    }
}

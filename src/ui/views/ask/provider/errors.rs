use crate::ui::views::ask::{
    provider::{Provider, ProviderKind},
    store::quota::local_clock,
};

pub(super) fn header_u64(headers: &reqwest::header::HeaderMap, name: &str) -> Option<u64> {
    headers.get(name)?.to_str().ok()?.trim().parse().ok()
}

/// A connection-level failure.
pub(super) fn send_error(provider: &Provider, e: &reqwest::Error) -> String {
    let base = provider.base();
    if e.is_timeout() {
        "The request timed out. Try again.".into()
    } else if e.is_connect() && provider.kind == ProviderKind::Ollama {
        format!("Can't reach Ollama at {base}. Is it running? Start it with `ollama serve`.")
    } else if e.is_connect() {
        format!("Can't reach {}. Check your connection.", host(base))
    } else {
        "The connection broke off. Try again.".into()
    }
}

/// A non-success HTTP status.
pub(super) fn http_error(
    provider: &Provider,
    status: u16,
    retry_after: Option<u64>,
    body: &str,
) -> String {
    let lower = body.to_ascii_lowercase();
    let model = &provider.model;
    match status {
        401 | 403 => "The API key was rejected. Check `api_key` in your config.".into(),
        400 if lower.contains("api_key_invalid") || lower.contains("api key not valid") => {
            "The API key was rejected. Check `api_key` in your config.".into()
        }
        404 if provider.kind == ProviderKind::Ollama => {
            format!("Model `{model}` isn't installed. Get it with `ollama pull {model}`.")
        }
        404 => format!("Model `{model}` wasn't found. Check `model` in your config."),
        429 if lower.contains("perday") || lower.contains("per day") => {
            let reset = provider.reset_zone().next_reset(chrono::Utc::now());
            format!("Daily quota reached. It resets at {}.", local_clock(reset))
        }
        429 => match retry_after.or_else(|| retry_delay(body)) {
            Some(s) => format!("Rate limit hit. Try again in {s} s."),
            None => "Rate limit hit. Try again in a moment.".into(),
        },
        500..=599 => format!(
            "{} is having problems ({status}). Try again shortly.",
            host(provider.base())
        ),
        _ => match api_message(body) {
            Some(msg) => format!("Request failed ({status}): {msg}"),
            None => format!("Request failed ({status})."),
        },
    }
}

/// Gemini's `"retryDelay": "17s"`.
fn retry_delay(body: &str) -> Option<u64> {
    let i = body.find("\"retryDelay\"")?;
    let rest = &body[i + 12..];
    let start = rest.find('"')? + 1;
    let digits: String = rest[start..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// `error.message` (OpenAI, Gemini) or `error` (Ollama).
fn api_message(body: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let v = if v.is_array() { v.get(0)? } else { &v };
    let err = v.get("error")?;
    let msg = err
        .get("message")
        .and_then(|m| m.as_str())
        .or_else(|| err.as_str())?;
    let mut msg = msg.trim().to_owned();
    if msg.len() > 160 {
        let cut = (0..=160)
            .rev()
            .find(|&i| msg.is_char_boundary(i))
            .unwrap_or(0);
        msg.truncate(cut);
        msg.push('…');
    }
    Some(msg)
}

fn host(url: &str) -> &str {
    let rest = url.split("://").nth(1).unwrap_or(url);
    rest.split('/').next().unwrap_or(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_errors() {
        let gemini = Provider::gemini("k", "gemini-3.5-flash-lite");
        let body = r#"[{"error":{"code":429,"status":"RESOURCE_EXHAUSTED","details":[{"quotaId":"GenerateRequestsPerDayPerProjectPerModel-FreeTier"},{"@type":"type.googleapis.com/google.rpc.RetryInfo","retryDelay":"17s"}]}}]"#;
        assert!(http_error(&gemini, 429, None, body).starts_with("Daily quota reached"));
        let rpm = r#"{"error":{"details":[{"retryDelay":"17s"}]}}"#;
        assert_eq!(
            http_error(&gemini, 429, None, rpm),
            "Rate limit hit. Try again in 17 s."
        );
        assert!(http_error(&gemini, 400, None, "API_KEY_INVALID").contains("API key"));
        let ollama = Provider::default().resolved().0;
        assert!(http_error(&ollama, 404, None, "").contains("ollama pull llama3.2"));
        let odd = r#"{"error":{"message":"context too long"}}"#;
        assert_eq!(
            http_error(&ollama, 413, None, odd),
            "Request failed (413): context too long"
        );
        assert_eq!(
            host("https://generativelanguage.googleapis.com/v1beta/openai"),
            "generativelanguage.googleapis.com"
        );
    }
}

use std::process::{Command, Stdio};

use super::Provider;

/// Whether the key is known or a command has to print it first.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeySource {
    /// Inline or from the environment: already in the provider (or none needed).
    Ready,
    /// Run `read_key` with this command.
    Command(String),
}

impl Provider {
    /// A copy with the defaults filled in and where the key
    /// still has to come from.
    pub fn resolved(&self) -> (Provider, KeySource) {
        let mut p = self.clone();
        if p.model.trim().is_empty() {
            p.model = p.kind.api().default_model().into();
        }
        if p.api_key.as_deref().is_some_and(|k| !k.trim().is_empty()) {
            return (p, KeySource::Ready);
        }
        if let Some(cmd) = p.api_key_cmd.as_deref().filter(|c| !c.trim().is_empty()) {
            let cmd = cmd.to_owned();
            return (p, KeySource::Command(cmd));
        }
        let var = p.api_key_env.as_deref().or(p.kind.api().key_env());
        p.api_key = var
            .and_then(|v| std::env::var(v).ok())
            .map(|k| k.trim().to_owned())
            .filter(|k| !k.is_empty());
        (p, KeySource::Ready)
    }
}

/// Runs `cmd` through `sh`; the key is the first line of its output. Blocking: call off the
/// UI thread.
pub fn read_key(cmd: &str) -> Result<String, String> {
    let out = Command::new("sh")
        .args(["-c", cmd])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("API key command failed to start: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        let err = err.lines().next().unwrap_or("").trim();
        return Err(match out.status.code() {
            Some(code) if err.is_empty() => format!("API key command exited with {code}."),
            _ if err.is_empty() => "API key command was killed.".into(),
            _ => format!("API key command failed: {err}"),
        });
    }
    // `pass` prints the secret on the first line, metadata after it.
    let out = String::from_utf8_lossy(&out.stdout);
    match out.lines().next().map(str::trim).filter(|k| !k.is_empty()) {
        Some(k) => Ok(k.to_owned()),
        None => Err("API key command printed nothing.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::views::ask::provider::ProviderKind;

    #[test]
    fn from_launcher_args() {
        let p: Provider = serde_json::from_value(serde_json::json!({
            "kind": "gemini",
            "api_key_cmd": "echo k",
            "daily_requests": 500
        }))
        .unwrap();
        assert_eq!(p.kind, ProviderKind::Gemini);
        let (p, key) = p.resolved();
        assert_eq!(p.model, "gemini-2.5-flash-lite");
        assert_eq!(key, KeySource::Command("echo k".into()));
    }

    #[test]
    fn inline_key_wins() {
        let mut p = Provider::gemini("inline", "");
        p.api_key_cmd = Some("echo cmd".into());
        let (p, key) = p.resolved();
        assert_eq!(key, KeySource::Ready);
        assert_eq!(p.api_key.as_deref(), Some("inline"));
    }

    #[test]
    fn env_key() {
        let p = Provider {
            api_key_env: Some("ASK_KEY_TEST_VAR".into()),
            ..Provider::default()
        };
        // SAFETY: only this test touches this variable.
        unsafe { std::env::set_var("ASK_KEY_TEST_VAR", " k1 \n") };
        assert_eq!(p.resolved().0.api_key.as_deref(), Some("k1"));
    }

    #[test]
    fn key_command() {
        assert_eq!(read_key("printf 'secret\\nurl: x\\n'").unwrap(), "secret");
        assert!(read_key("true").unwrap_err().contains("nothing"));
        assert!(
            read_key("echo nope >&2; exit 3")
                .unwrap_err()
                .contains("nope")
        );
        assert!(read_key("exit 2").unwrap_err().contains("2"));
    }
}

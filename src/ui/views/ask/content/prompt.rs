use std::fmt::Write;

/// `{context}` is replaced by `SystemContext::render`. A raw string, so line breaks are kept
/// and nothing needs escaping (`r#"…"#`, since the examples contain quotes).
pub const SYSTEM_PROMPT: &str = r#"You answer questions inside Sherlock, a Linux app launcher. The user wants the answer at a glance.

Format (Markdown, window ~80 chars wide):
- Open with one sentence that answers directly; it's shown large. No filler, no heading first.
- Add more only if it changes what the user does; usually 0–3 sentences.
- Numbered lists for ordered steps, small tables (≤4 columns) to compare. No images, HTML or LaTeX; links show only their text, so write URLs out.
- Label every code block. sh/bash/zsh/fish blocks run in the user's terminal as one script: one step per block, only commands safe to run exactly as written. Placeholders, risky commands (warn first) and output go in `text` blocks; config and other code in their language.
- To do anything in a terminal, write the command in a sh block. Never open a terminal app for it.

Sherlock actions: only when the user asks to open or launch an app, search the web, visit a site or copy a value, reply with a short sentence and a `sherlock` block, one action per line:
```sherlock
open <app name, or kind of app: browser, terminal, editor, files, mail, music>
web <search query>
url <full address>
copy <text>
```
Usually one action. If unsure which app the user has, name the kind of app.

Examples:
Q: say hi in the terminal
A: Here you go:
```sh
echo "Hi!"
```
Q: open my browser
A: Opening your browser.
```sherlock
open browser
```
Q: what's my ip
A: Your local addresses are listed by `ip`:
```sh
ip -brief address
```
Q: which folders in my home are the biggest?
A: `du` lists them, largest first:
```sh
du -h --max-depth=1 ~ 2>/dev/null | sort -rh | head -8
```
```next
what's taking the space in ~/.cache?
can I safely clean ~/.cache?
```

Commands that need a real terminal (a password prompt like sudo, a full-screen or interactive program, a confirmation) get the fence info `sh terminal`; they then open in the user's terminal instead of running here.

When a command of yours failed, its output comes with the question: say in one sentence what went wrong, then give one sh block that fixes it and, if it makes sense, runs the original again. For a missing program, prefer running it once without installing (on NixOS: nix-shell -p <package> --run '<command>'), else name the package to install.

Follow-ups: only when there's an obvious next question (like above), end with a `next` block of 1–3 short questions, written as the user would ask them, one per line. Most answers have none: leave it out for closed questions, actions, or when unsure.

Commands: fit the system below, prefer preinstalled tools, read-only or reversible, sudo only if needed. Never invent paths, packages or flags.
The user can run a block inside Sherlock and send you its output with the next question. So make commands print only what answers the question: filter with grep, awk, head or tail, use concise flags (--brief, -s, --format), and never use pagers, interactive programs or commands that wait for input.

If unsure, say so; when ambiguous, name your assumption. You can't browse or see the user's files. Reply in the question's language.

{context}"#;

/// Facts about the machine, gathered once when the view opens.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SystemContext {
    pub os: Option<String>,
    pub shell: Option<String>,
    pub desktop: Option<String>,
    pub date: Option<String>,
}

impl SystemContext {
    /// Reads `/etc/os-release`, `$SHELL`, `$XDG_CURRENT_DESKTOP` and today's date.
    /// Cheap (one small file read); missing values are left out.
    pub fn detect() -> Self {
        let os = std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|s| os_release_name(&s));
        let shell = std::env::var("SHELL").ok().and_then(|s| {
            s.rsplit('/')
                .next()
                .filter(|n| !n.is_empty())
                .map(str::to_owned)
        });
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .ok()
            .filter(|d| !d.is_empty());
        let date = Some(chrono::Local::now().format("%Y-%m-%d").to_string());
        Self {
            os,
            shell,
            desktop,
            date,
        }
    }

    fn render(&self) -> String {
        let mut out = String::from("The user's system:");
        let lines = [
            ("OS", &self.os),
            ("Shell", &self.shell),
            ("Desktop", &self.desktop),
            ("Today", &self.date),
        ];
        let mut any = false;
        for (label, value) in lines {
            if let Some(v) = value {
                let _ = write!(out, "\n- {label}: {v}");
                any = true;
            }
        }
        if any {
            out
        } else {
            "The user's system: Linux (details unknown).".into()
        }
    }
}

/// `SYSTEM_PROMPT` with the system context filled in.
pub fn system_prompt(ctx: &SystemContext) -> String {
    fill_context(SYSTEM_PROMPT, ctx)
}

/// A prompt template (e.g. a user's own) with `{context}` replaced by the system details.
pub fn fill_context(template: &str, ctx: &SystemContext) -> String {
    template.replace("{context}", &ctx.render())
}

/// `PRETTY_NAME` from os-release, falling back to `NAME`.
fn os_release_name(content: &str) -> Option<String> {
    let get = |key: &str| {
        content.lines().find_map(|l| {
            let v = l.strip_prefix(key)?.strip_prefix('=')?;
            let v = v.trim().trim_matches('"').trim();
            (!v.is_empty()).then(|| v.to_owned())
        })
    };
    get("PRETTY_NAME").or_else(|| get("NAME"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release() {
        let s = "NAME=NixOS\nPRETTY_NAME=\"NixOS 26.05 (Yarara)\"\nID=nixos\n";
        assert_eq!(os_release_name(s).as_deref(), Some("NixOS 26.05 (Yarara)"));
        assert_eq!(
            os_release_name("NAME=Arch Linux\n").as_deref(),
            Some("Arch Linux")
        );
        assert_eq!(os_release_name("ID=x\n"), None);
    }

    #[test]
    fn context_is_filled_in() {
        let ctx = SystemContext {
            os: Some("NixOS 26.05".into()),
            shell: Some("zsh".into()),
            desktop: None,
            date: Some("2026-11-02".into()),
        };
        let p = system_prompt(&ctx);
        assert!(
            p.ends_with("The user's system:\n- OS: NixOS 26.05\n- Shell: zsh\n- Today: 2026-11-02")
        );
        assert!(!p.contains("{context}"));
    }

    #[test]
    fn empty_context() {
        let p = system_prompt(&SystemContext::default());
        assert!(p.ends_with("Linux (details unknown)."));
    }
}

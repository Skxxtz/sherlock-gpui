/// GPUI's names for named keys.
const NAMED: &[&str] = &[
    "enter",
    "escape",
    "backspace",
    "delete",
    "tab",
    "space",
    "insert",
    "home",
    "end",
    "pageup",
    "pagedown",
    "up",
    "down",
    "left",
    "right",
];

fn key_alias(key: &str) -> &str {
    match key {
        "return" | "ret" => "enter",
        "esc" => "escape",
        "del" => "delete",
        "bs" => "backspace",
        "ins" => "insert",
        "pgup" | "page_up" => "pageup",
        "pgdn" | "pgdown" | "page_down" => "pagedown",
        "arrowup" => "up",
        "arrowdown" => "down",
        "arrowleft" => "left",
        "arrowright" => "right",
        other => other,
    }
}

fn modifier(m: &str) -> Option<&'static str> {
    Some(match m {
        "ctrl" | "control" => "ctrl",
        "alt" | "option" | "opt" => "alt",
        "shift" => "shift",
        "super" | "cmd" | "win" | "meta" | "platform" => "cmd",
        "fn" | "function" => "fn",
        _ => return None,
    })
}

fn valid_key(key: &str) -> bool {
    NAMED.contains(&key)
        || key == "<digit>"
        || key.chars().count() == 1
        || key
            .strip_prefix('f')
            .and_then(|n| n.parse::<u8>().ok())
            .is_some_and(|n| (1..=24).contains(&n))
}

/// One keystroke (`"Ctrl-Return"` → `"ctrl-enter"`). The key is the last part; `"ctrl--"` is
/// ctrl plus minus.
fn normalize_keystroke(stroke: &str) -> Result<String, String> {
    let lower = stroke.to_lowercase();
    let (mods, key) = match lower.strip_suffix("--") {
        Some(mods) => (mods, "-"),
        None => lower.rsplit_once('-').unwrap_or(("", &lower)),
    };
    let key = key_alias(key);
    if !valid_key(key) {
        return Err(format!("unknown key `{key}` in `{stroke}`"));
    }
    let mut parts = Vec::new();
    for m in mods.split('-').filter(|m| !m.is_empty()) {
        parts.push(modifier(m).ok_or_else(|| format!("unknown modifier `{m}` in `{stroke}`"))?);
    }
    parts.dedup();
    Ok(parts.into_iter().chain([key]).collect::<Vec<_>>().join("-"))
}

/// A config key: an optional context and keystrokes separated by spaces
/// (`"variable.Tab"` → `"variable.tab"`, `"ctrl-k ctrl-c"` stays a sequence).
pub fn normalize(key: &str) -> Result<String, String> {
    let (scope, keys) = crate::app::bindings::split_scope(key);
    let keys = keys
        .split_whitespace()
        .map(normalize_keystroke)
        .collect::<Result<Vec<_>, _>>()?
        .join(" ");
    Ok(match scope {
        Some(scope) => format!("{scope}.{keys}"),
        None => keys,
    })
}

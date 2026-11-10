use crate::utils::url_detection::is_url;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verb {
    Open,
    Web,
    Url,
    Copy,
}

impl Verb {
    pub const ALL: [Verb; 4] = [Verb::Open, Verb::Web, Verb::Url, Verb::Copy];

    pub fn keyword(self) -> &'static str {
        match self {
            Verb::Open => "open",
            Verb::Web => "web",
            Verb::Url => "url",
            Verb::Copy => "copy",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|v| v.keyword().eq_ignore_ascii_case(word))
    }
}

/// An installed desktop app `open` can launch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub terminal: bool,
    /// Lowercased GenericName, Categories and Keywords from the desktop file.
    pub tags: Vec<String>,
}

/// What an action line turned out to be.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    App(AppEntry),
    Web(String),
    Url(String),
    Copy(String),
    /// Can't be run; the reason is shown instead.
    Invalid(String),
}

impl Resolved {
    pub fn is_valid(&self) -> bool {
        !matches!(self, Resolved::Invalid(_))
    }

    /// Row label and detail, e.g. ("Open", "Firefox").
    pub fn label(&self) -> (&'static str, &str) {
        match self {
            Resolved::App(app) => ("Open", &app.name),
            Resolved::Web(q) => ("Search the web", q),
            Resolved::Url(u) => ("Open link", u),
            Resolved::Copy(t) => ("Copy", t),
            Resolved::Invalid(reason) => ("Can't run", reason),
        }
    }
}

/// Parses and resolves every non-empty line of a `sherlock` block.
pub fn resolve_block(body: &str, apps: &[AppEntry]) -> Vec<Resolved> {
    body.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|line| resolve_line(line, apps))
        .collect()
}

fn resolve_line(line: &str, apps: &[AppEntry]) -> Resolved {
    let (word, arg) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
    let arg = unquote(arg.trim());
    let Some(verb) = Verb::parse(word) else {
        return Resolved::Invalid(format!("unknown action “{word}”"));
    };
    if arg.is_empty() {
        return Resolved::Invalid(format!("“{word}” needs an argument"));
    }
    match verb {
        Verb::Open => match find_app(arg, apps) {
            Some(app) => Resolved::App(app.clone()),
            None => Resolved::Invalid(format!("no installed app “{arg}”")),
        },
        Verb::Web => Resolved::Web(arg.to_owned()),
        Verb::Url if is_url(arg) => Resolved::Url(arg.to_owned()),
        Verb::Url => Resolved::Invalid(format!("not a link: {arg}")),
        Verb::Copy => Resolved::Copy(arg.to_owned()),
    }
}

/// Strips one pair of matching quotes, as models like to quote arguments.
fn unquote(s: &str) -> &str {
    for q in ['"', '\'', '`'] {
        if let Some(inner) = s.strip_prefix(q).and_then(|r| r.strip_suffix(q)) {
            return inner.trim();
        }
    }
    s
}

/// Best match: exact name, a name starting with the query, the executable's name, a name
/// containing it, then the kind of app (GenericName, category or keyword, e.g. "browser").
/// Ties go to the shortest name.
pub fn find_app<'a>(query: &str, apps: &'a [AppEntry]) -> Option<&'a AppEntry> {
    let q = query.to_lowercase();
    let kinds = kind_tags(&q);
    let binary = |app: &AppEntry| {
        let first = app.exec.split_whitespace().next().unwrap_or("");
        first.rsplit('/').next().unwrap_or(first).to_lowercase()
    };
    let rank = |app: &AppEntry| {
        let name = app.name.to_lowercase();
        if name == q {
            Some(0)
        } else if name.starts_with(&q) {
            Some(1)
        } else if binary(app) == q {
            Some(2)
        } else if q.len() >= 3 && name.contains(&q) {
            Some(3)
        } else if app
            .tags
            .iter()
            .any(|t| *t == q || kinds.contains(&t.as_str()))
        {
            Some(4)
        } else {
            None
        }
    };
    apps.iter()
        .filter_map(|a| rank(a).map(|r| (r, a.name.len(), a)))
        .min_by_key(|(r, len, _)| (*r, *len))
        .map(|(_, _, a)| a)
}

/// Categories a common word for a kind of app stands for (freedesktop main/additional
/// categories, lowercased). The word itself also matches GenericName and Keywords.
fn kind_tags(q: &str) -> &'static [&'static str] {
    match q {
        "browser" | "web browser" | "internet" => &["webbrowser"],
        "terminal" | "console" | "shell" => &["terminalemulator"],
        "editor" | "text editor" => &["texteditor"],
        "ide" | "code editor" => &["ide", "texteditor"],
        "files" | "file manager" | "explorer" => &["filemanager"],
        "mail" | "email" | "e-mail" => &["email"],
        "music" | "music player" => &["music", "player"],
        "video" | "video player" | "media player" => &["video", "player"],
        "calculator" => &["calculator"],
        "chat" | "messenger" => &["chat", "instantmessaging"],
        "image viewer" | "photos" => &["viewer", "photography"],
        "pdf" | "pdf viewer" | "document viewer" => &["viewer"],
        "settings" => &["settings"],
        "calendar" => &["calendar"],
        "notes" => &["notes"],
        _ => &[],
    }
}

/// Tags of a desktop file: GenericName (whole and its words), Categories and Keywords, from the
/// `[Desktop Entry]` group, unlocalized keys only.
pub fn desktop_tags(content: &str) -> Vec<String> {
    let mut tags = Vec::new();
    let mut in_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().to_lowercase();
        match key.trim() {
            "GenericName" => {
                tags.extend(value.split_whitespace().map(str::to_owned));
                tags.push(value);
            }
            "Categories" | "Keywords" => {
                tags.extend(
                    value
                        .split(';')
                        .map(str::trim)
                        .filter(|v| !v.is_empty())
                        .map(str::to_owned),
                );
            }
            _ => {}
        }
    }
    tags.sort();
    tags.dedup();
    tags
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<AppEntry> {
        let app = |name: &str, exec: &str, tags: &[&str]| AppEntry {
            name: name.into(),
            exec: exec.into(),
            terminal: false,
            tags: tags.iter().map(|t| t.to_string()).collect(),
        };
        vec![
            app(
                "Firefox",
                "firefox %u",
                &["web browser", "webbrowser", "network"],
            ),
            app("Firefox Developer Edition", "firefox-dev", &["webbrowser"]),
            app(
                "Visual Studio Code",
                "/usr/bin/code --unity-launch %F",
                &["texteditor", "ide"],
            ),
            app("Files", "nautilus", &["filemanager"]),
            app("Obsidian", "obsidian", &["office", "notes", "markdown"]),
            app("kitty", "kitty", &["terminalemulator", "system"]),
        ]
    }

    #[test]
    fn app_matching() {
        let apps = apps();
        assert_eq!(find_app("firefox", &apps).unwrap().name, "Firefox");
        assert_eq!(find_app("FIRE", &apps).unwrap().name, "Firefox");
        assert_eq!(find_app("code", &apps).unwrap().name, "Visual Studio Code");
        assert_eq!(
            find_app("studio", &apps).unwrap().name,
            "Visual Studio Code"
        );
        assert!(find_app("gimp", &apps).is_none());
        assert!(
            find_app("fi", &apps).is_some(),
            "prefix match on short queries"
        );
        assert!(
            find_app("le", &apps).is_none(),
            "no substring match under 3 chars"
        );
    }

    #[test]
    fn kind_of_app() {
        let apps = apps();
        assert_eq!(find_app("browser", &apps).unwrap().name, "Firefox");
        assert_eq!(find_app("Web Browser", &apps).unwrap().name, "Firefox");
        assert_eq!(find_app("terminal", &apps).unwrap().name, "kitty");
        assert_eq!(
            find_app("editor", &apps).unwrap().name,
            "Visual Studio Code"
        );
        assert_eq!(find_app("notes", &apps).unwrap().name, "Obsidian");
        assert_eq!(find_app("file manager", &apps).unwrap().name, "Files");
        assert!(find_app("spreadsheet", &apps).is_none());
    }

    #[test]
    fn desktop_file_tags() {
        let file = "[Desktop Entry]\nName=Firefox\nGenericName=Web Browser\nGenericName[de]=Webbrowser\n\
                    Categories=GNOME;GTK;Network;WebBrowser;\nKeywords=Internet;WWW;\n\
                    [Desktop Action new-window]\nCategories=Ignored;\n";
        assert_eq!(
            desktop_tags(file),
            [
                "browser",
                "gnome",
                "gtk",
                "internet",
                "network",
                "web",
                "web browser",
                "webbrowser",
                "www"
            ]
        );
    }

    #[test]
    fn block_lines() {
        let r = resolve_block(
            "open \"firefox\"\n\nweb rust gpui\nurl https://zed.dev\ncopy 10.0.0.1\nlaunch x\nurl not a link\nopen\n# note",
            &apps(),
        );
        assert_eq!(r.len(), 7);
        assert!(matches!(&r[0], Resolved::App(a) if a.name == "Firefox"));
        assert_eq!(r[1], Resolved::Web("rust gpui".into()));
        assert_eq!(r[2], Resolved::Url("https://zed.dev".into()));
        assert_eq!(r[3], Resolved::Copy("10.0.0.1".into()));
        assert!(!r[4].is_valid() && !r[5].is_valid() && !r[6].is_valid());
    }
}

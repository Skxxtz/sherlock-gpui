use core::fmt;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::{
    launcher::variant_type::LauncherVariant, ui::widgets::RenderableChild, utils::files::desktop_id,
};

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct ItemKey(Arc<str>);

impl From<String> for ItemKey {
    fn from(s: String) -> Self {
        Self(s.into())
    }
}

impl From<ItemKey> for String {
    fn from(value: ItemKey) -> Self {
        value.0.to_string()
    }
}

impl ItemKey {
    /// Single entry point: the launcher type decides how its items are identified.
    pub fn of(child: &RenderableChild) -> Option<Self> {
        if let RenderableChild::App { launcher, inner } = child {
            match launcher.launcher_type.variant() {
                LauncherVariant::Apps => return inner.desktop_file.as_deref().and_then(Self::app),
                LauncherVariant::Categories | LauncherVariant::Commands => {
                    return inner.name.as_deref().map(Self::command);
                }
                _ => {} // other App entries: fall through to the launcher key
            }
        }

        let id = child
            .with_launcher(|l| launcher_id(l.launcher_type.variant().as_str(), l.name.as_deref()));
        Some(Self::launcher(&id))
    }

    /// `<namespace>:<id>`. Namespaces keep e.g. a command named "Firefox" apart from the Firefox
    /// desktop app.
    pub fn new(namespace: &str, id: &str) -> Self {
        Self(format!("{namespace}:{id}").into())
    }

    /// Desktop app, keyed by its XDG desktop file ID.
    pub fn app(desktop_file: &std::path::Path) -> Option<Self> {
        desktop_id(desktop_file).map(|id| Self::new("app", &id))
    }

    /// Entry from a `commands` / `categories` map, keyed by its map key.
    /// Same key in to launchers = same history; they name the same thing.
    pub fn command(name: &str) -> Self {
        Self::new("cmd", name)
    }

    /// Launcher that is the selected result (calculator, weather, ...)
    pub fn launcher(id: &str) -> Self {
        Self::new("launcher", id)
    }

    /// Action belonging to this item (desktop action, context action, ...).
    pub fn action(&self, name: &str) -> Self {
        Self(format!("{}#{name}", self.0).into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ItemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Debug for ItemKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ItemKey({})", self.0)
    }
}

/// Stable launcher ID: `variant:name`, or just `variant` for unnamed ones.
///
/// The alias is deliberately not used: it's a mode prefix that several
/// launchers may share (e.g. two launchers both under `app`), not an identity.
pub fn launcher_id(variant: &str, name: Option<&str>) -> String {
    match name {
        Some(n) => format!("{variant}:{n}"),
        None => variant.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_ids() {
        let id = |p: &str| desktop_id(std::path::Path::new(p));
        assert_eq!(
            id("/usr/share/applications/firefox.desktop").as_deref(),
            Some("firefox.desktop")
        );
        assert_eq!(
            id("/usr/share/applications/kde/org.kde.dolphin.desktop").as_deref(),
            Some("kde-org.kde.dolphin.desktop")
        );
        // NixOS profile path: unresolved, no store hash
        assert_eq!(
            id("/run/current-system/sw/share/applications/firefox.desktop").as_deref(),
            Some("firefox.desktop")
        );
        assert_eq!(
            id("/home/u/.local/share/flatpak/exports/share/applications/org.mozilla.firefox.desktop")
                .as_deref(),
            Some("org.mozilla.firefox.desktop")
        );
        assert_eq!(id("/tmp/firefox.desktop"), None);
    }

    #[test]
    fn keys() {
        let app = ItemKey::app(std::path::Path::new(
            "/usr/share/applications/firefox.desktop",
        ))
        .unwrap();
        assert_eq!(app.as_str(), "app:firefox.desktop");
        assert_eq!(
            app.action("New Private Window").as_str(),
            "app:firefox.desktop#New Private Window"
        );
        assert_eq!(ItemKey::command("GitHub").as_str(), "cmd:GitHub");
        assert_ne!(ItemKey::command("Firefox"), ItemKey::new("app", "Firefox"));
    }

    #[test]
    fn launcher_ids() {
        assert_eq!(launcher_id("web", Some("Web Search")), "web:Web Search");
        // same name, different variant → distinct
        assert_ne!(
            launcher_id("apps", Some("App Launcher")),
            launcher_id("commands", Some("App Launcher"))
        );
        assert_eq!(launcher_id("event", None), "event");
    }
}

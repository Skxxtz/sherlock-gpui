use std::sync::Arc;

use gpui::SharedString;

use crate::launcher::LauncherConfig;

#[derive(Debug, PartialEq, Clone, Default)]
pub enum LauncherMode {
    /// All launchers. Shown as "Search" while there's a query.
    #[default]
    Home,
    Alias {
        short: SharedString,
        name: SharedString,
        launcher: Arc<LauncherConfig>,
    },
}

#[derive(Debug, PartialEq)]
pub enum Route {
    /// Keep the mode; the query is a search.
    Stay,
    /// Switch the current view to this mode and clear the input.
    Switch(LauncherMode),
    /// Open the mode's launcher as a new view and clear the input.
    Push(LauncherMode),
}

impl LauncherMode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Home => "all",
            Self::Alias { short, .. } => short.as_ref(),
        }
    }

    /// The mode label.
    pub fn display_str(&self, searching: bool) -> SharedString {
        match self {
            Self::Home if searching => "Search".into(),
            Self::Home => "All".into(),
            Self::Alias { name, .. } => name.clone(),
        }
    }

    pub fn launcher(&self) -> Option<&Arc<LauncherConfig>> {
        match self {
            Self::Home => None,
            Self::Alias { launcher, .. } => Some(launcher),
        }
    }

    /// Where `query` leads from this mode.
    /// - alias + <space> → switches to that alias or opens the launcher as a view
    /// - otherwise its a search and changes nothing
    pub fn route(&self, query: &str, modes: &[Self]) -> Route {
        let Some(alias) = query.strip_suffix(' ').filter(|a| !a.is_empty()) else {
            return Route::Stay;
        };
        let found = modes.iter().find(
            |mode| matches!(mode, Self::Alias { short, .. } if short.eq_ignore_ascii_case(alias)),
        );
        match found {
            Some(mode) if mode.launcher().is_some_and(|l| l.needs_stack_push()) => {
                Route::Push(mode.clone())
            }
            Some(mode) => Route::Switch(mode.clone()),
            None => Route::Stay,
        }
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;

    fn alias(short: &'static str) -> LauncherMode {
        LauncherMode::Alias {
            short: short.into(),
            name: short.into(),
            launcher: Arc::new(LauncherConfig::default()),
        }
    }

    #[test]
    fn plain_queries_stay() {
        let modes = [alias("s")];
        assert_eq!(LauncherMode::Home.route("s", &modes), Route::Stay);
        assert_eq!(LauncherMode::Home.route(" ", &modes), Route::Stay);
        assert_eq!(LauncherMode::Home.route("x ", &modes), Route::Stay);
    }

    #[test]
    fn alias_switches_from_any_mode_even_pasted() {
        let modes = [alias("s"), alias("calc")];
        assert_eq!(
            LauncherMode::Home.route("s ", &modes),
            Route::Switch(alias("s"))
        );
        assert_eq!(
            LauncherMode::Home.route("CALC ", &modes),
            Route::Switch(alias("calc"))
        );
        assert_eq!(
            alias("s").route("calc ", &modes),
            Route::Switch(alias("calc"))
        );
    }
}

use std::sync::Arc;

use gpui::{AnyElement, SharedString};

pub mod app_data;
pub mod calc_data;
pub mod weather_data;

use crate::{
    launcher::{ExecMode, Launcher, LauncherType, weather_launcher::WeatherData},
    loader::utils::{AppData, ApplicationAction, ExecVariable},
    utils::{config::HomeType, errors::SherlockError},
};

use calc_data::CalcData;

/// Creates enum RenderableChild,
/// ## Example:
/// ```
/// renderable_enum! {
///     enum RenderableChild {
///         AppLike(AppData),
///         WeatherLike(WeatherData),
///     }
/// }
/// ```
macro_rules! renderable_enum {
    (
        enum $name:ident {
            $($variant:ident($inner:ty)),* $(,)?
        }
    ) => {
        #[derive(Clone)]
        pub enum $name {
            $(
                $variant {
                    launcher: Arc<Launcher>,
                    inner: $inner,
                }
            ),*
        }

        impl<'a> RenderableChildDelegate<'a> for $name {
            fn render(&self, is_selected: bool) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, is_selected)),*
                }
            }

            fn execute(&self, keyword: &str, variables: &[(SharedString, SharedString)]) -> Result<bool, SherlockError> {
                match self {
                    $(Self::$variant {inner, launcher} => inner.execute(launcher, keyword, variables)),*
                }
            }

            fn execute_action(&self, action: &'a ApplicationAction) -> Result<bool, SherlockError> {
                match self {
                    $(Self::$variant {launcher, ..} => {
                        let what = ExecMode::from_app_action(action, launcher);
                        launcher.execute(&what, "", &[])
                    }),*
                }
            }

            fn search(&'a self) -> &'a str {
                match self {
                    $(Self::$variant {inner, launcher} => inner.search(launcher)),*
                }
            }


            fn vars(&self) -> Option<&[ExecVariable]> {
                match self {
                    Self::AppLike { inner, .. } => Some(&inner.vars), // Works for Vec or SmallVec
                    _ => None,
                }
            }

            fn actions(&self) -> Option<Arc<[Arc<ApplicationAction>]>> {
                match self {
                    Self::AppLike { inner, ..} => Some(inner.actions.clone()),
                    _ => None
                }
            }
        }

        impl<'a> LauncherValues<'a> for $name {
            fn name(&'a self) -> Option<&'a str> {
                self.launcher().name.as_deref()
            }

            fn display_name(&self) -> Option<SharedString> {
                self.launcher().display_name.clone()
            }

            fn home(&self) -> HomeType {
                self.launcher().home
            }

            fn is_async(&self) -> bool {
                self.launcher().r#async
            }

            fn alias(&'a self) -> Option<&'a str> {
                self.launcher().alias.as_deref()
            }

            fn priority(&self) -> f32 {
                match self {
                    $(Self::$variant {inner, launcher} => inner.priority(launcher)),*
                }
            }

            fn launcher_type(&'a self) -> &'a LauncherType {
                &self.launcher().launcher_type
            }
        }

        impl <'a> $name {
            #[inline(always)]
            fn launcher(&'a self) -> &'a Launcher {
                match self {
                    $(Self::$variant {launcher, ..} => &launcher),*
                }
            }
        }

        impl RenderableChild {
            pub fn based_show(&self, query: &str) -> Option<bool> {
                match self {
                    Self::CalcLike { inner, ..} => Some(inner.based_show(query)),
                    _ => None
                }
            }
        }
    };
}
renderable_enum! {
    enum RenderableChild {
        AppLike(AppData),
        WeatherLike(WeatherData),
        CalcLike(CalcData),
    }
}

impl RenderableChild {
    pub fn get_exec(&self) -> Option<String> {
        match self {
            Self::AppLike { inner, launcher } => inner.get_exec(launcher),
            _ => None,
        }
    }
}

pub trait RenderableChildDelegate<'a> {
    fn render(&self, is_selected: bool) -> AnyElement;
    fn execute(
        &self,
        keyword: &str,
        variables: &[(SharedString, SharedString)],
    ) -> Result<bool, SherlockError>;
    fn execute_action(&self, action: &'a ApplicationAction) -> Result<bool, SherlockError>;
    fn search(&'a self) -> &'a str;
    fn vars(&self) -> Option<&[ExecVariable]>;
    fn actions(&self) -> Option<Arc<[Arc<ApplicationAction>]>>;
}

#[allow(dead_code)]
pub trait LauncherValues<'a> {
    fn name(&'a self) -> Option<&'a str>;
    fn display_name(&self) -> Option<SharedString>;
    fn alias(&'a self) -> Option<&'a str>;
    fn priority(&self) -> f32;
    fn is_async(&self) -> bool;
    fn home(&self) -> HomeType;
    fn launcher_type(&'a self) -> &'a LauncherType;
}

pub trait RenderableChildImpl<'a> {
    fn render(&self, launcher: &Arc<Launcher>, is_selected: bool) -> AnyElement;
    fn execute(
        &self,
        launcher: &Arc<Launcher>,
        keyword: &str,
        variables: &[(SharedString, SharedString)],
    ) -> Result<bool, SherlockError>;
    fn priority(&self, launcher: &Arc<Launcher>) -> f32;
    fn search(&'a self, launcher: &Arc<Launcher>) -> &'a str;
}

pub trait SherlockSearch {
    /// Both self and substring should already be lowercased to increase performance
    fn fuzzy_match<'a>(&'a self, substring: &'a str) -> bool;
}

impl<T: AsRef<str>> SherlockSearch for T {
    fn fuzzy_match(&self, pattern: &str) -> bool {
        let t_bytes = self.as_ref().as_bytes();
        let p_bytes = pattern.as_bytes();

        // Early return for empty bytes
        if p_bytes.is_empty() {
            return true;
        }
        if t_bytes.is_empty() {
            return false;
        }

        let mut current_target = t_bytes;

        // memchr find first search byte
        while let Some(pos) = memchr::memchr(p_bytes[0], current_target) {
            if sequential_check(p_bytes, &current_target[pos..], 5) {
                return true;
            }
            // Move past the current match to find the next possible start
            if pos + 1 >= current_target.len() {
                break;
            }
            current_target = &current_target[pos + 1..];
        }

        false
    }
}

fn sequential_check(pattern: &[u8], target: &[u8], window_size: usize) -> bool {
    // pattern[0] was already matched by memchr at target[0]
    let mut t_idx = 1;

    // We start from the second character (index 1)
    for &pattern_char in &pattern[1..] {
        // The window starts at t_idx and ends at t_idx + window_size
        let limit = std::cmp::min(t_idx + window_size, target.len());
        let mut found = false;

        while t_idx < limit {
            if target[t_idx] == pattern_char {
                t_idx += 1; // Start searching for the NEXT char from here
                found = true;
                break;
            }
            t_idx += 1;
        }

        // If the inner loop finishes without finding the char, the chain is broken
        if !found {
            return false;
        }
    }

    true
}

use std::sync::Arc;

use gpui::AnyElement;

use crate::{
    launcher::Launcher, launcher::weather_launcher::WeatherData, loader::utils::AppData,
    utils::errors::SherlockError,
};

pub mod app_data;
pub mod weather_data;

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

        impl RenderableChildDelegate for $name {
            fn render(&self, icon: Option<Arc<std::path::Path>>, is_selected: bool) -> AnyElement {
                match self {
                    $(Self::$variant {inner, launcher} => inner.render(launcher, icon, is_selected)),*
                }
            }

            fn execute(&self, keyword: &str) -> Result<bool, SherlockError> {
                match self {
                    $(Self::$variant {inner, launcher} => inner.execute(launcher, keyword)),*
                }
            }

            fn priority(&self) -> f32 {
                match self {
                    $(Self::$variant {inner, launcher} => inner.priority(launcher)),*
                }
            }

            fn search(&self) -> String {
                match self {
                    $(Self::$variant {inner, launcher} => inner.search(launcher)),*
                }
            }

            fn icon(&self) -> Option<String> {
                match &self {
                    $(Self::$variant {inner, launcher} => inner.icon(launcher)),*
                }
            }
        }
    };
}
renderable_enum! {
    enum RenderableChild {
        AppLike(AppData),
        WeatherLike(WeatherData),
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

pub trait RenderableChildDelegate {
    fn render(&self, icon: Option<Arc<std::path::Path>>, is_selected: bool) -> AnyElement;
    fn execute(&self, keyword: &str) -> Result<bool, SherlockError>;
    fn priority(&self) -> f32;
    fn search(&self) -> String;
    fn icon(&self) -> Option<String>;
}

pub trait RenderableChildImpl {
    fn render(
        &self,
        launcher: &Arc<Launcher>,
        icon: Option<Arc<std::path::Path>>,
        is_selected: bool,
    ) -> AnyElement;
    fn execute(&self, launcher: &Arc<Launcher>, keyword: &str) -> Result<bool, SherlockError>;
    fn priority(&self, launcher: &Arc<Launcher>) -> f32;
    fn search(&self, launcher: &Arc<Launcher>) -> String;
    fn icon(&self, launcher: &Arc<Launcher>) -> Option<String>;
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

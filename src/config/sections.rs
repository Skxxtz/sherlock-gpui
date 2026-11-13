mod appearance;
mod apps;
mod backdrop;
mod behavior;
mod caching;
mod debug;
mod expand;
mod files;
mod keybinds;
mod runtime;
mod search_bar;
mod status_bar;
mod units;

pub use appearance::ConfigAppearance;
pub use apps::ConfigDefaultApps;
pub use backdrop::ConfigBackdrop;
pub use behavior::ConfigBehavior;
pub use caching::ConfigCaching;
pub use debug::ConfigDebug;
pub use expand::ConfigExpand;
pub use files::ConfigFiles;
pub use keybinds::ConfigKeybinds;
pub use runtime::Runtime;
pub use search_bar::SearchBarIcon;
pub use status_bar::StatusBar;
pub use units::ConfigUnits;

/// A part of the config
pub trait Section: Default {
    /// Fixes the values that were read fine but make no sense, with a warning for each.
    fn validate(&mut self, _warn: &mut Vec<String>) {}
}

/// Resets `field` to `default` when `bad` holds and emits a warning.
pub(super) fn check<T: std::fmt::Debug>(
    warn: &mut Vec<String>,
    key: &str,
    field: &mut T,
    bad: impl FnOnce(&T) -> bool,
    default: T,
) {
    if bad(field) {
        warn.push(format!("`{key}` = {field:?} is invalid, using {default:?}"));
        *field = default;
    }
}

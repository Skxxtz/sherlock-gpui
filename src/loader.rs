pub mod application_loader;
pub mod assets;
mod flag_loader;
mod icon;
mod launcher_loader;
pub mod named_items;
pub mod pipe;
mod setup;
pub mod utils;

pub struct Loader;
pub use icon::{CustomIconTheme, IconThemeGuard, IconType, find_desktop_icon, resolve_icon_path};
pub use launcher_loader::{LauncherLoadResult, LoadContext, RawLaunchers};
pub use setup::SetupResult;

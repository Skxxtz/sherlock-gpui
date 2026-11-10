//! - `ui`: the view, its input handling and rendering

pub mod content;
pub mod exec;
pub mod model;
pub mod provider;
pub mod store;
mod ui;

pub use ui::view::AskView;

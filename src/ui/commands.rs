mod adapter;
mod command;
mod dispatch;

pub use adapter::ActionCommands;
pub use command::{Command, Hint, Key, Pressed, always};
pub use dispatch::{dispatch, hints, run_id};

use std::path::PathBuf;

use crate::loader::flag_loader::parser::{ArgParser, ParsedArgs};

use super::Loader;

mod actions;
mod flags;
mod parser;
mod utils;

#[derive(PartialEq)]
pub enum DebugAction {
    Help,
    Version,
    GenerateDocs,
    Repair,
    ClearCache,
    Init { path: PathBuf, extension: String },
}

impl Loader {
    /// This loads the application flags.
    pub fn load_flags() -> Option<ParsedArgs> {
        ArgParser::from_env()
    }
}

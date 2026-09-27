use md_rs::components::Component;
use std::path::Path;

use crate::{
    docs::{
        book::{Book, BookEntry},
        configuration::Configuration,
        contributing::Contributing,
        installation::Installation,
        readme::Readme,
    },
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        string::TrimInPlace,
    },
};

pub mod book;
mod configuration;
mod contributing;
mod exec_variable;
mod installation;
pub mod launcher;
pub mod plugins;
mod readme;

pub trait Documentation {
    type Docs: Component;
    fn docs() -> Self::Docs;
    fn docs_md() -> String {
        let mut out = String::new();
        let _ = Self::docs().render(&mut out);
        out.trim_in_place();
        out
    }
}

pub struct SherlockDocumentation;
impl SherlockDocumentation {
    pub fn generate() {
        [
            Self::create_book,
            Self::write_readme,
            Self::write_contributing,
        ]
        .into_iter()
        .map(|f| (f)())
        .filter_map(Result::err)
        .for_each(|e| eprintln!("{:?}", e));
    }
    pub fn create_book() -> Result<(), SherlockMessage> {
        let dir: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/src/").as_ref();

        Book(vec![
            BookEntry::from(Installation),
            BookEntry::from(Configuration),
        ])
        .generate(dir)
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, dir.to_path_buf()),
                e
            )
        })
    }

    pub fn write_readme() -> Result<(), SherlockMessage> {
        let path: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md").as_ref();

        std::fs::write(path, Readme::docs_md()).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, path.to_path_buf()),
                e
            )
        })
    }

    pub fn write_contributing() -> Result<(), SherlockMessage> {
        let path: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/.github/CONTRIBUTING.md").as_ref();

        std::fs::write(path, Contributing::docs_md()).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, path.to_path_buf()),
                e
            )
        })
    }
}

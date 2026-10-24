use std::path::Path;

use md_rs::components::{Component, IntoComponent};

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
mod installation;
pub mod launcher;
pub mod plugins;
mod readme;
pub mod runtime_args;
mod test;

pub trait Documentation {
    type Docs: IntoComponent;
    fn docs() -> Self::Docs;
    fn docs_md() -> String {
        let mut out = String::new();
        let _ = Self::docs().into_component().render(&mut out);
        out.trim_in_place();
        out
    }
}

pub struct SherlockDocumentation {
    book: Book,
    readme: String,
    contributing: String,
}
impl SherlockDocumentation {
    fn new() -> Self {
        Self {
            book: Book(vec![
                BookEntry::from(Installation),
                BookEntry::from(Configuration),
            ]),
            readme: Readme::docs_md(),
            contributing: Contributing::docs_md(),
        }
    }

    #[cfg(all(test, feature = "docs"))]
    pub fn render() -> Vec<String> {
        let docs = Self::new();
        let mut out = Vec::from([docs.readme, docs.contributing]);
        out.extend(docs.book.render());
        out
    }

    pub fn write() -> Result<(), SherlockMessage> {
        let docs = Self::new();

        let book_dir: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/src/").as_ref();
        docs.book.write(book_dir).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, book_dir.to_path_buf()),
                e
            )
        })?;

        fn write_file<C: AsRef<[u8]>>(path: &Path, content: C) -> Result<(), SherlockMessage> {
            std::fs::write(path, content).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Write, path.to_path_buf()),
                    e
                )
            })
        }

        let readme_path: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/README.md").as_ref();
        write_file(readme_path, docs.readme)?;

        let contributing_path: &Path =
            concat!(env!("CARGO_MANIFEST_DIR"), "/.github/CONTRIBUTING.md").as_ref();
        write_file(contributing_path, docs.contributing)?;

        Ok(())
    }
}

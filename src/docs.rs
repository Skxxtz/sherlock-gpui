use std::path::Path;

use md_rs::{
    components::{Component, IntoComponent, ParentComponentExt, container::Container},
    md,
};

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
pub mod link_scope;
pub mod plugins;
mod readme;
pub mod runtime_args;
mod test;
pub mod utils;

pub trait Documentation {
    type Docs: IntoComponent;
    fn docs() -> Self::Docs;
    fn docs_md() -> String {
        let mut out = String::new();
        let _ = Self::docs().into_component().render(&mut out);
        out.trim_in_place();
        out
    }

    /// `docs()` embedded in another page: registers `Self` there, so `link_to::<Self>` lands
    /// on this section.
    fn section() -> Container
    where
        Self: Sized + 'static,
    {
        link_scope::register_here::<Self>(Self::docs_md);
        md().child(Self::docs())
    }
}

/// Everything rendered, links resolved.
pub struct SherlockDocumentation {
    pages: Vec<(Option<&'static str>, String)>,
    summary: String,
    readme: String,
    contributing: String,
}
impl SherlockDocumentation {
    fn book() -> Book {
        Book(vec![
            BookEntry::from(Installation),
            BookEntry::from(Configuration),
        ])
    }

    /// Renders the book, then README and CONTRIBUTING (outside the book: they link into
    /// `docs/src/`), then resolves every `link_to`. `Err`: types linked to but documented
    /// nowhere.
    fn new() -> Result<Self, String> {
        link_scope::reset();
        let book = Self::book();
        let pages = book.render_pages();
        let readme = Readme::docs_md();
        let contributing = Contributing::docs_md();

        let mut missing = Vec::new();
        let mut resolve = |text: &str, prefix: &str| {
            link_scope::resolve(text, prefix).unwrap_or_else(|names| {
                missing.extend(names);
                text.to_string()
            })
        };
        let docs = Self {
            pages: pages
                .iter()
                .map(|(file, text)| (*file, resolve(text, "")))
                .collect(),
            summary: book.summary_md(),
            readme: resolve(&readme, "docs/src/"),
            contributing: resolve(&contributing, "../docs/src/"),
        };
        if missing.is_empty() {
            Ok(docs)
        } else {
            missing.sort_unstable();
            missing.dedup();
            Err(format!("links to undocumented types: {}", missing.join(", ")))
        }
    }

    #[cfg(all(test, feature = "docs"))]
    pub fn render() -> Vec<String> {
        let docs = Self::new().unwrap_or_else(|e| panic!("{e}"));
        let mut out = Vec::from([docs.readme, docs.contributing, docs.summary]);
        out.extend(docs.pages.into_iter().map(|(_, text)| text));
        out
    }

    pub fn write() -> Result<(), SherlockMessage> {
        let docs = Self::new().map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("documentation".into()),
                e
            )
        })?;

        let book_dir: &Path = concat!(env!("CARGO_MANIFEST_DIR"), "/docs/src/").as_ref();
        let write_err = |e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Write, book_dir.to_path_buf()),
                e
            )
        };
        std::fs::write(book_dir.join("SUMMARY.md"), &docs.summary).map_err(write_err)?;
        for (file, text) in docs.pages.iter().filter_map(|(f, t)| Some((f.as_ref()?, t))) {
            std::fs::write(book_dir.join(file), text).map_err(write_err)?;
        }

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

use md_rs::{
    components::{
        ParentComponentExt,
        TextComponentExt,
        container::Container,
        heading::h2,
        span::{br, html_underline},
        span_nodes::p,
    },
    item,
    list_iter,
    md,
    p,
};

use crate::{
    docs::{
        book::{BookEntry, BookEntryBuilder, TopLevelEntry},
        commands::Commands,
    },
    launcher::LauncherConfig,
    ui::runtime_arguments::RuntimeArgument,
};

pub struct Configuration;
impl TopLevelEntry for Configuration {
    type Summary = Container;
    fn summary() -> Self::Summary {
        md!(
            h2(html_underline("Configuration")),
            p!(
                "Sherlock can be customized in many ways.",
                br(),
                "Choose one topic to get started:"
            ),
            list_iter!(
                Dash,
                Self::children()
                    .map(|child| item!(p().link(child.title, child.file.unwrap_or("#"))))
            )
        )
    }
    fn children() -> impl Iterator<Item = BookEntry> + 'static {
        [
            BookEntryBuilder::new_toplevel::<LauncherConfig>()
                .title("Launchers")
                .file("launchers.md")
                .build(),
            BookEntryBuilder::new::<RuntimeArgument>()
                .title("Runtime Arguments")
                .file("runtime_arguments.md")
                .build(),
            BookEntry::from(Commands),
        ]
        .into_iter()
    }
}

impl From<Configuration> for BookEntry {
    fn from(_: Configuration) -> Self {
        BookEntryBuilder::new_toplevel::<Configuration>()
            .title("Configuration")
            .file("configuration.md")
            .children(Configuration::children())
            .build()
    }
}

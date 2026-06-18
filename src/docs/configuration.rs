use md_rs::{
    components::{
        ParentComponentExt, TextComponentExt,
        container::Container,
        heading::h2,
        span::{br, html_underline},
        span_nodes::p,
    },
    item, list_iter, md, p,
};

use crate::{
    docs::book::{BookEntry, TopLevelEntry},
    launcher::LauncherConfig,
    loader::utils::ExecVariable,
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
            BookEntry::of::<LauncherConfig>()
                .with_title("Launchers")
                .with_file("launchers.md"),
            BookEntry::of::<ExecVariable>()
                .with_title("Exec Variables")
                .with_file("exec-variables.md"),
        ]
        .into_iter()
    }
}

impl From<Configuration> for BookEntry {
    fn from(_: Configuration) -> Self {
        Self {
            title: "Configuration",
            file: Some("configuration.md"),
            render_fn: Some(Configuration::summary_md),
            children: Configuration::children().collect(),
        }
    }
}

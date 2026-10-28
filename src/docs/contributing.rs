use md_rs::{
    components::{
        ParentComponentExt,
        code_block::codeblock,
        container::Container,
        heading::{h1, h2, h3},
        span::{bold, code, link},
    },
    item,
    list,
    md,
    p,
};

use crate::docs::Documentation;

pub(super) struct ContributingShort;
impl Documentation for ContributingShort {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h1("Contributing"),
            "Contributions are welcome! Please follow these guidelines:",
            md!(
                bold("Prerequisites"),
                p!(
                    "Ensure you have the latest stable Rust toolchain installed along with",
                    code("rustfmt"),
                    "and",
                    code("clippy"),
                    "."
                ),
            ),
            bold("Branching"),
            list!(
                Dash,
                p!(code("main"), ": stable releases only"),
                p!("Feature branches:", code("feat/your-feature")),
                p!("Feature branches:", code("fix/description")),
            ),
            md!(
                bold("Before opening a PR"),
                codeblock()
                    .lang("bash")
                    .line("cargo fmt")
                    .line("cargo clippy -- -D warnings")
                    .line("cargo test"),
            ),
            md!(
                bold("Releasing"),
                p!(
                    "Releases are automated via",
                    code("GitHub Actions"),
                    "on version tags:"
                ),
                codeblock()
                    .lang("bash")
                    .line("git tag v0.x.0")
                    .line("git push origin v0.x.0"),
            )
        )
    }
}

pub struct Contributing;
impl Documentation for Contributing {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h1("Contributing to Sherlock"),
            "Thanks for considering contributing to our project! \
            Here are some guidelines for submitting \
            issues and contributions.",
            section_reporting_bugs(),
            section_suggesting_features(),
            section_pr()
        )
    }
}

fn section_reporting_bugs() -> Container {
    md!(
        h2("Reporting Bugs and Issues"),
        "If you encounter a bug, please make sure to:",
        list!(
            Ordered,
            p!(
                bold("Search the Issues"),
                "to check if the bug \
                has already been reported.",
            ),
            p!(
                bold("Follow the Bug Report Template"),
                r#"by using the "Bug Report" issue template"#,
                "This will help us gather all the information \
                    we need to fix the issue efficiently.",
            )
        )
    )
}

fn section_suggesting_features() -> Container {
    md!(
        h2("Suggesting Features"),
        "We are always open to suggestiongs for new features. \
        To suggest a feature:",
        list!(
            Ordered,
            p!(
                bold("Search the Issues"),
                "to see if someone has already \
                suggested the same feature.",
            ),
            p!(
                bold("Use the Feature Request Template"),
                "when creating a new feature request."
            ),
        ),
        "Please include:",
        list!(
            Dash,
            "A detailed description of the feature",
            "Possible use caess for the feature",
            "Any relevant context",
        )
    )
}

fn section_pr() -> Container {
    let pr_guidelines = list!(
        Ordered,
        item!(
            p!(
                bold("Fork the repository"),
                "and create a new branch for your feature or bug fix.",
            ),
            list!(
                Dash,
                p!("Avoid working directly on the", code("main"), "branch",)
            )
        ),
        item!(
            p!(
                bold("Make sure your code follows Sherlock's coding conventions"),
                "and adds tests if necessary."
            ),
            list!(
                Dash,
                "If you're fixing a bug, try to include test that reproduce the issue.",
                "If you're adding a feature, try to provide \
                tests that cover your new functionality.",
            )
        ),
        item!(
            bold("Keep your commits clear and concise."),
            list!(
                Dash,
                "Write meaningful commit messages that  explain why \
                the change was made.",
                "Break large changes into smaller, more managagle commits.",
            )
        ),
        item!(
            bold("Update documentation"),
            list!(
                Dash,
                p!(
                    "If your PR introduces new features or changes existing \
                    functionality, make sure to update the relevant \
                    documentation.  Sherlock uses a custom  documentation \
                    renderer, provided by the",
                    code("Documentation"),
                    "trait. For more documentation, please see:",
                    link("documentation generator", "#"), // TODO
                )
            )
        ),
        item!(list!(
            Dash,
            "make sure your changes work properly and \
            don't break existing functionality.",
            "if applicable, run the project to verify that \
            everything is functioning as expected.",
        )),
    );

    let list_after_pr = list!(
        Ordered,
        item!(p!(
            bold("Fork"),
            "the repository and",
            bold("clone"),
            "it into your local machine.",
        )),
        item!(
            p!("Create a", bold("new branch"), "for your changes."),
            list!(
                Dash,
                p!("Example:", code("git checkout -b fix/bug-description")),
            )
        ),
        item!(
            p!(bold("Make your changes"), "and commit them to your branch"),
            list!(
                Dash,
                p!(
                    "Example:",
                    code(format!(
                        r#"git commit -m "fix(category): {}""#,
                        "fixed issue with bug description in README"
                    ))
                ),
            )
        ),
        item!(
            p!(bold("Push"), "your changes to your forked repository."),
            list!(
                Dash,
                p!("Example:", code("git push origin fix/bug-description")),
            )
        ),
        item!(p!(
            bold("Open a Pull Request"),
            "in the original repository from your fork. \
            Usually, PRs should be targeting the",
            code("dev"),
            "branch and",
            bold("not"),
            "the",
            code("main"),
            "branch. If you're createing a new feature where \
            others should also be able to contribute to, \
            feel free to mention @skxxtz to create a \
            new development branch for you.",
        )),
    );

    let general_guidelines = list!(
        Dash,
        "Be respectful of others",
        "Provide enough information in your issue or PR description \
        to help us understand the problemm or propsed feature",
        "If youre reporting a bug, please write steps to reproduce the issue.",
        "If your PR is ready for review, tag a maintainer or submit for review."
    );

    md!(
        h2("Pull Requests (PRs)"),
        "We appreciate your contributions! \
        When submitting a PR, please follow these guidelines:",
        pr_guidelines,
        h3("Steps for Submitting a PR:"),
        list_after_pr,
        h2("General Guidelines"),
        general_guidelines,
        "Thank you for your contributions!"
    )
}

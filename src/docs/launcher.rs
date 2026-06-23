use indoc::indoc;
use md_rs::{
    cached_component,
    components::{
        Component, ParentComponentExt,
        code_block::codeblock,
        container::Container,
        details::details,
        heading::{h1, h2, h3},
        hr::hr,
        raw::{Raw, raw},
        span::{bold, code, html_strong, italic},
        span_nodes::Paragraph,
        table::{Table, table},
    },
    md, p,
};

use crate::{
    docs::Documentation,
    launcher::{LauncherConfig, variant_type::LauncherType},
    utils::intent::CAPABILITY_DOCS,
};

pub mod plugin_launcher;

pub const BASE_FIELDS: &[FieldDoc] = &[
    FieldDoc {
        name: "name",
        ty: "string",
        required: false,
        default: None,
        description: "Display name of the launcher. Shown in the widget if set.",
    },
    FieldDoc {
        name: "alias",
        ty: "string",
        required: false,
        default: None,
        description: "Short trigger prefix (e.g. `app`) that scopes the launcher to alias-only mode.",
    },
    FieldDoc {
        name: "type",
        ty: "LauncherVariant",
        required: true,
        default: None,
        description: "The category and functional variant of the launcher.",
    },
    FieldDoc {
        name: "priority",
        ty: "u16",
        required: true,
        default: None,
        description: "Display order weight. Lower values appear first. `0` only appears in alias mode.",
    },
    FieldDoc {
        name: "limit",
        ty: "u16",
        required: false,
        default: None,
        description: "The number of items to display per launcher. Useful to limit the number of search results to the best `n` items.",
    },
    FieldDoc {
        name: "home",
        ty: "HomeType",
        required: false,
        default: Some("Home"),
        description: "Controls when the launcher is shown: `Home`, `OnlyHome`, `Search`, or `Persist`.",
    },
    FieldDoc {
        name: "exit",
        ty: "bool",
        required: false,
        default: Some("true"),
        description: "Whether Sherlock closes after the launcher is executed.",
    },
    FieldDoc {
        name: "shortcut",
        ty: "bool",
        required: false,
        default: Some("true"),
        description: "Whether a UI shortcut key is assigned to this launcher.",
    },
    FieldDoc {
        name: "spawn_focus",
        ty: "bool",
        required: false,
        default: Some("true"),
        description: "Whether this launcher can receive spawned focus.",
    },
    FieldDoc {
        name: "on_return",
        ty: "string",
        required: false,
        default: None,
        description: "Command or action executed when the user confirms this launcher.",
    },
    FieldDoc {
        name: "args",
        ty: "object",
        required: false,
        default: Some("{}"),
        description: "Launcher-specific arguments. Shape depends on the launcher type.",
    },
    FieldDoc {
        name: "binds",
        ty: "Bind[]",
        required: false,
        default: None,
        description: "Key bindings attached to this launcher.",
    },
    FieldDoc {
        name: "actions",
        ty: "ApplicationAction[]",
        required: false,
        default: None,
        description: "Primary context menu actions. Overwrites any actions defined in desktop files.",
    },
    FieldDoc {
        name: "add_actions",
        ty: "ApplicationAction[]",
        required: false,
        default: None,
        description: "Supplementary actions appended to the primary action list.",
    },
    FieldDoc {
        name: "variables",
        ty: "ExecVariable[]",
        required: false,
        default: None,
        description: "Runtime variable substitutions available to this launcher's commands.",
    },
];

pub trait LauncherDoc {
    fn doc() -> LauncherDocEntry;
}

pub struct LauncherDocEntry {
    pub name: &'static str,
    pub variant_name: &'static str,
    pub description: &'static str,
    pub args: &'static [FieldDoc],
    pub args_explanations: &'static [fn() -> Raw],
    pub inner_functions: &'static [InnerFunctionDoc],
    pub examples: &'static [Example],
    pub hidden: bool,
}

impl LauncherDocEntry {
    pub fn new() -> Self {
        Self {
            name: "",
            variant_name: "",
            description: "",
            args: &[],
            args_explanations: &[],
            inner_functions: &[],
            examples: &[],
            hidden: false,
        }
    }
    pub fn new_hidden(
        name: &'static str,
        variant_name: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            variant_name,
            description,
            hidden: true,
            ..Self::new()
        }
    }
    fn has_visible_functions(&self) -> bool {
        self.inner_functions.iter().any(|f| f.user_facing)
    }
}

pub struct InnerFunctionDoc {
    pub name: &'static str,
    pub identifier: &'static str,
    pub description: &'static str,
    pub user_facing: bool,
}

pub struct FieldDoc {
    pub name: &'static str,
    pub ty: &'static str,
    pub required: bool,
    pub default: Option<&'static str>,
    pub description: &'static str,
}

pub struct Example {
    pub description: &'static str,
    pub json: &'static str,
}

impl From<&LauncherDocEntry> for Container {
    fn from(value: &LauncherDocEntry) -> Self {
        md!(
            h2(value.name),
            format!("`type = {}`", value.variant_name),
            value.description,
        )
        .when(!value.args.is_empty(), |this| {
            this.child(h3("Args")).child(field_table(value.args))
        })
        .when(!value.args_explanations.is_empty(), |this| {
            this.children(value.args_explanations.iter().map(|f| f()))
        })
        .when(value.has_visible_functions(), |this| {
            this.child(h3("Inner Functions"))
                .child(function_table(value.inner_functions))
        })
        .when(!value.examples.is_empty(), |this| {
            this.child(h3("Examples"))
                .children(value.examples.iter().map(|ex| {
                    md!(
                        italic(ex.description),
                        codeblock().lang("json").content(ex.json.trim()),
                    )
                }))
        })
        .child(hr())
    }
}

fn field_table(fields: &[FieldDoc]) -> Table {
    table()
        .headers(["Field", "Type", "Required", "Default", "Description"])
        .rows(fields.iter().map(|f| {
            [
                format!("`{}`", f.name),
                format!("`{}`", f.ty),
                if f.required { "✓" } else { "" }.into(),
                f.default.unwrap_or("—").into(),
                f.description.into(),
            ]
        }))
}

fn function_table(functions: &[InnerFunctionDoc]) -> Table {
    table().headers(["Name", "Identifier", "Description"]).rows(
        functions.iter().filter(|f| f.user_facing).map(|f| {
            [
                f.name.into(),
                format!("`{}`", f.identifier),
                f.description.into(),
            ]
        }),
    )
}

impl Documentation for LauncherConfig {
    type Docs = Container;
    fn docs() -> Self::Docs {
        fn get_intro() -> Paragraph {
            p!(
                "Sherlock separates",
                bold("Launchers"),
                "(The Logic) from",
                bold("Widgets"),
                "(The View). One Launcher configuration can generate multiple Widgets \
                (like a weather tile and a clock tile), but they all follow the same",
                code("priority"),
                "and",
                code("home"),
                "rules."
            )
        }
        const EXAMPLE: &str = indoc! {"
        [Weather Launcher]
            [Widget] Weather Display
        [App Launcher]
            [Widget] App 1
            [Widget] App 2
            [Widget] App 3"};

        md!(
            h1("Launchers"),
            get_intro(),
            codeblock().content(EXAMPLE),
            p!(
                "The Widgets get sorted based by a tiered sort:",
                code("Launcher Priority"),
                "then",
                code("Search Score"),
                "then",
                code("Number of Executions"),
            ),
            h2("Shared Launcher Configuration"),
            h3("Fields"),
            field_table(BASE_FIELDS),
            hr(),
            LauncherType::docs()
        )
    }
}

pub fn capabilities_section() -> Raw {
    cached_component!(
        8 * 1024,
        md!(details()
            .summary(html_strong("Capabilities:"))
            .child(
                "Capabilities control what the calculator can compute. \
                Pass them via the `capabilities` arg:",
            )
            .child(
                codeblock()
                    .lang("json")
                    .content(r#"{ "capabilities": ["calc.math", "calc.units"] }"#),
            )
            .children(CAPABILITY_DOCS.iter().map(|cap| {
                details()
                    .summary(cap.name)
                    .child(format!("`{}`", cap.identifier))
                    .when(!cap.units.is_empty(), |this| {
                        this.child(
                            table().headers(["Unit", "Aliases", "Symbol"]).rows(
                                cap.units.iter().map(|u| {
                                    [u.name.into(), u.aliases.join(", "), u.symbol.into()]
                                }),
                            ),
                        )
                    })
            })),)
    )
}

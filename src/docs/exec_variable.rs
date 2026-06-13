use indoc::indoc;
use md_rs::{
    components::{
        ParentComponentExt,
        code_block::{CodeBlock, codeblock},
        container::Container,
        details::details,
        heading::{h1, h2, h3, h4},
        span::{bold, code, italic},
        span_nodes::Paragraph,
    },
    github::components::alert::tip,
    list, list_iter, md, p,
};

use crate::{docs::Documentation, ui::variables::runtime_argument::RuntimeArgument};

struct ExecVariableDoc {
    title: &'static str,
    name: &'static str,
    description: fn() -> Paragraph,
    example: &'static str,
}

const EXEC_VARIABLE_DOCS: &[ExecVariableDoc] = &[
    ExecVariableDoc {
        title: "String Input",
        name: "string_input",
        description: || "A plain text input field.".into(),
        example: r#"{ "type": "string_input", "value": "hello world" }"#,
    },
    ExecVariableDoc {
        title: "Password Input",
        name: "password_input",
        description: || "Like string_input but the value is masked in the UI.".into(),
        example: r#"{ "password_input": "sudo" }"#,
    },
    ExecVariableDoc {
        title: "Path Input",
        name: "path_input",
        description: || {
            p!(
                "A text input featuring path completion. \
                By default, completion paths are resolved relative to the",
                code("$HOME"),
                "directory. Starting the input with a",
                code("/"),
                "prefix will search from the system root instead.",
            )
        },
        example: r#"{ "path_input": "path" }"#,
    },
    ExecVariableDoc {
        title: "Command Input",
        name: "command_input",
        description: || {
            p!(
                "A text input featuring path completion. Unlike",
                code("path_input"),
                ", this will only look at executeable files. \
                First, it will look at the",
                code("$PATH"),
                ", then it will try to complete like",
                code("path_input"),
                "."
            )
        },
        example: r#"{ "command_input": "command" }"#,
    },
    ExecVariableDoc {
        title: "Choice Input",
        name: "choice",
        description: || {
            p!(
                "Presents the user with a predefined list of \
                options to select from. Each choice has a",
                code("label"),
                "shown in the UI and a",
                code("value"),
                "passed to the command."
            )
        },
        example: indoc! {r#"{
            "choice": {
                "name": "temperature",
                "choices": [
                    {"label": "5000", "value": "5000"},
                    {"label": "6000 <span color='#555555'><i>default</i></span>", "value": "6000"},
                    {"label": "7000", "value": "7000"},
                    {"label": "8000", "value": "8000"}
                ]
            }
        }"#},
    },
];

impl Documentation for RuntimeArgument {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md().child(h1("Exec Variables"))
            .child(ReplacementVariables::docs())
            .child(p!(
                "In Sherlock",
                bold("Exec Variables"),
                "are dynamic placeholders that allow you \
                to inject real-time arguments into your applications, scripts, \
                and commands right at the moment you launch them. Instead of \
                relying on hardcoded shortcuts,",
                italic("Exec Variables"),
                "turn your launcher into an interactive CLI shell."
            ))
            .children(EXEC_VARIABLE_DOCS.iter().map(|doc| {
                md!(
                    h2(doc.title),
                    code(doc.name),
                    (doc.description)(),
                    codeblock().lang("json").content(doc.example)
                )
            }))
    }
}

struct ReplacementVariableToken {
    title: Paragraph,
    body: Container,
    syntax_example: CodeBlock,
}

struct ReplacementVariables;
impl Documentation for ReplacementVariables {
    type Docs = Container;
    fn docs() -> Self::Docs {
        let tokens: Vec<ReplacementVariableToken> = vec![
            ReplacementVariableToken {
                title: p!(code("{keyword}")),
                body: md!("Replaces the token with the exact text \
                    currently typed into the search bar.",),
                syntax_example: codeblock().line("https://www.example.com/search?q={keyword}"),
            },
            ReplacementVariableToken {
                title: p!(code("{terminal}")),
                body: md!(
                    "Automatically resolbes the user-defined \
                    or system-detected default terminal emulator.",
                    tip().text(p!(
                        "Most terminal emulators close instant \
                        once their child process finishes executing. \
                        To keep the terminal window \
                        open after your command runs,\
                        wrap the execution string like this:",
                        code(r#"{terminal} sh -c "<command>; exec $SHELL""#),
                    )),
                ),
                syntax_example: codeblock().lang("json").line(
                    r#"{terminal} sh -c \"ssh {variable:user}@{variable:host}; exec $SHELL\""#,
                ),
            },
            ReplacementVariableToken {
                title: p!(code("{variable:<name>}")),
                body: md!(p!(
                    "Inserts the Exec Variable into the command. This token \
                    only works if a matching input field is explicitly \
                    declared in the configuration's",
                    code("variables"),
                    "array"
                )),
                syntax_example: codeblock().lang("json").content(indoc! {r#"
                    {
                        "variables": [
                            { "string_input": "query" }
                        ],
                        "exec": "https://example.com/search?q={variable:query}"
                    }
                "#}),
            },
            ReplacementVariableToken {
                title: p!(code("{prefix[<variable name>]:<prefix text>}")),
                body: md!(
                    "A conditional modifier token used used to \
                    handle optional inputs gracefully.",
                    list!(
                        Dash,
                        p!(
                            "If the specified variable",
                            bold("contains a value"),
                            ", the entire token evaluates to the",
                            code("<prefix text>"),
                            ".",
                        ),
                        p!(
                            "If the variable",
                            bold("is empty or unassigned"),
                            ", the entire token resolves to an emptry string",
                            code(r#""""#),
                            "."
                        ),
                    ),
                    "This is highly effective for injecting optional CLI flags \
                    or toggling between a website's landing pgae and its search index.",
                ),
                syntax_example: codeblock().lang("json").content(indoc! {r#"
                        {
                            "variables": [
                                { "string_input": "query" }
                            ],
                            "exec": "https://example.com/{prefix[query]:search?q=}{variable:query}"
                        }
                    "#}),
            },
        ];
        let example = codeblock().lang("json").content(indoc! {r#"
        {
            "name": "System & Network Utils",
            "type": "command",
            "args": {
                "commands": {
                    "SSH Tunnel": {
                        "icon": "sherlock-link",
                        "variables": [
                            { "string_input": "User" },
                            { "string_input": "Host" }
                        ],
                        "exec": "{terminal} ssh {variable:User}@{variable:Host}",
                        "search_string": "ssh"
                    },
                    "NordVPN Connect": {
                        "icon": "nordvpn",
                        "variables": [
                            { "choice": { "name": "Server", "choices": ["us", "uk", "de"] } }
                        ],
                        "exec": "{terminal} sh -c \"nordvpn c {variable:Server}; exec $SHELL\"",
                        "search_string": "nordvpn"
                    },
                    "NordVPN Daemon": {
                        "icon": "nordvpn",
                        "exec": "systemctl --user start nordvpnd",
                        "search_string": "nordvpn daemon"
                    }
                }
            },
            "priority": 1
        }"#});

        md!(
            h2("Replacement Variable Notation"),
            "The replacement variable notation allows you to \
                dynamically replace tokens in commands with contextual data–\
                such as the user's active search query, system environment settings \
                or runtime Exec Variables.",
            h3("Available Tokens"),
            list_iter!(
                Ordered,
                tokens.into_iter().map(|token| {
                    md().child(token.title)
                        .child(token.body)
                        .child(h4("Syntax Example:"))
                        .child(token.syntax_example)
                })
            ),
            details()
                .summary(bold("Complete Configuration Example:"))
                .child(
                    "Heres a practical look at how there replacement variables mesh \
                        inside a launcher configuration file.",
                )
                .child(example),
        )
    }
}

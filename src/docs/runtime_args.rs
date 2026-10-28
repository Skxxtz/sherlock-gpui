use indoc::indoc;
use md_rs::{
    components::{
        IntoComponent,
        ParentComponentExt,
        code_block::{CodeBlock, codeblock},
        container::Container,
        details::details,
        heading::{h1, h2, h3, h4},
        span::{bold, code, italic},
        span_nodes::Paragraph,
    },
    github::components::alert::tip,
    list,
    list_iter,
    md,
    p,
};

use crate::{
    docs::{
        Documentation,
        launcher::{FieldDoc, field_table},
    },
    ui::runtime_arguments::{
        RuntimeArgument,
        choice::ChoiceVar,
        command::CommandVar,
        number::NumberVar,
        password::PasswordVar,
        path::PathVar,
        string::StringVar,
    },
};

pub struct RuntimeArgumentDoc {
    pub title: &'static str,
    pub name: &'static str,
    pub description: fn() -> Paragraph,
    pub args: &'static [FieldDoc],
    pub example: &'static str,
}

impl IntoComponent for RuntimeArgumentDoc {
    type Comp = Container;
    fn into_component(self) -> Self::Comp {
        md!(
            h2(self.title),
            code(self.name),
            (self.description)(),
            codeblock().lang("json").content(self.example),
            field_table(self.args)
        )
    }
}

impl Documentation for RuntimeArgument {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md().child(h1("Runtime Arguments"))
            .child(ReplacementVariables::docs())
            .child(p!(
                "In Sherlock",
                bold("Runtime Arguments"),
                "are dynamic placeholders that allow you \
                to inject real-time arguments into your applications, scripts, \
                and commands right at the moment you launch them. Instead of \
                relying on hardcoded shortcuts,",
                italic("Runtime Arguments"),
                "turn your launcher into an interactive CLI shell."
            ))
            .children([
                StringVar::docs(),
                NumberVar::docs(),
                PathVar::docs(),
                CommandVar::docs(),
                ChoiceVar::docs(),
                PasswordVar::docs(),
            ])
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
                syntax_example: codeblock().lang("sh").line(
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
                or Runtime Arguments.",
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

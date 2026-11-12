use std::marker::PhantomData;

use md_rs::{
    components::{
        Component,
        ParentComponentExt,
        code_block::codeblock,
        container::Container,
        heading::{h1, h2, h3},
        span::{bold, code},
    },
    item,
    list_iter,
    md,
    p,
};

use crate::{
    app::bindings::split_scope,
    docs::{
        Documentation,
        book::{BookEntry, BookEntryBuilder, TopLevelEntry},
        link_scope::{self, link_to, link_to_name},
    },
    ui::{
        actions::{Back, ShortcutAction},
        commands::{Command, ComponentCommands, Key},
        components::{
            color_picker::ColorInput,
            number_input::NumberInput,
            search_bar::{self, TextInput},
            select::Select,
            tiny_input::TinyInput,
        },
        launcher::{LauncherView, context_menu::wrapper::ContextMenu},
        views::{SherlockView, settings::SettingsView},
    },
    utils::config::KeybindDefaults,
};

/// The "Commands" overview: what keys do and how to rebind them; one page per layer below it.
pub struct Commands;

const COMMANDS_EXAMPLE: &str = r#"[commands]
"ctrl-n" = "ask.new_chat"
"ctrl-k" = "launcher.open_context"
# stop a running answer, then let ctrl-c copy as usual
"ctrl-c" = "ask.stop;continue""#;

const BINDS_EXAMPLE: &str = r#"{
    "binds": [
        { "bind": "ctrl-n", "callback": "new_chat" },
        { "bind": "ctrl-c", "callback": "stop;continue" }
    ]
}"#;

/// A type whose commands get their own page, `CommandDocs<Self>`: link to the page with
/// `link_to::<CommandDocs<AskView>>(…)`, to one command with `link_to_name("ask.stop", …)`.
pub trait CommandLayer: 'static {
    const TITLE: &'static str;
    const FILE: &'static str;
    const INTRO: &'static str;
    fn commands() -> Vec<CommandDoc>;
}

/// What the docs show of a command.
pub struct CommandDoc {
    pub id: &'static str,
    pub label: &'static str,
    pub keys: &'static [Key],
}

fn command_docs<V>(commands: &[Command<V>]) -> Vec<CommandDoc> {
    commands
        .iter()
        .map(|c| CommandDoc {
            id: c.id,
            label: c.label,
            keys: c.keys,
        })
        .collect()
}

/// The page listing `L`'s commands, one section each (linkable by id).
pub struct CommandDocs<L>(PhantomData<L>);

impl<L: CommandLayer> Documentation for CommandDocs<L> {
    type Docs = Container;
    fn docs() -> Self::Docs {
        md!(
            h1(L::TITLE),
            p!(
                L::INTRO,
                "Commands on the same key are tried top to bottom; the first one available runs."
            ),
        )
        .children(L::commands().into_iter().map(|c| {
            let heading = h3(code(c.id));
            link_scope::register_name_here(c.id, &render(&heading));
            md!(heading, p!(bold(c.label), "· keys:", describe_keys(c.keys)))
        }))
    }
}

/// The "Commands" section of a launcher's docs (`LauncherDocEntry::commands`): every command of
/// its view, linked.
pub fn launcher_commands<L: CommandLayer>() -> Container {
    md!(
        h3("Commands"),
        p!(
            "Keys in this launcher's view run",
            link_to::<CommandDocs<L>>("these commands"),
            ". Rebind them in the launcher's",
            code("binds"),
            "(the part after the dot is enough:",
            code("new_chat"),
            ") or in",
            link_to::<Commands>("`[commands]`"),
            "."
        ),
        list_iter!(
            Dash,
            L::commands().into_iter().map(|c| item!(p!(
                link_to_name(c.id, format!("`{}`", c.id)),
                "–",
                c.label
            )))
        ),
    )
}

fn render(component: &impl Component) -> String {
    let mut out = String::new();
    let _ = component.render(&mut out);
    out
}

impl<L: CommandLayer> CommandDocs<L> {
    fn entry() -> BookEntry {
        BookEntryBuilder::new::<Self>()
            .title(L::TITLE)
            .file(L::FILE)
            .build()
    }

    /// One line of the overview: the page, linked, and what it covers.
    fn overview_item() -> md_rs::components::span_nodes::Paragraph {
        p!(link_to::<Self>(L::TITLE), "–", L::INTRO)
    }
}

impl CommandLayer for Select {
    const TITLE: &'static str = "Dropdown";
    const FILE: &'static str = "commands_select.md";
    const INTRO: &'static str = "An open dropdown (Settings). Typing a letter jumps to the next \
        option starting with it.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(Self::COMMANDS)
    }
}

impl CommandLayer for NumberInput {
    const TITLE: &'static str = "Number Field";
    const FILE: &'static str = "commands_number_field.md";
    const INTRO: &'static str = "A focused number field (Settings). Typing a digit edits the \
        value directly.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(Self::COMMANDS)
    }
}

impl CommandLayer for TinyInput {
    const TITLE: &'static str = "Text Field";
    const FILE: &'static str = "commands_text_field.md";
    const INTRO: &'static str = "A focused small text field (Settings, the color picker's hex \
        value). Keys it doesn't use go to the control around it.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(Self::COMMANDS)
    }
}

impl CommandLayer for ColorInput {
    const TITLE: &'static str = "Color Picker";
    const FILE: &'static str = "commands_color_picker.md";
    const INTRO: &'static str = "An open color picker (Settings). Typing a hex digit starts \
        editing the hex value.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(Self::COMMANDS)
    }
}

impl CommandLayer for ContextMenu {
    const TITLE: &'static str = "Context Menu";
    const FILE: &'static str = "commands_context_menu.md";
    const INTRO: &'static str = "While the context menu is open, it gets its keys first.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(LauncherView::MENU_COMMANDS)
    }
}

#[cfg(feature = "ai")]
impl CommandLayer for crate::ui::views::ask::AskView {
    const TITLE: &'static str = "Ask View";
    const FILE: &'static str = "commands_ask.md";
    const INTRO: &'static str = "The Ask launcher's view.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(<Self as SherlockView>::commands())
    }
}

impl CommandLayer for SettingsView {
    const TITLE: &'static str = "Settings";
    const FILE: &'static str = "commands_settings.md";
    const INTRO: &'static str = "The settings page.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(<Self as SherlockView>::commands())
    }
}

impl CommandLayer for TextInput {
    const TITLE: &'static str = "Search Bar";
    const FILE: &'static str = "commands_search_bar.md";
    const INTRO: &'static str = "Editing the search bar, while it has focus.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(search_bar::commands::COMMANDS)
    }
}

impl CommandLayer for LauncherView {
    const TITLE: &'static str = "Launcher";
    const FILE: &'static str = "commands_launcher.md";
    const INTRO: &'static str = "What a key does when nothing else took it.";
    fn commands() -> Vec<CommandDoc> {
        command_docs(Self::DEFAULT_COMMANDS)
    }
}

/// The keys as the docs show them: fixed keys as written, action keys as the built-in key and
/// the `[keybinds]` function that moves it.
fn describe_keys(keys: &[Key]) -> String {
    if keys.is_empty() {
        return "— (bind it)".into();
    }
    keys.iter()
        .map(|key| match key {
            Key::Stroke(stroke) => format!("`{stroke}`"),
            Key::Action(make) => describe_action(&*make()),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn describe_action(action: &dyn gpui::Action) -> String {
    if action.partial_eq(&Back) {
        return "`backspace` in an empty search bar".into();
    }
    if let Some(shortcut) = action.as_any().downcast_ref::<ShortcutAction>() {
        return format!("shortcut modifier + `{}`", shortcut.index);
    }
    let function = function_name(action.name());
    let defaults: Vec<_> = KeybindDefaults::defaults()
        .iter()
        .filter(|(_, f)| {
            f.binding("a", None)
                .is_some_and(|b| b.action().partial_eq(action))
        })
        .map(|(key, _)| format!("`{}`", split_scope(key).1))
        .collect();
    if defaults.is_empty() {
        format!("`{function}` (unbound)")
    } else {
        format!("{} (`{function}`)", defaults.join(" / "))
    }
}

/// `"launcher::ItemUp"` → `"item_up"`, the name `[keybinds]` uses.
fn function_name(action: &str) -> String {
    let name = action.rsplit("::").next().unwrap_or(action);
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

impl TopLevelEntry for Commands {
    type Summary = Container;
    fn summary() -> Self::Summary {
        md!(
            h1("Commands"),
            p!(
                "Everything a key does in Sherlock is a",
                bold("command"),
                "with an id like",
                code("ask.stop"),
                "or",
                code("search.copy"),
                ". A focused control (in Settings) gets a key first. Otherwise it goes to the \
                context menu (while open), then the current view, then the search bar, then the \
                launcher. The first available command bound to it runs; otherwise the next one \
                gets the key."
            ),
            list_iter!(Dash, layer_items().into_iter().map(|p| item!(p))),
            p!(
                "The status bar's hints are the current view's available commands; clicking a \
                hint runs it."
            ),
            h2("Binding keys"),
            p!(
                "The",
                code("[commands]"),
                "table in",
                code("config.toml"),
                "binds keys to commands in every view. A value can chain commands with",
                code(";"),
                ", each runs if it's available. Add",
                code("continue"),
                "to let the key go on to what it does by default."
            ),
            codeblock().lang("toml").content(COMMANDS_EXAMPLE),
            p!(
                "A launcher's",
                code("binds"),
                "do the same for that launcher's view and take precedence. Inside a view, the \
                part after the dot is enough:",
                code("new_chat"),
                "for",
                code("ask.new_chat"),
                "."
            ),
            codeblock().lang("json").content(BINDS_EXAMPLE),
            p!(
                "Rebinding a function in",
                code("[keybinds]"),
                "(the name in brackets in the tables, e.g.",
                code("execute"),
                ") moves every command listed with it.",
                code("[commands]"),
                "entries need one keystroke without a context; others are reported on startup \
                and ignored."
            ),
        )
    }

    fn children() -> impl Iterator<Item = BookEntry> + 'static {
        [
            Some(CommandDocs::<Select>::entry()),
            Some(CommandDocs::<NumberInput>::entry()),
            Some(CommandDocs::<TinyInput>::entry()),
            Some(CommandDocs::<ColorInput>::entry()),
            Some(CommandDocs::<ContextMenu>::entry()),
            #[cfg(feature = "ai")]
            Some(CommandDocs::<crate::ui::views::ask::AskView>::entry()),
            #[cfg(not(feature = "ai"))]
            None,
            Some(CommandDocs::<SettingsView>::entry()),
            Some(CommandDocs::<TextInput>::entry()),
            Some(CommandDocs::<LauncherView>::entry()),
        ]
        .into_iter()
        .flatten()
    }
}

/// The overview's list of command pages, in the order keys reach them.
fn layer_items() -> Vec<md_rs::components::span_nodes::Paragraph> {
    vec![
        CommandDocs::<Select>::overview_item(),
        CommandDocs::<NumberInput>::overview_item(),
        CommandDocs::<TinyInput>::overview_item(),
        CommandDocs::<ColorInput>::overview_item(),
        CommandDocs::<ContextMenu>::overview_item(),
        #[cfg(feature = "ai")]
        CommandDocs::<crate::ui::views::ask::AskView>::overview_item(),
        CommandDocs::<SettingsView>::overview_item(),
        CommandDocs::<TextInput>::overview_item(),
        CommandDocs::<LauncherView>::overview_item(),
    ]
}

impl From<Commands> for BookEntry {
    fn from(_: Commands) -> Self {
        BookEntryBuilder::new_toplevel::<Commands>()
            .title("Commands")
            .file("commands.md")
            .build()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::config::ConfigCommands;

    fn known(name: &str) -> bool {
        fn any<V>(commands: &[Command<V>], name: &str) -> bool {
            commands.iter().any(|c| c.named(name))
        }
        let ask = {
            #[cfg(feature = "ai")]
            {
                any(
                    <crate::ui::views::ask::AskView as SherlockView>::commands(),
                    name,
                )
            }
            #[cfg(not(feature = "ai"))]
            false
        };
        name == "continue"
            || ask
            || any(LauncherView::MENU_COMMANDS, name)
            || any(LauncherView::DEFAULT_COMMANDS, name)
            || any(search_bar::commands::COMMANDS, name)
            || any(<SettingsView as SherlockView>::commands(), name)
    }

    #[test]
    fn examples_bind_real_commands() {
        #[derive(serde::Deserialize)]
        struct File {
            commands: ConfigCommands,
        }
        let file: File = toml::from_str(COMMANDS_EXAMPLE).unwrap();
        let (binds, errors) = file.commands.binds();
        assert!(errors.is_empty(), "{errors:?}");

        let raw: serde_json::Value = serde_json::from_str(BINDS_EXAMPLE).unwrap();
        let launcher_chains = raw["binds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["callback"].as_str().unwrap().to_string());

        for chain in binds
            .iter()
            .map(|b| b.callback.to_string())
            .chain(launcher_chains)
        {
            for name in chain.split(';') {
                assert!(known(name.trim()), "`{name}` in the docs isn't a command");
            }
        }
    }

    #[test]
    fn every_command_is_linkable_from_launcher_docs() {
        use crate::{docs::link_scope, ui::views::settings::SettingsView};

        link_scope::reset();
        let docs = super::super::SherlockDocumentation::book();
        let pages = docs.render_pages();
        let launcher = render(&launcher_commands::<SettingsView>());
        for (_, page) in &pages {
            link_scope::resolve(page, "").unwrap();
        }
        let resolved = link_scope::resolve(&launcher, "").unwrap();
        assert!(
            resolved.contains("(commands_settings.md#settingsup)"),
            "{resolved}"
        );
    }

    #[test]
    fn function_names_match_keybinds() {
        assert_eq!(function_name("launcher::ItemUp"), "item_up");
        assert_eq!(function_name("Execute"), "execute");
    }
}

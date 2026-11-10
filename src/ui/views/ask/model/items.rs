use crate::ui::views::ask::{
    content::{
        actions::{AppEntry, Resolved, resolve_block},
        markdown::{self, Block},
    },
    exec::runner::{Shell, needs_terminal},
};

/// Something in an answer the user can run.
#[derive(Clone, Debug, PartialEq)]
pub enum Item {
    /// A shell script (a code block's commands).
    Script(Script),
    /// A resolved Sherlock action.
    Action(Resolved),
}

/// A code block's commands, and how to run them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Script {
    pub text: String,
    /// The shell named by the fence (`fish` scripts aren't POSIX).
    pub shell: Shell,
    /// Runs in a real terminal: marked by the model (```` ```sh terminal ````) or an
    /// interactive program at a command position (`needs_terminal`).
    pub terminal: bool,
}

/// A ```sherlock block of actions.
pub fn is_action_block(lang: &str) -> bool {
    lang.eq_ignore_ascii_case("sherlock")
}

/// A ```next block: follow-up questions the model suggests.
pub fn is_next_block(lang: &str) -> bool {
    lang.eq_ignore_ascii_case("next")
}

/// At most this many follow-ups are shown.
pub const MAX_SUGGESTIONS: usize = 3;

/// The follow-up questions of a block's body: non-empty lines, list markers stripped.
pub fn suggestion_lines(body: &str) -> impl Iterator<Item = &str> {
    body.lines()
        .map(|l| l.trim().trim_start_matches(['-', '*', '•']).trim())
        .filter(|l| !l.is_empty())
        .take(MAX_SUGGESTIONS)
}

/// The follow-ups of `blocks`: the lines of the first complete ```next block.
pub fn suggestions(blocks: &[Block]) -> Vec<String> {
    markdown::code_blocks(blocks)
        .into_iter()
        .find_map(|b| match b {
            Block::Code {
                lang,
                body,
                closed: true,
                ..
            } if is_next_block(lang) => Some(suggestion_lines(body).map(str::to_owned).collect()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The runnable items of `blocks`: complete shell blocks, and the valid actions of complete
/// action blocks.
pub fn items(blocks: &[Block], apps: &[AppEntry]) -> Vec<Item> {
    let mut items = Vec::new();
    for block in markdown::code_blocks(blocks) {
        match block {
            Block::Code {
                lang,
                body,
                closed: true,
                ..
            } if is_action_block(lang) => items.extend(
                resolve_block(body, apps)
                    .into_iter()
                    .filter(Resolved::is_valid)
                    .map(Item::Action),
            ),
            Block::Code {
                lang,
                body,
                terminal,
                ..
            } if block.is_runnable() => {
                let text = markdown::script(body);
                items.push(Item::Script(Script {
                    terminal: *terminal || needs_terminal(&text),
                    shell: Shell::from_lang(lang),
                    text,
                }))
            }
            _ => {}
        }
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apps() -> Vec<AppEntry> {
        vec![AppEntry {
            name: "Firefox".into(),
            exec: "firefox".into(),
            terminal: false,
            tags: vec!["webbrowser".into()],
        }]
    }

    #[test]
    fn scripts_and_actions_in_order() {
        let text = "Run:\n\n```sh\nls\n```\n\n```toml\na = 1\n```\n\n```sherlock\nopen browser\nopen gimp\n```\n\n```bash\npwd\n```\n";
        let items = items(&markdown::parse(text), &apps());
        assert_eq!(
            items.len(),
            3,
            "toml isn't runnable, `open gimp` doesn't resolve"
        );
        assert!(matches!(&items[0], Item::Script(s) if s.text == "ls" && !s.terminal));
        assert!(matches!(&items[1], Item::Action(Resolved::App(a)) if a.name == "Firefox"));
        assert!(matches!(&items[2], Item::Script(s) if s.text == "pwd"));
    }

    #[test]
    fn scripts_carry_their_shell_and_terminal_mark() {
        let text = "```fish\nset x 1\n```\n\n```sh terminal\nncdu-like-tool\n```\n\n```bash\nsudo ls\n```\n";
        let items = items(&markdown::parse(text), &[]);
        let [Item::Script(a), Item::Script(b), Item::Script(c)] = &items[..] else {
            panic!("three scripts: {items:?}");
        };
        assert_eq!((a.shell, a.terminal), (Shell::Fish, false));
        assert!(b.terminal, "marked by the fence");
        assert_eq!(
            (c.shell, c.terminal),
            (Shell::Bash, true),
            "sudo at a command position"
        );
    }

    #[test]
    fn suggestions_from_a_next_block() {
        let text = "Big ones:\n\n```sh\ndu -h ~\n```\n\n```next\n- what's in ~/.cache?\n\ncan I clean it?\na\nb\n```\n";
        let blocks = markdown::parse(text);
        assert_eq!(
            suggestions(&blocks),
            ["what's in ~/.cache?", "can I clean it?", "a"]
        );
        assert_eq!(items(&blocks, &[]).len(), 1, "a next block isn't runnable");
        assert!(suggestions(&markdown::parse("```next\nhalf")).is_empty());
    }

    #[test]
    fn unfinished_blocks_are_not_runnable() {
        let blocks = markdown::parse("```sh\nsudo rm");
        assert!(items(&blocks, &[]).is_empty());
    }
}

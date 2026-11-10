use crate::{ui::views::ask::exec::runner::Shell, utils::command_launch::spawn_detached};

/// Opens the user's terminal (`{terminal}` from the config) running `script` through `shell`;
/// the terminal stays open afterwards with the user's shell.
pub fn run_in_terminal(script: &str, shell: Shell) {
    let _ = spawn_detached(&terminal_command(script, shell), "", &[]);
}

/// The launcher command line for `script`: quoted for Sherlock's command splitter, which
/// unescapes `\` inside double quotes.
fn terminal_command(script: &str, shell: Shell) -> String {
    // Then the user's own shell stays open (fish has no `${…:-…}`).
    let keep_open = match shell {
        Shell::Fish => "exec $SHELL",
        _ => "exec \"${SHELL:-sh}\"",
    };
    let script = format!("{script}\n{keep_open}")
        .replace('\\', "\\\\")
        .replace('"', "\\\"");
    format!("{{terminal}} {} -c \"{script}\"", shell.program())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::command_launch::split_as_command;

    #[test]
    fn script_survives_the_command_splitter() {
        let script = "echo \"hi there\" | grep -o 'hi' && printf '%s\\n' a\\ b";
        let parts = split_as_command(&terminal_command(script, Shell::Sh));
        assert_eq!(parts[0], "{terminal}");
        assert_eq!(parts[1..3], ["sh", "-c"]);
        assert_eq!(parts[3], format!("{script}\nexec \"${{SHELL:-sh}}\""));
        assert_eq!(parts.len(), 4);
    }

    #[test]
    fn fish_runs_in_fish() {
        let parts = split_as_command(&terminal_command("ls", Shell::Fish));
        assert_eq!(parts[1..3], ["fish", "-c"]);
        assert_eq!(parts[3], "ls\nexec $SHELL");
    }
}

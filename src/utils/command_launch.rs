use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use gpui::SharedString;
use regex::{Captures, Regex};

use crate::{
    sherlock_error,
    utils::{
        config::{ConfigGuard, SherlockConfig},
        errors::{SherlockError, SherlockErrorType},
    },
};

/// Spawnes a command completely detatched from the current process.
///
/// This function uses a "double-fork" strategy to ensure that the spawned process is adopted by
/// the system init process (PID 1). This prevents empty "zombie" process from cluttering the
/// process table and ensures the child survives even if the daemon exits.
///
/// # Safety
/// This function uses `unsafe` and `pre_exec`. `pre_exec` runs in a restricted environment between
/// `fork` and `exec`. It is generally safe here as it only performs a single syscall and exit, but
/// complex logic (like memory allocation or locking) shuold be avoided inside the `pre_exec`
/// block!
///
/// # Arguments
/// * `cmd` -  A string containing the program name followed by its arguments (e.g, `foot -e`).
pub fn spawn_detached(
    cmd: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
) -> Result<(), SherlockError> {
    let config = ConfigGuard::read().unwrap();
    let cmd = parse_variables(cmd, keyword, variables, &config);

    drop(config);

    let parts = split_as_command(&cmd);
    if parts.is_empty() {
        return Ok(());
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut command = Command::new(program);
    command.args(args);

    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    unsafe {
        command.pre_exec(|| {
            // Fork again inside the child
            match libc::fork() {
                -1 => return Err(std::io::Error::last_os_error()),
                0 => {
                    // detatch grandchild
                    libc::setsid();
                    Ok(())
                }
                _ => {
                    // exit child immediately
                    // this orphans the grandchild, will get adopted by PID 1.
                    libc::_exit(0);
                }
            }
        });
    }

    let mut child = command.spawn().map_err(|e| {
        sherlock_error!(
            SherlockErrorType::CommandExecutionError(cmd.to_string()),
            e.to_string()
        )
    })?;
    let _ = child.wait();

    Ok(())
}

pub fn split_as_command(cmd: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut double_quoting = false;
    let mut single_quoting = false;
    let mut escaped = false;

    let mut it = cmd.chars().peekable();

    while let Some(c) = it.next() {
        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        match c {
            '\\' if !single_quoting => {
                escaped = true;
            }
            '"' if !single_quoting => {
                double_quoting = !double_quoting;
            }
            '\'' if !double_quoting => {
                single_quoting = !single_quoting;
            }
            c if c.is_whitespace() && !double_quoting && !single_quoting => {
                if !current.is_empty() {
                    parts.push(current.split_off(0));
                }
            }
            c => {
                current.push(c);
            }
        }
    }

    if !current.is_empty() {
        parts.push(current);
    }

    parts.retain(|s| !s.starts_with('%'));
    parts
}

pub fn parse_variables<'a>(
    exec_input: &'a str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
    config: &SherlockConfig,
) -> String {
    let mut exec = exec_input.to_string();

    // Handle standard variables
    let pattern = r#"\{([a-zA-Z_]+)(?::(.*?))?\}"#;
    let re = Regex::new(pattern).unwrap();

    exec = re
        .replace_all(&exec, |caps: &Captures| {
            let key = &caps[1];
            let value = caps.get(2).map(|m| m.as_str());

            match key {
                "terminal" => format!("{} -e", config.default_apps.terminal),
                "keyword" => keyword.to_string(),
                "variable" => variables
                    .iter()
                    .find(|v| Some(v.0.as_ref()) == value)
                    .map(|v| v.1.to_string())
                    .unwrap_or_else(|| caps[0].to_string()),
                _ => caps[0].to_string(),
            }
        })
        .into_owned();

    // Handle prefixes
    let prefix_pattern = r#"\{prefix\[(.*?)\]:(.*?)\}"#;
    let re_prefix = Regex::new(prefix_pattern).unwrap();

    exec = re_prefix
        .replace_all(&exec, |caps: &Captures| {
            let prefix_for = &caps[1];
            let prefix = &caps[2];

            let has_value = variables
                .iter()
                .find(|v| v.0.as_ref() == prefix_for)
                .map_or(false, |v| !v.1.is_empty());

            if has_value {
                prefix.to_string()
            } else {
                "".to_string()
            }
        })
        .into_owned();

    exec
}

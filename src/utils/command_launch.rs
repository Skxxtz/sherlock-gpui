use std::{
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use gpui::SharedString;

use crate::{
    sherlock_error,
    utils::errors::{SherlockError, SherlockErrorType},
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
    variables: &[(SharedString, SharedString)],
) -> Result<(), SherlockError> {
    let parts = split_as_command(cmd);
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

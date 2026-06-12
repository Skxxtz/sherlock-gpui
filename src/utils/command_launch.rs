use std::{
    fs::File,
    io::{BufRead, BufReader, Write},
    os::unix::process::CommandExt,
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use gpui::SharedString;

use crate::{
    loader::application_loader::ApplicationLoader,
    sherlock_msg,
    utils::{
        config::{ConfigGuard, SherlockConfig},
        errors::{SherlockMessage, types::SherlockErrorType},
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
    input: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
) -> Result<(), SherlockMessage> {
    let config = ConfigGuard::read().unwrap();
    let mut cmd = String::with_capacity(input.len());
    parse_variables(input, keyword, variables, &config, &mut cmd);

    drop(config);

    let mut parts = split_as_command(&cmd);
    if parts.is_empty() {
        return Ok(());
    }

    // if sudo, insert -S flag to read sudo from stdin
    let mut sudo_used = false;
    if let Some(idx) = parts.iter().position(|p| *p == "sudo") {
        parts.insert(idx + 1, "-S".into());
        sudo_used = true;
    }

    let program = &parts[0];
    let args = &parts[1..];

    let mut command = Command::new(program);
    command
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    if sudo_used {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }

    unsafe {
        command.pre_exec(|| {
            // Fork again inside the child
            match libc::fork() {
                -1 => Err(std::io::Error::last_os_error()),
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

    let mut child = command
        .spawn()
        .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::CommandError(cmd.clone()), e))?;

    // pass sudo password
    if sudo_used
        && let Some((_, password)) = variables
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("sudo"))
    {
        send_sudo(&mut child, password.as_str())
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::CommandError(cmd.clone()), e))?;
    }

    let _ = child.wait();

    Ok(())
}

pub fn split_as_command(cmd: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();

    // State machine
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    let chars: Vec<char> = cmd.chars().collect();

    for i in 0..chars.len() {
        let c = chars[i];

        if escaped {
            current.push(c);
            escaped = false;
            continue;
        }

        match c {
            // Handle backslash escaping
            '\\' => {
                // Only mark as escaped if there's a character following it
                if i + 1 < chars.len() {
                    escaped = true;
                } else {
                    current.push('\\');
                }
            }
            // Single quote toggle — strip the quote character itself
            '\'' if !in_double_quote => {
                in_single_quote = !in_single_quote;
            }
            // Double quote toggle — strip the quote character itself
            '"' if !in_single_quote => {
                in_double_quote = !in_double_quote;
            }
            // Whitespace split
            c if c.is_whitespace() && !in_single_quote && !in_double_quote => {
                if !current.is_empty() {
                    args.push(std::mem::take(&mut current));
                }
            }
            // Literal characters
            _ => {
                current.push(c);
            }
        }
    }

    // Push the final buffer
    if !current.is_empty() {
        args.push(current);
    }

    args.retain(|s| !s.starts_with('%'));

    args
}

/// Replaces variables like `{variable:variable name}` and applies prefixes such as `{prefix[variable name]:string}` with
/// the corresponding values.
///
/// This function is designed to efficiently find and replace occurences of variables and prefixes
/// using regex while folling the `Sherlock variable input scheme`.
pub fn parse_variables(
    exec_input: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
    config: &SherlockConfig,
    out: &mut String, // ← take out by &mut String so callers can pass their buffer
) {
    let mut token = String::new();
    let mut depth = 0u8;

    for c in exec_input.chars() {
        match c {
            '{' => {
                depth += 1;
                if depth > 1 {
                    token.push(c);
                }
            }
            '}' if depth > 0 => {
                depth -= 1;
                if depth == 0 {
                    resolve_token(&token, keyword, variables, config, out);
                    token.clear();
                } else {
                    token.push(c);
                }
            }
            _ if depth > 0 => token.push(c),
            _ => out.push(c),
        }
    }

    if !token.is_empty() {
        out.push('{');
        out.push_str(&token);
    }
}

fn resolve_token(
    token: &str,
    keyword: &str,
    variables: &[(SharedString, SharedString)],
    config: &SherlockConfig,
    out: &mut String,
) {
    if token == "keyword" {
        out.push_str(keyword);
    } else if token == "terminal" {
        out.push_str(&config.default_apps.terminal);
        out.push_str(" -e");
    } else if let Some(var_name) = token.strip_prefix("variable:") {
        let value = variables
            .iter()
            .find(|(k, _)| k.as_ref() == var_name)
            .map_or("", |(_, v)| v.as_ref());
        out.push_str(value);
    } else if let Some(rest) = token.strip_prefix("prefix[") {
        if let Some(bracket) = rest.find(']') {
            let var_name = &rest[..bracket];
            let prefix_text = rest[bracket + 1..].strip_prefix(':').unwrap_or("");
            let has_value = variables
                .iter()
                .any(|(k, v)| k.as_ref() == var_name && !v.is_empty());
            if has_value {
                // ← no longer allocates a String; writes directly into out
                parse_variables(prefix_text, keyword, variables, config, out);
            }
        }
    } else {
        out.push('{');
        out.push_str(token);
        out.push('}');
    }
}

/// Send the sudo password to a child process's stdin.
///
/// This function is designed for commands prefixed with `sudo -S`. It writes the provided password
/// followed by a newlines and flushes the buffer to ensure the child process reveices the
/// credentials immediately.
///
/// # Errors
/// * The child process doesn't have a piped stdin (e.g., was not spawned with `Stdio::piped()`)
/// * The pipe is broken or the write operation failes
fn send_sudo(child: &mut Child, sudo: &str) -> Result<(), std::io::Error> {
    let mut stdin = child.stdin.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Stdin pipe not available.")
    })?;

    {
        stdin.write_all(sudo.as_bytes())?;
        stdin.write_all(b"\n")?;
        stdin.flush()?;
    }

    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_as_command() {
        assert_eq!(
            split_as_command(
                "flatpak run --command=bottles-cli com.usebottles.bottles run -p 'FL Studio 2025' -b 'Apps' -- %u"
            ),
            vec![
                "flatpak",
                "run",
                "--command=bottles-cli",
                "com.usebottles.bottles",
                "run",
                "-p",
                "FL Studio 2025",
                "-b",
                "Apps",
                "--"
            ]
        );
        // 1. Basic splitting
        assert_eq!(split_as_command("ls -la /home"), vec!["ls", "-la", "/home"]);

        // 2. Double quotes with spaces
        assert_eq!(
            split_as_command("echo \"hello world\""),
            vec!["echo", "hello world"]
        );

        // 3. Single quotes
        assert_eq!(
            split_as_command("grep 'pattern with spaces' file.txt"),
            vec!["grep", "pattern with spaces", "file.txt"]
        );

        // 4. Nested quotes (single inside double)
        assert_eq!(
            split_as_command("echo \"it's a trap\""),
            vec!["echo", "it's a trap"]
        );

        // 5. Escaped characters inside double quotes
        assert_eq!(
            split_as_command("echo \"shout \\\"hello\\\"\""),
            vec!["echo", "shout \"hello\""]
        );

        // 6. Filtering variables starting with %
        assert_eq!(
            split_as_command("mpv %file --fullscreen"),
            vec!["mpv", "--fullscreen"]
        );

        // 7. Multiple spaces between arguments
        assert_eq!(
            split_as_command("rsync    -avz   source/   dest/"),
            vec!["rsync", "-avz", "source/", "dest/"]
        );

        // 8. Empty input
        let empty: Vec<String> = Vec::new();
        assert_eq!(split_as_command(""), empty);
        assert_eq!(split_as_command("   "), empty);

        // 9. URL in double quotes (the original bug)
        assert_eq!(
            split_as_command("browser \"https://www.google.com\""),
            vec!["browser", "https://www.google.com"]
        );
    }
}

pub fn mime_lookup(mime: &str) -> Option<String> {
    fn find_desktop_file(name: &str) -> Option<PathBuf> {
        let app_dirs = ApplicationLoader::get_applications_dir();
        for dir in app_dirs.iter() {
            let full_path = dir.join(name);
            if full_path.exists() {
                return Some(full_path);
            }
        }
        None
    }

    fn parse_exec_line(path: &std::path::Path) -> Option<String> {
        let file = File::open(path).ok()?;
        let reader = BufReader::new(file);
        for line in reader.lines() {
            let line = line.ok()?;
            if line.starts_with("Exec=") {
                return Some(line.trim_start_matches("Exec=").to_string());
            }
        }
        None
    }

    // query mime handler
    let output = Command::new("xdg-settings")
        .args(["get", "default-url-scheme-handler", mime])
        .output()
        .ok()?;

    // get desktop file name from output
    let desktop_file_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if desktop_file_name.is_empty() {
        return None;
    }

    // find full path of desktop file
    let path = find_desktop_file(&desktop_file_name)?;

    // parse exec command
    parse_exec_line(&path)
}

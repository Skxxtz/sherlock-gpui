use crate::{
    sherlock_msg,
    utils::{
        errors::types::{DirAction, SherlockErrorType},
        files,
    },
};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn get_xdg_dirs() -> xdg::BaseDirectories {
    xdg::BaseDirectories::with_prefix("sherlock")
}

fn legacy_path() -> Result<PathBuf, crate::utils::errors::SherlockMessage> {
    let home_dir = files::home_dir()?;
    Ok(home_dir.join(".sherlock"))
}

/// Returns the configuration directory.
///
/// It first checks for the legacy `~/.sherlock` directory. If it exists, it returns that path.
/// Otherwise, it returns the XDG standard configuration path, `$XDG_CONFIG_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_config_dir() -> Result<PathBuf, crate::utils::errors::SherlockMessage> {
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs
        .get_config_home()
        .ok_or_else(|| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), ""))?;
    fs::create_dir_all(&dir).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DirError(DirAction::Create, dir.to_path_buf()),
            e
        )
    })?;
    Ok(dir)
}

/// Returns the config directory (usually ~/.config/)
pub fn get_config_dir_raw() -> Result<PathBuf, crate::utils::errors::SherlockMessage> {
    let xdg_dirs = xdg::BaseDirectories::new();
    xdg_dirs
        .get_config_home()
        .ok_or_else(|| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), ""))
}

/// Returns the data directory.
///
/// It first checks for the legacy `~/.sherlock` directory. If it exists, it returns that path.
/// Otherwise, it returns the XDG standard data path, `$XDG_DATA_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_data_dir() -> Result<PathBuf, crate::utils::errors::SherlockMessage> {
    let legacy_path = legacy_path()?;
    if legacy_path.exists() {
        return Ok(legacy_path);
    }
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs
        .get_data_home()
        .ok_or_else(|| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), ""))?;
    fs::create_dir_all(&dir).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DirError(DirAction::Create, dir.to_path_buf()),
            e
        )
    })?;
    Ok(dir)
}

/// Returns the cache directory.
///
/// This function returns the XDG standard cache path, `$XDG_CACHE_HOME/sherlock`.
/// If the directory does not exist, it will be created.
pub fn get_cache_dir() -> Result<PathBuf, crate::utils::errors::SherlockMessage> {
    let xdg_dirs = get_xdg_dirs();
    let dir = xdg_dirs
        .get_cache_home()
        .ok_or_else(|| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), ""))?;
    fs::create_dir_all(&dir).map_err(|e| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::DirError(DirAction::Create, dir.clone()),
            e
        )
    })?;
    Ok(dir)
}

/// Returns the nth completion candidate for a given file system path input.
///
/// This function simulates terminal-style path completion by:
/// 1. Expanding `~` to the user's HOME directory.
/// 2. Treating relative paths as relative to the user's HOME directory.
/// 3. Searching the parent directory of the input for matches starting with the same prefix.
/// 4. Sorting matches alphabetically and selecting one using modulo indexing (`n % count`).
/// 5. Appending a trailing `/` if the match is a directory.
///
/// # Arguments
/// * `input` - The partial path string typed by the user (e.g., "~/Down" or "Documents/").
/// * `n` - The index used to cycle through multiple matches.
/// * `executable_only` - To only look for files that satisfy the executionable flag
///
/// # Returns
/// * `Some(String)` - The "ghost text" remainder of the completion (the part after `input`).
/// * `None` - If the input is empty, the directory is unreadable, or no matches are found.
///
/// # Examples
/// If the home directory contains `Documents/` and `Downloads/`:
/// ```
/// // input: "~/Doc", match: "Documents/"
/// // returns: Some("uments/")
/// ```
pub fn get_nth_path_completion(input: &str, n: usize, executable_only: bool) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    // Get absolute path. Default to home dir
    let full_path = if input.starts_with('/') {
        PathBuf::from(input)
    } else if input.starts_with('~') {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        PathBuf::from(input.replacen('~', &home, 1))
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        let mut path = PathBuf::from(home);
        path.push(input);
        path
    };

    // Determine search dir and the partial stub
    let (search_dir, prefix) = if full_path.is_dir() && input.ends_with('/') {
        (full_path.clone(), "")
    } else {
        (
            full_path
                .parent()
                .unwrap_or_else(|| Path::new("/"))
                .to_path_buf(),
            full_path.file_name().and_then(|s| s.to_str()).unwrap_or(""),
        )
    };

    let entries = std::fs::read_dir(&search_dir).ok()?;

    // Enables .config dir for path completion
    let is_hidden = |name: &str| -> bool { name.starts_with('.') && !name.starts_with(".config") };

    // Enables support to filter out non-executable files
    fn is_executable(meta: &std::fs::Metadata) -> bool {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }

    let mut matches: Vec<String> = entries
        .filter_map(|res| res.ok())
        .filter(|e| !executable_only || e.metadata().is_ok_and(|m| is_executable(&m)))
        .filter_map(|e| {
            let os_name = e.file_name();
            let name = os_name.to_str()?;
            (!is_hidden(name) && name.starts_with(prefix)).then(|| name.to_string())
        })
        .collect();

    if matches.is_empty() {
        return None;
    }

    matches.sort();

    let chosen_match = &matches[n % matches.len()];

    let last_slash_idx = input.rfind('/').map(|i| i + 1).unwrap_or(0);
    let mut result = input[..last_slash_idx].to_string();
    result.push_str(chosen_match);

    if search_dir.join(chosen_match).is_dir() {
        result.push('/');
    }

    result.strip_prefix(input).map(|s| s.to_string())
}

/// Returns the nth completion candidate for a given command name input.
///
/// This function simulates terminal-style command completion by:
/// 1. Searching all directories listed in the `PATH` environment variable.
/// 2. Collecting executables whose names start with the given input prefix.
/// 3. Deduplicating across PATH dirs, sorting alphabetically, and selecting via `n % count`.
///
/// # Arguments
/// * `input` - The partial command string typed by the user (e.g., "git" or "sys").
/// * `n` - The index used to cycle through multiple matches.
///
/// # Returns
/// * `Some(String)` - The "ghost text" remainder of the completion (the part after `input`).
/// * `None` - If the input is empty, `PATH` is unset, or no matches are found.
///
/// # Examples
/// If `PATH` contains `git`, `gitk`, and `git-lfs`:
/// ```
/// // input: "git", n: 0 → Some("")       (exact match: "git")
/// // input: "git", n: 1 → Some("k")      (next match: "gitk")
/// // input: "git", n: 2 → Some("-lfs")   (next match: "git-lfs")
/// ```
pub fn get_nth_command_completion(input: &str, n: usize) -> Option<String> {
    if input.is_empty() {
        return None;
    }

    let path_var = std::env::var("PATH").unwrap_or_default();
    let mut matches: Vec<String> = std::env::split_paths(&path_var)
        .filter_map(|dir| std::fs::read_dir(dir).ok())
        .flatten()
        .filter_map(|res| res.ok())
        .filter(|e| e.file_name().to_str().is_some_and(|n| n.starts_with(input)))
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();

    if matches.is_empty() {
        return None;
    }

    // Deduplicate (same command can appear in multiple PATH dirs)
    matches.sort();
    matches.dedup();

    let chosen = &matches[n % matches.len()];
    chosen.strip_prefix(input).map(|s| s.to_string())
}

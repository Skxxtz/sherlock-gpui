use std::{
    env,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
    sync::OnceLock,
    time::SystemTime,
};

use super::errors::SherlockMessage;
use crate::{sherlock_msg, utils::errors::types::SherlockErrorType};

static HOME: OnceLock<PathBuf> = OnceLock::new();
static HOME_STR: OnceLock<String> = OnceLock::new();

pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}
pub fn expand_path<T: AsRef<Path>>(path: T, home: &Path) -> PathBuf {
    let path = path.as_ref();
    let mut components = path.components();
    if let Some(std::path::Component::Normal(first)) = components.next()
        && first == "~"
    {
        return home.join(components.as_path());
    }
    path.to_path_buf()
}

/// Desktop file ID per the XDG spec: path relative to the `applications/` dir, `/` → `-`. Uses the
/// *unresolved* path, so symlinks into `/nix/store` dont leak into the ID.
pub fn desktop_id(path: &Path) -> Option<String> {
    let s = path.to_str()?;
    let (_, rel) = s.rsplit_once("/applications/")?;
    Some(rel.replace('/', "-"))
}

pub fn home_dir() -> Result<&'static Path, SherlockMessage> {
    if let Some(home) = HOME.get() {
        return Ok(home);
    }
    let home = env::var_os("HOME").map(PathBuf::from).ok_or_else(|| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::EnvError("$HOME".into()),
            "Variable `$HOME` is not set."
        )
    })?;
    Ok(HOME.get_or_init(|| home))
}

#[inline(always)]
pub fn home_dir_owned() -> Result<PathBuf, SherlockMessage> {
    home_dir().map(|p| p.to_path_buf())
}

#[inline(always)]
pub fn home_str() -> Result<&'static str, SherlockMessage> {
    let home = home_dir()?;
    Ok(HOME_STR.get_or_init(|| home.to_string_lossy().into_owned()))
}

pub fn binary_exists(name: &str) -> bool {
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| is_executable(&dir.join(name)))
}

#[cfg(unix)]
fn is_executable(p: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    p.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[inline(always)]
pub fn modified_since(path: &Path, since: SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|t| t > since)
}

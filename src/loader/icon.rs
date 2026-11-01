use std::{
    collections::HashMap,
    env,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::{
    loader::{
        assets::Assets,
        icon::{
            render::{classify, classify_embedded},
            theme::lookup_themed,
        },
    },
    utils::files::home_dir,
};

const PLACEHOLDER: &str = "icons/400.svg";

mod cache;
mod render;
mod theme;

pub use cache::{CustomIconTheme, IconThemeGuard, IconType};

/// Resolve an icon name or absolute path to an `IconType`, memoized by `name`.
pub fn resolve_icon_path(name: &str) -> Option<IconType> {
    if let Ok(Some(icon)) = IconThemeGuard::lookup_icon(name) {
        return icon;
    }

    let result = resolve_uncached(name).or_else(|| {
        Assets::get(PLACEHOLDER).map(|a| classify_embedded("400", PLACEHOLDER.into(), &a.data))
    });

    if let Ok(mut cache) = IconThemeGuard::get_write() {
        cache.buf.insert(name.to_string().into(), result.clone());
    }
    result
}

fn resolve_uncached(name: &str) -> Option<IconType> {
    // Absolute path
    if name.starts_with('/') {
        return classify(Path::new(name));
    }

    // User-configured icon directories
    if let Some(icon) = IconThemeGuard::get_read()
        .ok()
        .and_then(|t| t.custom_icon(name))
    {
        return Some(icon);
    }

    // Embedded icons
    for ext in ["svg", "png"] {
        let key = format!("icons/{name}.{ext}");
        if let Some(asset) = Assets::get(&key) {
            return Some(classify_embedded(name, key, &asset.data));
        }
    }

    // System icon theme: only check that it exists. Whether it's drawn
    // tinted is decided by the name, not by whichever size variant we probe.
    let symbolic = name.ends_with("-symbolic");
    lookup_themed(name, 24, 1, symbolic)?;
    Some(IconType::Themed {
        name: name.to_string().into(),
        symbolic,
    })
}

/// Finds the icon name for an application by its executable / process name.
///
/// Desktop entries are indexed once by Exec binary, `StartupWMClass` and
/// file id, so lookups are a hash hit instead of a directory scan.
pub fn find_desktop_icon(name: &str) -> Option<String> {
    static INDEX: OnceLock<HashMap<String, String>> = OnceLock::new();
    let index = INDEX.get_or_init(build_desktop_index);

    let needle = Path::new(name)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(name)
        .to_lowercase();

    index.get(&needle).cloned().or_else(|| {
        // /proc comm names are truncated to 15 bytes
        (needle.len() == 15)
            .then(|| index.iter().find(|(k, _)| k.starts_with(&needle)))
            .flatten()
            .map(|(_, v)| v.clone())
    })
}

fn application_dirs() -> Vec<PathBuf> {
    let data_home = env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| home_dir().ok().map(|h| h.join(".local/share")));
    let data_dirs = env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());

    data_home
        .into_iter()
        .chain(env::split_paths(&data_dirs))
        .map(|d| d.join("applications"))
        .collect()
}

fn build_desktop_index() -> HashMap<String, String> {
    let mut index = HashMap::new();
    // Earlier dirs win (user entries override system ones).
    for dir in application_dirs() {
        index_dir(&dir, &mut index);
    }
    index
}

fn index_dir(dir: &Path, index: &mut HashMap<String, String>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            index_dir(&path, index);
            continue;
        }
        if path.extension().is_none_or(|e| e != "desktop") {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };

        let (mut icon, mut exec, mut wm_class) = (None, None, None);
        for line in content
            .lines()
            .skip_while(|l| l.trim() != "[Desktop Entry]")
            .skip(1)
            .take_while(|l| !l.starts_with('['))
        {
            if let Some((k, v)) = line.split_once('=') {
                match k.trim() {
                    "Icon" => icon = Some(v.trim()),
                    "Exec" => exec = exec_binary(v),
                    "StartupWMClass" => wm_class = Some(v.trim()),
                    _ => {}
                }
            }
        }
        let Some(icon) = icon.filter(|i| !i.is_empty()) else {
            continue;
        };
        let file_id = path.file_stem().and_then(|s| s.to_str());

        for key in [exec, wm_class, file_id].into_iter().flatten() {
            index
                .entry(key.to_lowercase())
                .or_insert_with(|| icon.to_string());
        }
    }
}

/// Basename of the program an `Exec=` line runs, skipping `env` and `VAR=x`.
fn exec_binary(exec: &str) -> Option<&str> {
    exec.split_whitespace()
        .map(|t| t.trim_matches(['"', '\'']))
        .find(|t| *t != "env" && !t.contains('=') && !t.starts_with('-'))
        .and_then(|t| Path::new(t).file_name()?.to_str())
}

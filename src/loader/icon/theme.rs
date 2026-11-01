use std::{
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};

use crate::utils::paths::get_config_dir_raw;

static ICON_THEME: OnceLock<Option<String>> = OnceLock::new();

/// Get the configured icon theme from KDE/GTK settings
pub fn get_current_theme() -> String {
    ICON_THEME
        .get_or_init(|| {
            read_kde_icon_theme()
                .or(read_gtk3_icon_theme())
                .or(read_gtk4_icon_theme())
                .or(read_gsettings_icon_theme())
        })
        .clone()
        .unwrap_or(String::from("hicolor"))
}

fn parse_ini_value(content: &str, section: &str, key: &str) -> Option<String> {
    let mut in_section = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_section = line == section;
            continue;
        }
        if in_section
            && let Some(value) = line.strip_prefix(key)
            && let Some(value) = value.strip_prefix('=')
        {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn read_kde_icon_theme() -> Option<String> {
    let config_path = get_config_dir_raw().ok()?.join("kdeglobals");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Icons]", "Theme")
}

fn read_gtk3_icon_theme() -> Option<String> {
    let config_path = get_config_dir_raw().ok()?.join("gtk-3.0/settings.ini");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Settings]", "gtk-icon-theme-name")
}

fn read_gtk4_icon_theme() -> Option<String> {
    let config_path = get_config_dir_raw().ok()?.join("gtk-4.0/settings.ini");
    let content = fs::read_to_string(config_path).ok()?;
    parse_ini_value(&content, "[Settings]", "gtk-icon-theme-name")
}

fn read_gsettings_icon_theme() -> Option<String> {
    let output = std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "icon-theme"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8(output.stdout).ok()?;
    // gsettings wraps strings in single quotes: 'Adwaita'
    Some(raw.trim().trim_matches('\'').to_string())
}

/// Look up a themed icon for a specific logical size and scale.
/// Falls back to the closest size, then to an unthemed lookup.
///
/// For full-color icons (`symbolic == false`), monochrome artwork is skipped:
/// themes like Breeze ship `currentColor` variants at small sizes (e.g. a
/// 16/22/24px `folder`), so we step up to the next size with real colors.
pub fn lookup_themed(name: &str, size: u16, scale: u16, symbolic: bool) -> Option<PathBuf> {
    let theme = get_current_theme();
    let find = |size: u16| {
        let l = freedesktop_icons::lookup(name)
            .with_size(size)
            .with_scale(scale)
            .with_cache();
        let l = if symbolic { l.force_svg() } else { l };
        l.with_theme(&theme).find().or_else(|| {
            let l = freedesktop_icons::lookup(name)
                .with_size(size)
                .with_scale(scale);
            if symbolic { l.force_svg() } else { l }.find()
        })
    };

    let found = find(size)?;
    if symbolic || !is_monochrome(&found) {
        return Some(found);
    }
    [32, 48, 64, 128, 256]
        .into_iter()
        .filter(|s| *s > size)
        .filter_map(find)
        .find(|p| !is_monochrome(p))
        .or(Some(found))
}

fn is_monochrome(path: &Path) -> bool {
    path.extension().is_some_and(|e| e == "svg")
        && fs::read(path).is_ok_and(|d| d.windows(12).any(|w| w == b"currentColor"))
}

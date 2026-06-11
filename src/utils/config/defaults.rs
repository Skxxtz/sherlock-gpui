use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
};

use gpui::SharedString;

use crate::{
    loader::application_loader::ApplicationLoader,
    sherlock_msg,
    ui::UIFunction,
    utils::{
        config::ConfigKeybinds,
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        files::read_lines,
        paths,
    },
};

pub struct ConstantDefaults {}
impl ConstantDefaults {
    pub fn terminal() -> String {
        Self::get_terminal().unwrap_or_default()
    }
    pub fn get_terminal() -> Result<String, SherlockMessage> {
        let mut terminal = None;

        //Check if $TERMAINAL is set
        if let Ok(term) = std::env::var("TERMINAL")
            && Self::is_terminal_installed(&term)
        {
            terminal = Some(term);
        }
        // Try other terminals
        if terminal.is_none() {
            let terminals = [
                "kitty",
                "gnome-terminal",
                "xterm",
                "konsole",
                "alacritty",
                "urxvt",
                "mate-terminal",
                "terminator",
                "sakura",
                "terminology",
                "st",
                "xfce4-terminal",
                "guake",
                "x11-terminal",
                "macos-terminal",
                "iterm2",
                "lxterminal",
                "foot",
                "wezterm",
                "tilix",
            ];
            for t in terminals {
                if Self::is_terminal_installed(t) {
                    terminal = Some(t.to_string());
                    break;
                }
            }
        }
        if let Some(t) = terminal {
            Ok(t)
        } else {
            Err(sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to get terminal".into()),
                "Unable to locate or parse a valid terminal app. Ensure that the terminal app is correctly specified in the configuration file or environment variables."
            ))
        }
    }
    fn is_terminal_installed(terminal: &str) -> bool {
        Command::new(terminal).arg("--version").output().is_ok()
    }
    pub fn browser() -> Result<String, SherlockMessage> {
        // Find default browser desktop file
        let output = Command::new("xdg-settings")
            .arg("get")
            .arg("default-web-browser")
            .output()
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::EnvError("default browser".into()),
                    e
                )
            })?;

        let desktop_file: String = if output.status.success() {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        } else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::EnvError("default browser".into()),
                "Command 'xdg-settings get default-web-browser' failed to produce a valid output."
            ));
        };
        let desktop_files = ApplicationLoader::get_desktop_files();
        let browser_file = desktop_files
            .iter()
            .find(|f| f.ends_with(&desktop_file))
            .ok_or_else(|| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::EnvError("default browser".into()),
                    ""
                )
            })?;
        // read default browser desktop file
        let browser = read_lines(browser_file)
            .map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Read, browser_file.clone()),
                    e
                )
            })?
            .map_while(Result::ok)
            .find(|line| line.starts_with("Exec="))
            .and_then(|line| line.strip_prefix("Exec=").map(|l| l.to_string()))
            .ok_or_else(|| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::FileError(FileAction::Parse, browser_file.clone()),
                    ""
                )
            })?;
        Ok(browser)
    }
    pub fn teams() -> String {
        String::from(
            "teams-for-linux --enable-features=UseOzonePlatform --ozone-platform=wayland --url {meeting_url}",
        )
    }
    pub fn calendar_client() -> String {
        String::from("thunderbird")
    }
    pub fn lengths() -> String {
        String::from("meters")
    }
    pub fn weights() -> String {
        String::from("kg")
    }
    pub fn volumes() -> String {
        String::from("l")
    }
    pub fn temperatures() -> String {
        String::from("C")
    }
    pub fn currency() -> String {
        String::from("eur")
    }
}

pub struct KeybindDefaults;
impl KeybindDefaults {
    pub fn binds() -> ConfigKeybinds {
        ConfigKeybinds(HashMap::from([(
            "ctrl-<digit>".into(),
            UIFunction::Shortcut,
        )]))
    }
}

pub struct AppearanceDefaults;
impl AppearanceDefaults {
    pub fn modkey_ascii() -> [char; 4] {
        ['⌘', '^', '⎇', '⇧']
    }
    pub fn icon_size() -> i32 {
        22
    }
    pub fn search_icon() -> SharedString {
        SharedString::from("system-search-symbolic")
    }
    pub fn search_icon_back() -> SharedString {
        SharedString::from("sherlock-back")
    }
    pub fn placeholder() -> SharedString {
        SharedString::from("Search:")
    }
}

pub struct FileDefaults {}
impl FileDefaults {
    pub fn cache() -> Arc<Path> {
        paths::get_cache_dir()
            .unwrap()
            .join("desktop_files.bin")
            .into()
    }
    pub fn config() -> Arc<Path> {
        paths::get_config_dir().unwrap().join("config.toml").into()
    }
    pub fn fallback() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("fallback.json")
            .into()
    }
    pub fn alias() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlock_alias.json")
            .into()
    }
    pub fn ignore() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlockignore")
            .into()
    }
    pub fn actions() -> Arc<Path> {
        paths::get_config_dir()
            .unwrap()
            .join("sherlock_actions.json")
            .into()
    }
    pub fn icon_paths() -> Vec<PathBuf> {
        vec![
            paths::get_config_dir()
                .unwrap()
                .join("icons/")
                .to_path_buf(),
        ]
    }
}

pub struct OtherDefaults {}
impl OtherDefaults {
    pub fn bool_true() -> bool {
        true
    }
    pub fn one() -> f64 {
        1.0
    }
    pub fn five() -> u8 {
        5
    }
    pub fn backdrop_opacity() -> f64 {
        0.6
    }
    pub fn backdrop_edge() -> String {
        String::from("top")
    }
}

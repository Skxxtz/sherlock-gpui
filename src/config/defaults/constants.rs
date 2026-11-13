use std::{process::Command, sync::OnceLock};

use crate::{
    config::types::Text,
    loader::application_loader::ApplicationLoader,
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        files::read_lines,
    },
};
/// Values that have to be looked up on the system. Each is looked up once.
pub struct ConstantDefaults;
impl ConstantDefaults {
    /// `$TERMINAL`, or the first known terminal that is installed.
    pub fn detected_terminal() -> Option<Text> {
        static TERMINAL: OnceLock<Option<Text>> = OnceLock::new();
        TERMINAL
            .get_or_init(|| Self::get_terminal().ok().map(Text::from))
            .clone()
    }
    fn get_terminal() -> Result<String, SherlockMessage> {
        if let Ok(term) = std::env::var("TERMINAL")
            && Self::is_terminal_installed(&term)
        {
            return Ok(term);
        }
        const TERMINALS: &[&str] = &[
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
        TERMINALS
            .iter()
            .find(|t| Self::is_terminal_installed(t))
            .map(|t| t.to_string())
            .ok_or_else(|| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::ConfigError("Failed to get terminal".into()),
                    "Unable to locate or parse a valid terminal app. Ensure that the terminal app is correctly specified in the configuration file or environment variables."
                )
            })
    }
    fn is_terminal_installed(terminal: &str) -> bool {
        Command::new(terminal).arg("--version").output().is_ok()
    }

    /// The system's default browser. Remembered once found; a failure is retried next time.
    pub fn browser() -> Result<Text, SherlockMessage> {
        static BROWSER: OnceLock<Text> = OnceLock::new();
        if let Some(b) = BROWSER.get() {
            return Ok(b.clone());
        }
        let b = Text::from(Self::find_browser()?);
        Ok(BROWSER.get_or_init(|| b).clone())
    }
    fn find_browser() -> Result<String, SherlockMessage> {
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
}

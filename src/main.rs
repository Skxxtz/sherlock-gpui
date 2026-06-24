use ::tokio::net::UnixStream;
use gpui::{App, Application, QuitMode, SharedString};
use once_cell::sync::OnceCell;
use std::sync::{OnceLock, RwLock};
use tokio::io::AsyncWriteExt;

use crate::{
    app::{bindings::ShortcutKeyMod, run_app},
    loader::{CustomIconTheme, Loader, assets::Assets, pipe::read_stdin_piped},
    tokio_utils::{AsyncSizedMessage, SizedMessageObj},
    utils::{
        clipboard::spawn_clipboard_watcher,
        config::SherlockConfig,
        networking::{ClientMessage, ServerResponse},
    },
};

mod app;
#[cfg(feature = "docs")]
mod docs;
mod launcher;
mod loader;
mod prelude;
mod tokio_utils;
mod ui;
mod utils;
mod wayland;

/// Holds the icon cache, containing all known icon names and their file locations.
static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();
/// Holed the global config struct for user-specified config values.
static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();
/// Holds the string used to show and hide the context menu.
static CONTEXT_MENU_BIND: OnceLock<String> = OnceLock::new();
/// Holds the modifier key char
static SHORTCUT_MOD: OnceLock<ShortcutKeyMod> = OnceLock::new();
/// Holds the socket location for the sherlock socket
static SOCKET_PATH: &str = "/tmp/sherlock.sock";

#[tokio::main]
async fn main() {
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = UnixStream::connect(socket_path).await {
        // update flags
        let Some(mut flags) = Loader::load_flags() else {
            return;
        };
        if flags.execute_startup() {
            return;
        }
        let (flags, startup_action) = (flags.flags, flags.startup);

        let config_update = ClientMessage::ConfigUpdate(Box::new(flags));
        if let Ok(config_bin) = SizedMessageObj::from_struct(&config_update) {
            let _ = stream.write_sized(config_bin).await;
        }
        let ClientMessage::ConfigUpdate(flags) = config_update else {
            unreachable!()
        };

        // write message from flags to server
        if let Some(action) = startup_action
            && let Ok(payload_bin) = SizedMessageObj::try_from(&action)
            && stream.write_sized(payload_bin).await.is_ok()
            && action.exit()
        {
            return;
        }

        let piped = read_stdin_piped();
        if !piped.is_empty() {
            let piped_string = String::from_utf8_lossy(&piped).into_owned();
            let string_vec: Vec<SharedString> = piped_string
                .lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string().into())
                .collect();
            let payload = ClientMessage::Dmenu(string_vec);

            if let Ok(payload_bin) = SizedMessageObj::from_struct(&payload) {
                let _ = stream.write_sized(payload_bin).await;
            }
        }

        let payload = ClientMessage::Open;
        if let Ok(payload_bin) = SizedMessageObj::from_struct(&payload) {
            let _ = stream.write_sized(payload_bin).await;
        }

        if flags.wait
            && let Ok(ServerResponse::Print(response)) = stream.read_sized::<ServerResponse>().await
        {
            println!("{response}");
        }

        stream.shutdown().await.ok();
        return;
    } else {
        std::fs::remove_file(SOCKET_PATH).ok();
    }

    spawn_clipboard_watcher();

    // This top part
    let plat = gpui_platform::current_platform(false);
    let app = Application::with_platform(plat)
        .with_assets(Assets)
        .with_quit_mode(QuitMode::Explicit);

    if let Some(setup) = Loader::setup() {
        app.run(|cx: &mut App| run_app(cx, setup));
    }
}

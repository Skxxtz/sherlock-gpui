use bytes::Bytes;
use gpui::{App, Image, ImageFormat, SharedString};
use serde_json::Value;
use simd_json::prelude::ArrayTrait;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use zbus::blocking::{Connection, Proxy};

use crate::launcher::utils::binds::Bind;
use crate::launcher::variant_type::InnerFunction;
use crate::ui::widgets::RenderableChild;
use crate::ui::widgets::audio::MusicPlayerWidget;
use crate::utils::config::ConfigGuard;
use crate::utils::errors::SherlockMessage;
use crate::utils::errors::types::{
    DBusAction, DirAction, FileAction, SherlockErrorType, SocketAction,
};
use crate::{define_inner_functions, ensure_func, sherlock_msg, skip_func_if_nav};

pub mod utils;

use utils::MprisData;

use crate::launcher::{ExecEffect, LauncherProvider, LauncherType};
use crate::loader::utils::RawLauncher;

/// The following inner functions are available:
/// - `TogglePlayback`: Toggles current media playback
/// - `Previous`: Skips to previous song
/// - `Next`: Skips to next song
#[derive(Debug, Clone, Default)]
pub struct MusicPlayerLauncher {
    binds: Option<Arc<Vec<Bind>>>,
}

define_inner_functions! {
    pub enum MusicPlayerFunctions {
        TogglePlayback,
        Previous,
        Next,
    }
}

impl LauncherProvider for MusicPlayerLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let binds = raw
            .binds
            .as_ref()
            .map(|vec| Arc::new(vec.iter().filter_map(|b| Bind::try_from(b).ok()).collect()));
        Ok(LauncherType::MusicPlayer(MusicPlayerLauncher { binds }))
    }
    fn objects(
        &self,
        launcher: Arc<super::LauncherConfig>,
        _: &crate::loader::LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        Ok(vec![RenderableChild::Music {
            launcher,
            inner: MusicPlayerWidget::new(cx),
        }])
    }
    fn binds(&self) -> Option<Arc<Vec<Bind>>> {
        self.binds.clone()
    }
    fn execute_function(
        &self,
        func: InnerFunction,
        child: &RenderableChild,
        _variables: &[(SharedString, SharedString)],
        cx: &mut App,
    ) -> Result<ExecEffect, SherlockMessage> {
        skip_func_if_nav!(func);
        let func = ensure_func!(func, InnerFunction::MusicPlayer);

        let RenderableChild::Music { inner, .. } = child else {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::Unreachable,
                format!("Tried to unpack music tile but received: {:?}", child)
            ));
        };

        let Ok(Some(state)) = inner.entity.read(cx).as_ref() else {
            return Ok(ExecEffect::None);
        };

        match func {
            MusicPlayerFunctions::Next => MprisData::next(&state.player)?,
            MusicPlayerFunctions::Previous => MprisData::previous(&state.player)?,
            MusicPlayerFunctions::TogglePlayback => MprisData::playpause(&state.player)?,
        }

        Ok(ExecEffect::UpdateAsync)
    }
}

impl MprisData {
    /// Get current image
    /// Return:
    /// image: Pixbuf
    /// was_cached: bool
    pub async fn get_image(&self) -> Option<(Arc<Image>, bool)> {
        let art_url = self.metadata.art.as_ref()?;
        let loc = art_url.split("/").last()?.to_string();
        let mut was_cached = true;
        let bytes = match Self::read_cached_cover(&loc) {
            Ok(b) => b,
            Err(_) => {
                if art_url.starts_with("file") {
                    Self::read_image_file(art_url).ok()?
                } else {
                    let response = reqwest::get(art_url).await.ok()?;
                    let bytes = response.bytes().await.ok()?;
                    let _ = Self::cache_cover(&bytes, &loc);
                    was_cached = false;
                    bytes.into()
                }
            }
        };

        // mimetype parsing
        let mime = identify_image_type(&bytes);
        let format = ImageFormat::from_mime_type(mime)?;

        let image_arc = Arc::new(Image::from_bytes(format, bytes));
        Some((image_arc, was_cached))
    }
    fn cache_cover(image: &Bytes, loc: &str) -> Result<(), SherlockMessage> {
        // Create dir and parents
        let home = env::var("HOME").map_err(|e| {
            sherlock_msg!(Warning, SherlockErrorType::EnvError("HOME".to_string()), e)
        })?;

        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    SherlockErrorType::DirError(DirAction::Create, parent.to_path_buf(),),
                    e.to_string()
                )
            })?;
        };

        let mut file = if path.exists() {
            File::open(&path)
        } else {
            File::create(&path)
        }
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;

        file.write_all(image).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        // if file not exist, create and write it
        Ok(())
    }
    fn read_cached_cover(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let home = env::var("HOME")
            .map_err(|e| sherlock_msg!(Warning, SherlockErrorType::EnvError("$HOME".into()), e))?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);

        let mut file = File::open(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.clone()),
                e
            )
        })?;
        Ok(buffer)
    }
    fn read_image_file(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let path = PathBuf::from(loc.trim_start_matches("file://"));

        let mut file = File::open(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Find, path.clone()),
                e
            )
        })?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.clone()),
                e
            )
        })?;
        Ok(buffer)
    }
    pub fn playpause(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "PlayPause")
    }
    pub fn next(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "Next")
    }
    pub fn previous(player: &str) -> Result<(), SherlockMessage> {
        Self::player_method(player, "Previous")
    }
    fn player_method(player: &str, method: &str) -> Result<(), SherlockMessage> {
        let conn = Connection::session().map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::DBusError(DBusAction::Connect, "Session Bus".into()),
                e
            )
        })?;
        let proxy = Proxy::new(
            &conn,
            player,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DBusError(DBusAction::Construct, player.to_string()),
                e
            )
        })?;
        proxy.call_method(method, &()).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DBusError(DBusAction::Call, method.to_string()),
                e
            )
        })?;
        Ok(())
    }
}

pub struct AudioLauncherFunctions {
    conn: Connection,
}

impl AudioLauncherFunctions {
    pub fn new() -> Result<Self, SherlockMessage> {
        let conn = Connection::session().map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::SocketError(SocketAction::Connect),
                e
            )
        })?;
        Ok(AudioLauncherFunctions { conn })
    }
    pub fn get_current_player(&self) -> Option<String> {
        let proxy = Proxy::new(
            &self.conn,
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
        )
        .ok()?;
        let mut names: Vec<String> = proxy.call("ListNames", &()).ok()?;
        names.retain(|n| n.starts_with("org.mpris.MediaPlayer2."));
        let first = names.first().cloned();
        if let Ok(config) = ConfigGuard::read()
            && let Some(m) = config.default_apps.mpris.as_ref()
        {
            let preferred = names.into_iter().find(|name| name.contains(m));
            if preferred.is_some() {
                return preferred;
            }
        }
        first
    }
    pub fn get_metadata(&self, player: &str) -> Option<MprisData> {
        let proxy = Proxy::new(
            &self.conn,
            player,
            "/org/mpris/MediaPlayer2", // Object path for the player
            "org.freedesktop.DBus.Properties",
        )
        .ok()?;
        let message = proxy
            .call_method("GetAll", &("org.mpris.MediaPlayer2.Player"))
            .ok()?;
        let body = message.body();
        body.deserialize().ok()
    }
}

/// This function reads the "magic bytes" of images files to identify its mimetype
pub fn identify_image_type(bytes: &[u8]) -> &'static str {
    if bytes.len() < 4 {
        return "image/png";
    }

    match &bytes[0..4] {
        [0x89, 0x50, 0x4E, 0x47] => "image/png",
        [0xFF, 0xD8, 0xFF, _] => "image/jpeg",
        [0x47, 0x49, 0x46, 0x38] => "image/gif",
        [0x42, 0x4D, _, _] => "image/bmp",
        [0x52, 0x49, 0x46, 0x46] if &bytes[8..12] == b"WEBP" => "image/webp",
        _ => "image/png",
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::MusicPlayerLauncher;
    use crate::docs::launcher::{Example, InnerFunctionDoc, LauncherDoc, LauncherDocEntry};
    use crate::{display_name, variant_name};
    use indoc::indoc;

    impl LauncherDoc for MusicPlayerLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(MusicPlayerLauncher),
                variant_name: variant_name!(MusicPlayer),
                description: "Shows the currently played song or video with thumbnail, title, and artists.",
                inner_functions: &[
                    InnerFunctionDoc {
                        name: "Toggle Playback",
                        identifier: "inner.toggle_playback",
                        description: "Toggles current media playback status (playing/paused).",
                        user_facing: true,
                    },
                    InnerFunctionDoc {
                        name: "Previous",
                        identifier: "inner.previous",
                        description: "Skips to the previous audio element (song, video).",
                        user_facing: true,
                    },
                    InnerFunctionDoc {
                        name: "Next",
                        identifier: "inner.next",
                        description: "Skips to the next audio element (song, video).",
                        user_facing: true,
                    },
                ],
                examples: &[Example {
                    description: "Basic music player",
                    json: indoc! {
                        r#"{
                        "name": "Spotify",
                        "type": "music_player",
                        "args": {},
                        "priority": 2,
                        "home": "OnlyHome",
                        "spawn_focus": false,
                        "exit": false,
                        "binds": [
                            {
                                "bind": "ctrl-l",
                                "callback": "next",
                                "exit": false
                            },
                            {
                                "bind": "ctrl-h",
                                "callback": "previous",
                                "exit": false
                            }
                        ]
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

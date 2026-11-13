use std::{
    env,
    fs::{self, File},
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
};

use bytes::Bytes;
use gpui::{App, Image, ImageFormat, SharedString};
use serde_json::Value;
use simd_json::prelude::ArrayTrait;
use zbus::blocking::{Connection, Proxy};

use super::io::MprisData;
use crate::{
    config::ConfigGuard,
    define_inner_functions,
    ensure_func,
    launcher::{
        ExecEffect,
        LauncherConfig,
        LauncherProvider,
        LauncherType,
        utils::binds::Bind,
        variant_type::InnerFunction,
    },
    loader::utils::RawLauncher,
    sherlock_msg,
    skip_func_if_nav,
    ui::widgets::{ChildKind, RenderableChild, audio::MusicPlayerWidget},
    utils::errors::{
        ResultExt,
        SherlockMessage,
        types::{DBusAction, DirAction, ErrorKind, FileAction, SocketAction},
    },
};

/// The following inner functions are available:
/// - `TogglePlayback`: Toggles current media playback
/// - `Previous`: Skips to previous song
/// - `Next`: Skips to next song
#[derive(Debug, Clone, Default)]
pub struct MusicPlayerLauncher {
    binds: Option<Arc<[Bind]>>,
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
        let binds: Option<Arc<[Bind]>> = raw
            .binds
            .as_ref()
            .map(|vec| {
                vec.iter()
                    .map(|b| {
                        Bind::try_from(b)
                            .or_kind(ErrorKind::InvalidLauncher)
                            .map_err(SherlockMessage::as_error)
                    })
                    .collect::<Result<Arc<[Bind]>, _>>()
            })
            .transpose()?;
        Ok(LauncherType::MusicPlayer(MusicPlayerLauncher { binds }))
    }
    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _: &crate::loader::LoadContext,
        _opts: Arc<Value>,
        _messages: &mut Vec<SherlockMessage>,
        cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        Ok(vec![RenderableChild::new(
            launcher,
            MusicPlayerWidget::new(cx),
        )])
    }
    fn binds(&self) -> Option<Arc<[Bind]>> {
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

        let ChildKind::Music(inner) = &child.kind else {
            return Err(sherlock_msg!(
                Warning,
                ErrorKind::Unreachable,
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
        let home = env::var("HOME").or_kind_with(|| ErrorKind::Env("HOME".to_string()))?;

        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                sherlock_msg!(
                    Warning,
                    ErrorKind::Dir(DirAction::Create, parent.to_path_buf(),),
                    e.to_string()
                )
            })?;
        };

        let mut file = if path.exists() {
            File::open(&path)
        } else {
            File::create(&path)
        }
        .or_kind_with(|| ErrorKind::File(FileAction::Find, path.clone()))?;

        file.write_all(image)
            .or_kind_with(|| ErrorKind::File(FileAction::Find, path.clone()))?;
        // if file not exist, create and write it
        Ok(())
    }
    fn read_cached_cover(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let home = env::var("HOME").or_kind_with(|| ErrorKind::Env("$HOME".into()))?;
        let home_dir = PathBuf::from(home);
        let path = home_dir.join(".cache/sherlock/mpris-cache/").join(loc);

        let mut file =
            File::open(&path).or_kind_with(|| ErrorKind::File(FileAction::Find, path.clone()))?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)
            .or_kind_with(|| ErrorKind::File(FileAction::Read, path.clone()))?;
        Ok(buffer)
    }
    fn read_image_file(loc: &str) -> Result<Vec<u8>, SherlockMessage> {
        let path = PathBuf::from(loc.trim_start_matches("file://"));

        let mut file =
            File::open(&path).or_kind_with(|| ErrorKind::File(FileAction::Find, path.clone()))?;
        let mut buffer = vec![];
        file.read_to_end(&mut buffer)
            .or_kind_with(|| ErrorKind::File(FileAction::Read, path.clone()))?;
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
        let conn = Connection::session()
            .or_kind_with(|| ErrorKind::DBus(DBusAction::Connect, "Session Bus".into()))
            .map_err(SherlockMessage::as_error)?;
        let proxy = Proxy::new(
            &conn,
            player,
            "/org/mpris/MediaPlayer2",
            "org.mpris.MediaPlayer2.Player",
        )
        .or_kind_with(|| ErrorKind::DBus(DBusAction::Construct, player.to_string()))?;
        proxy
            .call_method(method, &())
            .or_kind_with(|| ErrorKind::DBus(DBusAction::Call, method.to_string()))?;
        Ok(())
    }
}

pub struct AudioLauncherFunctions {
    conn: Connection,
}

impl AudioLauncherFunctions {
    pub fn new() -> Result<Self, SherlockMessage> {
        let conn =
            Connection::session().or_kind_with(|| ErrorKind::Socket(SocketAction::Connect))?;
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
            let preferred = names.into_iter().find(|name| name.contains(&**m));
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
    use indoc::indoc;

    use super::MusicPlayerLauncher;
    use crate::{
        display_name,
        docs::{
            Documentation,
            launcher::{Example, InnerFunctionDoc, LauncherDocEntry},
        },
        variant_name,
    };

    impl Documentation for MusicPlayerLauncher {
        type Docs = LauncherDocEntry;
        fn docs() -> LauncherDocEntry {
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

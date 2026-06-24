use gpui::SharedString;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    launcher::{LauncherConfig, LauncherProvider, LauncherType, app_launcher::app_data::AppData},
    loader::{
        application_loader::file_has_changed,
        resolve_icon_path,
        utils::{PriorityGuard, RawLauncher, construct_search},
    },
    sher_log, sherlock_msg,
    ui::widgets::RenderableChild,
    utils::{
        cache::BinaryCache,
        config::{ConfigGuard, ConstantDefaults},
        errors::{
            SherlockMessage,
            types::{DbAction, FileAction, SherlockErrorType},
        },
        files::home_dir,
        paths::get_cache_dir,
    },
};

/// The following arguments are available to users:
/// - `browser`: The browser from which the bookmarks should be parsed
#[derive(Clone, Debug)]
pub struct BookmarkLauncher {
    pub target_browser: String,
}

impl LauncherProvider for BookmarkLauncher {
    fn try_parse(raw: &RawLauncher) -> Result<LauncherType, SherlockMessage> {
        let target_browser = raw
            .args
            .get("browser")
            .and_then(|s| s.as_str().map(|str| str.to_string()))
            .or_else(|| {
                ConfigGuard::read()
                    .ok()
                    .and_then(|c| c.default_apps.browser.clone())
            })
            .or_else(|| ConstantDefaults::browser().ok())
            .ok_or(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidData,
                format!(
                    "Failed to fetch browser for `{}`",
                    raw.name.as_deref().unwrap_or("BookmarkLauncher")
                )
            ))?;

        Ok(LauncherType::Bookmarks(BookmarkLauncher { target_browser }))
    }
    fn objects(
        &self,
        launcher: Arc<LauncherConfig>,
        _ctx: &crate::loader::LoadContext,
        _opts: Arc<serde_json::Value>,
        _messages: &mut Vec<SherlockMessage>,
        _cx: &mut gpui::App,
    ) -> Result<Vec<RenderableChild>, SherlockMessage> {
        BookmarkLauncher::find_bookmarks(&self.target_browser, Arc::clone(&launcher)).map(|ad| {
            ad.into_iter()
                .map(|inner| RenderableChild::App {
                    launcher: Arc::clone(&launcher),
                    inner,
                })
                .collect()
        })
    }
}

impl BookmarkLauncher {
    pub fn find_bookmarks(
        browser: &str,
        launcher: Arc<LauncherConfig>,
    ) -> Result<Vec<AppData>, SherlockMessage> {
        match browser.to_lowercase().as_str() {
            "zen" | "zen-browser" | "/opt/zen-browser-bin/zen-bin %u" => {
                BookmarkParser::zen(launcher)
            }
            "brave" | "brave %u" => BookmarkParser::brave(launcher),
            "firefox" | "/usr/lib/firefox/firefox %u" => BookmarkParser::firefox(launcher),
            "chrome" | "google-chrome" | "/usr/bin/google-chrome-stable %u" => {
                BookmarkParser::chrome(launcher)
            }
            "thorium" | "/usr/bin/thorium-browser %u" => BookmarkParser::thorium(launcher),
            _ => {
                sher_log!(format!(
                    r#"Failed to gather bookmarks for browser: "{}""#,
                    browser
                ))?;
                Err(sherlock_msg!(
                    Warning,
                    SherlockErrorType::ConfigError("invalid browser configuration".into()),
                    format!(
                        "The browser \"{}\" is either not supported or not recognized.",
                        browser
                    )
                ))
            }
        }
    }
}

struct BookmarkParser;
impl BookmarkParser {
    fn brave(launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        let path = home_dir()?.join(".config/BraveSoftware/Brave-Browser/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            )
        })?;

        ChromeParser::parse(launcher, data)
    }
    fn thorium(launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        let path = home_dir()?.join(".config/thorium/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            )
        })?;
        ChromeParser::parse(launcher, data)
    }
    fn chrome(launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        let path = home_dir()?.join(".config/google-chrome/Default/Bookmarks");
        let data = fs::read_to_string(&path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(FileAction::Read, path.to_path_buf()),
                e
            )
        })?;
        ChromeParser::parse(launcher, data)
    }

    fn zen(launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        fn get_path() -> Option<PathBuf> {
            let zen_root = home_dir().ok()?.join(".zen");
            fs::read_dir(&zen_root)
                .ok()?
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_dir() && path.join("places.sqlite").exists() {
                        Some(path.join("places.sqlite"))
                    } else {
                        None
                    }
                })
                .next()
        }
        let path = get_path().ok_or_else(|| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(
                    FileAction::Find,
                    PathBuf::from("~/.zen/../places.sqlite")
                ),
                "File does not exist"
            )
        })?;
        let parser = MozillaSqliteParser::new(path, "zen");
        parser.read(launcher, "zen")
    }
    fn firefox(launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        fn get_path() -> Option<PathBuf> {
            let zen_root = home_dir().ok()?.join(".mozilla/firefox/");
            fs::read_dir(&zen_root)
                .ok()?
                .filter_map(|entry| {
                    let path = entry.ok()?.path();
                    if path.is_dir() && path.join("places.sqlite").exists() {
                        Some(path.join("places.sqlite"))
                    } else {
                        None
                    }
                })
                .next()
        }
        let path = get_path().ok_or_else(|| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::FileError(
                    FileAction::Find,
                    PathBuf::from("~/.mozilla/firefox/../places.sqlite",)
                ),
                "File does not exist"
            )
        })?;
        let parser = MozillaSqliteParser::new(path, "firefox");
        parser.read(launcher, "firefox")
    }
}
struct MozillaSqliteParser {
    path: PathBuf,
}
impl MozillaSqliteParser {
    fn new(file: PathBuf, prefix: &str) -> Self {
        let path = if let Ok(cache) = get_cache_dir() {
            let target = cache.join(format!("bookmarks/{}-places.sqlite", prefix));
            Self::copy_if_needed(&file, &target);
            target
        } else {
            file.to_path_buf()
        };
        Self { path }
    }
    fn read(
        &self,
        launcher: Arc<LauncherConfig>,
        prefix: &str,
    ) -> Result<Vec<AppData>, SherlockMessage> {
        let cache_dir = get_cache_dir()?;
        let cache = cache_dir.join(format!("bookmarks/{}-cache.bin", prefix));

        if !file_has_changed(&cache, &self.path)
            && let Ok(app_data) = BinaryCache::read::<Vec<AppData>, _>(&cache)
        {
            return Ok(app_data);
        }

        let bookmarks = self.read_new(launcher)?;
        rayon::spawn_fifo({
            let bookmarks = bookmarks.clone();
            move || {
                let _ = BinaryCache::write(&cache, &bookmarks);
            }
        });
        Ok(bookmarks)
    }
    fn read_new(&self, launcher: Arc<LauncherConfig>) -> Result<Vec<AppData>, SherlockMessage> {
        let mut res: Vec<AppData> = Vec::new();
        let query = "
            SELECT b.title, p.url
            FROM moz_bookmarks b
            JOIN moz_places p ON b.fk = p.id
            WHERE b.type = 1
            AND b.title IS NOT NULL
            AND p.url IS NOT NULL
            AND b.parent != 7;
            ";
        let conn = Connection::open(&self.path).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DatabaseError(DbAction::Connect),
                e
            )
        })?;

        if let Ok(mut stmt) = conn.prepare(query) {
            let event_iter = stmt.query_map([], |row| {
                let title: String = row.get(0)?;
                let url: String = row.get(1)?;

                Ok((title, url))
            });

            if let Ok(rows) = event_iter {
                for row in rows.flatten() {
                    let bookmark = AppData {
                        name: Some(SharedString::from(&row.0)),
                        icon: resolve_icon_path("sherlock-bookmark"),
                        search_string: construct_search(Some(&row.0), &row.1, true),
                        exec: Some(row.1),
                        priority: PriorityGuard::new_with_launcher(&launcher, 0),
                        ..AppData::new()
                    };
                    res.push(bookmark);
                }
            }
        }
        Ok(res)
    }
    fn should_update_cache(dest: &PathBuf, source: &PathBuf) -> bool {
        if !dest.exists() {
            return true;
        }

        let source_mod = fs::metadata(source)
            .ok()
            .and_then(|meta| meta.modified().ok());
        let dest_mod = fs::metadata(dest)
            .ok()
            .and_then(|meta| meta.modified().ok());

        if let (Some(source), Some(dest)) = (source_mod, dest_mod) {
            return source > dest;
        }
        true
    }
    fn copy_if_needed(src: &PathBuf, dst: &PathBuf) {
        if Self::should_update_cache(dst, src) {
            let _ = sher_log!(format!(
                r#"Bookmark database "{}" is copied to "{}""#,
                src.display(),
                dst.display()
            ));
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, dst);
        }
    }
}
struct ChromeParser;
impl ChromeParser {
    fn parse(launcher: Arc<LauncherConfig>, data: String) -> Result<Vec<AppData>, SherlockMessage> {
        mod parser {
            use std::collections::HashMap;

            use serde::Deserialize;

            #[derive(Deserialize)]
            pub struct ChromeBookmark {
                pub name: String,
                pub r#type: String,
                pub children: Option<Vec<ChromeBookmark>>,
                pub url: Option<String>,
            }

            #[derive(Deserialize)]
            pub struct ChromeFile {
                pub roots: HashMap<String, ChromeBookmark>,
            }
        }

        let mut bookmarks = Vec::new();
        let file = serde_json::from_str::<parser::ChromeFile>(&data).map_err(|e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DeserializationError("Chrome Bookmarks".into()),
                e
            )
        })?;

        fn process_bookmark(
            launcher: Arc<LauncherConfig>,
            bookmarks: &mut Vec<AppData>,
            bookmark: parser::ChromeBookmark,
        ) {
            match bookmark.r#type.as_ref() {
                "folder" => {
                    if let Some(children) = bookmark.children {
                        for child in children {
                            process_bookmark(Arc::clone(&launcher), bookmarks, child);
                        }
                    }
                }
                "url" => {
                    if let Some(url) = bookmark.url {
                        bookmarks.push(AppData {
                            name: Some(SharedString::from(&bookmark.name)),
                            icon: resolve_icon_path("sherlock-bookmark"),
                            exec: Some(url.clone()),
                            search_string: construct_search(Some(&bookmark.name), &url, true),
                            priority: PriorityGuard::new_with_launcher(&launcher, 0),
                            ..AppData::new()
                        });
                    }
                }
                _ => {}
            };
        }

        for (_name, bookmark) in file.roots {
            process_bookmark(launcher.clone(), &mut bookmarks, bookmark);
        }

        Ok(bookmarks)
    }
}

// DOCS
#[cfg(feature = "docs")]
mod docs {
    use super::BookmarkLauncher;
    use crate::{
        display_name,
        docs::launcher::{Example, FieldDoc, LauncherDoc, LauncherDocEntry},
        variant_name,
    };
    use indoc::indoc;
    impl LauncherDoc for BookmarkLauncher {
        fn doc() -> LauncherDocEntry {
            LauncherDocEntry {
                name: display_name!(BookmarkLauncher),
                variant_name: variant_name!(Bookmarks),
                description: "Launches browser bookmarks in your default browser.",
                args: &[FieldDoc {
                    name: "browser",
                    ty: "string",
                    required: false,
                    default: Some("Default browser"),
                    description: "The browser from which the bookmarks should be parsed",
                }],
                examples: &[Example {
                    description: "Basic bookmarks launcher",
                    json: indoc! {
                        r#"{
                        "name": "Bookmarks",
                        "type": "bookmarks",
                        "alias": "bm",
                        "args": {
                            "browser": "brave"
                        },
                        "priority": 7
                    }"#
                    },
                }],
                ..LauncherDocEntry::new()
            }
        }
    }
}

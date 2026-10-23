use rusqlite::{Connection, OpenFlags};
use std::collections::HashSet;
use std::hash::{DefaultHasher, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fs, hash::Hash};

use crate::ui::widgets::list_item::{ItemMeta, ListItem, web::WebPayload};
use crate::utils::errors::types::DirAction;
use crate::{
    launcher::{LauncherConfig, LauncherProvider, LauncherType},
    loader::{
        application_loader::file_has_changed,
        resolve_icon_path,
        utils::{RawLauncher, construct_search},
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
                .map(|inner| RenderableChild::new(Arc::clone(&launcher), inner))
                .collect()
        })
    }
}

type Bookmarks = Vec<ListItem<WebPayload>>;

impl BookmarkLauncher {
    pub fn find_bookmarks(
        browser: &str,
        launcher: Arc<LauncherConfig>,
    ) -> Result<Bookmarks, SherlockMessage> {
        match browser.to_lowercase().as_str() {
            "zen" | "zen-browser" | "/opt/zen-browser-bin/zen-bin %u" => BookmarkParser::zen(),
            "brave" | "brave %u" => BookmarkParser::brave(launcher),
            "firefox" | "/usr/lib/firefox/firefox %u" => BookmarkParser::firefox(),
            "librewolf" => BookmarkParser::librewolf(),
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
    fn brave(launcher: Arc<LauncherConfig>) -> Result<Bookmarks, SherlockMessage> {
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
    fn thorium(launcher: Arc<LauncherConfig>) -> Result<Bookmarks, SherlockMessage> {
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
    fn chrome(launcher: Arc<LauncherConfig>) -> Result<Bookmarks, SherlockMessage> {
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

    fn zen() -> Result<Bookmarks, SherlockMessage> {
        let root = home_dir()?.join(".zen");
        MozillaSqliteParser::run_for_browser("zen", || root)
    }
    fn librewolf() -> Result<Bookmarks, SherlockMessage> {
        let root = home_dir()?.join(".config/librewolf/librewolf");
        MozillaSqliteParser::run_for_browser("librewolf", || root)
    }
    fn firefox() -> Result<Bookmarks, SherlockMessage> {
        let root = home_dir()?.join(".mozilla/firefox/");
        MozillaSqliteParser::run_for_browser("firefox", || root)
    }
}
struct MozillaSqliteParser {
    paths: Vec<PathBuf>,
}

impl MozillaSqliteParser {
    fn new(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let cache = get_cache_dir().ok().map(|c| c.join("bookmarks"));

        let paths = paths
            .into_iter()
            .map(|p| {
                let Some(dir) = &cache else { return p };

                let mut hasher = DefaultHasher::new();
                p.hash(&mut hasher);
                let target = dir.join(format!("{:016x}.sqlite", hasher.finish()));

                match Self::copy_if_needed(&p, &target) {
                    Ok(()) => target,
                    Err(_) => p,
                }
            })
            .collect();

        Self { paths }
    }

    fn run_for_browser<R>(prefix: &'static str, root: R) -> Result<Bookmarks, SherlockMessage>
    where
        R: FnOnce() -> PathBuf,
    {
        let root = root();

        let entries = fs::read_dir(&root).map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::DirError(DirAction::Read, root),
                e.to_string()
            )
        })?;
        let paths = entries.filter_map(|entry| {
            let db = entry.ok()?.path().join("places.sqlite");
            db.is_file().then_some(db)
        });

        let parser = MozillaSqliteParser::new(paths);
        parser.read(prefix)
    }

    fn read(&self, prefix: &'static str) -> Result<Bookmarks, SherlockMessage> {
        let cache = get_cache_dir()?.join(format!("bookmarks/{prefix}-cache.bin"));

        let changed = self.paths.iter().any(|p| file_has_changed(&cache, p));
        if !changed && let Ok(app_data) = BinaryCache::read::<Bookmarks, _>(&cache) {
            return Ok(app_data);
        }

        let bookmarks = self.read_new()?;
        rayon::spawn_fifo({
            let bookmarks = bookmarks.clone();
            move || {
                let _ = BinaryCache::write(&cache, &bookmarks);
            }
        });
        Ok(bookmarks)
    }

    fn read_new(&self) -> Result<Bookmarks, SherlockMessage> {
        let mut res = Vec::new();
        let mut seen = HashSet::new();

        for path in &self.paths {
            match Self::read_db(path) {
                Ok(rows) => {
                    for (title, url) in rows {
                        if !seen.insert(url.clone()) {
                            continue;
                        }
                        res.push(ListItem {
                            meta: ItemMeta {
                                icon: resolve_icon_path("sherlock-bookmark"),
                                search: construct_search(Some(&title), &url, true).into(),
                                ..ItemMeta::new(title)
                            },
                            payload: WebPayload {
                                url: Some(url.into()),
                            },
                        });
                    }
                }
                Err(e) => {
                    let _ = sher_log!(format!("{}: {e:?}", path.display()));
                }
            }
        }
        Ok(res)
    }
    fn read_db(path: &Path) -> Result<Vec<(String, String)>, SherlockMessage> {
        const QUERY: &str = "
            SELECT b.title, p.url
            FROM moz_bookmarks b
            JOIN moz_places p ON b.fk = p.id
            WHERE b.type = 1
              AND b.title IS NOT NULL
              AND p.url IS NOT NULL
              AND b.parent != 7";

        let db_err = |e| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::DatabaseError(DbAction::Connect),
                e
            )
        };

        let conn =
            Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(db_err)?;
        let mut stmt = conn.prepare(QUERY).map_err(db_err)?;
        let rows = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(db_err)?
            .flatten()
            .collect();
        Ok(rows)
    }
    fn should_update_cache(dest: &Path, source: &Path) -> bool {
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
    fn copy_if_needed(src: &Path, dst: &Path) -> std::io::Result<()> {
        if !Self::should_update_cache(dst, src) {
            return Ok(());
        }

        let _ = sher_log!(format!(
            r#"Bookmark database "{}" is copied to "{}""#,
            src.display(),
            dst.display()
        ));

        if let Some(parent) = dst.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(src, dst)?;

        let (src_wal, dst_wal) = (
            src.with_extension("sqlite-wal"),
            dst.with_extension("sqlite-wal"),
        );
        if src_wal.exists() {
            fs::copy(&src_wal, &dst_wal)?;
        } else {
            let _ = fs::remove_file(&dst_wal);
        }
        Ok(())
    }
}
struct ChromeParser;
impl ChromeParser {
    fn parse(launcher: Arc<LauncherConfig>, data: String) -> Result<Bookmarks, SherlockMessage> {
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
            bookmarks: &mut Bookmarks,
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
                        bookmarks.push(ListItem {
                            meta: ItemMeta {
                                search: construct_search(Some(&bookmark.name), &url, true).into(),
                                icon: resolve_icon_path("sherlock-bookmark"),
                                ..ItemMeta::new(bookmark.name)
                            },
                            payload: WebPayload {
                                url: Some(url.into()),
                            },
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

pub mod app_launcher;
pub mod audio_launcher;
pub mod bookmark_launcher;
pub mod calc_launcher;
pub mod category_launcher;
pub mod children;
pub mod event_launcher;
pub mod system_cmd_launcher;
pub mod utils;
pub mod weather_launcher;
pub mod web_launcher;
// Integrate later: TODO
// pub mod clipboard_launcher;
// pub mod bulk_text_launcher;
// pub mod pipe_launcher;
// pub mod emoji_picker;
// pub mod file_launcher;
// pub mod pomodoro_launcher;
// pub mod process_launcher;
// pub mod theme_picker;

use serde::de::IntoDeserializer;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    vec,
};

use crate::{
    launcher::{
        children::{RenderableChild, calc_data::CalcData},
        weather_launcher::WeatherData,
    },
    loader::{
        Loader,
        application_loader::parse_priority,
        resolve_icon_path,
        utils::{
            AppData, ApplicationAction, CounterReader, RawLauncher, deserialize_named_appdata,
        },
    },
    utils::{
        command_launch::spawn_detached, config::HomeType, errors::SherlockError,
        websearch::websearch,
    },
};

use app_launcher::AppLauncher;
use audio_launcher::MusicPlayerLauncher;
use bookmark_launcher::BookmarkLauncher;
use calc_launcher::CalculatorLauncher;
use category_launcher::CategoryLauncher;
use event_launcher::EventLauncher;
use gpui::{App, AsyncApp, Entity, SharedString};
use serde_json::Value;
use system_cmd_launcher::CommandLauncher;
use weather_launcher::WeatherLauncher;
use web_launcher::WebLauncher;

// Integrate later: TODO
// use bulk_text_launcher::BulkTextLauncher;
// use clipboard_launcher::ClipboardLauncher;
// use emoji_picker::EmojiPicker;
// use file_launcher::FileLauncher;
// use pomodoro_launcher::Pomodoro;
// use process_launcher::ProcessLauncher;
// use theme_picker::ThemePicker;

#[derive(Clone, Debug, Default)]
pub enum LauncherType {
    App(AppLauncher),
    Bookmark(BookmarkLauncher),
    Calc(CalculatorLauncher),
    Category(CategoryLauncher),
    Command(CommandLauncher),
    Event(EventLauncher),
    MusicPlayer(MusicPlayerLauncher),
    Weather(WeatherLauncher),
    Web(WebLauncher),
    #[default]
    Empty,
    // Integrate later: TODO
    // Pipe(PipeLauncher),
    // Api(BulkTextLauncher),
    // Clipboard(ClipboardLauncher),
    // Emoji(EmojiPicker),
    // File(FileLauncher),
    // Pomodoro(Pomodoro),
    // Process(ProcessLauncher),
    // Theme(ThemePicker),
}

impl LauncherType {
    pub fn get_render_obj(
        &self,
        launcher: Arc<Launcher>,
        opts: Arc<Value>,
        counts: &HashMap<String, u32>,
        decimals: i32,
        cx: &mut App,
        data_handle: Entity<Arc<Vec<RenderableChild>>>,
    ) -> Option<Vec<RenderableChild>> {
        match self {
            Self::App(app) => {
                Loader::load_applications(Arc::clone(&launcher), counts, decimals, app.use_keywords)
                    .map(|ad| {
                        ad.into_iter()
                            .map(|inner| RenderableChild::AppLike {
                                launcher: Arc::clone(&launcher),
                                inner,
                            })
                            .collect()
                    })
                    .ok()
            }

            Self::Bookmark(bkm) => {
                BookmarkLauncher::find_bookmarks(&bkm.target_browser, Arc::clone(&launcher))
                    .map(|ad| {
                        ad.into_iter()
                            .map(|inner| RenderableChild::AppLike {
                                launcher: Arc::clone(&launcher),
                                inner,
                            })
                            .collect()
                    })
                    .ok()
            }

            Self::Calc(_) => {
                let capabilities: HashSet<String> = match opts.get("capabilities") {
                    Some(Value::Array(arr)) => arr
                        .iter()
                        .filter_map(|v| v.as_str().map(str::to_string))
                        .collect(),
                    _ => HashSet::from([String::from("calc.math"), String::from("calc.units")]),
                };
                let inner = CalcData::new(capabilities);

                Some(vec![RenderableChild::CalcLike { launcher, inner }])
            }

            Self::Command(_) => {
                let cmds = opts.get("commands")?;
                let app_data =
                    deserialize_named_appdata(cmds.clone().into_deserializer()).unwrap_or_default();
                let children: Vec<RenderableChild> = app_data
                    .into_iter()
                    .map(|mut inner| {
                        let count = inner
                            .exec
                            .as_deref()
                            .and_then(|exec| counts.get(exec))
                            .copied()
                            .unwrap_or(0u32);
                        inner.icon = inner
                            .icon
                            .and_then(|i| i.to_str().and_then(resolve_icon_path));
                        inner.priority =
                            Some(parse_priority(launcher.priority as f32, count, decimals));
                        RenderableChild::AppLike {
                            launcher: Arc::clone(&launcher),
                            inner,
                        }
                    })
                    .collect();

                Some(children)
            }

            Self::Weather(wttr) => {
                match WeatherData::from_cache(wttr) {
                    Some(inner) => Some(vec![RenderableChild::WeatherLike { launcher, inner }]),
                    None => {
                        // 1. Data isn't cached, start the fetch
                        let wttr_clone = wttr.clone();

                        cx.spawn(|cx: &mut AsyncApp| {
                            let cx = cx.clone();
                            async move {
                                if let Some((new_weather_data, _)) =
                                    WeatherData::fetch_async(&wttr_clone).await
                                {
                                    let _ = cx.update(|cx| {
                                        // Update the entity's inner data
                                        data_handle.update(cx, {
                                            |items_arc, cx| {
                                                let items = Arc::make_mut(items_arc);

                                                for item in items.iter_mut() {
                                                    if let RenderableChild::WeatherLike {
                                                        inner,
                                                        ..
                                                    } = item
                                                    {
                                                        *inner = new_weather_data.clone();
                                                    }
                                                }

                                                cx.notify();
                                            }
                                        });
                                    });
                                }
                            }
                        })
                        .detach();

                        // Return None or a "Loading" placeholder for now
                        Some(vec![RenderableChild::WeatherLike {
                            launcher: Arc::clone(&launcher),
                            inner: WeatherData::uninitialized(),
                        }])
                    }
                }
            }

            Self::Web(_) => {
                let mut inner = AppData::new();
                inner.icon = opts
                    .get("icon")
                    .and_then(Value::as_str)
                    .and_then(|i| resolve_icon_path(i));

                Some(vec![RenderableChild::AppLike { launcher, inner }])
            }

            _ => None,
        }
    }
}

// // Async tiles
// LauncherType::BulkText(bulk_text) => Tile::bulk_text_tile(launcher, &bulk_text).await,
// LauncherType::MusicPlayer(mpris) => Tile::mpris_tile(launcher, &mpris).await,
// LauncherType::Weather(_) => Tile::weather_tile_loader(launcher).await,
/// # Launcher
/// ### Fields:
/// - **name:** Specifies the name of the launcher – such as a category e.g. `App Launcher`
/// - **alias:** Also referred to as `mode` – specifies the mode in which the launcher children should
/// be active in
/// - **tag_start:** Specifies the text displayed in a custom UI Label
/// - **tag_end:** Specifies the text displayed in a custom UI Label
/// - **method:** Specifies the action that should be executed on `row-should-activate` action
/// - **next_content:** Specifies the content to be displayed whenever method is `next`
/// - **priority:** Base priority all children inherit from. Children priority will be a combination
/// of this together with their execution counts and levenshtein similarity
/// - **r#async:** Specifies whether the tile should be loaded/executed asynchronously
/// - **home:** Specifies whether the children should show on the `home` mode (empty
/// search entry & mode == `all`)
/// - **launcher_type:** Used to specify the kind of launcher and subsequently its children
/// - **shortcut:** Specifies whether the child tile should show `modekey + number` shortcuts
/// - **spawn_focus:** Specifies whether the tile should have focus whenever Sherlock launches
/// search entry & mode == `all`)
#[derive(Clone, Debug, Default)]
pub struct Launcher {
    pub name: Option<String>,
    pub display_name: Option<SharedString>,
    pub icon: Option<String>,
    pub alias: Option<String>,
    pub tag_end: Option<String>,
    pub method: String,
    pub exit: bool,
    pub next_content: Option<String>,
    pub priority: u32,
    pub r#async: bool,
    pub home: HomeType,
    pub launcher_type: LauncherType,
    pub shortcut: bool,
    pub spawn_focus: bool,
    pub actions: Option<Vec<ApplicationAction>>,
    pub add_actions: Option<Vec<ApplicationAction>>,
}
impl Launcher {
    pub fn from_raw(
        raw: RawLauncher,
        method: String,
        launcher_type: LauncherType,
        icon: Option<String>,
    ) -> Self {
        Self {
            name: raw.name,
            display_name: raw.display_name.map(|n| SharedString::from(n)),
            icon,
            alias: raw.alias,
            tag_end: raw.tag_end,
            method,
            exit: raw.exit,
            next_content: raw.next_content,
            priority: raw.priority as u32,
            r#async: raw.r#async,
            home: raw.home,
            launcher_type,
            shortcut: raw.shortcut,
            spawn_focus: raw.spawn_focus,
            actions: raw.actions,
            add_actions: raw.add_actions,
        }
    }

    pub fn execute<'a>(
        &self,
        what: &'a ExecMode,
        keyword: &str,
        variables: &[(SharedString, SharedString)],
    ) -> Result<bool, SherlockError> {
        match what {
            ExecMode::App { exec, terminal } => {
                let cmd = if *terminal {
                    format!(r#"{{terminal}} {exec}"#)
                } else {
                    exec.to_string()
                };
                spawn_detached(&cmd, keyword, variables)?;
                increment(exec);
            }
            ExecMode::Commmand { exec } => {
                spawn_detached(exec, keyword, variables)?;
                increment(exec);
            }
            ExecMode::Web {
                engine,
                browser,
                exec,
            } => {
                let engine = engine.as_deref().unwrap_or("plain");
                let query = if let Some(query) = exec {
                    query
                } else {
                    keyword
                };
                websearch(engine, query, browser.as_deref(), variables)?;
            }
            _ => {}
        };

        Ok(true)
    }
}
fn increment(key: &str) {
    if let Ok(count_reader) = CounterReader::new() {
        let _ = count_reader.increment(key);
    };
}

pub enum ExecMode<'a> {
    App {
        exec: &'a str,
        terminal: bool,
    },
    Commmand {
        exec: &'a str,
    },
    Web {
        engine: Option<&'a str>,
        browser: Option<&'a str>,
        exec: Option<&'a str>,
    },
    None,
}
impl<'a> ExecMode<'a> {
    pub fn from_appdata(app_data: &'a AppData, launcher: &'a Arc<Launcher>) -> Self {
        match &launcher.launcher_type {
            LauncherType::App(_) => Self::App {
                exec: app_data.exec.as_deref().unwrap_or(""),
                terminal: app_data.terminal,
            },
            LauncherType::Bookmark(bkm) => Self::Web {
                engine: None,
                browser: Some(&bkm.target_browser),
                exec: app_data.exec.as_deref(),
            },
            LauncherType::Command(_) => Self::Commmand {
                exec: app_data.exec.as_deref().unwrap_or(""),
            },
            LauncherType::Web(web) => Self::Web {
                engine: Some(&web.engine),
                browser: web.browser.as_deref(),
                exec: app_data.exec.as_deref(),
            },
            _ => Self::None,
        }
    }
    pub fn from_app_action(action: &'a ApplicationAction, _launcher: &'a Arc<Launcher>) -> Self {
        match action.method.as_str() {
            "app_launcher" | "command" => Self::Commmand {
                exec: action.exec.as_deref().unwrap_or(""),
            },

            _ => Self::None,
        }
    }
}

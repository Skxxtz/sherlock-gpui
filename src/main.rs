use once_cell::sync::OnceCell;
use std::{
    io::Write,
    sync::{Arc, RwLock},
};
use tokio::net::UnixListener;

use gpui::{
    layer_shell::{Layer, LayerShellOptions},
    *,
};

use crate::{
    launcher::children::RenderableChild,
    loader::{CustomIconTheme, IconThemeGuard, Loader, assets::Assets},
    ui::{
        main_window::{LauncherMode, NextVar, OpenContext, PrevVar},
        search_bar::EmptyBackspace,
    },
    utils::{
        config::{ConfigGuard, SherlockConfig},
        errors::SherlockErrorType,
    },
};

mod launcher;
mod loader;
mod prelude;
mod ui;
mod utils;

use ui::main_window::{Execute, FocusNext, FocusPrev, Quit, SherlockMainWindow};
use ui::search_bar::{
    Backspace, Copy, Cut, Delete, DeleteAll, End, Home, Left, Paste, Right, SelectAll, TextInput,
};

use utils::errors::SherlockError;

static ICONS: OnceCell<RwLock<CustomIconTheme>> = OnceCell::new();
static CONFIG: OnceCell<RwLock<SherlockConfig>> = OnceCell::new();

fn setup() -> Result<(), SherlockError> {
    let mut flags = Loader::load_flags()?;

    let config = flags.to_config().map_or_else(
        |e| {
            eprintln!("{e}");
            let defaults = SherlockConfig::default();
            SherlockConfig::apply_flags(&mut flags, defaults)
        },
        |(cfg, non_crit)| {
            if !non_crit.is_empty() {
                eprintln!("{:?}", non_crit);
            }
            cfg
        },
    );

    // Load custom icons
    let _ = ICONS.set(RwLock::new(CustomIconTheme::new()));
    config.appearance.icon_paths.iter().for_each(|path| {
        if let Err(e) = IconThemeGuard::add_path(path) {
            eprintln!("{:?}", e);
        }
    });

    // Create global config
    CONFIG
        .set(RwLock::new(config.clone()))
        .map_err(|_| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;

    Ok(())
}

#[tokio::main]
async fn main() {
    // connect to existing socket
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let _ = stream.write_all(b"open");
        return;
    }

    if let Err(e) = setup() {
        eprintln!("{e}");
    }

    // start primary instance
    let app = Application::new().with_assets(Assets);
    app.with_quit_mode(QuitMode::Explicit).run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("backspace", Backspace, None),
            KeyBinding::new("delete", Delete, None),
            KeyBinding::new("ctrl-backspace", DeleteAll, None),
            KeyBinding::new("ctrl-a", SelectAll, None),
            KeyBinding::new("ctrl-v", Paste, None),
            KeyBinding::new("ctrl-c", Copy, None),
            KeyBinding::new("ctrl-x", Cut, None),
            KeyBinding::new("home", Home, None),
            KeyBinding::new("end", End, None),
            KeyBinding::new("left", Left, None),
            KeyBinding::new("right", Right, None),
            KeyBinding::new("escape", Quit, None),
            KeyBinding::new("down", FocusNext, None),
            KeyBinding::new("up", FocusPrev, None),
            KeyBinding::new("enter", Execute, None),
            KeyBinding::new("tab", NextVar, None),
            KeyBinding::new("shift-tab", PrevVar, None),
            KeyBinding::new("ctrl-l", OpenContext, None),
        ]);

        let socket_path = "/tmp/sherlock.sock";
        let data: Entity<Arc<Vec<RenderableChild>>> = cx.new(|_| Arc::new(Vec::new()));
        let modes = match Loader::load_launchers(cx, data.clone()) {
            Ok(modes) => modes,
            Err(e) => {
                eprintln!("{e}");
                return;
            }
        };

        spawn_launcher(cx, data.clone(), Arc::clone(&modes));

        // listen for open requests
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path).unwrap();

        cx.spawn(|cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                let mut win: Option<AnyWindowHandle> = None;
                loop {
                    if let Ok((_stream, _)) = listener.accept().await {
                        cx.update(|cx| {
                            // Close old window
                            if let Some(old_win) = win.take() {
                                let _ = old_win.update(cx, |_, win, _| {
                                    win.remove_window();
                                });
                            }

                            // Create new window
                            win = Some(spawn_launcher(cx, data.clone(), Arc::clone(&modes)));
                        })
                        .ok();
                    } else {
                        eprintln!("Broken UNIX Socket.");
                    }
                }
            }
        })
        .detach();
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: Entity<Arc<Vec<RenderableChild>>>,
    modes: Arc<[LauncherMode]>,
) -> AnyWindowHandle {
    // For now load application here
    let window = cx
        .open_window(get_window_options(), |_, cx| {
            let text_input = cx.new(|cx| TextInput {
                focus_handle: cx.focus_handle(),
                content: "".into(),
                placeholder: "Search:".into(),
                variable: None,
                selected_range: 0..0,
                selection_reversed: false,
                marked_range: None,
                last_layout: None,
                last_bounds: None,
                is_selecting: false,
            });
            cx.new(|cx| {
                let data_len = data.read(cx).len();
                let sub = cx.observe(
                    &text_input,
                    move |this: &mut SherlockMainWindow, _ev, cx| {
                        this.selected_index = 0;
                        this.filter_and_sort(cx);
                    },
                );
                let backspace_sub =
                    cx.subscribe(&text_input, |this, _, _ev: &EmptyBackspace, cx| {
                        if this.mode != LauncherMode::Home {
                            this.mode = LauncherMode::Home;

                            // Propagate changes to ui
                            this.last_query = None;
                            this.selected_index = 0;
                            this.filter_and_sort(cx);
                        }
                    });

                let list_state = ListState::new(data_len, ListAlignment::Top, px(48.));

                let mut view = SherlockMainWindow {
                    text_input,
                    focus_handle: cx.focus_handle(),
                    list_state,
                    _subs: vec![sub, backspace_sub],
                    selected_index: 0,
                    // modes
                    mode: LauncherMode::Home,
                    modes,
                    // context menu
                    context_idx: None,
                    context_actions: Arc::new([]),
                    // variable inputs
                    variable_input: Vec::new(),
                    active_bar: 0,
                    // Data model
                    data,
                    deferred_render_task: None,
                    last_query: None,
                    filtered_indices: (0..data_len).collect(),
                };
                view.filter_and_sort(cx);

                view
            })
        })
        .unwrap();

    window
        .update(cx, |view, window, cx| {
            window.focus(&view.text_input.focus_handle(cx));
            cx.activate(true);
        })
        .unwrap();

    window.into()
}

fn get_window_options() -> WindowOptions {
    let (width, height) = ConfigGuard::read()
        .map(|c| (c.appearance.width, c.appearance.height))
        .unwrap_or((900i32, 600i32));

    WindowOptions {
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "sherlock".to_string(),
            layer: Layer::Overlay,
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(width as f32), px(height as f32)),
        })),
        window_background: WindowBackgroundAppearance::Blurred,
        ..Default::default()
    }
}

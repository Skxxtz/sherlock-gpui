use once_cell::sync::OnceCell;
use std::{collections::HashMap, io::Write, sync::RwLock};

use gpui::{
    layer_shell::{Layer, LayerShellOptions},
    *,
};

use crate::{
    loader::Loader,
    search_view::{
        Backspace, Copy, Cut, Delete, DeleteAll, End, Execute, FocusNext, FocusPrev, Home,
        InputExample, Left, Paste, Quit, Right, SelectAll, TextInput,
    },
    utils::{config::SherlockConfig, errors::SherlockErrorType},
};

mod loader;
mod prelude;
mod search_view;
mod ui;
mod utils;

use utils::errors::SherlockError;

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

    // Create global config
    CONFIG
        .set(RwLock::new(config.clone()))
        .map_err(|_| sherlock_error!(SherlockErrorType::ConfigError(None), ""))?;

    Ok(())
}

fn main() {
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
    let app = Application::new();
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
        ]);

        let socket_path = "/tmp/sherlock.sock";

        spawn_launcher(cx);

        // listen for open requests
        let _ = std::fs::remove_file(socket_path);
        let listener = async_net::unix::UnixListener::bind(socket_path).unwrap();

        cx.spawn(|cx: &mut AsyncApp| {
            let cx = cx.clone();
            async move {
                loop {
                    if let Ok((_stream, _)) = listener.accept().await {
                        cx.update(|cx| {
                            spawn_launcher(cx);
                        })
                        .ok();
                    }
                }
            }
        })
        .detach();
    });
}

fn spawn_launcher(cx: &mut App) -> AnyWindowHandle {
    // For now load application here
    let counts = HashMap::new();

    let window = cx
        .open_window(get_window_options(), |_, cx| {
            let text_input = cx.new(|cx| TextInput {
                focus_handle: cx.focus_handle(),
                content: "".into(),
                placeholder: "Search:".into(),
                selected_range: 0..0,
                selection_reversed: false,
                marked_range: None,
                last_layout: None,
                last_bounds: None,
                is_selecting: false,
            });
            cx.new(|cx| {
                // let sub = cx.observe_keystrokes(move |this: &mut InputExample, ev, _, cx| {
                //     let old_count = this.data.len();
                //     this.data.push(ev.keystroke.clone());

                //     this.list_state.splice(old_count..old_count, 1);
                //     cx.notify();
                // });
                let apps = Loader::load_applications(1.0, &counts, 2, true).unwrap_or_default();

                let list_state = ListState::new(apps.len(), ListAlignment::Top, px(48.));

                InputExample {
                    text_input,
                    data: apps,
                    focus_handle: cx.focus_handle(),
                    list_state,
                    _subs: vec![],
                    selected_index: 0,
                }
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
    WindowOptions {
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "sherlock".to_string(),
            layer: Layer::Overlay,
            ..Default::default()
        }),
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: Size::new(px(900.), px(600.)),
        })),
        window_background: WindowBackgroundAppearance::Blurred,
        ..Default::default()
    }
}

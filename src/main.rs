use std::io::Write;

use gpui::{
    layer_shell::{Layer, LayerShellOptions},
    *,
};

use crate::search_bar::{
    Backspace, Copy, Cut, Delete, DeleteAll, End, Home, InputExample, Paste, Quit, SelectAll,
    TextInput,
};

mod search_bar;

fn main() {
    // connect to existing socket
    let socket_path = "/tmp/sherlock.sock";
    if let Ok(mut stream) = std::os::unix::net::UnixStream::connect(socket_path) {
        let _ = stream.write_all(b"open");
        return;
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
            KeyBinding::new("escape", Quit, None),
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
                let sub = cx.observe_keystrokes(move |this: &mut InputExample, ev, _, cx| {
                    this.recent_keystrokes.push(ev.keystroke.clone());
                    cx.notify();
                });

                InputExample {
                    text_input,
                    recent_keystrokes: vec![],
                    focus_handle: cx.focus_handle(),
                    subs: vec![sub],
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
        window_background: WindowBackgroundAppearance::Blurred,
        ..Default::default()
    }
}

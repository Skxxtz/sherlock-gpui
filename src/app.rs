use std::{
    os::unix::net::UnixStream,
    sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    },
};

use gpui::{
    App,
    AppContext,
    AsyncApp,
    Bounds,
    Entity,
    Global,
    Size,
    WeakEntity,
    WindowBackgroundAppearance,
    WindowBounds,
    WindowHandle,
    WindowKind,
    WindowOptions,
    layer_shell::{Layer, LayerShellOptions},
    point,
    px,
};
use tokio::net::UnixListener;

use crate::{
    SOCKET_PATH,
    app::{data::LauncherData, theme::ActiveTheme},
    config::{ConfigGuard, ConfigWatcher},
    features::plugin::runtime::LuaRuntimeHandle,
    loader::{LauncherLoadResult, Loader, SetupResult},
    ui::{
        backdrop::Backdrop,
        components::search_bar::TextInput,
        launcher::{LauncherMode, LauncherView},
    },
    utils::{errors::SherlockMessage, preview_lru::PreviewCache},
};

pub mod bindings;
pub mod data;
pub mod theme;
mod updates;
mod warmup;

pub static LAUNCH_GENERATION: AtomicU32 = AtomicU32::new(0);
pub fn reset_generation() {
    LAUNCH_GENERATION.fetch_add(1, Ordering::Relaxed);
}

pub type LauncherEntity = Entity<LauncherEntityInner>;
pub type LauncherWeakEntity = WeakEntity<LauncherEntityInner>;
pub type LauncherEntityInner = LauncherData;

#[derive(Clone)]
pub struct LauncherEntityGlobal(pub LauncherWeakEntity);
impl Global for LauncherEntityGlobal {}

#[derive(Clone)]
pub struct LauncherViewGlobal(pub WeakEntity<LauncherView>);
impl gpui::Global for LauncherViewGlobal {}

/// A closed launcher kept alive for `remember_query`: a strong handle, so the view (search text,
/// view stack, selection, a running AI answer…) survives its window and is reopened as is.
#[derive(Clone, Default)]
pub struct RetainedLauncher(pub Option<Entity<LauncherView>>);
impl Global for RetainedLauncher {}

impl RetainedLauncher {
    /// Takes the retained view out, if there is one.
    pub fn take(cx: &mut App) -> Option<Entity<LauncherView>> {
        cx.try_global::<Self>()?;
        cx.global_mut::<Self>().0.take()
    }

    /// Drops the retained view (e.g. when the config changed under it).
    pub fn clear(cx: &mut App) {
        if cx.try_global::<Self>().is_some() {
            cx.global_mut::<Self>().0 = None;
        }
    }
}

pub fn run_app(cx: &mut App, result: SetupResult) {
    #[cfg(feature = "ai")]
    super::fonts::register(cx);

    LuaRuntimeHandle::spawn(cx);

    let SetupResult {
        config_dir,
        mut messages,
    } = result;
    let watcher = ConfigWatcher::new(config_dir);

    bindings::register_bindings(cx);

    let theme = ActiveTheme::default();
    cx.set_global(theme);

    let data: LauncherEntity = cx.new(|_| LauncherData::default());
    let modes = load_modes(cx, &data, &mut messages);
    cx.set_global(LauncherEntityGlobal(data.downgrade()));

    let listener = UnixListener::bind(SOCKET_PATH).unwrap();
    let initial_messages = messages;

    warmup::warmup(cx);

    cx.spawn(|cx: &mut AsyncApp| {
        let cx = cx.clone();
        async move {
            updates::run_event_loop(cx, data, modes, watcher, listener, initial_messages).await;
        }
    })
    .detach();
}

fn load_modes(
    cx: &mut App,
    data: &LauncherEntity,
    messages: &mut Vec<SherlockMessage>,
) -> Arc<[LauncherMode]> {
    match Loader::load_launchers(cx, data.clone(), None) {
        Ok(LauncherLoadResult {
            modes,
            messages: msgs,
        }) => {
            messages.extend(msgs);
            modes
        }
        Err(e) => {
            messages.push(e);
            Arc::from([])
        }
    }
}

#[inline(always)]
pub async fn run_async_updates(cx: &mut AsyncApp, win: WindowHandle<LauncherView>) {
    let _ = win.update(cx, |this, _win, cx| {
        this.update_async(cx);
    });
}

fn spawn_launcher(
    cx: &mut App,
    data: LauncherEntity,
    modes: Arc<[LauncherMode]>,
    initial_messages: Vec<SherlockMessage>,
    response_socket: Option<Arc<UnixStream>>,
) -> WindowHandle<LauncherView> {
    let backdrop = spawn_backdrop(cx);

    cx.open_window(get_window_options(), move |win, cx| {
        let placeholder = ConfigGuard::get(|c| c.appearance.placeholder.shared());
        let text_input = cx.new(|cx| TextInput::builder().placeholder(placeholder).build(cx));

        attach_window_hooks(win, cx);

        cx.new(|cx| {
            LauncherView::new(
                text_input,
                backdrop,
                data,
                modes,
                initial_messages,
                response_socket,
                cx,
            )
        })
    })
    .unwrap()
}

/// Opens a new window for a retained launcher view (`RetainedLauncher`), with its state as it
/// was. The view's subscriptions belong to the entity, not the window, so only the window's own
/// hooks and the backdrop are set up again.
pub fn reopen_launcher(
    cx: &mut App,
    view: Entity<LauncherView>,
) -> Option<WindowHandle<LauncherView>> {
    let backdrop = spawn_backdrop(cx);
    view.update(cx, |v, cx| {
        v.text_input.update(cx, |this, cx| this.select_all(cx));
        v.backdrop = backdrop;
        v.navigation.evict_unretained(cx);
    });
    cx.open_window(get_window_options(), move |win, cx| {
        attach_window_hooks(win, cx);
        view
    })
    .ok()
}

/// The backdrop window, if enabled.
fn spawn_backdrop(cx: &mut App) -> Option<WindowHandle<Backdrop>> {
    ConfigGuard::read()
        .ok()
        .filter(|c| c.backdrop.enable)
        .and_then(|c| {
            cx.open_window(Backdrop::window_options(), |_, cx| {
                cx.new(|_| Backdrop::new(c.backdrop.opacity.get(), c.backdrop.animation_duration))
            })
            .ok()
        })
}

/// Hooks that belong to a launcher window rather than its view.
fn attach_window_hooks(win: &mut gpui::Window, cx: &mut App) {
    // cleanup hook for the preview cache
    win.on_window_should_close(cx, |window, cx| {
        cx.default_global::<PreviewCache>().clear(window);
        true
    });
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

use gpui::{App, AsyncApp};
// runtime.rs
use mlua::prelude::*;
use std::{
    cell::RefCell,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, OnceLock},
};
use tokio::sync::{mpsc, oneshot};

use crate::{app::theme::ThemeData, launcher::plugin_launcher::api::protocol::PluginDeferFunction};

use super::{
    capabilities::PluginCapability,
    job_handler::handle_job,
    registry::PluginRegistry,
    subscribers::{TileSubscribers, TileSubscribersGlobal},
    ui_schema::{PluginNodeRegistration, PluginUiNode},
};

#[derive(Clone, Debug, Default)]
pub struct PluginHandle {
    pub id: PathBuf,
    pub name: String,
}

#[allow(unused)]
pub enum LuaJob {
    LoadPlugin {
        capabilities: PluginCapability,
        path: Arc<Path>,
        reply: oneshot::Sender<LuaResult<PluginHandle>>,
    },
    CallTiles {
        handle: Arc<PluginHandle>,
        reply: oneshot::Sender<LuaResult<Vec<PluginNodeRegistration>>>,
    },
    CallRefresh {
        handle: Arc<PluginHandle>,
        tile_id: String,
        reply: oneshot::Sender<LuaResult<PluginUiNode>>,
    },
    CallInit {
        handle: Arc<PluginHandle>,
        theme: Arc<ThemeData>,
        reply: oneshot::Sender<LuaResult<()>>,
    },
    SpawnLive {
        handle: Arc<PluginHandle>,
        tile_id: String,
    },
    HasFn {
        handle: Arc<PluginHandle>,
        func_name: String,
        reply: oneshot::Sender<bool>,
    },
    Unload {
        handle: Arc<PluginHandle>,
    },
}

pub static LUA_RUNTIME: OnceLock<LuaRuntimeHandle> = OnceLock::new();

/// Creates a mock instance of the LuaRuntimeHandle.
/// The receiver is dropped, so sends will succeed but jobs go nowhere.
#[cfg(test)]
pub fn init_mock_runtime(){
    let (tx, _rx) = mpsc::unbounded_channel();
    let _ = LUA_RUNTIME.set(LuaRuntimeHandle { tx });
}

#[derive(Clone)]
pub struct LuaRuntimeHandle {
    tx: mpsc::UnboundedSender<LuaJob>,
}

impl LuaRuntimeHandle {
    pub fn get() -> &'static Self {
        match LUA_RUNTIME.get() {
            Some(rt) => rt,
            None => panic!("LuaRuntimeHandle::get called before it was initialized."),
        }
    }
    async fn send_and_recv<T>(
        &self,
        job: LuaJob,
        rx: oneshot::Receiver<LuaResult<T>>,
    ) -> LuaResult<T> {
        self.tx
            .send(job)
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }
    /// Spawns the dedicated OS thread that owns the Lua VM. Call once at
    /// startup. Returns both the handle for sending jobs, and the receiving
    /// end of the tile-update stream — the caller (GPUI side, which has
    /// `cx`) is responsible for draining that into entity updates.
    pub fn spawn(cx: &mut App) {
        if LUA_RUNTIME.get().is_some() {
            panic!("LuaRuntimeHandle::spawn called more than once!");
        }

        let (tx, rx) = mpsc::unbounded_channel::<LuaJob>();
        let (update_tx, mut update_rx) = mpsc::unbounded_channel::<PluginDeferFunction>();

        std::thread::Builder::new()
            .name("lua-runtime".into())
            .spawn(move || {
                run_lua_thread(rx, update_tx);
            })
            .expect("failed to spawn lua-runtime thread");

        // Handle messages from the lua-runtime thread (via channels)
        let subscribers = TileSubscribers::default();
        {
            let subscribers = subscribers.clone();
            cx.spawn(async move |cx: &mut AsyncApp| {
                while let Some(defer_fn) = update_rx.recv().await {
                    match defer_fn {
                        PluginDeferFunction::WriteClipboard(content) => {
                            let _ = cx.update(|cx| cx.write_to_clipboard(content.into()));
                        }
                        PluginDeferFunction::Update { tile_id, node } => {
                            if let Some(weak) = subscribers.get(&tile_id)
                                && let Some(entity) = weak.upgrade()
                            {
                                cx.update(|cx| {
                                    entity.update(cx, |state, cx| state.set_data(node, cx));
                                });
                            }
                        }
                    }
                }
            })
            .detach();
        }
        cx.set_global(TileSubscribersGlobal(subscribers));

        let _ = LUA_RUNTIME.set(LuaRuntimeHandle { tx });
    }

    pub async fn load_plugin(
        &self,
        path: Arc<Path>,
        capabilities: PluginCapability,
    ) -> LuaResult<PluginHandle> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(LuaJob::LoadPlugin {
                path,
                reply,
                capabilities,
            })
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }

    pub async fn call_tiles(
        &self,
        handle: Arc<PluginHandle>,
    ) -> LuaResult<Vec<PluginNodeRegistration>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(LuaJob::CallTiles { handle, reply })
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }

    pub async fn call_refresh(
        &self,
        handle: Arc<PluginHandle>,
        tile_id: String,
    ) -> LuaResult<PluginUiNode> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(LuaJob::CallRefresh {
                handle,
                tile_id,
                reply,
            })
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }

    pub async fn call_init(
        &self,
        handle: Arc<PluginHandle>,
        theme: Arc<ThemeData>,
    ) -> LuaResult<()> {
        let (reply, rx) = oneshot::channel();
        self.send_and_recv(
            LuaJob::CallInit {
                handle,
                theme,
                reply,
            },
            rx,
        )
        .await
    }

    /// Fire-and-forget: starts the plugin's `live(tile_id)` loop. Does not
    /// wait for it to finish — it may run forever.
    pub fn spawn_live(&self, handle: Arc<PluginHandle>, tile_id: String) {
        let _ = self.tx.send(LuaJob::SpawnLive { handle, tile_id });
    }

    #[allow(unused)]
    pub fn unload(&self, handle: Arc<PluginHandle>) {
        let _ = self.tx.send(LuaJob::Unload { handle });
    }

    pub async fn has_fn(&self, handle: Arc<PluginHandle>, func_name: &str) -> bool {
        let (reply, rx) = oneshot::channel();
        let _ = self.tx.send(LuaJob::HasFn {
            handle,
            func_name: func_name.to_string(),
            reply,
        });
        rx.await.unwrap_or(false)
    }
}

fn run_lua_thread(
    mut rx: mpsc::UnboundedReceiver<LuaJob>,
    update_tx: mpsc::UnboundedSender<PluginDeferFunction>,
) {
    let local = tokio::task::LocalSet::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build lua-thread tokio runtime");

    let registry = Rc::new(RefCell::new(PluginRegistry::default()));

    let lua = Lua::new_with(
        LuaStdLib::TABLE
            | LuaStdLib::STRING
            | LuaStdLib::MATH
            | LuaStdLib::COROUTINE
            | LuaStdLib::PACKAGE,
        LuaOptions::default(),
    )
    .expect("failed to init Lua runtime");

    // update_tx is captured here, at registration time, by the
    // sherlock.update closure — this is the only place it needs to exist.
    super::api::setup_global_api(&lua, update_tx).expect("failed to setup Lua API");

    rt.block_on(local.run_until(async move {
        while let Some(job) = rx.recv().await {
            let lua = lua.clone();
            let registry = Rc::clone(&registry);
            tokio::task::spawn_local(async move {
                handle_job(lua, registry, job).await;
            });
        }
    }));
}

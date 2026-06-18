// runtime.rs
use mlua::prelude::*;
use std::{cell::RefCell, sync::Arc};
use tokio::sync::{mpsc, oneshot};

use crate::launcher::plugin_launcher::{job_handler::handle_job, registry::PluginRegistry};

#[derive(Clone)]
pub struct LuaRuntimeGlobal(pub LuaRuntimeHandle);
impl gpui::Global for LuaRuntimeGlobal {}

#[derive(Clone)]
pub struct PluginHandle {
    pub id: u64,
    pub name: String,
}

pub enum LuaJob {
    LoadPlugin {
        code: String,
        name: String,
        reply: oneshot::Sender<LuaResult<PluginHandle>>,
    },
    CallTiles {
        handle: PluginHandle,
        reply: oneshot::Sender<LuaResult<Vec<TileDescriptor>>>,
    },
    CallRefresh {
        handle: PluginHandle,
        tile_id: String,
        reply: oneshot::Sender<LuaResult<TileDescriptor>>,
    },
    SpawnLive {
        handle: PluginHandle,
        tile_id: String,
    },
    HasFn {
        handle: PluginHandle,
        func_name: String,
        reply: oneshot::Sender<bool>,
    },
    Unload {
        handle: PluginHandle,
    },
}

#[derive(Clone, Debug)]
pub struct TileDescriptor {
    pub id: String,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
}

impl FromLua for TileDescriptor {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        let table = match value {
            LuaValue::Table(t) => t,
            other => {
                return Err(LuaError::FromLuaConversionError {
                    from: other.type_name(),
                    to: "TileDescriptor".to_string(),
                    message: Some("expected a table".into()),
                });
            }
        };
        Ok(TileDescriptor {
            id: table.get("id")?,
            title: table.get("title")?,
            subtitle: table.get("subtitle").ok(),
            icon: table.get("icon").ok(),
        })
    }
}

/// Emitted by `sherlock.update(tile_id, data)` calls from any plugin's
/// `live()` loop. Consumed by a GPUI-side task that has real `cx` access.
pub type TileUpdate = (String, TileDescriptor);

#[derive(Clone)]
pub struct LuaRuntimeHandle {
    tx: mpsc::UnboundedSender<LuaJob>,
}

impl LuaRuntimeHandle {
    /// Spawns the dedicated OS thread that owns the Lua VM. Call once at
    /// startup. Returns both the handle for sending jobs, and the receiving
    /// end of the tile-update stream — the caller (GPUI side, which has
    /// `cx`) is responsible for draining that into entity updates.
    pub fn spawn() -> (Self, mpsc::UnboundedReceiver<TileUpdate>) {
        let (tx, rx) = mpsc::unbounded_channel::<LuaJob>();
        let (update_tx, update_rx) = mpsc::unbounded_channel::<TileUpdate>();

        std::thread::Builder::new()
            .name("lua-runtime".into())
            .spawn(move || {
                run_lua_thread(rx, update_tx);
            })
            .expect("failed to spawn lua-runtime thread");

        (Self { tx }, update_rx)
    }

    pub async fn load_plugin(&self, code: String, name: String) -> LuaResult<PluginHandle> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(LuaJob::LoadPlugin { code, name, reply })
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }

    pub async fn call_tiles(&self, handle: PluginHandle) -> LuaResult<Vec<TileDescriptor>> {
        let (reply, rx) = oneshot::channel();
        self.tx
            .send(LuaJob::CallTiles { handle, reply })
            .map_err(|_| LuaError::RuntimeError("lua runtime thread is gone".into()))?;
        rx.await
            .map_err(|_| LuaError::RuntimeError("lua runtime dropped reply".into()))?
    }

    pub async fn call_refresh(
        &self,
        handle: PluginHandle,
        tile_id: String,
    ) -> LuaResult<TileDescriptor> {
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

    /// Fire-and-forget: starts the plugin's `live(tile_id)` loop. Does not
    /// wait for it to finish — it may run forever.
    pub fn spawn_live(&self, handle: PluginHandle, tile_id: String) {
        let _ = self.tx.send(LuaJob::SpawnLive { handle, tile_id });
    }

    pub fn unload(&self, handle: PluginHandle) {
        let _ = self.tx.send(LuaJob::Unload { handle });
    }

    pub async fn has_fn(&self, handle: PluginHandle, func_name: &str) -> bool {
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
    update_tx: mpsc::UnboundedSender<TileUpdate>,
) {
    let local = tokio::task::LocalSet::new();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build lua-thread tokio runtime");

    let registry = Arc::new(RefCell::new(PluginRegistry::default()));

    let lua = Lua::new_with(
        LuaStdLib::TABLE | LuaStdLib::STRING | LuaStdLib::MATH | LuaStdLib::COROUTINE,
        LuaOptions::default(),
    )
    .expect("failed to init Lua runtime");

    // update_tx is captured here, at registration time, by the
    // sherlock.update closure — this is the only place it needs to exist.
    super::api::setup_global_api(&lua, update_tx).expect("failed to setup Lua API");

    rt.block_on(local.run_until(async move {
        while let Some(job) = rx.recv().await {
            let lua = lua.clone();
            let registry = Arc::clone(&registry);
            tokio::task::spawn_local(async move {
                handle_job(lua, registry, job).await;
            });
        }
    }));
}

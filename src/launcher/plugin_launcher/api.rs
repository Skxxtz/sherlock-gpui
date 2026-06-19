use crate::launcher::plugin_launcher::{
    api::{
        clipboard::ClipboardModule, http::HttpModule, json::JsonModule, log::LogModule,
        time::TimeModule, ui::UiModule,
    },
    runtime::TileUpdate,
};
use mlua::prelude::*;
use std::{fmt::Write, sync::OnceLock};
use tokio::sync::mpsc;

mod clipboard;
mod http;
mod json;
mod log;
mod time;
mod ui;

static UI_UPDATE_CHANNEL: OnceLock<mpsc::UnboundedSender<TileUpdate>> = OnceLock::new();

/// Shared context passed to every module's `register` call.
/// Add fields here as new shared resources show up (e.g. an HTTP client,
/// a config handle, a cancellation token) instead of widening function signatures.
pub struct ApiContext {
    pub update_tx: &'static mpsc::UnboundedSender<TileUpdate>,
}

pub struct FnEntry {
    pub name: &'static str,
    pub register: fn(&Lua, &LuaTable, &ApiContext) -> LuaResult<()>,
    pub docs: fn() -> LuaApiDoc,
}

#[macro_export]
macro_rules! fn_list {
    ($($fn:ty),*) => {
        &[$($crate::launcher::plugin_launcher::api::FnEntry {
            name: <$fn as SherlockPluginFn>::NAME,
            register: <$fn as SherlockPluginFn>::register,
            docs: <$fn as SherlockPluginFn>::docs,
        }),*]
    }
}

/// One Lua API domain. Each module owns its own table and its own functions.
trait PluginModuleDeclaration {
    const NAME: &'static str;
    const FUNCTIONS: &'static [FnEntry];
    const RESTRICTED: &'static [FnEntry];
}

/// Every Lua-exposed function is a zero-sized type implementing this trait.
/// The trait carries both the doc metadata (compile-time, always available)
/// and the actual registration logic (only runs when `register` is called).
pub trait SherlockPluginFn {
    const NAME: &'static str;
    const PARAMS: &'static [(&'static str, &'static str)];
    const RETURNS: &'static str;
    const DOC: &'static str;

    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()>;
    fn docs() -> LuaApiDoc {
        LuaApiDoc {
            name: Self::NAME,
            params: Self::PARAMS,
            returns: Self::RETURNS,
            doc: Self::DOC,
        }
    }
}

#[inline(always)]
fn register<M: PluginModuleDeclaration>(
    lua: &Lua,
    table: &LuaTable,
    ctx: &ApiContext,
) -> LuaResult<()> {
    for func in M::FUNCTIONS {
        (func.register)(lua, table, ctx)?;
    }
    Ok(())
}

#[inline(always)]
fn register_restricted<M: PluginModuleDeclaration>(
    lua: &Lua,
    table: &LuaTable,
    ctx: &ApiContext,
) -> LuaResult<()> {
    for func in M::RESTRICTED {
        (func.register)(lua, table, ctx)?;
    }
    Ok(())
}

fn lua_err(e: impl std::fmt::Display) -> LuaError {
    LuaError::RuntimeError(e.to_string())
}

pub struct LuaApiDoc {
    pub name: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub returns: &'static str,
    pub doc: &'static str,
}

/// Defines a Lua-callable function and simultaneously records its signature
/// for LSP stub generation. Expands to a `lua.create_function` registration
/// plus an entry in the global `LUA_API_DOCS` registry (used by `gen-lua-defs`).
#[macro_export]
macro_rules! lua_fn {
    // sync
    (
        $table:expr, $lua:expr,
        |$lua_arg:ident, ($($arg:ident : $argty:ty),*)| $body:expr
    ) => {
        $table.set(
            Self::NAME,
            $lua.create_function(move |$lua_arg, ($($arg,)*): ($($argty,)*)| $body)?,
        )
    };

    // async — distinguished by leading `@async` marker, not a bare keyword
    (
        @async
        $table:expr, $lua:expr,
        |$lua_arg:ident, ($($arg:ident : $argty:ty),*)| $body:expr
    ) => {
        $table.set(
            Self::NAME,
            $lua.create_async_function(move |$lua_arg, ($($arg,)*): ($($argty,)*)| $body)?,
        )
    };
}

macro_rules! generate_modules {
    ( $( $variant:ident ),* $(,)? ) => {
        #[allow(dead_code)]
        fn _assert_plugin_module_impls() {
            fn assert<T: PluginModuleDeclaration>() {}
            $( assert::<$variant>(); )*
        }

        pub fn setup_global_api(
            lua: &Lua,
            update_tx: mpsc::UnboundedSender<TileUpdate>,
        ) -> LuaResult<()> {
            if let Err(_) = UI_UPDATE_CHANNEL.set(update_tx) {
                panic!("Tried to set lua globals more than once.")
            }
            Ok(())
        }

        pub fn init_local_api(lua: &Lua, local_env: &LuaTable) -> LuaResult<()> {
            let Some(update_tx) = UI_UPDATE_CHANNEL.get() else {
                panic!("Tried to initialize local lua env before globals have been set.");
            };
            let ctx = ApiContext { update_tx };
            let sherlock = lua.create_table()?;
            $(
                let t = lua.create_table()?;
                register::<$variant>(lua, &t, &ctx)?;       // unrestricted
                register_restricted::<$variant>(lua, &t, &ctx)?; // restricted
                sherlock.set($variant::NAME, t)?;
            )*
            local_env.set("sherlock", sherlock)?;
            Ok(())
        }

        pub struct LuaApiDocumentation;
        impl LuaApiDocumentation {
            pub fn gather_docs() -> Vec<(&'static str, Vec<LuaApiDoc>)> {
                vec![
                    $(
                        (
                            $variant::NAME,
                            $variant::FUNCTIONS
                                .iter()
                                .chain($variant::RESTRICTED.iter())
                                .map(|f| (f.docs)())
                                .collect::<Vec<_>>(),
                        )
                    ),*
                ]
            }

            pub fn generate_lua_stub() -> String {
                let by_module = Self::gather_docs();

                let mut out = String::from("---@meta sherlock\n\n");
                writeln!(out, "---@class sherlock").unwrap();
                writeln!(out, "sherlock = {{}}").unwrap();

                for (namespace, fns) in by_module {
                    writeln!(out, "---@class sherlock.{namespace}").unwrap();
                    writeln!(out, "sherlock.{namespace} = {{}}\n").unwrap();

                    for doc in fns {
                        writeln!(out, "--- {}", doc.doc).unwrap();
                        for (pname, ptype) in doc.params {
                            writeln!(out, "---@param {pname} {ptype}").unwrap();
                        }
                        writeln!(out, "---@return {}", doc.returns).unwrap();
                        writeln!(
                            out,
                            "function sherlock.{}.{}({}) end\n",
                            namespace,
                            doc.name,
                            doc.params.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", ")
                        ).unwrap();
                    }
                }
                out
            }
        }
    };
}

generate_modules! {
    ClipboardModule,
    HttpModule,
    JsonModule,
    LogModule,
    TimeModule,
    UiModule,
}

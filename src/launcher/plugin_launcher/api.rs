use crate::launcher::plugin_launcher::{
    api::{http::HttpModule, json::JsonModule, log::LogModule, time::TimeModule, ui::UiModule},
    runtime::TileUpdate,
};
use mlua::prelude::*;
use std::fmt::Write;
use tokio::sync::mpsc;

mod http;
mod json;
mod log;
mod time;
mod ui;

/// Shared context passed to every module's `register` call.
/// Add fields here as new shared resources show up (e.g. an HTTP client,
/// a config handle, a cancellation token) instead of widening function signatures.
pub struct ApiContext {
    pub update_tx: mpsc::UnboundedSender<TileUpdate>,
}

/// One Lua API domain. Each module owns its own table and its own functions.
trait SherlockPluginModule {
    /// The name the module is exposed under, e.g. `sherlock.http`
    const NAME: &'static str;
    fn register(lua: &Lua, table: &LuaTable, ctx: &ApiContext) -> LuaResult<()>;
    fn docs() -> Vec<LuaApiDoc>;
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
fn register<M: SherlockPluginModule>(
    lua: &Lua,
    sherlock: &LuaTable,
    ctx: &ApiContext,
) -> LuaResult<()> {
    let table = lua.create_table()?;
    M::register(lua, &table, ctx)?;
    sherlock.set(M::NAME, table)?;
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
            fn assert<T: SherlockPluginModule>() {}
            $( assert::<$variant>(); )*
        }

        pub fn setup_global_api(
            lua: &Lua,
            update_tx: mpsc::UnboundedSender<TileUpdate>,
        ) -> LuaResult<()> {
            let ctx = ApiContext { update_tx };
            let sherlock = lua.create_table()?;
            $(
                register::<$variant>(lua, &sherlock, &ctx)?;
            )*
            lua.globals().set("sherlock", sherlock)?;
            Ok(())
        }


        pub struct LuaApiDocumentation;
        impl LuaApiDocumentation {
            pub fn gather_docs() -> Vec<(&'static str, Vec<LuaApiDoc>)> {
                vec![
                    $(
                        ($variant::NAME, $variant::docs()),
                    )*
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
    HttpModule,
    JsonModule,
    LogModule,
    TimeModule,
    UiModule,
}

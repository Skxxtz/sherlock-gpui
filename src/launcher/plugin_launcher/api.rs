use super::capabilities::{HasCapabilityBit, PluginCapability};
use crate::{
    docs::launcher::plugin_launcher::{PluginCapabilityFunctionDoc, PluginCapabilityModuleDoc},
    launcher::plugin_launcher::api::protocol::PluginDeferFunction,
};
use mlua::prelude::*;
use std::{fmt::Write, sync::OnceLock};
use tokio::sync::mpsc;

pub mod clipboard;
pub mod http;
pub mod json;
pub mod log;
pub mod protocol;
pub mod time;
pub mod ui;

static UI_UPDATE_CHANNEL: OnceLock<mpsc::UnboundedSender<PluginDeferFunction>> = OnceLock::new();

pub struct ApiContext {
    pub update_tx: &'static mpsc::UnboundedSender<PluginDeferFunction>,
}

pub trait SherlockPluginFn: HasCapabilityBit {
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

pub fn lua_err(e: impl std::fmt::Display) -> LuaError {
    LuaError::RuntimeError(e.to_string())
}

pub struct LuaApiDoc {
    pub name: &'static str,
    pub params: &'static [(&'static str, &'static str)],
    pub returns: &'static str,
    pub doc: &'static str,
}

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
    // async
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

/// generate_modules!
///
/// Syntax:
///   generate_modules! {
///       ModuleType("name") => [path::FnA, path::FnB],
///       ...
///   }
///
/// Emits:
///   - Per function:  impl HasCapabilityBit (sequential bits, declaration order)
///   - Per module:    pub struct + NAME + register(caps) + docs()
///   - Global:        setup_global_api, init_local_api(lua, env, caps), capabilities_from_names,
///     LuaApiDocumentation
macro_rules! generate_modules {
    (
        $( $module:ident ( $name:literal ) => [ $( $fn_path:path ),* $(,)? ] ),* $(,)?
    ) => {
        generate_modules!(@bits 0u64; $( $( $fn_path, )* )* );
        generate_modules!(@build $( $module($name) => [ $($fn_path),* ] ),* );
    };

    (@bits $n:expr; ) => {};
    (@bits $n:expr; $head:path, $($tail:path,)*) => {
        impl $crate::launcher::plugin_launcher::api::HasCapabilityBit for $head {
            const CAPABILITY: $crate::launcher::plugin_launcher::api::PluginCapability =
                $crate::launcher::plugin_launcher::api::PluginCapability::from_bit($n);
        }
        generate_modules!(@bits $n + 1u64; $($tail,)*);
    };

    (@build $( $module:ident($name:literal) => [ $($fn_path:path),* ] ),* ) => {

        $(
            pub struct $module;

            impl $module {
                pub const NAME: &'static str = $name;

                pub fn register(
                    lua: &mlua::Lua,
                    table: &mlua::Table,
                    ctx: &$crate::launcher::plugin_launcher::api::ApiContext,
                    caps: $crate::launcher::plugin_launcher::api::PluginCapability,
                ) -> mlua::Result<()> {
                    use $crate::launcher::plugin_launcher::api::{HasCapabilityBit, SherlockPluginFn};
                    $(
                        if caps.allows(<$fn_path as HasCapabilityBit>::CAPABILITY) {
                            <$fn_path as SherlockPluginFn>::register(lua, table, ctx)?;
                        }
                    )*
                    Ok(())
                }

                pub fn docs() -> Vec<$crate::launcher::plugin_launcher::api::LuaApiDoc> {
                    use $crate::launcher::plugin_launcher::api::SherlockPluginFn;
                    vec![ $( <$fn_path>::docs(), )* ]
                }
            }
        )*

        pub fn setup_global_api(
            lua: &mlua::Lua,
            update_tx: tokio::sync::mpsc::UnboundedSender<
                $crate::launcher::plugin_launcher::api::protocol::PluginDeferFunction,
            >,
        ) -> mlua::Result<()> {
            let _ = lua;
            if UI_UPDATE_CHANNEL.set(update_tx).is_err() {
                panic!("Tried to set lua globals more than once.");
            }
            Ok(())
        }

        pub fn init_local_api(
            lua: &mlua::Lua,
            local_env: &mlua::Table,
            caps: $crate::launcher::plugin_launcher::api::PluginCapability,
        ) -> mlua::Result<()> {
            let Some(update_tx) = UI_UPDATE_CHANNEL.get() else {
                panic!("Tried to initialize local lua env before globals have been set.");
            };
            let ctx = $crate::launcher::plugin_launcher::api::ApiContext { update_tx };
            let sherlock = lua.create_table()?;
            $(
                let t = lua.create_table()?;
                $module::register(lua, &t, &ctx, caps)?;
                sherlock.set($module::NAME, t)?;
            )*
            local_env.set("sherlock", sherlock)?;
            Ok(())
        }

        pub fn capabilities_from_names<'a>(
            names: impl IntoIterator<Item = &'a str>,
        ) -> PluginCapability {
            names.into_iter().fold(PluginCapability::NONE, |acc, name| {
                acc | match name.split_once('.') {
                    // "http.get" — single function
                    Some((module, func)) => {
                        match module {
                            $(
                                $name => match func {
                                    $(
                                        _ if func == <$fn_path as SherlockPluginFn>::NAME =>
                                        <$fn_path as HasCapabilityBit>::CAPABILITY,
                                    )*
                                        _ => PluginCapability::NONE,
                                },
                            )*
                                _ => PluginCapability::NONE,
                        }
                    }
                    // "http" — whole module
                    None => {
                        match name {
                            $(
                                $name => {
                                    let mut cap = PluginCapability::NONE;
                                    $( cap |= <$fn_path as HasCapabilityBit>::CAPABILITY; )*
                                        cap
                                    }
                            )*
                                _ => PluginCapability::NONE,
                        }
                    }
                }
            })
        }
        pub fn plugin_capability_docs() -> &'static [PluginCapabilityModuleDoc] {
            &[
                $(
                    PluginCapabilityModuleDoc {
                        module: $name,
                        functions: &[
                            $(
                                PluginCapabilityFunctionDoc {
                                    name: <$fn_path as SherlockPluginFn>::NAME,
                                    doc: <$fn_path as SherlockPluginFn>::DOC,
                                },
                            )*
                        ],
                    },
                )*
            ]
        }

        pub struct LuaApiDocumentation;

        impl LuaApiDocumentation {
            pub fn gather_docs() -> Vec<(&'static str, Vec<$crate::launcher::plugin_launcher::api::LuaApiDoc>)> {
                vec![ $( ($module::NAME, $module::docs()), )* ]
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
                            doc.params.iter().map(|(n, _)| *n).collect::<Vec<_>>().join(", "),
                        ).unwrap();
                    }
                }
                out
            }
        }
    };
}

generate_modules! {
    ClipboardModule("clipboard") => [clipboard::Get, clipboard::Set],
    HttpModule("http")           => [http::Get, http::Post],
    JsonModule("json")           => [json::Decode],
    LogModule("log")             => [log::Info, log::Error],
    TimeModule("time")           => [time::Sleep],
    UiModule("ui")               => [ui::Update],
}

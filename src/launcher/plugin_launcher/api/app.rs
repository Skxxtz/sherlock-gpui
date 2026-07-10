use crate::launcher::plugin_launcher::api::{ApiContext, SherlockPluginFn};
use crate::lua_fn;
use mlua::prelude::{Lua, LuaResult, LuaTable};

pub struct Version;
impl SherlockPluginFn for Version {
    const NAME: &'static str = "version";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Gives the current Sherlock version.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            env!("CARGO_PKG_VERSION")
        ))
    }
}

pub struct VersionMajor;
impl SherlockPluginFn for VersionMajor {
    const NAME: &'static str = "version_major";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Gives the major version component of Sherlock.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            env!("CARGO_PKG_VERSION_MAJOR")
        ))
    }
}

pub struct VersionMinor;
impl SherlockPluginFn for VersionMinor {
    const NAME: &'static str = "version_minor";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Gives the minor version component of Sherlock.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            env!("CARGO_PKG_VERSION_MINOR")
        ))
    }
}

pub struct VersionPatch;
impl SherlockPluginFn for VersionPatch {
    const NAME: &'static str = "version_patch";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Gives the patch version component of Sherlock.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            env!("CARGO_PKG_VERSION_PATCH")
        ))
    }
}

pub struct AppName;
impl SherlockPluginFn for AppName {
    const NAME: &'static str = "app_name";
    const PARAMS: &'static [(&'static str, &'static str)] = &[];
    const RETURNS: &'static str = "string";
    const DOC: &'static str = "Gives the crate/application name.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, ()| Ok(
            env!("CARGO_PKG_NAME")
        ))
    }
}

pub struct HasFeature;
impl SherlockPluginFn for HasFeature {
    const NAME: &'static str = "has_feature";
    const PARAMS: &'static [(&'static str, &'static str)] = &[("feature", "string")];
    const RETURNS: &'static str = "boolean";
    const DOC: &'static str = "Checks if Sherlock was compiled with the given feature enabled.";
    fn register(lua: &Lua, table: &LuaTable, _ctx: &ApiContext) -> LuaResult<()> {
        lua_fn!(table, lua, |_lua, (feature: String)| {
            Ok(match feature.as_str() {
                "wayland" => cfg!(feature = "wayland"),
                "nixos" => cfg!(feature = "nixos"),
                // add other feature names here
                _ => false,
            })
        })
    }
}

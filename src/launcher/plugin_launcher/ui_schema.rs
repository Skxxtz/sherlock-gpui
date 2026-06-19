use mlua::prelude::*;
use serde::Deserialize;

use crate::launcher::plugin_launcher::ui::style::PluginStyle;

#[derive(Clone, Debug, Deserialize)]
pub struct PluginNodeRegistration {
    pub id: String,
    pub node: PluginUiNode,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginUiNode {
    Container {
        #[serde(default)]
        style: PluginStyle,
        #[serde(default)]
        children: Vec<PluginUiNode>,
    },
    Text {
        content: String,
        #[serde(default)]
        style: PluginStyle,
    },
    Icon {
        name: String,
        #[serde(default)]
        style: PluginStyle,
    },
    Button {
        label: String,
        #[serde(default)]
        style: PluginStyle,
        #[serde(default)]
        on_click: Option<String>, // callback id, looked up in plugin env
    },
}

impl FromLua for PluginNodeRegistration {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let json: serde_json::Value = lua.from_value(value)?;
        serde_json::from_value(json)
            .map_err(|e| LuaError::RuntimeError(format!("invalid ui tile: {e}")))
    }
}

impl FromLua for PluginUiNode {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let json: serde_json::Value = lua.from_value(value)?;
        serde_json::from_value(json)
            .map_err(|e| LuaError::RuntimeError(format!("invalid ui node: {e}")))
    }
}

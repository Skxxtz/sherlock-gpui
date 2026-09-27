use mlua::prelude::*;
use serde::Deserialize;

use crate::launcher::plugin_launcher::ui::style::PluginStyle;

#[derive(Clone, Debug, Deserialize)]
pub struct PluginNodeRegistration {
    pub id: String,
    pub node: PluginTileContent,
}

/// A context-menu entry defined by the plugin.
#[derive(Clone, Debug, Deserialize)]
pub struct PluginTileAction {
    pub name: String,
    #[serde(default)]
    pub icon: Option<String>,
    /// Close the launcher after running.
    #[serde(default)]
    pub exit: bool,
    /// Callback index (see `sherlock._prepare`).
    pub run: u32,
}

/// Tile-level behaviour, read from the root node's table.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PluginTileMeta {
    /// Callback index run when the tile is activated (Enter).
    pub on_activate: Option<u32>,
    pub actions: Vec<PluginTileAction>,
}

/// What a tile displays: its root node plus tile-level behaviour.
#[derive(Clone, Debug)]
pub struct PluginTileContent {
    pub node: PluginUiNode,
    pub meta: PluginTileMeta,
}

impl<'de> Deserialize<'de> for PluginTileContent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        use serde::de::Error;
        let value = serde_json::Value::deserialize(d)?;
        Ok(Self {
            meta: serde_json::from_value(value.clone()).map_err(D::Error::custom)?,
            node: serde_json::from_value(value).map_err(D::Error::custom)?,
        })
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PluginUiNode {
    Container {
        #[serde(default)]
        on_click: Option<u32>,
        #[serde(default)]
        style: PluginStyle,
        #[serde(default)]
        children: Vec<PluginUiNode>,
    },
    Text {
        #[serde(default)]
        on_click: Option<u32>,
        content: String,
        #[serde(default)]
        style: PluginStyle,
    },
    Icon {
        #[serde(default)]
        on_click: Option<u32>,
        name: String,
        #[serde(default)]
        style: PluginStyle,
    },
    Button {
        #[serde(default)]
        on_click: Option<u32>,
        label: String,
        #[serde(default)]
        style: PluginStyle,
    },
}

/// Turns `sherlock.ui` builder nodes into plain tables by calling their
/// `build` method. Also descends into `node` and `children`, so builder
/// nodes can be mixed into hand-written tables.
fn normalize(value: LuaValue) -> LuaResult<LuaValue> {
    let LuaValue::Table(table) = value else {
        return Ok(value);
    };
    if let LuaValue::Function(build) = table.get::<LuaValue>("build")? {
        return build.call(table);
    }
    if let LuaValue::Table(_) = table.raw_get::<LuaValue>("node")? {
        let node = normalize(table.raw_get("node")?)?;
        table.raw_set("node", node)?;
    }
    if let LuaValue::Table(children) = table.raw_get::<LuaValue>("children")? {
        for i in 1..=children.raw_len() {
            let child = normalize(children.raw_get(i)?)?;
            children.raw_set(i, child)?;
        }
    }
    Ok(LuaValue::Table(table))
}

impl FromLua for PluginNodeRegistration {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let value = normalize(value)?;
        let json: serde_json::Value = lua.from_value(value)?;
        serde_json::from_value(json)
            .map_err(|e| LuaError::RuntimeError(format!("invalid ui tile: {e}")))
    }
}

impl FromLua for PluginTileContent {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let value = normalize(value)?;
        let json: serde_json::Value = lua.from_value(value)?;
        serde_json::from_value(json)
            .map_err(|e| LuaError::RuntimeError(format!("invalid ui node: {e}")))
    }
}

impl FromLua for PluginUiNode {
    fn from_lua(value: LuaValue, lua: &Lua) -> LuaResult<Self> {
        let value = normalize(value)?;
        let json: serde_json::Value = lua.from_value(value)?;
        serde_json::from_value(json)
            .map_err(|e| LuaError::RuntimeError(format!("invalid ui node: {e}")))
    }
}

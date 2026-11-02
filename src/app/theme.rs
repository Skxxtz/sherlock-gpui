use std::sync::Arc;

use gpui::{Hsla, SharedString, hsla};
use mlua::{IntoLua, LuaSerdeExt};
use serde::{Deserialize, Deserializer, Serialize};

mod catppuccin_mocha;
mod libre;
mod nord;

#[derive(Clone)]
pub struct ActiveTheme(pub Arc<ThemeData>);

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct ThemeData {
    #[serde(flatten)]
    pub colors: ThemeColors,
    #[serde(flatten)]
    pub typography: Typography,
    #[serde(flatten)]
    pub metrics: Metrics,
}

impl std::ops::Deref for ThemeData {
    type Target = ThemeColors;
    fn deref(&self) -> &Self::Target {
        &self.colors
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Typography {
    pub font_family: SharedString,
    pub monospace: SharedString,
    /// Base text size in px.
    pub font_size: f32,
    pub font_size_sm: f32,
    pub font_size_lg: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            font_size: 14.0,
            font_size_sm: 12.0,
            font_size_lg: 16.0,
        }
    }
}

/// Sizes in px.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Metrics {
    /// List items, cards, inputs.
    pub radius: f32,
    /// Badges, keybind chips.
    pub radius_sm: f32,
    /// The launcher window.
    pub radius_window: f32,
    pub border_width: f32,
    pub item_padding_x: f32,
    pub item_padding_y: f32,
    pub icon_size: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        Self {
            radius: 8.0,
            radius_sm: 4.0,
            radius_window: 8.0,
            border_width: 1.0,
            item_padding_x: 16.0,
            item_padding_y: 8.0,
            icon_size: 24.0,
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct ThemeColors {
    // Cursor and Selection
    #[serde(deserialize_with = "hsla_from_hex")]
    pub cursor: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub selection: Hsla,
    // Backgrounds
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_app: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_overlay: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_selected: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_idle: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_status_bar: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_keybind: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_code: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_muted: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_badge: Hsla,
    // Borders
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border_selected: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border_idle: Hsla,
    // Text
    #[serde(deserialize_with = "hsla_from_hex")]
    pub primary_text: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub secondary_text: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub tertiary_text: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_status_bar: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_mode_label: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_search_icon: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_code: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_placeholder: Hsla,
    // Status colors
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_warn: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_err: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_succ: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_info: Hsla,
    // Config banner
    #[serde(deserialize_with = "hsla_from_hex")]
    pub banner_bg: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub banner_border: Hsla,
    #[serde(deserialize_with = "hsla_from_hex")]
    pub banner_text: Hsla,
}
impl gpui::Global for ActiveTheme {}

impl Default for ActiveTheme {
    fn default() -> Self {
        Self(Arc::new(ThemeData::default()))
    }
}

impl Default for ThemeData {
    fn default() -> Self {
        Self::dark()
    }
}

impl IntoLua for &ThemeData {
    fn into_lua(self, lua: &mlua::Lua) -> mlua::Result<mlua::Value> {
        lua.to_value(self)
    }
}

impl ThemeData {
    pub fn dark() -> Self {
        Self {
            typography: Typography::default(),
            metrics: Metrics::default(),
            colors: ThemeColors {
                // Cursor and Selection
                cursor: hsla(0.0, 0.0, 0.8, 1.0),
                selection: hsla(0.639, 1.0, 0.53, 0.19),
                // Backgrounds
                bg_app: hsla(0.0, 0.0, 0.059, 1.0), // 0x0F0F0F
                bg_overlay: hsla(0.0, 0.0, 0.078, 1.0),
                bg_selected: hsla(0.0, 0.0, 1.0, 0.1),
                bg_idle: hsla(0.0, 0.0, 0.0, 0.0),
                bg_status_bar: hsla(0.0, 0.0, 0.098, 1.0),
                bg_keybind: hsla(0.0, 0.0, 0.149, 1.0), // 0x262626
                bg_code: hsla(0.0, 0.0, 0.118, 1.0),    // 0x1e1e1e
                bg_muted: hsla(0.0, 0.0, 1.0, 0.05),
                bg_badge: hsla(0.0, 0.0, 1.0, 0.08),
                // Borders
                border: hsla(0.0, 0.0, 0.1882, 1.0),
                border_selected: hsla(0.0, 0.0, 1.0, 0.2),
                border_idle: hsla(0.0, 0.0, 1.0, 0.05),
                // Text
                primary_text: hsla(0.0, 0.0, 0.95, 1.0),
                secondary_text: hsla(0.0, 0.0, 0.6, 1.0),
                tertiary_text: hsla(0.0, 0.0, 0.35, 1.0),
                text_status_bar: hsla(0.6, 0.0217, 0.3608, 1.0),
                text_mode_label: hsla(0.0, 0.0, 0.18, 1.0), // 0x2e2e2e
                text_search_icon: hsla(0.0, 0.0, 0.533, 1.0), // 0x888888
                text_code: hsla(0.558, 0.85, 0.76, 1.0),    // 0x89d4f5
                text_placeholder: hsla(0.0, 0.0, 1.0, 0.2),
                // Status
                color_warn: hsla(45.0 / 360.0, 0.85, 0.65, 1.0),
                color_err: hsla(0.0, 0.85, 0.65, 1.0),
                color_succ: hsla(145.0 / 360.0, 0.75, 0.60, 1.0),
                color_info: hsla(210.0 / 360.0, 0.85, 0.65, 1.0),
                // Config banner
                banner_bg: hsla(0.11, 0.8, 0.12, 1.0),
                banner_border: hsla(0.11, 0.9, 0.35, 1.0),
                banner_text: hsla(0.11, 1.0, 0.65, 1.0),
            },
        }
    }
}

fn hsla_from_hex<'de, D: Deserializer<'de>>(d: D) -> Result<Hsla, D::Error> {
    let hex = String::deserialize(d)?;
    let hex = hex.trim_start_matches('#');
    let n = u32::from_str_radix(&hex[..6], 16).map_err(serde::de::Error::custom)?;
    Ok(gpui::rgb(n).into())
}

impl ThemeData {
    /// The one highlight color (active numbers, modified tags, switches that are on). Themes
    /// have no own accent yet, so this is the warning color: amber in the default theme.
    pub fn accent(&self) -> Hsla {
        self.color_warn
    }

    pub fn companion(&self, c: Hsla) -> Hsla {
        const DL: f32 = 0.25;
        // lightness: away from the background if there's room, otherwise the other way
        let away = if self.bg_app.l < 0.5 { DL } else { -DL };
        let l = if (0.08..=0.92).contains(&(c.l + away)) {
            c.l + away
        } else {
            c.l - away
        };
        // hue: complement, but only where hue is visible at all
        let h = if c.s < 0.15 { c.h } else { (c.h + 0.5).fract() };
        Hsla {
            h,
            l: l.clamp(0., 1.),
            ..c
        }
    }
}

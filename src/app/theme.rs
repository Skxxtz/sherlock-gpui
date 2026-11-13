use std::sync::Arc;

use gpui::{Hsla, SharedString, hsla};
use mlua::{IntoLua, LuaSerdeExt};
use serde::{Deserialize, Deserializer, Serialize};

mod catppuccin_mocha;
mod libre;
mod nord;

#[cfg(test)]
mod tests;

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

#[cfg_attr(feature = "docs", derive(sherlock_derive::DocFields))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Typography {
    /// Main UI font.
    pub font_family: SharedString,
    /// Font for code and keybinds.
    pub monospace: SharedString,
    /// Font for headings in reading views (e.g. the ask view).
    pub serif: SharedString,
    /// Base text size in px.
    pub font_size: f32,
    /// Small text size in px (hints, secondary labels).
    pub font_size_sm: f32,
    /// Large text size in px (titles).
    pub font_size_lg: f32,
}

impl Default for Typography {
    fn default() -> Self {
        Self {
            font_family: "Inter".into(),
            monospace: "Noto Sans Mono".into(),
            serif: "Newsreader".into(),
            font_size: 14.0,
            font_size_sm: 12.0,
            font_size_lg: 16.0,
        }
    }
}

/// Sizes in px.
#[cfg_attr(feature = "docs", derive(sherlock_derive::DocFields))]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(default)]
pub struct Metrics {
    /// Corner radius of list items, cards and inputs.
    pub radius: f32,
    /// Corner radius of badges and keybind chips.
    pub radius_sm: f32,
    /// Corner radius of the launcher window.
    pub radius_window: f32,
    /// Width of item and window borders.
    pub border_width: f32,
    /// Horizontal padding inside list items.
    pub item_padding_x: f32,
    /// Vertical padding inside list items.
    pub item_padding_y: f32,
    /// Size of item icons.
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

#[cfg_attr(feature = "docs", derive(sherlock_derive::DocFields))]
#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct ThemeColors {
    // Cursor and Selection
    /// Text cursor in the search bar.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub cursor: Hsla,
    /// Background of selected text.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub selection: Hsla,
    /// Accent for highlights and active elements.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub accent: Hsla,
    // Backgrounds
    /// Main window background.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_app: Hsla,
    /// Background of overlays like the context menu.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_overlay: Hsla,
    /// Background of the selected item.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_selected: Hsla,
    /// Background of unselected items.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_idle: Hsla,
    /// Status bar background.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_status_bar: Hsla,
    /// Background of keybind chips.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_keybind: Hsla,
    /// Background of code blocks.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_code: Hsla,
    /// Subtle background for secondary surfaces.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_muted: Hsla,
    /// Background of badges.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub bg_badge: Hsla,
    // Borders
    /// Default border, e.g. below the search bar.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border: Hsla,
    /// Border of the selected item.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border_selected: Hsla,
    /// Border of unselected items.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub border_idle: Hsla,
    // Text
    /// Main text.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub primary_text: Hsla,
    /// Less prominent text, e.g. descriptions.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub secondary_text: Hsla,
    /// Least prominent text, e.g. hints.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub tertiary_text: Hsla,
    /// Status bar text.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_status_bar: Hsla,
    /// Mode label in the search bar.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_mode_label: Hsla,
    /// Search icon tint.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_search_icon: Hsla,
    /// Text in code blocks.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_code: Hsla,
    /// Search bar placeholder.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub text_placeholder: Hsla,
    // Status colors
    /// Warnings.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_warn: Hsla,
    /// Errors.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_err: Hsla,
    /// Success states.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_succ: Hsla,
    /// Informational messages.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub color_info: Hsla,
    // Config banner
    /// Config banner background.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub banner_bg: Hsla,
    /// Config banner border.
    #[serde(deserialize_with = "hsla_from_hex")]
    pub banner_border: Hsla,
    /// Config banner text.
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
                accent: hsla(45. / 360., 0.85, 0.65, 1.),
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

/// Accepts `#rgb`, `#rrggbb` or `#rrggbbaa`.
fn hsla_from_hex<'de, D: Deserializer<'de>>(d: D) -> Result<Hsla, D::Error> {
    let hex = String::deserialize(d)?;
    parse_hex_color(&hex)
        .ok_or_else(|| serde::de::Error::custom(format!("invalid color \"{hex}\"")))
}

/// Parses `#rgb`, `#rrggbb` or `#rrggbbaa` (the `#` is optional), keeping the alpha.
pub fn parse_hex_color(text: &str) -> Option<Hsla> {
    let hex = text.trim().trim_start_matches('#');
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let rgba = match hex.len() {
        3 => {
            let n = u32::from_str_radix(hex, 16).ok()?;
            let (r, g, b) = ((n >> 8) & 0xf, (n >> 4) & 0xf, n & 0xf);
            ((r * 0x11) << 24) | ((g * 0x11) << 16) | ((b * 0x11) << 8) | 0xff
        }
        6 => (u32::from_str_radix(hex, 16).ok()? << 8) | 0xff,
        8 => u32::from_str_radix(hex, 16).ok()?,
        _ => return None,
    };
    Some(gpui::rgba(rgba).into())
}

type ThemeCtor = fn() -> ThemeData;
const BUILTIN_THEMES: [(&str, ThemeCtor); 4] = [
    ("Default", ThemeData::dark),
    ("Nord", ThemeData::nord),
    ("Libre", ThemeData::libre),
    ("Catppuccin Mocha", ThemeData::catppuccin_mocha),
];

impl ThemeData {
    pub fn builtins() -> impl Iterator<Item = (&'static str, ThemeData)> {
        BUILTIN_THEMES.iter().map(|(name, f)| (*name, f()))
    }

    /// Looks up a built-in theme by name, ignoring case and `_`/`-`/space differences.
    pub fn builtin(name: &str) -> Option<Self> {
        let norm = |s: &str| s.to_lowercase().replace(['_', '-', ' '], "");
        let wanted = norm(name);
        if wanted == "dark" {
            return Some(Self::dark());
        }
        BUILTIN_THEMES
            .iter()
            .find(|(n, _)| norm(n) == wanted)
            .map(|(_, f)| f())
    }

    /// Loads a theme file. A top-level `extends = "<name>"` names the base: a built-in
    /// theme or another `<name>.toml` next to this file. Every key the file sets
    /// overrides the base; without `extends`, the default theme is the base.
    pub fn from_file(path: &std::path::Path) -> Result<Self, String> {
        let mut seen = Vec::new();
        let table = Self::resolve(path, &mut seen)?;
        toml::Value::Table(table)
            .try_into()
            .map_err(|e| format!("{}: {e}", path.display()))
    }

    fn resolve(
        path: &std::path::Path,
        seen: &mut Vec<std::path::PathBuf>,
    ) -> Result<toml::Table, String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if seen.contains(&canonical) {
            return Err(format!("{}: circular `extends`", path.display()));
        }
        seen.push(canonical);

        let content =
            std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut own: toml::Table =
            toml::from_str(&content).map_err(|e| format!("{}: {e}", path.display()))?;

        let mut base = match own.remove("extends") {
            None => Self::to_table(&Self::dark())?,
            Some(toml::Value::String(name)) => match Self::builtin(&name) {
                Some(theme) => Self::to_table(&theme)?,
                None => {
                    let file = path.with_file_name(format!("{name}.toml"));
                    if !file.is_file() {
                        return Err(format!(
                            "{}: `extends = \"{name}\"` is neither a built-in theme nor {}",
                            path.display(),
                            file.display()
                        ));
                    }
                    Self::resolve(&file, seen)?
                }
            },
            Some(other) => {
                return Err(format!(
                    "{}: `extends` must be a string, got {}",
                    path.display(),
                    other.type_str()
                ));
            }
        };
        base.extend(own);
        Ok(base)
    }

    /// Colors serialize as 8-bit hex (gpui's `Hsla` format), so a base theme passes
    /// through `extends` at hex precision, which is what's rendered anyway.
    fn to_table(theme: &ThemeData) -> Result<toml::Table, String> {
        toml::Table::try_from(theme).map_err(|e| e.to_string())
    }
}

impl ThemeData {
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

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard},
};

use gpui::{
    AnyElement,
    Hsla,
    ImageSource,
    Img,
    IntoElement,
    Pixels,
    SharedString,
    Styled,
    Svg,
    img,
    svg,
};
use serde::{Deserialize, Serialize};

use crate::{
    ICONS,
    loader::icon::{
        render::{IconRaster, classify},
        theme::lookup_themed,
    },
    sherlock_msg,
    utils::{
        errors::{SherlockMessage, types::SherlockErrorType},
        files::home_dir,
    },
};

/// A resolved icon. Stores *where* the icon comes from, not pixels.
#[derive(Hash, Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub enum IconType {
    /// Monochrome file, drawn with `svg()` so it can be tinted.
    Symbolic(SharedString),
    /// Full-color file (svg, png, ...).
    #[serde(alias = "Png")]
    Image(SharedString),
    /// Icon from the system theme. The concrete file is picked per draw size,
    /// since themes ship different artwork for 16px vs 48px.
    Themed { name: SharedString, symbolic: bool },
}

impl IconType {
    pub fn is_symbolic(&self) -> bool {
        matches!(
            self,
            Self::Symbolic(_) | Self::Themed { symbolic: true, .. }
        )
    }

    /// File to use when drawn at `size` logical pixels and `scale`.
    fn path_for(&self, size: Pixels, scale: f32) -> Option<SharedString> {
        match self {
            Self::Symbolic(p) | Self::Image(p) => Some(p.clone()),
            Self::Themed { name, symbolic } => {
                let size = f32::from(size).round().max(1.0) as u16;
                let scale = scale.ceil().max(1.0) as u16;
                IconThemeGuard::themed_path(name, size, scale, *symbolic)
            }
        }
    }

    /// Tintable element for symbolic icons, sized to `size`.
    pub fn svg(&self, size: Pixels) -> Option<Svg> {
        if !self.is_symbolic() {
            return None;
        }
        let path = self.path_for(size, 1.0)?;
        Some(svg().path(path).size(size))
    }

    /// Raster element for color icons, rendered at exactly `size` × window scale.
    pub fn img(&self, size: Pixels) -> Option<Img> {
        if self.is_symbolic() {
            return None;
        }
        let icon = self.clone();
        let source = ImageSource::Custom(Arc::new(move |window, cx| {
            let scale = window.scale_factor();
            let path = icon.path_for(size, scale)?;
            let device_px = (f32::from(size) * scale).ceil() as u32;
            window.use_asset::<IconRaster>(&(path, device_px), cx)
        }));
        Some(img(source).size(size))
    }

    /// Either variant as a ready element; `tint` only applies to symbolic icons.
    pub fn element(&self, size: Pixels, tint: impl Into<Hsla>) -> AnyElement {
        match self.svg(size) {
            Some(svg) => svg.text_color(tint).into_any_element(),
            None => self
                .img(size)
                .map(IntoElement::into_any_element)
                .unwrap_or_else(|| gpui::div().size(size).into_any_element()),
        }
    }
}

#[derive(Default)]
pub struct CustomIconTheme {
    /// Resolved lookups (including misses), keyed by icon name.
    pub buf: HashMap<SharedString, Option<IconType>>,
    /// Icons found in user-configured directories: name -> file. Read on use.
    index: HashMap<String, PathBuf>,
    /// Themed file per (name, size, scale), including misses.
    sized: HashMap<(SharedString, u16, u16), Option<SharedString>>,
}
impl CustomIconTheme {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_path<T: AsRef<Path>>(&mut self, path: T) {
        let path = path.as_ref();
        let path = match path.to_str().and_then(|s| s.strip_prefix("~/")) {
            Some(stripped) => match home_dir() {
                Ok(home) => home.join(stripped),
                Err(_) => return,
            },
            None => path.to_path_buf(),
        };
        Self::scan_path(&path, &mut self.index);
    }
    pub fn lookup_icon(&self, name: &str) -> Option<Option<IconType>> {
        self.buf.get(name).cloned()
    }
    pub fn custom_icon(&self, name: &str) -> Option<IconType> {
        self.index.get(name).and_then(|p| classify(p))
    }
    fn scan_path(path: &Path, index: &mut HashMap<String, PathBuf>) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            if entry_path.is_dir() {
                Self::scan_path(&entry_path, index);
                continue;
            }
            let is_icon = entry_path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "png" | "svg"));
            if is_icon && let Some(stem) = entry_path.file_stem().and_then(|s| s.to_str()) {
                index.entry(stem.to_string()).or_insert(entry_path);
            }
        }
    }
}

pub struct IconThemeGuard;
impl<'g> IconThemeGuard {
    fn get_theme() -> Result<&'g RwLock<CustomIconTheme>, SherlockMessage> {
        ICONS.get().ok_or_else(|| {
            sherlock_msg!(
                Error,
                SherlockErrorType::ConfigError("Failed to get ICONS singleton".into()),
                "Config not initialized"
            )
        })
    }

    pub fn get_read() -> Result<RwLockReadGuard<'g, CustomIconTheme>, SherlockMessage> {
        Self::get_theme()?.read().map_err(|_| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to read icon theme".into()),
                "Failed to acquire read lock on icon theme"
            )
        })
    }

    pub fn get_write() -> Result<RwLockWriteGuard<'g, CustomIconTheme>, SherlockMessage> {
        Self::get_theme()?.write().map_err(|_| {
            sherlock_msg!(
                Warning,
                SherlockErrorType::ConfigError("Failed to get mutable icon theme".into()),
                "Failed to acquire write lock on icon theme"
            )
        })
    }

    pub fn add_path<T: AsRef<Path>>(path: T) -> Result<(), SherlockMessage> {
        Self::get_write()?.add_path(path);
        Ok(())
    }

    pub fn lookup_icon(name: &str) -> Result<Option<Option<IconType>>, SherlockMessage> {
        Ok(Self::get_read()?.lookup_icon(name))
    }

    /// Memoized size-aware theme lookup (called on every render of themed icons).
    fn themed_path(name: &SharedString, size: u16, scale: u16, svg: bool) -> Option<SharedString> {
        let key = (name.clone(), size, scale);
        if let Some(hit) = Self::get_read().ok()?.sized.get(&key) {
            return hit.clone();
        }
        let found = lookup_themed(name, size, scale, svg)
            .map(|p| SharedString::from(p.to_string_lossy().into_owned()));
        if let Ok(mut t) = Self::get_write() {
            t.sized.insert(key, found.clone());
        }
        found
    }
}

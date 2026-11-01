use std::{path::Path, sync::Arc};

use gpui::{App, Asset, ImageCacheError, RenderImage, SharedString};
use image::{Frame, RgbaImage, imageops::FilterType};
use resvg::{tiny_skia, usvg};

use crate::loader::icon::cache::IconType;

fn is_symbolic_svg(name: &str, data: &[u8]) -> bool {
    name.ends_with("-symbolic") || data.windows(12).any(|w| w == b"currentColor")
}

/// Classify an icon file on disk into `IconType`.
pub fn classify(path: &Path) -> Option<IconType> {
    let key = SharedString::from(path.to_string_lossy().into_owned());
    match path.extension().and_then(|e| e.to_str()) {
        Some("svg") => {
            let data = std::fs::read(path).ok()?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let symbolic = is_symbolic_svg(stem, &data)
                || path.components().any(|c| c.as_os_str() == "symbolic");
            Some(if symbolic {
                IconType::Symbolic(key)
            } else {
                IconType::Image(key)
            })
        }
        Some("png" | "jpg" | "jpeg") if path.exists() => Some(IconType::Image(key)),
        _ => None,
    }
}

/// Classify an embedded asset (`icons/<name>.svg|png`).
pub fn classify_embedded(name: &str, key: String, data: &[u8]) -> IconType {
    if key.ends_with(".svg") && is_symbolic_svg(name, data) {
        IconType::Symbolic(key.into())
    } else {
        IconType::Image(key.into())
    }
}

/// gpui asset: rasterizes an icon at a given device-pixel size.
/// Cached by gpui per `(path, size)` and loaded on the background executor.
pub enum IconRaster {}

impl Asset for IconRaster {
    type Source = (SharedString, u32);
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        (path, size): Self::Source,
        cx: &mut App,
    ) -> impl Future<Output = Self::Output> + Send + 'static {
        let assets = cx.asset_source().clone();
        async move {
            let err = |m: &str| ImageCacheError::Asset(format!("{path}: {m}").into());
            let bytes = assets
                .load(&path)
                .ok()
                .flatten()
                .ok_or_else(|| err("not found"))?;

            let mut rgba = if path.ends_with(".svg") {
                render_svg(&bytes, size).ok_or_else(|| err("invalid svg"))?
            } else {
                let img = image::load_from_memory(&bytes)
                    .map_err(|e| err(&e.to_string()))?
                    .into_rgba8();
                downscale(img, size)
            };

            // gpui expects BGRA
            for px in rgba.chunks_exact_mut(4) {
                px.swap(0, 2);
            }
            Ok(Arc::new(RenderImage::new(vec![Frame::new(rgba)])))
        }
    }
}

fn render_svg(data: &[u8], size: u32) -> Option<RgbaImage> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default()).ok()?;
    let (w, h) = (tree.size().width(), tree.size().height());
    let zoom = size as f32 / w.max(h);
    let (pw, ph) = ((w * zoom).round() as u32, (h * zoom).round() as u32);

    let mut pixmap = tiny_skia::Pixmap::new(pw.max(1), ph.max(1))?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom, zoom),
        &mut pixmap.as_mut(),
    );

    // tiny-skia is premultiplied; gpui wants straight alpha
    let mut buf = pixmap.take();
    for px in buf.chunks_exact_mut(4) {
        let a = px[3] as u32;
        if a != 0 && a != 255 {
            for c in &mut px[..3] {
                *c = ((*c as u32 * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    RgbaImage::from_raw(pw.max(1), ph.max(1), buf)
}

fn downscale(img: RgbaImage, size: u32) -> RgbaImage {
    let (w, h) = img.dimensions();
    if w.max(h) <= size {
        return img;
    }
    let scale = size as f32 / w.max(h) as f32;
    image::imageops::resize(
        &img,
        ((w as f32 * scale).round() as u32).max(1),
        ((h as f32 * scale).round() as u32).max(1),
        FilterType::Triangle,
    )
}

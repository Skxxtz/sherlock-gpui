use crate::loader::icon::cache::IconType;
use crate::utils::paths::get_cache_dir;
use resvg::usvg;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub fn handle_symblic(name: &str, svg_data: &[u8]) -> Option<IconType> {
    let is_symbolic =
        name.ends_with("-symbolic") || svg_data.windows(12).any(|w| w == b"currentColor");
    if is_symbolic {
        copy_svg_to_cache(name, svg_data).map(IconType::Symbolic)
    } else {
        None
    }
}

pub fn render_svg_to_cache(key: &str, path: PathBuf) -> Option<IconType> {
    if !path.exists() {
        return None;
    }

    let ext = path.extension().and_then(|e| e.to_str());

    match ext {
        Some("svg") => {
            let svg_data = std::fs::read(&path).ok()?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            let is_symbolic = stem.ends_with("-symbolic")
                || path.components().any(|c| c.as_os_str() == "symbolic")
                || svg_data.windows(12).any(|w| w == b"currentColor");
            if is_symbolic {
                copy_svg_to_cache(key, &svg_data).map(IconType::Symbolic)
            } else {
                render_to_png_cache(key, &svg_data).map(IconType::Png)
            }
        }
        Some("png") => Some(IconType::Png(Arc::from(path.into_boxed_path()))),
        _ => None,
    }
}

pub fn render_to_png_cache(key: &str, svg_data: &[u8]) -> Option<Arc<Path>> {
    let mut out = get_cache_dir().ok()?.join("icons");
    std::fs::create_dir_all(&out).ok()?;
    out.push(format!("{}.png", key.replace('/', "_")));

    if out.exists() {
        return Some(Arc::from(out.into_boxed_path()));
    }

    let opt = usvg::Options {
        text_rendering: usvg::TextRendering::OptimizeLegibility,
        image_rendering: usvg::ImageRendering::OptimizeQuality,
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(svg_data, &opt)
        .map_err(|e| eprintln!("Failed to parse SVG {key}: {e}"))
        .ok()?;

    let svg_w = tree.size().width();
    let svg_h = tree.size().height();
    let render_size = (48.0_f32 * 2.0).max(64.0);
    let zoom = render_size / svg_w.max(svg_h);
    let width = (svg_w * zoom).round() as u32;
    let height = (svg_h * zoom).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height).or_else(|| {
        eprintln!("Failed to create pixmap for {key}");
        None
    })?;

    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(zoom, zoom),
        &mut pixmap.as_mut(),
    );

    pixmap
        .save_png(&out)
        .map_err(|e| eprintln!("Failed to cache {key}: {e}"))
        .ok()?;

    Some(Arc::from(out.into_boxed_path()))
}

pub fn copy_png_to_cache(name: &str, png_data: &[u8]) -> Option<Arc<Path>> {
    let mut out = get_cache_dir().ok()?.join("icons");
    std::fs::create_dir_all(&out).ok()?;
    out.push(format!("{}.png", name.replace('/', "_")));
    if !out.exists() {
        fs::write(&out, png_data).ok()?;
    }

    Some(out.into_boxed_path().into())
}

pub fn copy_svg_to_cache(name: &str, svg_data: &[u8]) -> Option<Arc<Path>> {
    let mut out = get_cache_dir().ok()?.join("icons");
    std::fs::create_dir_all(&out).ok()?;
    out.push(format!("{}.svg", name.replace('/', "_")));
    if !out.exists() {
        fs::write(&out, svg_data).ok()?;
    }

    Some(out.into_boxed_path().into())
}

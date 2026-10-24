use std::sync::Arc;

use gpui::{App, AppContext, RenderImage, SharedString};
use image::{Frame, RgbaImage};

use crate::utils::{
    filekinds::{FileCategory, FileKind},
    preview_lru::PreviewCache,
};

mod pdf;
mod raster;
mod svg;
mod text;

pub use text::TextPreview;

/// Longest side of a thumbnail, in pixels. The sidebar reserves this much height.
pub const THUMB_PX: u32 = 300;

#[derive(Clone)]
pub enum Preview {
    Image(Arc<RenderImage>),
    Text(SharedString),
}

/// Result of asking for a preview.
pub enum PreviewState {
    /// Rendered and cached; ready to draw.
    Ready(Preview),
    /// Rendering in the background; reserve the space, windows refresh when done.
    Loading(PreviewKind),
    /// Rendering failed or the format isn't supported; show no preview.
    Failed,
}

/// What kind of preview a file gets.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum PreviewKind {
    /// An image thumbnail, rendered by the given renderer.
    Thumbnail(ThumbKind),
    /// The first lines of the file as text.
    Text,
    /// No preview for this file type.
    None,
}

impl From<FileKind> for PreviewKind {
    fn from(value: FileKind) -> Self {
        match value {
            FileKind::Svg => PreviewKind::Thumbnail(ThumbKind::Svg),
            FileKind::Pdf => PreviewKind::Thumbnail(ThumbKind::Pdf),
            FileKind::JpegImage
            | FileKind::PngImage
            | FileKind::GifImage
            | FileKind::WebpImage
            | FileKind::Icon
            | FileKind::Bitmap
            | FileKind::TiffImage => PreviewKind::Thumbnail(ThumbKind::Raster),
            _ if value.is_text() || matches!(value.category(), FileCategory::Other) => {
                PreviewKind::Text
            }
            _ => PreviewKind::None,
        }
    }
}

impl PreviewState {
    /// Returns the preview for `path`, starting a background renderer if needed.
    ///
    /// * `Ready` - cached result
    /// * `Loading` - render in progress; windows refresh when it's done.
    /// * `Failed` - the file has no preview type, or rendering failed (also cached).
    pub fn request(path: SharedString, kind: FileKind, cx: &mut App) -> Self {
        let kind = PreviewKind::from(kind);
        if kind == PreviewKind::None {
            return Self::Failed; // don't cache or spawn for unpreviewable files
        }

        let cache = cx.default_global::<PreviewCache>();
        if let Some(hit) = cache.get(&path) {
            return hit.map_or(Self::Failed, Self::Ready);
        }
        if !cache.pending.insert(path.clone()) {
            return Self::Loading(kind);
        }

        let task = cx.background_spawn({
            let path = path.clone();
            async move {
                match kind {
                    PreviewKind::Thumbnail(t) => {
                        let img = t.render(&path, THUMB_PX).map(Preview::Image);
                        // Return freed decode buffers to the OS.
                        #[cfg(target_os = "linux")]
                        unsafe {
                            libc::malloc_trim(0);
                        }
                        img
                    }
                    PreviewKind::Text => TextPreview::default()
                        .read(path.as_str())
                        .map(|s| Preview::Text(s.into())),
                    // is early returned already
                    PreviewKind::None => unreachable!(),
                }
            }
        });

        cx.spawn(async move |cx| {
            let preview = task.await;
            cx.update(|cx| {
                cx.global_mut::<PreviewCache>().insert(path, preview);
                cx.refresh_windows();
            })
        })
        .detach();

        Self::Loading(kind)
    }
}

// Which renderer handles a thumbnail
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ThumbKind {
    /// For all kinds of image files
    Raster,
    /// For svg files
    Svg,
    /// For pdf files
    Pdf,
}

impl ThumbKind {
    /// Renders a GPU-ready (BGRA) thumbnail. Blocking; **run off UI thread**
    fn render(self, path: &str, max_px: u32) -> Option<Arc<RenderImage>> {
        let mut img = match self {
            Self::Raster => raster::render(path, max_px)?,
            Self::Svg => svg::render(path, max_px)?,
            Self::Pdf => pdf::render(path, max_px)?,
        };

        // Renders produce RGBA; GPUI expects BRGA.
        img.chunks_exact_mut(4).for_each(|px| px.swap(0, 2));
        Some(Arc::new(RenderImage::new([Frame::new(img)])))
    }
}

/// Target size for fitting a `width`x`height` source into a square box, never upscaling.
#[derive(Clone, Copy)]
struct Fit {
    /// Factor to apply to the source (<= 1.0).
    scale: f32,
    /// Output width in pixels (>= 1).
    width: u32,
    /// Output height in pixels (>= 1).
    height: u32,
}

impl Fit {
    /// Scaled `width`x`height` so its longest side is at most `max_px`.
    /// Smaller sources keep their size.
    fn new(width: f32, height: f32, max_px: u32) -> Self {
        let scale = (max_px as f32 / width.max(height).max(1.)).min(1.);

        Self {
            scale,
            width: ((width * scale) as u32).max(1),
            height: ((height * scale) as u32).max(1),
        }
    }
}

// Convenience for renderers.
type Rendered = Option<RgbaImage>;

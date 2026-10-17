use crate::{
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        image_buf::ImageBuf,
        preview_lru::PreviewCache,
    },
};
use gpui::{App, AppContext, RenderImage, SharedString};
use image::{ColorType, Frame, ImageBuffer, ImageDecoder, ImageReader, Rgba, imageops};
use memmap2::MmapMut;
use std::{path::PathBuf, sync::Arc};

/// Result of asking for a thumbnail
pub enum PreviewState {
    /// Rendered and cached; ready to draw.
    Ready(Arc<RenderImage>),
    /// Rendering in the background; reserve the space, windows refresh when done.
    Loading,
    /// Rendering failed or the format isn't supported; show no preview.
    Failed,
}

fn load_rgba(path: &str) -> Result<ImageBuf, SherlockMessage> {
    let decoder = ImageReader::open(path)
        .map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
                e.to_string()
            )
        })?
        .with_guessed_format()
        .map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
                e.to_string()
            )
        })?
        .into_decoder()
        .map_err(|e| {
            sherlock_msg!(
                Error,
                SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
                e.to_string()
            )
        })?;

    let (w, h) = decoder.dimensions();
    let mut out = ImageBuf::new(w, h).map_err(|e| {
        sherlock_msg!(
            Error,
            SherlockErrorType::InvalidData,
            format!("failed to create image buf of size {w}x{h}. {e}")
        )
    })?;

    // Bytes per source pixel for the 8-bit formats we expand to RGBA.
    let src_bpp = match decoder.color_type() {
        ColorType::Rgba8 => {
            decoder.read_image(&mut out).map_err(|e| {
                sherlock_msg!(
                    Error,
                    SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
                    e.to_string()
                )
            })?;
            return Ok(out);
        }
        ColorType::Rgb8 => 3,
        ColorType::La8 => 2,
        ColorType::L8 => 1,
        other => {
            return Err(sherlock_msg!(
                Warning,
                SherlockErrorType::InvalidFunction,
                format!("unsupported color type: {other:?}")
            ));
        }
    };

    // Temporary OS-backed buffer, unmapped when this function returns.
    let mut tmp = MmapMut::map_anon(decoder.total_bytes() as usize)
        .map_err(|e| sherlock_msg!(Error, SherlockErrorType::IO, e.to_string()))?;
    decoder.read_image(&mut tmp).map_err(|e| {
        sherlock_msg!(
            Error,
            SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
            e.to_string()
        )
    })?;

    for (src, dst) in tmp.chunks_exact(src_bpp).zip(out.chunks_exact_mut(4)) {
        let px = match src_bpp {
            3 => [src[0], src[1], src[2], 255],
            2 => [src[0], src[0], src[0], src[1]],
            _ => [src[0], src[0], src[0], 255],
        };
        dst.copy_from_slice(&px);
    }
    Ok(out)
}

/// Runs on a background thread. Big decode buffer is unmapped before returning.
pub fn make_preview(path: &str, max_px: u32) -> Option<Arc<RenderImage>> {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);

    let mut thumb = match ext.as_deref() {
        Some("svg") => render_svg(path, max_px)?,
        Some("pdf") => render_pdf(path, max_px)?,
        _ => raster_thumb(path, max_px)?,
    };

    for px in thumb.chunks_exact_mut(4) {
        px.swap(0, 2);
    } // RGBA → BGRA for GPUI
    Some(Arc::new(RenderImage::new([Frame::new(thumb)])))
}

fn raster_thumb(path: &str, max_px: u32) -> Option<image::RgbaImage> {
    let full = load_rgba(path).ok()?;
    let (w, h) = (full.width.max(1), full.height.max(1));
    let s = (max_px as f32 / w.max(h) as f32).min(1.0); // fit, never upscale
    let (tw, th) = (
        ((w as f32 * s) as u32).max(1),
        ((h as f32 * s) as u32).max(1),
    );
    let view = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(full.width, full.height, &full)?;
    Some(imageops::thumbnail(&view, tw, th))
}

fn render_svg(path: &str, max_px: u32) -> Option<image::RgbaImage> {
    let data = std::fs::read(path).ok()?;
    let tree = resvg::usvg::Tree::from_data(&data, &Default::default()).ok()?;
    let s = tree.size();
    let scale = (max_px as f32 / s.width().max(s.height())).min(1.0);
    let (w, h) = (
        ((s.width() * scale) as u32).max(1),
        ((s.height() * scale) as u32).max(1),
    );
    let mut pm = resvg::tiny_skia::Pixmap::new(w, h)?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(scale, scale),
        &mut pm.as_mut(),
    );
    image::RgbaImage::from_raw(w, h, pm.take())
}

/// Returns the preview if ready; otherwise starts loading it in the background.
pub fn request_preview(path: SharedString, cx: &mut App) -> PreviewState {
    let cache = cx.default_global::<PreviewCache>();
    if let Some(hit) = cache.get(&path) {
        return hit.map_or(PreviewState::Failed, PreviewState::Ready);
    }
    if !cache.pending.insert(path.clone()) {
        return PreviewState::Loading;
    }

    let task = cx.background_spawn({
        let path = path.clone();
        async move {
            let img = make_preview(&path, 300);
            #[cfg(target_os = "linux")]
            unsafe {
                libc::malloc_trim(0);
            }
            img
        }
    });
    cx.spawn(async move |cx| {
        let img = task.await;
        cx.update(|cx| {
            cx.global_mut::<PreviewCache>().insert(path, img);
            cx.refresh_windows();
        });
    })
    .detach();

    PreviewState::Loading
}

/// First lines of a text file, or None if it looks binary.
pub fn read_text_preview<P: AsRef<std::path::Path>>(
    path: P,
    max_bytes: usize,
    max_lines: usize,
) -> Option<String> {
    use std::io::Read;

    let path = path.as_ref();

    let mut buf = Vec::with_capacity(max_bytes);
    std::fs::File::open(path)
        .ok()?
        .take(max_bytes as u64)
        .read_to_end(&mut buf)
        .ok()?;

    if buf.contains(&0) {
        return None; // NUL byte → binary
    }
    // Cut off a UTF-8 character that may have been split at the 4 KB boundary.
    let text = match std::str::from_utf8(&buf) {
        Ok(s) => s,
        Err(e) if e.error_len().is_none() => std::str::from_utf8(&buf[..e.valid_up_to()]).ok()?,
        Err(_) => return None, // not UTF-8
    };

    let preview: Vec<String> = text
        .lines()
        .take(max_lines)
        .map(|l| l.replace('\t', "    ").trim_end().to_owned())
        .collect();
    (!preview.iter().all(|l| l.is_empty())).then(|| preview.join("\n"))
}

fn render_pdf(path: &str, max_px: u32) -> Option<image::RgbaImage> {
    use mupdf::{Colorspace, Document, Matrix};

    let doc = Document::open(path).ok()?;
    let page = doc.load_page(0).ok()?;
    let b = page.bounds().ok()?;
    let scale = (max_px as f32 / (b.x1 - b.x0).max(b.y1 - b.y0)).min(1.0);
    // No alpha: pages without a painted background would otherwise be transparent,
    // which makes black text invisible on a dark theme.
    let pix = page
        .to_pixmap(
            &Matrix::new_scale(scale, scale),
            &Colorspace::device_rgb(),
            false,
            false,
        )
        .ok()?;

    let (w, h) = (pix.width(), pix.height());
    let src = pix.samples();
    let stride = src.len() / h.max(1) as usize;
    let mut out = Vec::with_capacity(w as usize * h as usize * 4);
    for row in src.chunks_exact(stride) {
        for px in row[..w as usize * 3].chunks_exact(3) {
            out.extend_from_slice(&[px[0], px[1], px[2], 255]);
        }
    }
    image::RgbaImage::from_raw(w, h, out)
}

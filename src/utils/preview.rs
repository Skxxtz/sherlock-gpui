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
    let mut thumb = if path.to_lowercase().ends_with(".svg") {
        render_svg(path, max_px)?
    } else {
        let full = load_rgba(path).ok()?;
        let view = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(full.width, full.height, &full)?;
        imageops::thumbnail(
            &view,
            max_px,
            max_px.min(max_px * full.height / full.width.max(1)).max(1),
        )
    };
    for px in thumb.chunks_exact_mut(4) {
        px.swap(0, 2);
    } // RGBA → BGRA for GPUI
    Some(Arc::new(RenderImage::new([Frame::new(thumb)])))
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
pub fn request_preview(path: SharedString, cx: &mut App) -> Option<Arc<RenderImage>> {
    let cache = cx.default_global::<PreviewCache>();
    if let Some(hit) = cache.get(&path) {
        return hit;
    }
    if !cache.pending.insert(path.clone()) {
        return None;
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
    None
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

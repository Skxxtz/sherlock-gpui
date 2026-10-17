use std::path::PathBuf;

use image::{ColorType, ImageBuffer, ImageDecoder, ImageReader, Rgba, imageops};
use memmap2::MmapMut;

use crate::{
    sherlock_msg,
    utils::{
        errors::{
            SherlockMessage,
            types::{FileAction, SherlockErrorType},
        },
        image_buf::ImageBuf,
        preview::{Fit, Rendered},
    },
};

/// Decodes the image and scales it down to fit `max_px`.
pub(super) fn render(path: &str, max_px: u32) -> Rendered {
    let full = decode_rgba(path).ok()?;
    let fit = Fit::new(full.width as f32, full.height as f32, max_px);
    let view = ImageBuffer::<Rgba<u8>, &[u8]>::from_raw(full.width, full.height, &full)?;
    Some(imageops::thumbnail(&view, fit.width, fit.height))
}

/// 8-bit source layouts that are expandable to RGBA.
#[derive(Clone, Copy)]
enum SrcLayout {
    Rgba,
    Rgb,
    GrayAlpha,
    Gray,
}

impl SrcLayout {
    fn from_color(c: ColorType) -> Option<Self> {
        Some(match c {
            ColorType::Rgba8 => Self::Rgba,
            ColorType::Rgb8 => Self::Rgb,
            ColorType::La8 => Self::GrayAlpha,
            ColorType::L8 => Self::Gray,
            _ => return None,
        })
    }

    fn bytes_per_px(self) -> usize {
        match self {
            Self::Rgba => 4,
            Self::Rgb => 3,
            Self::GrayAlpha => 2,
            Self::Gray => 1,
        }
    }

    fn to_rgba(self, s: &[u8]) -> [u8; 4] {
        match self {
            Self::Rgba => [s[0], s[1], s[2], s[3]],
            Self::Rgb => [s[0], s[1], s[2], 255],
            Self::GrayAlpha => [s[0], s[0], s[0], s[1]],
            Self::Gray => [s[0], s[0], s[0], 255],
        }
    }
}

fn decode_rgba(path: &str) -> Result<ImageBuf, SherlockMessage> {
    let read_err = |e: &dyn std::fmt::Display| {
        sherlock_msg!(
            Error,
            SherlockErrorType::FileError(FileAction::Read, PathBuf::from(path)),
            e.to_string()
        )
    };

    let decoder = ImageReader::open(path)
        .map_err(|e| read_err(&e))?
        .with_guessed_format()
        .map_err(|e| read_err(&e))?
        .into_decoder()
        .map_err(|e| read_err(&e))?;

    let color = decoder.color_type();
    let layout = SrcLayout::from_color(color).ok_or_else(|| {
        sherlock_msg!(
            Warning,
            SherlockErrorType::InvalidData,
            format!("unsupported color type: {color:?}")
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

    // Already RGBA: decode straight into output.
    if let SrcLayout::Rgba = layout {
        decoder.read_image(&mut out).map_err(|e| read_err(&e))?;
        return Ok(out);
    }

    // Temporary OS-backed buffer, unmapped when this function returns.
    let mut tmp = MmapMut::map_anon(decoder.total_bytes() as usize)
        .map_err(|e| sherlock_msg!(Error, SherlockErrorType::IO, e.to_string()))?;
    decoder.read_image(&mut tmp).map_err(|e| read_err(&e))?;

    for (src, dst) in tmp
        .chunks_exact(layout.bytes_per_px())
        .zip(out.chunks_exact_mut(4))
    {
        dst.copy_from_slice(&layout.to_rgba(src));
    }

    Ok(out)
}

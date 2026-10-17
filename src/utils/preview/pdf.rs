use image::RgbaImage;
use mupdf::{Colorspace, Document, Matrix};

use super::{Fit, Rendered};

pub(super) fn render(path: &str, max_px: u32) -> Rendered {
    let doc = Document::open(path).ok()?;
    let page = doc.load_page(0).ok()?;
    let b = page.bounds().ok()?;
    let fit = Fit::new(b.x1 - b.x0, b.y1 - b.y0, max_px);

    let pix = page
        .to_pixmap(
            &Matrix::new_scale(fit.scale, fit.scale),
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

    RgbaImage::from_raw(w, h, out)
}

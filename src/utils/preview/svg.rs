use image::RgbaImage;
use resvg::{tiny_skia, usvg};

use super::{Fit, Rendered};

/// Rasterizes the SVG to fit `max_px`.
pub(super) fn render(path: &str, max_px: u32) -> Rendered {
    let data = std::fs::read(path).ok()?;
    let tree = usvg::Tree::from_data(&data, &Default::default()).ok()?;
    let size = tree.size();
    let fit = Fit::new(size.width(), size.height(), max_px);

    let mut pm = tiny_skia::Pixmap::new(fit.width, fit.height)?;
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(fit.scale, fit.scale),
        &mut pm.as_mut(),
    );
    RgbaImage::from_raw(fit.width, fit.height, pm.take())
}

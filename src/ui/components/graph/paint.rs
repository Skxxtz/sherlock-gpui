use gpui::{Bounds, Corners, Hsla, PathBuilder, Pixels, Point, Window, fill, point, px, size};

use super::{GraphKind, scale::Scale};

/// Everything the canvas needs, owned so it can move into the paint closure.
pub(super) struct Plot {
    pub kind: GraphKind,
    pub x: Scale,
    pub y: Scale,
    pub y_ticks: Vec<f64>,
    pub series: Vec<(Vec<(f64, f64)>, Hsla)>,
    pub grid: Option<Hsla>,
    pub stroke: Pixels,
    /// Bar width as a fraction of the x domain, per series.
    pub bar_width: f64,
}

impl Plot {
    fn at(&self, b: &Bounds<Pixels>, (x, y): (f64, f64)) -> Point<Pixels> {
        point(
            b.origin.x + b.size.width * self.x.norm(x) as f32,
            b.origin.y + b.size.height * (1. - self.y.norm(y)) as f32,
        )
    }

    /// Y value areas and bars grow from: 0 if visible, otherwise the nearest edge.
    fn baseline(&self) -> f64 {
        0f64.clamp(self.y.min, self.y.max)
    }

    pub fn paint(&self, b: Bounds<Pixels>, window: &mut Window) {
        if let Some(grid) = self.grid {
            for &t in &self.y_ticks {
                let y = self.at(&b, (self.x.min, t)).y;
                let mut line = PathBuilder::stroke(px(1.)).dash_array(&[px(3.), px(3.)]);
                line.move_to(point(b.origin.x, y));
                line.line_to(point(b.top_right().x, y));
                if let Ok(path) = line.build() {
                    window.paint_path(path, grid);
                }
            }
        }

        let n = self.series.len();
        for (i, (points, color)) in self.series.iter().enumerate() {
            let pts: Vec<_> = points.iter().map(|&p| self.at(&b, p)).collect();
            match self.kind {
                GraphKind::Line => self.line(&pts, *color, window),
                GraphKind::Area => {
                    self.area(&b, &pts, color.opacity(0.25), window);
                    self.line(&pts, *color, window);
                }
                GraphKind::Scatter => dots(&pts, self.stroke * 2.5, *color, window),
                GraphKind::Bar => self.bars(&b, points, i, n, *color, window),
            }
        }
    }

    fn line(&self, pts: &[Point<Pixels>], color: Hsla, window: &mut Window) {
        if pts.len() < 2 {
            return dots(pts, self.stroke * 2.5, color, window);
        }
        let mut path = PathBuilder::stroke(self.stroke);
        path.add_polygon(pts, false);
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }

    fn area(&self, b: &Bounds<Pixels>, pts: &[Point<Pixels>], color: Hsla, window: &mut Window) {
        let (Some(first), Some(last)) = (pts.first(), pts.last()) else {
            return;
        };
        let base = self.at(b, (self.x.min, self.baseline())).y;
        let mut poly = Vec::with_capacity(pts.len() + 2);
        poly.push(point(first.x, base));
        poly.extend_from_slice(pts);
        poly.push(point(last.x, base));
        let mut path = PathBuilder::fill();
        path.add_polygon(&poly, true);
        if let Ok(path) = path.build() {
            window.paint_path(path, color);
        }
    }

    fn bars(
        &self,
        b: &Bounds<Pixels>,
        points: &[(f64, f64)],
        index: usize,
        count: usize,
        color: Hsla,
        window: &mut Window,
    ) {
        let width = b.size.width * (self.bar_width / self.x.span()) as f32;
        let base = self.baseline();
        for &(x, y) in points {
            // groups of `count` bars centred on x
            let offset = width * (index as f32 - (count as f32 - 1.) / 2.);
            let top = self.at(b, (x, y.max(base)));
            let bottom = self.at(b, (x, y.min(base)));
            let bounds = Bounds::new(
                point(top.x + offset - width * 0.5, top.y),
                size(width, (bottom.y - top.y).max(px(1.))),
            );
            let radius = (width * 0.25).min(px(3.));
            window.paint_quad(fill(bounds, color).corner_radii(Corners::all(radius)));
        }
    }
}

fn dots(pts: &[Point<Pixels>], diameter: Pixels, color: Hsla, window: &mut Window) {
    for &p in pts {
        let bounds = Bounds::centered_at(p, size(diameter, diameter));
        window.paint_quad(fill(bounds, color).corner_radii(Corners::all(diameter * 0.5)));
    }
}

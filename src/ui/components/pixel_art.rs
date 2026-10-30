use gpui::{Bounds, Hsla, IntoElement, Pixels, Styled, canvas, fill, point, px};

/// A finished icon: one palette char (as byte) per pixel.
pub type Grid<const N: usize> = [[u8; N]; N];

pub type Sprite = &'static [&'static str];

/// A sprite placed at column `x`, row `y`, with optional colour swaps.
#[derive(Clone, Copy)]
pub struct Layer {
    sprite: Sprite,
    x: usize,
    y: usize,
    swap: &'static [(u8, u8)],
}

pub const fn at(sprite: Sprite, x: usize, y: usize) -> Layer {
    Layer {
        sprite,
        x,
        y,
        swap: &[],
    }
}

impl Layer {
    /// Recolours the sprite, e.g. a white cloud into a dark one.
    pub const fn swap(mut self, swap: &'static [(u8, u8)]) -> Self {
        self.swap = swap;
        self
    }
}

/// Draws `layers` in order (later on top); `.` is transparent.
/// Panics at compile time if a sprite leaves the grid.
pub const fn compose<const N: usize>(layers: &[Layer]) -> Grid<N> {
    let mut grid = [[b'.'; N]; N];
    let mut i = 0;
    while i < layers.len() {
        let l = layers[i];
        let mut r = 0;
        while r < l.sprite.len() {
            let row = l.sprite[r].as_bytes();
            let mut c = 0;
            while c < row.len() {
                let (y, x) = (l.y + r, l.x + c);
                assert!(y < N && x < N, "sprite out of bounds");
                let mut px = row[c];
                if px != b'.' {
                    let mut s = 0;
                    while s < l.swap.len() {
                        if l.swap[s].0 == px {
                            px = l.swap[s].1;
                            break;
                        }
                        s += 1;
                    }
                    grid[y][x] = px;
                }
                c += 1;
            }
            r += 1;
        }
        i += 1;
    }
    grid
}

/// Pixel art from `rows`; each byte is one `pixel`×`pixel` square.
pub fn pixel_art<const N: usize>(
    grid: &'static Grid<N>,
    size: Pixels,
    color: fn(char) -> Option<Hsla>,
) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let sf = window.scale_factor();
            let cell = f32::from(bounds.size.width) / N as f32;
            // i-th grid line, rounded to the physical pixel grid
            let edge = |origin: Pixels, i: usize| {
                px(((f32::from(origin) + i as f32 * cell) * sf).round() / sf)
            };

            for (y, row) in grid.iter().enumerate() {
                let top = edge(bounds.origin.y, y);
                let bottom = edge(bounds.origin.y, y + 1);
                let mut x = 0;
                while x < N {
                    // merge runs of the same colour into one quad
                    let (start, c) = (x, row[x]);
                    while x < N && row[x] == c {
                        x += 1;
                    }
                    if let Some(col) = color(c as char) {
                        let l = edge(bounds.origin.x, start);
                        let r = edge(bounds.origin.x, x);
                        window.paint_quad(fill(
                            Bounds::from_corners(point(l, top), point(r, bottom)),
                            col,
                        ));
                    }
                }
            }
        },
    )
    .size(size)
    .flex_none()
}

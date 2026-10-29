use gpui::{Hsla, IntoElement, ParentElement, Styled, div, px};

use crate::app::theme::ThemeData;

/// Holmes in profile, facing left. `X` figure, `P` pipe, `S` smoke, `.` empty. 24×21.
const SHERLOCK: [&str; 21] = [
    "...........XX...........",
    "........XXXXXXXX........",
    ".......XXXXXXXXXX.......",
    "......XXXXXXXXXXXX......",
    ".....XXXXXXXXXXXXX......",
    "...XXXXXXXXXXXXXXXXX....",
    ".XXXXX............XXX...",
    ".......XXXXXXXXXX..XXX..",
    ".......XXXXXXXXXXX......",
    "......XXyyXXXXXXXX......",
    ".S...XXXXXXxxxxxxX......",
    "..S.XXXXXxxxxxxxxx......",
    "S.....XXxxxxxxxxxx......",
    ".S....XXXxxxxxxxxx......",
    "....PPXXXXxxxxxxxx......",
    ".PPPP.XXXXXxxxxxxxx.....",
    ".PPP...XXXXxxxxxxxxx....",
    "..P.......xxxxxxxxxxx...",
    ".........xxxxxxxxxxxxx..",
    "........xxxxxxxxxxxxxx..",
    "......xxxxxxxxxxxxxxxxx.",
];
const PIXEL: f32 = 4.;

/// Pixel art from `rows`; `color` maps a character to its pixel colour (`None`: empty).
fn pixel_art(rows: &[&str], color: impl Fn(char) -> Option<Hsla>) -> impl IntoElement {
    div().flex().flex_col().children(rows.iter().map(|row| {
        div().flex().children(row.chars().map(|c| {
            let pixel = div().size(px(PIXEL));
            match color(c) {
                Some(color) => pixel.bg(color),
                None => pixel,
            }
        }))
    }))
}

pub fn sherlock_pixel_art(theme: &ThemeData) -> impl IntoElement {
    let accent = theme.accent();
    let coat = Hsla {
        h: accent.h * 0.8,
        s: accent.s * 0.6,
        l: accent.l * 0.62,
        ..accent
    };
    let eye = Hsla {
        s: accent.s * 0.6,
        l: accent.l * 0.62,
        ..accent
    };
    let pipe = Hsla {
        h: accent.h * 0.6,
        s: accent.s * 0.55,
        l: accent.l * 0.45,
        ..accent
    };
    let smoke = Hsla {
        s: accent.s * 0.2,
        l: (accent.l * 1.25).min(0.9),
        a: accent.a * 0.6,
        ..accent
    };

    pixel_art(&SHERLOCK, move |c| match c {
        'X' => Some(accent),
        'x' => Some(coat),
        'P' => Some(pipe),
        'y' => Some(eye),
        'S' => Some(smoke),
        _ => None,
    })
}

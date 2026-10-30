use gpui::{Hsla, hsla};

// Palette
// .  empty
// Y  sun            O  sun outline
// W  cloud          G  cloud shade / back cloud
// D  dark cloud     K  dark cloud shade
// B  rain           S  snow / ice
// L  lightning      M  mist / neutral
pub fn weather_palette(c: char) -> Option<Hsla> {
    let deg = |d: f32| d / 360.;
    Some(match c {
        'Y' => hsla(deg(48.), 0.95, 0.60, 1.),
        'O' => hsla(deg(32.), 0.95, 0.50, 1.),
        'W' => hsla(0., 0., 0.93, 1.),
        'G' => hsla(deg(220.), 0.10, 0.72, 1.),
        'D' => hsla(deg(220.), 0.12, 0.45, 1.),
        'K' => hsla(deg(220.), 0.15, 0.30, 1.),
        'B' => hsla(deg(205.), 0.85, 0.60, 1.),
        'S' => hsla(deg(195.), 0.60, 0.90, 1.),
        'L' => hsla(deg(52.), 1.00, 0.60, 1.),
        'M' => hsla(0., 0., 0.65, 1.),
        _ => return None,
    })
}

/// Dark storm cloud.
pub const DARK: &[(u8, u8)] = &[(b'W', b'D'), (b'G', b'K')];
/// Grey cloud in the background.
pub const BACK: &[(u8, u8)] = &[(b'W', b'G'), (b'G', b'D')];

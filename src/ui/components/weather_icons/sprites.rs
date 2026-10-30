use crate::ui::components::pixel_art::Sprite;

pub(super) const SUN: Sprite = &[
    "......Y......",
    ".Y.........Y.",
    "....OOOOO....",
    "...OYYYYYO...",
    "..OYYYYYYYO..",
    "..OYYYYYYYO..",
    "Y.OYYYYYYYO.Y",
    "..OYYYYYYYO..",
    "..OYYYYYYYO..",
    "...OYYYYYO...",
    "....OOOOO....",
    ".Y.........Y.",
    "......Y......",
];

pub const SUN_SMALL: Sprite = &[
    "....Y....",
    ".Y..Y..Y.",
    "...OOO...",
    "..OYYYO..",
    "YYOYYYOYY",
    "..OYYYO..",
    "...OOO...",
    ".Y..Y..Y.",
    "....Y....",
];

pub const CLOUD: Sprite = &[
    ".....WWW.....",
    "....WWWWW.WW.",
    "..WWWWWWWWWW.",
    ".WWWWWWWWWWWW",
    "WWWWWWWWWWWWW",
    "WWWWWWWWWWWWW",
    ".GGGGGGGGGGGG",
];

pub const RAIN: Sprite = &[
    "..B...B...B..",
    ".B...B...B...",
    ".............",
    "...B...B...B.",
    "..B...B...B..",
];

pub const RAIN_LIGHT: Sprite = &[
    "....B.....B..",
    "...B.....B...",
    ".............",
    ".......B.....",
    "......B......",
];

pub const SLEET: Sprite = &[
    "..B.....S...B",
    ".B.....SSS.B.",
    "........S....",
    ".....B.......",
    "S...B......S.",
];

pub const SNOW: Sprite = &[
    "S...S...S...S",
    ".............",
    "..S...S...S..",
    ".............",
    "S...S...S...S",
];

pub const FLAKES: Sprite = &[
    "..S......S...",
    ".SSS....SSS..",
    "..S......S...",
    ".............",
    "......S......",
];

#[rustfmt::skip]
pub const BOLT: Sprite = &[
    "..LL.", 
    ".LL..", 
    "LLLLL", 
    "...LL", 
    "..LL.", 
    ".L..."
];

pub const MIST_LINES: Sprite = &[
    "..MMMMMMMMMMM..",
    "...............",
    "....MMMMMMMMMM.",
    "...............",
    ".MMMMMMMMMMM...",
    "...............",
    "...MMMMMMMMMMM.",
    "...............",
    "..MMMMMMMMM....",
];

#[rustfmt::skip]
pub const QUESTION: Sprite = &[
    "..MMM..",
    ".M...M.",
    ".....M.",
    ".....M.",
    "....M..",
    "...M...",
    "...M...",
    ".......",
    "...M...",
];

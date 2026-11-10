use gpui::{Pixels, px};

pub const SPACE_2XS: Pixels = px(2.);
pub const SPACE_XS: Pixels = px(4.);
pub const SPACE_S: Pixels = px(6.);
pub const SPACE_M: Pixels = px(8.);
pub const SPACE_L: Pixels = px(10.);
pub const SPACE_XL: Pixels = px(12.);
pub const SPACE_2XL: Pixels = px(14.);
pub const SPACE_3XL: Pixels = px(16.);

// PAGE

pub const PAGE_X: Pixels = px(32.);
pub const PAGE_TOP: Pixels = px(26.);
pub const PAGE_BOTTOM: Pixels = px(28.);
/// Readable line length for prose.
pub const MEASURE: Pixels = px(660.);
/// Tables and code may use more room than prose.
pub const WIDE: Pixels = px(920.);
/// Space around the hairline between two exchanges of a chat.
pub const EXCHANGE_GAP: Pixels = px(24.);
/// Gap between top-level blocks of an answer.
pub const BLOCK_GAP: Pixels = SPACE_XL;

// CARDS

/// Horizontal inset of card content.
pub const CARD_X: Pixels = SPACE_2XL;
/// Height of a code card's header bar and of the status line.
pub const BAR_H: Pixels = px(34.);
/// Height of an output panel's header bar.
pub const OUTPUT_BAR_H: Pixels = px(32.);
/// Output panels scroll beyond this height.
pub const OUTPUT_MAX_H: Pixels = px(320.);

// SMALL PARTS

/// Status dots (live phase, running command).
pub const DOT: Pixels = px(6.);
/// The dot in front of an action row.
pub const ACTION_DOT: Pixels = px(8.);
/// Rules, progress bar, underlines.
pub const HAIRLINE: Pixels = px(1.);
/// Width reserved for a list marker (`•`, `12.`).
pub const LIST_MARKER_W: Pixels = px(18.);
/// Key caps in hints.
pub const KBD_X: Pixels = px(5.);
pub const KBD_RADIUS: Pixels = px(4.);
/// Small text buttons (Run, Copy, Clear).
pub const BUTTON_X: Pixels = SPACE_M;
pub const BUTTON_Y: Pixels = px(3.);
pub const BUTTON_RADIUS: Pixels = px(5.);

// EMPTY VIEW

pub const EMPTY_MAX_W: Pixels = px(460.);
pub const EMPTY_X: Pixels = px(24.);
pub const EMPTY_GAP: Pixels = px(22.);
/// Vertical inset of a recent question row.
pub const RECENT_Y: Pixels = px(7.);

// TYPE

/// The lead sentence and the empty view's title, relative to the body size.
pub const LEAD_SCALE: f32 = 1.75;
/// Headings of level 1 and 2, relative to the body size.
pub const HEADING_SCALE: f32 = 1.15;

pub const LINE_LEAD: f32 = 1.25;
pub const LINE_BODY: f32 = 1.65;
pub const LINE_CODE: f32 = 1.6;
pub const LINE_OUTPUT: f32 = 1.55;

/// Actions that can't run (unknown app).
pub const OPACITY_DISABLED: f32 = 0.55;
/// Secondary labels in headers.
pub const OPACITY_MUTED: f32 = 0.6;
/// Exchanges of a chat other than the one being read.
pub const OPACITY_DIM: f32 = 0.45;

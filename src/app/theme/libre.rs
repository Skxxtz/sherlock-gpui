use gpui::hsla;

use super::{Metrics, ThemeColors, ThemeData, Typography};

impl ThemeData {
    pub fn libre() -> Self {
        Self {
            typography: Typography::default(),
            metrics: Metrics::default(),
            colors: ThemeColors {
                // Cursor and Selection - Sophisticated Indigo/Slate
                cursor: hsla(220.0 / 360.0, 0.20, 0.30, 1.0),
                selection: hsla(220.0 / 360.0, 0.40, 0.50, 0.12),
                accent: hsla(220. / 360., 0.55, 0.45, 1.),

                // Backgrounds - Warm Paper & Alabaster
                bg_app: hsla(40.0 / 360.0, 0.10, 0.98, 1.0), // Off-white/Bone 0xFCFAF8
                bg_overlay: hsla(40.0 / 360.0, 0.08, 0.95, 1.0), // Matte Alabaster surface
                bg_selected: hsla(220.0 / 360.0, 0.15, 0.92, 1.0), // Soft Blue-tinted White
                bg_idle: hsla(0.0, 0.0, 0.0, 0.0),
                bg_status_bar: hsla(40.0 / 360.0, 0.05, 0.94, 1.0), // Slightly darker paper
                bg_keybind: hsla(0.0, 0.0, 0.90, 1.0),              // Light Gray
                bg_code: hsla(210.0 / 360.0, 0.20, 0.96, 1.0),      // Clean Code Background
                bg_badge: hsla(220.0 / 360.0, 0.10, 0.10, 0.05),
                bg_muted: hsla(0.0, 0.0, 0.0, 0.0),

                // Borders - Very Thin and Subtle
                border: hsla(0.0, 0.0, 0.85, 1.0), // Light stroke
                border_selected: hsla(220.0 / 360.0, 0.30, 0.70, 1.0),
                border_idle: hsla(0.0, 0.0, 0.90, 1.0),

                // Text - Deep Ink and Slate
                primary_text: hsla(220.0 / 360.0, 0.25, 0.15, 1.0), // Deep "Ink" Blue-Black
                secondary_text: hsla(220.0 / 360.0, 0.10, 0.45, 1.0), // Muted Slate
                tertiary_text: hsla(220.0 / 360.0, 0.08, 0.60, 1.0),
                text_status_bar: hsla(220.0 / 360.0, 0.15, 0.40, 1.0),
                text_mode_label: hsla(0.0, 0.0, 0.20, 1.0),
                text_search_icon: hsla(0.0, 0.0, 0.50, 1.0),
                text_code: hsla(220.0 / 360.0, 0.60, 0.40, 1.0), // Professional Blue
                text_placeholder: hsla(0.0, 0.0, 0.0, 0.25),

                // Status - Classy, desaturated tones
                color_warn: hsla(38.0 / 360.0, 0.60, 0.45, 1.0), // Ochre / Gold
                color_err: hsla(0.0, 0.50, 0.50, 1.0),           // Soft Carmine
                color_succ: hsla(150.0 / 360.0, 0.40, 0.45, 1.0), // Sage Green
                color_info: hsla(210.0 / 360.0, 0.40, 0.50, 1.0), // Desaturated Blue

                // Config banner - Light Champagne/Gold
                banner_bg: hsla(45.0 / 360.0, 0.30, 0.92, 1.0),
                banner_border: hsla(45.0 / 360.0, 0.40, 0.80, 1.0),
                banner_text: hsla(45.0 / 360.0, 0.60, 0.35, 1.0),
            },
        }
    }
}

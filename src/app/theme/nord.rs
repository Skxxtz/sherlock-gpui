use gpui::hsla;

use super::{Metrics, ThemeColors, ThemeData, Typography};

impl ThemeData {
    pub fn nord() -> Self {
        Self {
            typography: Typography::default(),
            metrics: Metrics::default(),
            colors: ThemeColors {
                cursor: hsla(213.0 / 360.0, 0.32, 0.52, 1.0),
                selection: hsla(213.0 / 360.0, 0.32, 0.52, 0.25),
                accent: hsla(193.0 / 360.0, 0.43, 0.67, 1.),
                bg_app: hsla(220.0 / 360.0, 0.17, 0.14, 1.0),
                bg_overlay: hsla(220.0 / 360.0, 0.17, 0.12, 1.0),
                bg_selected: hsla(220.0 / 360.0, 0.17, 0.28, 1.0),
                bg_idle: hsla(220.0 / 360.0, 0.17, 0.18, 1.0),
                bg_status_bar: hsla(220.0 / 360.0, 0.17, 0.12, 1.0),
                bg_keybind: hsla(220.0 / 360.0, 0.17, 0.22, 1.0),
                bg_code: hsla(220.0 / 360.0, 0.17, 0.20, 1.0),
                bg_muted: hsla(220.0 / 360.0, 0.17, 0.18, 1.0),
                bg_badge: hsla(220.0 / 360.0, 0.17, 0.32, 0.6),
                border: hsla(220.0 / 360.0, 0.17, 0.32, 1.0),
                border_selected: hsla(213.0 / 360.0, 0.32, 0.52, 0.4),
                border_idle: hsla(220.0 / 360.0, 0.17, 0.28, 1.0),
                primary_text: hsla(218.0 / 360.0, 0.27, 0.92, 1.0),
                secondary_text: hsla(219.0 / 360.0, 0.14, 0.65, 1.0),
                tertiary_text: hsla(219.0 / 360.0, 0.10, 0.45, 1.0),
                text_status_bar: hsla(219.0 / 360.0, 0.14, 0.40, 1.0),
                text_mode_label: hsla(219.0 / 360.0, 0.14, 0.30, 1.0),
                text_search_icon: hsla(219.0 / 360.0, 0.14, 0.50, 1.0),
                text_code: hsla(210.0 / 360.0, 0.34, 0.63, 1.0), // Nord8 Frost
                text_placeholder: hsla(219.0 / 360.0, 0.14, 0.35, 1.0),
                color_warn: hsla(40.0 / 360.0, 0.70, 0.73, 1.0),
                color_err: hsla(354.0 / 360.0, 0.42, 0.65, 1.0),
                color_succ: hsla(92.0 / 360.0, 0.28, 0.65, 1.0),
                color_info: hsla(200.0 / 360.0, 0.45, 0.68, 1.0),
                banner_bg: hsla(40.0 / 360.0, 0.70, 0.12, 1.0),
                banner_border: hsla(40.0 / 360.0, 0.70, 0.35, 1.0),
                banner_text: hsla(40.0 / 360.0, 0.70, 0.73, 1.0),
            },
        }
    }
}

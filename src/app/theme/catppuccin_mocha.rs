use gpui::hsla;

use super::{Metrics, ThemeColors, ThemeData, Typography};
impl ThemeData {
    pub fn catppuccin_mocha() -> Self {
        Self {
            typography: Typography::default(),
            metrics: Metrics::default(),
            colors: ThemeColors {
                cursor: hsla(267.0 / 360.0, 0.84, 0.81, 1.0),
                selection: hsla(267.0 / 360.0, 0.84, 0.81, 0.2),
                bg_app: hsla(240.0 / 360.0, 0.21, 0.12, 1.0),
                bg_overlay: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
                bg_selected: hsla(248.0 / 360.0, 0.15, 0.22, 1.0),
                bg_idle: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
                bg_status_bar: hsla(240.0 / 360.0, 0.21, 0.10, 1.0),
                bg_keybind: hsla(240.0 / 360.0, 0.21, 0.18, 1.0),
                bg_code: hsla(240.0 / 360.0, 0.21, 0.17, 1.0),
                bg_badge: hsla(248.0 / 360.0, 0.15, 0.22, 0.6),
                bg_muted: hsla(240.0 / 360.0, 0.21, 0.15, 1.0),
                border: hsla(240.0 / 360.0, 0.21, 0.25, 1.0),
                border_selected: hsla(267.0 / 360.0, 0.84, 0.81, 0.3),
                border_idle: hsla(240.0 / 360.0, 0.21, 0.22, 1.0),
                primary_text: hsla(226.0 / 360.0, 0.64, 0.88, 1.0),
                secondary_text: hsla(228.0 / 360.0, 0.24, 0.57, 1.0),
                tertiary_text: hsla(228.0 / 360.0, 0.15, 0.40, 1.0),
                text_status_bar: hsla(228.0 / 360.0, 0.24, 0.40, 1.0),
                text_mode_label: hsla(228.0 / 360.0, 0.24, 0.30, 1.0),
                text_search_icon: hsla(228.0 / 360.0, 0.24, 0.45, 1.0),
                text_code: hsla(189.0 / 360.0, 0.71, 0.73, 1.0),
                text_placeholder: hsla(228.0 / 360.0, 0.24, 0.40, 1.0),
                color_warn: hsla(41.0 / 360.0, 0.86, 0.83, 1.0),
                color_err: hsla(343.0 / 360.0, 0.81, 0.75, 1.0),
                color_succ: hsla(115.0 / 360.0, 0.54, 0.76, 1.0),
                color_info: hsla(205.0 / 360.0, 0.57, 0.70, 1.0),
                banner_bg: hsla(41.0 / 360.0, 0.86, 0.12, 1.0),
                banner_border: hsla(41.0 / 360.0, 0.86, 0.35, 1.0),
                banner_text: hsla(41.0 / 360.0, 0.86, 0.75, 1.0),
            },
        }
    }
}

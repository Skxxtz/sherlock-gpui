use gpui::{
    AbsoluteLength, AlignItems, DefiniteLength, EdgesRefinement, Fill, FlexDirection, Hsla,
    JustifyContent, Length, Rgba, SharedString, SizeRefinement, StyleRefinement, TextAlign, px,
};
use serde::Deserialize;

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct PluginStyle {
    // Layout
    pub flex_direction: Option<PluginFlexDirection>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub align_items: Option<PluginAlignItems>,
    pub justify_content: Option<PluginJustifyContent>,
    pub gap: Option<f32>,

    // Sizing
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_width: Option<f32>,
    pub min_height: Option<f32>,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,

    // Spacing
    pub padding: Option<f32>,
    pub padding_x: Option<f32>,
    pub padding_y: Option<f32>,
    pub margin: Option<f32>,
    pub margin_x: Option<f32>,
    pub margin_y: Option<f32>,

    // Visual
    pub background: Option<String>,   // hex
    pub border_color: Option<String>, // hex
    pub border_width: Option<f32>,
    pub corner_radii: Option<f32>,
    pub opacity: Option<f32>,

    // Text
    pub color: Option<String>, // hex, forwarded to text style
    pub font_family: Option<SharedString>,
    pub font_size: Option<f32>,
    pub text_align: Option<PluginTextAlign>,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginFlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginAlignItems {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Baseline,
    Stretch,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginJustifyContent {
    Start,
    End,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
    SpaceBetween,
    SpaceEvenly,
    SpaceAround,
}

#[derive(Copy, Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginTextAlign {
    Left,
    Right,
    Center,
}

impl PluginStyle {
    pub fn apply_to_style_refinement(&self, style: &mut StyleRefinement) {
        if let Some(dir) = self.flex_direction {
            style.flex_direction = Some(match dir {
                PluginFlexDirection::Row => FlexDirection::Row,
                PluginFlexDirection::Column => FlexDirection::Column,
                PluginFlexDirection::RowReverse => FlexDirection::RowReverse,
                PluginFlexDirection::ColumnReverse => FlexDirection::ColumnReverse,
            });
        }
        if let Some(grow) = self.flex_grow {
            style.flex_grow = Some(grow);
        }
        if let Some(shrink) = self.flex_shrink {
            style.flex_shrink = Some(shrink);
        }
        if let Some(align) = self.align_items {
            style.align_items = Some(match align {
                PluginAlignItems::Start => AlignItems::Start,
                PluginAlignItems::End => AlignItems::End,
                PluginAlignItems::FlexStart => AlignItems::FlexStart,
                PluginAlignItems::FlexEnd => AlignItems::FlexEnd,
                PluginAlignItems::Center => AlignItems::Center,
                PluginAlignItems::Baseline => AlignItems::Baseline,
                PluginAlignItems::Stretch => AlignItems::Stretch,
            });
        }
        if let Some(justify) = self.justify_content {
            style.justify_content = Some(match justify {
                PluginJustifyContent::Start => JustifyContent::FlexStart,
                PluginJustifyContent::End => JustifyContent::FlexEnd,
                PluginJustifyContent::FlexStart => JustifyContent::FlexStart,
                PluginJustifyContent::FlexEnd => JustifyContent::FlexEnd,
                PluginJustifyContent::Center => JustifyContent::Center,
                PluginJustifyContent::Stretch => JustifyContent::Stretch,
                PluginJustifyContent::SpaceBetween => JustifyContent::SpaceBetween,
                PluginJustifyContent::SpaceEvenly => JustifyContent::SpaceEvenly,
                PluginJustifyContent::SpaceAround => JustifyContent::SpaceAround,
            });
        }
        if let Some(gap) = self.gap {
            let g = DefiniteLength::Absolute(AbsoluteLength::Pixels(px(gap)));
            style.gap = SizeRefinement {
                width: Some(g),
                height: Some(g),
            };
        }

        // Sizing
        if let Some(w) = self.width {
            style.size.width = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(w)),
            )));
        }
        if let Some(h) = self.height {
            style.size.height = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(h)),
            )));
        }
        if let Some(w) = self.min_width {
            style.min_size.width = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(w)),
            )));
        }
        if let Some(h) = self.min_height {
            style.min_size.height = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(h)),
            )));
        }
        if let Some(w) = self.max_width {
            style.max_size.width = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(w)),
            )));
        }
        if let Some(h) = self.max_height {
            style.max_size.height = Some(Length::Definite(DefiniteLength::Absolute(
                AbsoluteLength::Pixels(px(h)),
            )));
        }

        // Padding — specific axes override uniform
        let pad_x = self
            .padding_x
            .or(self.padding)
            .map(|v| DefiniteLength::Absolute(AbsoluteLength::Pixels(px(v))));
        let pad_y = self
            .padding_y
            .or(self.padding)
            .map(|v| DefiniteLength::Absolute(AbsoluteLength::Pixels(px(v))));
        if let Some(p) = pad_x {
            style.padding.left = Some(p);
            style.padding.right = Some(p);
        }
        if let Some(p) = pad_y {
            style.padding.top = Some(p);
            style.padding.bottom = Some(p);
        }

        // Margin
        let mar_x = self
            .margin_x
            .or(self.margin)
            .map(|v| Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(v)))));
        let mar_y = self
            .margin_y
            .or(self.margin)
            .map(|v| Length::Definite(DefiniteLength::Absolute(AbsoluteLength::Pixels(px(v)))));
        if let Some(m) = mar_x {
            style.margin.left = Some(m);
            style.margin.right = Some(m);
        }
        if let Some(m) = mar_y {
            style.margin.top = Some(m);
            style.margin.bottom = Some(m);
        }

        // Visual
        if let Some(color) = self.background.as_deref().and_then(parse_hex) {
            style.background = Some(Fill::Color(color.into()));
        }
        if let Some(color) = self.border_color.as_deref().and_then(parse_hex) {
            style.border_color = Some(color);
        }
        if let Some(w) = self.border_width {
            let bw = AbsoluteLength::Pixels(px(w));
            style.border_widths = EdgesRefinement {
                top: Some(bw),
                right: Some(bw),
                bottom: Some(bw),
                left: Some(bw),
            }
        }
        if let Some(r) = self.corner_radii {
            let t = AbsoluteLength::Pixels(px(r));
            style.corner_radii = gpui::CornersRefinement {
                top_left: Some(t),
                top_right: Some(t),
                bottom_right: Some(t),
                bottom_left: Some(t),
            }
        }
        if let Some(o) = self.opacity {
            style.opacity = Some(o);
        }

        // text stuff
        if let Some(color) = self.color.as_deref().and_then(parse_hex) {
            style.text.color = Some(color);
        }
        if let Some(family) = &self.font_family {
            style.text.font_family = Some(family.clone());
        }
        if let Some(size) = self.font_size {
            style.text.font_size = Some(AbsoluteLength::Pixels(px(size)));
        }
        if let Some(align) = self.text_align {
            style.text.text_align = Some(match align {
                PluginTextAlign::Left => TextAlign::Left,
                PluginTextAlign::Right => TextAlign::Right,
                PluginTextAlign::Center => TextAlign::Center,
            });
        }
    }
}

impl From<PluginStyle> for StyleRefinement {
    fn from(s: PluginStyle) -> Self {
        let mut style = StyleRefinement::default();
        s.apply_to_style_refinement(&mut style);
        style
    }
}

fn parse_hex(hex: &str) -> Option<Hsla> {
    let hex = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255u8)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(
        Rgba {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
        .into(),
    )
}

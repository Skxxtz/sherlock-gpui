use smallvec::{SmallVec, smallvec};

use crate::{
    match_ignore_ascii,
    utils::intent::{Cursor, Intent, colors::ColorConverter, cursor::is_connector},
};

pub struct ColorParser;
impl ColorParser {
    pub fn parse_intent(mut cursor: Cursor<'_>) -> Option<Intent> {
        #[inline]
        fn color_space(s: &str) -> Option<&'static str> {
            match_ignore_ascii! {s,
                "rgb"  => "rgb",
                "rgba" => "rgba",
                "hex"  => "hex",
                "hsl"  => "hsl",
                "hsv"  => "hsv",
                "lab"  => "lab",
            }
        }

        let first = cursor.peek()?;

        let (from_space, values): (&'static str, SmallVec<[f32; 4]>) = if first.starts_with('#') {
            let hex = cursor.advance().unwrap();
            let (r, g, b) = ColorConverter::hex_to_rgb(hex)?;
            ("hex", smallvec![r, g, b])
        } else {
            let space = color_space(first)?;
            cursor.advance();
            let mut vals = SmallVec::<[f32; 4]>::new();
            while let Some(t) = cursor.peek() {
                if is_connector(t) {
                    break;
                }
                if let Ok(v) = t.parse::<f32>() {
                    vals.push(v);
                }
                cursor.advance();
            }
            (space, vals)
        };

        if values.is_empty() {
            return None;
        }

        if cursor.peek().map(is_connector).unwrap_or(false) {
            cursor.advance(); // skip connector
            if let Some(to_space) = cursor.advance().and_then(color_space) {
                return Some(Intent::ColorConvert {
                    from_space,
                    values,
                    to_space,
                });
            }
        }

        Some(Intent::ColorDisplay { from_space, values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::intent::{Cursor, Intent, Intent::ColorConvert, Intent::ColorDisplay};
    use smallvec::smallvec;

    fn create_test_cursor(input: &str) -> Vec<&str> {
        Intent::tokenize_kill_noise(input).collect()
    }

    #[test]
    fn test_color_conversion_parsing() {
        let cases = vec![
            (
                "rgb 255 0 0 to hex",
                ColorConvert {
                    from_space: "rgb",
                    values: smallvec![255.0, 0.0, 0.0],
                    to_space: "hex",
                },
            ),
            (
                "hsl 360 100 50 in rgb",
                ColorConvert {
                    from_space: "hsl",
                    values: smallvec![360.0, 100.0, 50.0],
                    to_space: "rgb",
                },
            ),
            (
                "#ff0000 in rgb",
                ColorConvert {
                    from_space: "hex",
                    values: smallvec![255.0, 0.0, 0.0],
                    to_space: "rgb",
                },
            ),
            (
                "rgb 255 255 255 as hsl",
                ColorConvert {
                    from_space: "rgb",
                    values: smallvec![255.0, 255.0, 255.0],
                    to_space: "hsl",
                },
            ),
        ];

        for (input, expected) in cases {
            let tokens = create_test_cursor(input);
            let cursor = Cursor::new(&tokens);
            let result = ColorParser::parse_intent(cursor);

            assert_eq!(result, Some(expected), "Failed parsing: '{}'", input);
        }
    }

    #[test]
    fn test_color_display_parsing() {
        // Testing cases where only the color is provided without conversion
        let input = "rgb 255 0 0";
        let tokens = create_test_cursor(input);
        let cursor = Cursor::new(&tokens);

        let expected = ColorDisplay {
            from_space: "rgb",
            values: smallvec![255.0, 0.0, 0.0],
        };

        let result = ColorParser::parse_intent(cursor);
        assert_eq!(result, Some(expected));
    }

    #[test]
    fn test_invalid_color_input() {
        let invalid_cases = vec!["invalid 0 0 0", "notacolor 1 2 3"];

        for input in invalid_cases {
            let tokens = create_test_cursor(input);
            let cursor = Cursor::new(&tokens);
            let result = ColorParser::parse_intent(cursor);

            assert!(result.is_none(), "Expected None for input: '{}'", input);
        }
    }
}

use super::utils::{extract_target_unit, extract_value_unit};
use crate::utils::intent::{Capabilities, Cursor, Intent, cursor::is_connector};

pub struct UnitParser;
impl UnitParser {
    pub fn parse_intent(cursor: Cursor<'_>, caps: &Capabilities) -> Option<Intent> {
        let tokens = cursor.remaining(); // &[&str] — no allocation

        // ── Strategy 1: connector-based split ────────────────────────────────────
        // Find all connector positions, try from rightmost to leftmost so that
        // "in" inside "0.5in" loses to a later "as"/"to".
        let connector_pos = tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| is_connector(t))
            .map(|(i, _)| i);

        // Collect into a tiny stack array — there will almost never be >4 connectors.
        let positions: arrayvec::ArrayVec<usize, 8> = connector_pos.collect();

        for &pos in positions.iter().rev() {
            let pre = &tokens[..pos];
            let post = &tokens[pos + 1..];
            if pre.is_empty() || post.is_empty() {
                continue;
            }
            if let Some((value, from)) = extract_value_unit(pre, caps)
                && let Some(to) = extract_target_unit(post, from.category())
            {
                return Some(Intent::Conversion { value, from, to });
            }
        }

        // ── Strategy 2: no connector — rightmost token(s) are the target unit ───
        // Try splitting off 1 token, then 2 tokens from the right.
        for target_len in [1usize, 2] {
            if tokens.len() <= target_len {
                continue;
            }
            let split = tokens.len() - target_len;
            let pre = &tokens[..split];
            let post = &tokens[split..];
            if let Some((value, from)) = extract_value_unit(pre, caps)
                && let Some(to) = extract_target_unit(post, from.category())
            {
                return Some(Intent::Conversion { value, from, to });
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use smallvec::SmallVec;

    use super::*;
    use crate::utils::intent::units::Unit;

    fn tokenize(input: &str) -> SmallVec<[&str; 16]> {
        Intent::tokenize_kill_noise(input).collect()
    }

    #[test]
    fn test_parse_intent_with_connectors() {
        let caps = Capabilities(Capabilities::EVERYTHING);

        let tokens = tokenize("50m feet");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);
        assert!(matches!(result, Some(Intent::Conversion { .. })));

        let tokens = tokenize("100 kg in lbs");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);
        assert!(matches!(result, Some(Intent::Conversion { .. })));

        let tokens = tokenize("0.5in as cm");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);
        if let Some(Intent::Conversion { from, to, .. }) = result {
            assert_eq!(from, Unit::Inch);
            assert_eq!(to, Unit::Centimeter);
        } else {
            panic!("Failed to parse with 'as' connector");
        }
    }

    #[test]
    fn test_parse_intent_without_connectors() {
        let caps = Capabilities(Capabilities::EVERYTHING);

        let tokens = tokenize("50m feet");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);

        if let Some(Intent::Conversion { value, from, to }) = result {
            assert_eq!(value, 50.0);
            assert_eq!(from, Unit::Meter);
            assert_eq!(to, Unit::Feet);
        } else {
            panic!("Failed to parse implicit unit conversion");
        }
    }

    #[test]
    fn test_parse_intent_failure_cases() {
        let caps = Capabilities(Capabilities::EVERYTHING);

        let tokens = tokenize("50m");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);
        assert!(result.is_none());

        let tokens = tokenize("hello world");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_intent_multi_word_target() {
        let caps = Capabilities(Capabilities::EVERYTHING);
        let tokens = tokenize("10 m to square meters");
        let cursor = Cursor::new(&tokens);
        let result = UnitParser::parse_intent(cursor, &caps);

        assert!(matches!(result, Some(Intent::Conversion { .. })));
    }
}

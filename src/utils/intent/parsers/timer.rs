use std::time::Duration;

use super::utils::parse_f64;
use crate::utils::intent::{
    Capabilities,
    Cursor,
    Intent,
    units::{Unit, UnitCategory},
};

pub struct TimerParser;
impl TimerParser {
    pub fn parse_intent(mut cur: Cursor<'_>) -> Option<Intent> {
        #[inline]
        fn make_timer(value: f64, unit: Unit) -> Option<Intent> {
            Some(Intent::Timer {
                duration: Duration::from_secs_f64(value * unit.factor()?),
            })
        }

        let time_caps = Capabilities(Capabilities::TIME);
        let mut value: Option<f64> = None;
        let mut unit: Option<Unit> = None;

        // Scan forward; stop at "timer" or end
        while let Some(t) = cur.peek() {
            if t.eq_ignore_ascii_case("timer") {
                cur.advance();
                break;
            }
            if let Some(v) = parse_f64(t) {
                value = Some(v);
            } else if let Some(u) = Unit::parse_with_capabilities(t, &time_caps)
                && u.category() == UnitCategory::Time
            {
                unit = Some(u);
            }
            cur.advance();
        }

        // If we already have both, we're done
        if let (Some(v), Some(u)) = (value, unit) {
            return make_timer(v, u);
        }

        // Skip optional "for"
        if cur
            .peek()
            .map(|t| t.eq_ignore_ascii_case("for"))
            .unwrap_or(false)
        {
            cur.advance();
        }

        let v = value.or_else(|| parse_f64(cur.advance()?))?;
        let u = {
            let t = cur.advance()?;
            Unit::parse_with_capabilities(t, &time_caps)
                .filter(|u| u.category() == UnitCategory::Time)?
        };

        make_timer(v, u)
    }
}

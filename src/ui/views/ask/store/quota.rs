use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Utc, Weekday};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ResetZone {
    /// Midnight in the user's local time.
    #[default]
    Local,
    /// Midnight America/Los_Angeles (Google's quotas).
    Pacific,
}

impl ResetZone {
    /// Start of the current quota day, as unix seconds.
    pub fn last_reset(self, now: DateTime<Utc>) -> i64 {
        match self {
            Self::Local => {
                let local = now.with_timezone(&Local);
                local
                    .date_naive()
                    .and_time(NaiveTime::MIN)
                    .and_local_timezone(Local)
                    .earliest()
                    .map_or(now.timestamp(), |t| t.timestamp())
            }
            Self::Pacific => {
                let date = (now + Duration::hours(pacific_offset_hours(now))).date_naive();
                pacific_midnight(date)
            }
        }
    }

    /// Start of the next quota day, as unix seconds.
    pub fn next_reset(self, now: DateTime<Utc>) -> i64 {
        match self {
            Self::Local => {
                let tomorrow = now.with_timezone(&Local).date_naive() + Duration::days(1);
                tomorrow
                    .and_time(NaiveTime::MIN)
                    .and_local_timezone(Local)
                    .earliest()
                    .map_or(now.timestamp() + 86_400, |t| t.timestamp())
            }
            Self::Pacific => {
                let date = (now + Duration::hours(pacific_offset_hours(now))).date_naive();
                pacific_midnight(date + Duration::days(1))
            }
        }
    }
}

/// `09:00` in the user's local time, for "resets at …".
pub fn local_clock(ts: i64) -> String {
    Local
        .timestamp_opt(ts, 0)
        .earliest()
        .map_or_else(String::new, |t| t.format("%H:%M").to_string())
}

/// UTC offset of US Pacific time at `t`: -7 during daylight saving, else -8.
/// DST runs from the 2nd Sunday of March 02:00 PST (10:00 UTC) to the 1st Sunday of November
/// 02:00 PDT (09:00 UTC).
fn pacific_offset_hours(t: DateTime<Utc>) -> i64 {
    let year = t.year();
    let start = nth_sunday(year, 3, 2)
        .and_hms_opt(10, 0, 0)
        .unwrap()
        .and_utc();
    let end = nth_sunday(year, 11, 1)
        .and_hms_opt(9, 0, 0)
        .unwrap()
        .and_utc();
    if t >= start && t < end { -7 } else { -8 }
}

/// Midnight of a Pacific calendar date, as unix seconds. (DST switches happen at 02:00, so
/// midnight's offset is the offset at noon of the same day.)
fn pacific_midnight(date: NaiveDate) -> i64 {
    let noon = date.and_hms_opt(12, 0, 0).unwrap().and_utc();
    let offset = pacific_offset_hours(noon);
    date.and_time(NaiveTime::MIN).and_utc().timestamp() - offset * 3600
}

fn nth_sunday(year: i32, month: u32, n: u32) -> NaiveDate {
    NaiveDate::from_weekday_of_month_opt(year, month, Weekday::Sun, n as u8).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(s: &str) -> DateTime<Utc> {
        s.parse().unwrap()
    }

    #[test]
    fn pacific_offsets() {
        // 2026: DST from Mar 8 to Nov 1.
        assert_eq!(pacific_offset_hours(utc("2026-07-01T00:00:00Z")), -7);
        assert_eq!(pacific_offset_hours(utc("2026-11-02T16:00:00Z")), -8);
        assert_eq!(pacific_offset_hours(utc("2026-03-08T09:59:00Z")), -8);
        assert_eq!(pacific_offset_hours(utc("2026-03-08T10:00:00Z")), -7);
    }

    #[test]
    fn pacific_reset_window() {
        // 2026-11-02 16:00 UTC = 08:00 PST, so the quota day began 2026-11-02 08:00 UTC.
        let now = utc("2026-11-02T16:00:00Z");
        assert_eq!(
            ResetZone::Pacific.last_reset(now),
            utc("2026-11-02T08:00:00Z").timestamp()
        );
        assert_eq!(
            ResetZone::Pacific.next_reset(now),
            utc("2026-11-03T08:00:00Z").timestamp()
        );
        // Summer: midnight PDT = 07:00 UTC.
        let summer = utc("2026-07-01T06:30:00Z"); // 23:30 PDT on Jun 30
        assert_eq!(
            ResetZone::Pacific.last_reset(summer),
            utc("2026-06-30T07:00:00Z").timestamp()
        );
    }

    #[test]
    fn local_window_is_a_day() {
        let now = Utc::now();
        let (a, b) = (
            ResetZone::Local.last_reset(now),
            ResetZone::Local.next_reset(now),
        );
        assert!(a <= now.timestamp() && now.timestamp() < b);
        assert!((82_800..=90_000).contains(&(b - a)), "23–25 h around DST");
    }
}

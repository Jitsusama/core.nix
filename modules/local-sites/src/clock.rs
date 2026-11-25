//! Times as the looks an answer is checked against write them: read from
//! milliseconds or an ISO time naming its zone, as a session writes one,
//! and written as the shell writes `at`, in UTC. The arithmetic of days is
//! Howard Hinnant's, so no crate is needed for it.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// When `value` falls, in milliseconds since the epoch: a whole number of
/// them, or an ISO time naming its zone, as a session writes one; else none.
pub fn when(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => parsed(text),
        _ => None,
    }
}

/// An ISO time, `2026-11-24T23:10:00.123Z` or with an offset, in
/// milliseconds since the epoch; none where it names no zone, which a
/// browser reads as its own and this server can't know.
fn parsed(text: &str) -> Option<i64> {
    let (date, rest) = text.split_once('T')?;
    let mut parts = date.split('-').map(str::parse::<i64>);
    let (year, month, day) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    if parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let (clock, offset) = if let Some(clock) = rest.strip_suffix(['Z', 'z']) {
        (clock, 0)
    } else {
        let at = rest.rfind(['+', '-'])?;
        let (clock, zone) = rest.split_at(at);
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let (hours, minutes) = zone.get(1..)?.split_once(':')?;
        let minutes = hours.parse::<i64>().ok()? * 60 + minutes.parse::<i64>().ok()?;
        (clock, sign * minutes)
    };
    let (whole, fraction) = clock.split_once('.').unwrap_or((clock, ""));
    let mut parts = whole.split(':').map(str::parse::<i64>);
    let (hour, minute, second) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    if parts.next().is_some() || !fraction.bytes().all(|digit| digit.is_ascii_digit()) {
        return None;
    }
    let millis: String = fraction
        .chars()
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect();
    let millis = millis.parse::<i64>().ok()?;
    let days = days_from_civil(year, month, day);
    let seconds = ((days * 24 + hour) * 60 + minute - offset) * 60 + second;
    Some(seconds * 1000 + millis)
}

/// The days from 1970-01-01 to a day of the proleptic Gregorian calendar,
/// by Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let shifted = if month > 2 { month - 3 } else { month + 9 };
    let of_year = (153 * shifted + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// The day `days` after 1970-01-01, as its year, month and day, by Howard
/// Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days - era * 146_097;
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted + 2) / 5 + 1;
    let month = if shifted < 10 {
        shifted + 3
    } else {
        shifted - 9
    };
    (year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

/// `millis` since the epoch as an ISO time in UTC, as the shell writes `at`.
pub fn iso(millis: i64) -> String {
    let (days, of_day) = (millis.div_euclid(86_400_000), millis.rem_euclid(86_400_000));
    let (year, month, day) = civil_from_days(days);
    let (hour, minute) = (of_day / 3_600_000, of_day / 60_000 % 60);
    let (second, milli) = (of_day / 1000 % 60, of_day % 1000);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{milli:03}Z")
}

/// Now, in milliseconds since the epoch.
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            i64::try_from(since.as_millis()).unwrap_or(i64::MAX)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 23:10 UTC on 24 November 2025, in milliseconds since the epoch.
    const NOW: i64 = 1_764_025_800_000;

    #[test]
    fn a_time_reads_as_a_browser_reads_it_and_is_written_as_the_shell_writes_it() {
        assert_eq!(parsed("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(parsed("2000-03-01T00:00:00.5Z"), Some(951_868_800_500));
        assert_eq!(parsed("2025-11-24T23:10:00.000Z"), Some(NOW));
        assert_eq!(
            parsed("2025-11-24T18:10:00-05:00"),
            Some(NOW),
            "an offset is taken off"
        );
        assert_eq!(parsed("2025-11-25T00:40:00+01:30"), Some(NOW));
        assert_eq!(iso(NOW), "2025-11-24T23:10:00.000Z");
        assert_eq!(iso(951_868_800_500), "2000-03-01T00:00:00.500Z");
        for day in [-800_000, -1, 0, 59, 11_016, 20_416, 2_000_000] {
            assert_eq!(days_from_civil_of(civil_from_days(day)), day, "day {day}");
        }
        for unread in [
            "2025-11-24",
            "2025-11-24T23:10:00",
            "2025-13-01T00:00:00Z",
            "x-y-zT00:00:00Z",
            "2025-11-24T23:10Z",
        ] {
            assert_eq!(parsed(unread), None, "{unread}");
        }
    }

    fn days_from_civil_of((year, month, day): (i64, i64, i64)) -> i64 {
        days_from_civil(year, month, day)
    }
}

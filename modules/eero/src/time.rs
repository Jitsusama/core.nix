//! Times as the model holds them: RFC 3339 in UTC, with a `Z`.
//!
//! eero writes most of its times so already, but some with `+0000`, which a
//! reader of RFC 3339 refuses, and its events as milliseconds since 1970.
//! No time crate is pulled in for this: the calendar arithmetic is a few
//! lines, and these are the only times the program reads.

/// A time eero gives, moved to UTC and ended with `Z`, its fraction of a
/// second kept as eero wrote it. One that doesn't read as a time is `None`
/// rather than passed on, since the model promises RFC 3339.
pub fn utc(stamp: &str) -> Option<String> {
    let (clock, offset_minutes) = split_offset(stamp)?;
    let (whole, fraction) = match clock.split_once('.') {
        Some((whole, fraction)) => (whole, Some(fraction)),
        None => (clock, None),
    };
    if fraction.is_some_and(|f| f.is_empty() || !f.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let seconds = unix_seconds(whole)? - offset_minutes * 60;
    Some(match fraction {
        Some(fraction) => format!("{}.{fraction}Z", calendar(seconds)),
        None => format!("{}Z", calendar(seconds)),
    })
}

/// Seconds since 1970 of a time eero gives, its fraction of a second
/// dropped, as the history keeps times.
pub fn to_unix(stamp: &str) -> Option<i64> {
    let (clock, offset_minutes) = split_offset(stamp)?;
    let whole = clock.split_once('.').map_or(clock, |(whole, _)| whole);
    Some(unix_seconds(whole)? - offset_minutes * 60)
}

/// A moment given as milliseconds since 1970, to the second, as eero's
/// events give it.
pub fn from_unix_ms(ms: i64) -> String {
    from_unix(ms.div_euclid(1000))
}

/// A moment given as seconds since 1970.
pub fn from_unix(seconds: i64) -> String {
    format!("{}Z", calendar(seconds))
}

/// The clock part of a stamp and its offset from UTC in minutes.
fn split_offset(stamp: &str) -> Option<(&str, i64)> {
    if let Some(clock) = stamp.strip_suffix('Z') {
        return Some((clock, 0));
    }
    // The offset's sign comes after the seconds, so a date's own dashes
    // are never taken for it.
    let at = stamp.rfind(['+', '-']).filter(|&at| at >= 19)?;
    let (clock, offset) = stamp.split_at(at);
    let sign = if offset.starts_with('-') { -1 } else { 1 };
    let digits: String = offset[1..].chars().filter(|c| *c != ':').collect();
    if digits.len() != 4 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let hours: i64 = digits[..2].parse().ok()?;
    let minutes: i64 = digits[2..].parse().ok()?;
    Some((clock, sign * (hours * 60 + minutes)))
}

/// Seconds since 1970 of `YYYY-MM-DDTHH:MM:SS`, read as UTC.
fn unix_seconds(clock: &str) -> Option<i64> {
    let bytes = clock.as_bytes();
    let shaped = bytes.len() == 19
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes[10] == b'T'
        && bytes[13] == b':'
        && bytes[16] == b':';
    if !shaped {
        return None;
    }
    let part = |from: usize, to: usize| -> Option<i64> {
        let digits = clock.get(from..to)?;
        digits
            .bytes()
            .all(|b| b.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let (year, month, day) = (part(0, 4)?, part(5, 7)?, part(8, 10)?);
    let (hour, minute, second) = (part(11, 13)?, part(14, 16)?, part(17, 19)?);
    let sane = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    sane.then(|| days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

/// `YYYY-MM-DDTHH:MM:SS` of seconds since 1970, in UTC.
fn calendar(seconds: i64) -> String {
    let (days, of_day) = (seconds.div_euclid(86_400), seconds.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        of_day / 3600,
        of_day % 3600 / 60,
        of_day % 60
    )
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar,
/// after Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year.rem_euclid(400);
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// The date of a day since 1970-01-01, after Hinnant's `civil_from_days`.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let of_era = days.rem_euclid(146_097);
    let of_cycle = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * of_cycle + of_cycle / 4 - of_cycle / 100);
    let month_index = (5 * of_year + 2) / 153;
    let day = of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = of_cycle + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_in_utc_already_is_kept_with_its_fraction() {
        assert_eq!(
            utc("2026-11-07T07:22:42.035551983Z").as_deref(),
            Some("2026-11-07T07:22:42.035551983Z")
        );
    }

    #[test]
    fn eeros_zero_offset_becomes_z() {
        assert_eq!(
            utc("2026-11-05T10:52:15+0000").as_deref(),
            Some("2026-11-05T10:52:15Z")
        );
    }

    #[test]
    fn another_offset_moves_the_time_to_utc_across_a_day() {
        assert_eq!(
            utc("2026-12-31T21:30:00-05:00").as_deref(),
            Some("2027-01-01T02:30:00Z")
        );
        assert_eq!(
            utc("2024-03-01T01:00:00.5+0200").as_deref(),
            Some("2024-02-29T23:00:00.5Z")
        );
    }

    #[test]
    fn what_is_not_a_time_reads_as_none() {
        for stamp in [
            "",
            "yesterday",
            "2026-11-05",
            "2026-13-05T10:52:15Z",
            "2026-11-05T10:52:15+00",
        ] {
            assert_eq!(utc(stamp), None, "{stamp}");
        }
    }

    #[test]
    fn milliseconds_since_1970_read_to_the_second() {
        assert_eq!(from_unix_ms(1_794_091_355_077), "2026-11-07T22:42:35Z");
        assert_eq!(from_unix(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn a_time_reads_back_to_seconds_since_1970() {
        assert_eq!(to_unix("2026-11-07T22:42:35Z"), Some(1_794_091_355));
        assert_eq!(to_unix("2026-11-07T22:42:35.977Z"), Some(1_794_091_355));
        assert_eq!(to_unix("2026-11-07T23:42:35+0100"), Some(1_794_091_355));
        assert_eq!(to_unix("yesterday"), None);
    }
}

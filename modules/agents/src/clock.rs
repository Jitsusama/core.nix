//! Times as a drawer sends them, and today's date. Nothing that reads the
//! clock is worded here, or every drawer would change each minute: a time
//! goes out as milliseconds since 1970 and the shell words it (Y2YBTS's
//! ready rows, "Times"). Only dates are said here, today's and a landing's,
//! since the harness starts a session's figures over each day and dates
//! them by it, and Landed counts what landed each day.

use serde_json::Value;

/// Milliseconds since 1970 of a time the harness writes: a count of
/// milliseconds, as a session's `startedAt`, or RFC 3339 text in UTC or
/// with an offset, as a landing's `at`. None for anything else.
pub fn ms_of(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| value.as_str().and_then(parsed))
}

/// RFC 3339 text, `2026-11-10T22:41:32.003Z`, as milliseconds since 1970.
fn parsed(text: &str) -> Option<i64> {
    let bytes = text.as_bytes();
    let number = |from: usize, to: usize| -> Option<i64> {
        let digits = text.get(from..to)?;
        digits
            .bytes()
            .all(|byte| byte.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let apart = bytes.get(4) == Some(&b'-')
        && bytes.get(7) == Some(&b'-')
        && matches!(bytes.get(10), Some(b'T' | b't' | b' '))
        && bytes.get(13) == Some(&b':')
        && bytes.get(16) == Some(&b':');
    if !apart {
        return None;
    }
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    let sane = (1..=12).contains(&month)
        && (1..=31).contains(&day)
        && hour < 24
        && minute < 60
        && second <= 60;
    if !sane {
        return None;
    }
    let mut rest = text.get(19..)?;
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction.bytes().take_while(u8::is_ascii_digit).count();
        if digits == 0 {
            return None;
        }
        let first: String = fraction[..digits]
            .chars()
            .chain("00".chars())
            .take(3)
            .collect();
        millis = first.parse::<i64>().ok()?;
        rest = &fraction[digits..];
    }
    let offset = match rest {
        "Z" | "z" => 0,
        _ => {
            let sign = match rest.as_bytes().first() {
                Some(b'+') => 1,
                Some(b'-') => -1,
                _ => return None,
            };
            let hours = rest.get(1..3)?.parse::<i64>().ok()?;
            let minutes = rest.get(4..6)?.parse::<i64>().ok()?;
            if rest.len() != 6 || rest.as_bytes()[3] != b':' {
                return None;
            }
            sign * (hours * 60 + minutes)
        }
    };
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second
        - offset * 60;
    Some(seconds * 1000 + millis)
}

/// Days since 1970-01-01 of a date in the proleptic Gregorian calendar,
/// after Howard Hinnant's `days_from_civil`, as modules/eero counts them.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year.rem_euclid(400);
    let of_year = (153 * ((month + 9) % 12) + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_cycle - 719_468
}

/// Today's date where Joel is, `2026-11-10`, through the C library as
/// modules/disk-guard reads the local time; none where it can't say.
pub fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_millis());
    date_of(i64::try_from(now).unwrap_or(0))
}

/// The date where Joel is of `ms` since 1970, `2026-11-10`; none where it
/// can't say.
pub fn date_of(ms: i64) -> String {
    let seconds = libc::time_t::try_from(ms.div_euclid(1000)).unwrap_or(0);
    // SAFETY: tm is plain data, zeroed is a valid value, and localtime_r
    // writes only the struct it's given.
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: as above; seconds lives for the call.
    if unsafe { libc::localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return String::new();
    }
    format!(
        "{:04}-{:02}-{:02}",
        tm.tm_year + 1900,
        tm.tm_mon + 1,
        tm.tm_mday
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn a_count_of_milliseconds_is_taken_as_it_is() {
        assert_eq!(
            ms_of(&json!(1_794_352_932_300_i64)),
            Some(1_794_352_932_300)
        );
    }

    #[test]
    fn rfc_3339_text_reads_in_utc_or_with_its_offset() {
        assert_eq!(ms_of(&json!("1970-01-01T00:00:00Z")), Some(0));
        assert_eq!(
            ms_of(&json!("2026-11-10T22:41:32.003Z")),
            Some(1_794_350_492_003)
        );
        assert_eq!(
            ms_of(&json!("2026-11-10T17:41:32.003-05:00")),
            ms_of(&json!("2026-11-10T22:41:32.003Z"))
        );
        assert_eq!(
            ms_of(&json!("2026-11-10T22:41:32.5Z")),
            Some(1_794_350_492_500)
        );
    }

    #[test]
    fn anything_else_is_no_time() {
        for said in [
            "",
            "yesterday",
            "2026-11-10",
            "2026-13-10T00:00:00Z",
            "2026-11-10T00:00:00",
        ] {
            assert_eq!(ms_of(&json!(said)), None, "{said} is no time");
        }
        assert_eq!(ms_of(&json!(null)), None, "nothing is no time");
    }

    #[test]
    fn today_is_a_date() {
        let today = today();
        assert_eq!(today.len(), 10, "{today} reads as a date");
        assert_eq!(&today[4..5], "-", "{today} reads as a date");
        // Midday UTC is the same day on every clock within twelve hours of it.
        let noon = ms_of(&json!("2026-11-10T12:00:00Z")).expect("a time");
        assert_eq!(date_of(noon), "2026-11-10", "a moment's own date");
    }
}

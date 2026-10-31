//! Moments and lengths as they're typed, in the machine's own time.

use jiff::civil::{DateTime, Time};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};

/// Unix nanoseconds for a moment: `now`, a time of day today (`09:01`,
/// `09:01:57`, `09:01:57.7`) or a date with it (`2026-10-31 09:01:57`).
///
/// # Errors
///
/// When it's none of those.
pub fn moment(text: &str, zone: &TimeZone, now: Timestamp) -> Result<i64, String> {
    let unreadable = || format!("{text:?} isn't now, a time like 09:01:57 or a date and time");
    // A date and time is tried first, since jiff reads a time of day out of
    // one too and would drop its date.
    let zoned = if text == "now" {
        now.to_zoned(zone.clone())
    } else if let Ok(datetime) = text.replacen(' ', "T", 1).parse::<DateTime>() {
        datetime.to_zoned(zone.clone()).map_err(|_| unreadable())?
    } else {
        let time = text.parse::<Time>().map_err(|_| unreadable())?;
        now.to_zoned(zone.clone())
            .date()
            .to_datetime(time)
            .to_zoned(zone.clone())
            .map_err(|_| unreadable())?
    };
    nanos(&zoned).ok_or_else(unreadable)
}

fn nanos(zoned: &Zoned) -> Option<i64> {
    i64::try_from(zoned.timestamp().as_nanosecond()).ok()
}

/// Nanoseconds for a length: `500ms`, `30s`, `5m` or `1h`.
///
/// # Errors
///
/// When it's a number with no unit, or none of those.
pub fn length(text: &str) -> Result<i64, String> {
    let split = text
        .find(|c: char| !c.is_ascii_digit())
        .ok_or_else(|| format!("{text:?} needs a unit: ms, s, m or h"))?;
    let (number, unit) = text.split_at(split);
    let number: i64 = number
        .parse()
        .map_err(|_| format!("{text:?} isn't a length"))?;
    let unit = match unit {
        "ms" => 1_000_000,
        "s" => 1_000_000_000,
        "m" => 60 * 1_000_000_000,
        "h" => 3_600 * 1_000_000_000,
        _ => return Err(format!("{text:?} needs a unit: ms, s, m or h")),
    };
    number
        .checked_mul(unit)
        .ok_or_else(|| format!("{text:?} is too long"))
}

/// A moment as the machine's clock showed it, to the millisecond.
pub fn shown(nanos: i64, zone: &TimeZone) -> String {
    let Ok(at) = Timestamp::from_nanosecond(i128::from(nanos)) else {
        return nanos.to_string();
    };
    let zoned = at.to_zoned(zone.clone());
    format!(
        "{} {}.{:03}",
        zoned.date(),
        zoned.time().strftime("%H:%M:%S"),
        zoned.time().subsec_nanosecond() / 1_000_000
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toronto() -> TimeZone {
        TimeZone::get("America/Toronto").unwrap()
    }

    // 2026-10-31 12:00:00 in Toronto, which is 16:00 UTC.
    fn noon() -> Timestamp {
        "2026-10-31T16:00:00Z".parse().unwrap()
    }

    #[test]
    fn a_time_of_day_is_today_in_the_machines_zone() {
        let at = moment("09:01:57.7", &toronto(), noon()).unwrap();
        assert_eq!(shown(at, &toronto()), "2026-10-31 09:01:57.700");
        let utc = Timestamp::from_nanosecond(i128::from(at)).unwrap();
        assert_eq!(utc.to_string(), "2026-10-31T13:01:57.7Z");
    }

    #[test]
    fn a_date_and_time_and_now_are_read_too() {
        let at = moment("2026-10-30 23:59", &toronto(), noon()).unwrap();
        assert_eq!(shown(at, &toronto()), "2026-10-30 23:59:00.000");
        let now = moment("now", &toronto(), noon()).unwrap();
        assert_eq!(i128::from(now), noon().as_nanosecond());
        assert!(
            moment("teatime", &toronto(), noon())
                .unwrap_err()
                .contains("teatime")
        );
    }

    #[test]
    fn a_length_needs_its_unit() {
        assert_eq!(length("500ms").unwrap(), 500_000_000);
        assert_eq!(length("30s").unwrap(), 30_000_000_000);
        assert_eq!(length("5m").unwrap(), 300_000_000_000);
        assert!(length("30").unwrap_err().contains("unit"));
        assert!(length("3d").unwrap_err().contains("unit"));
    }
}

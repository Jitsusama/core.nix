//! Times as net keeps them: seconds since the epoch, read from the forms
//! its sources write.

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;

/// Now, in seconds since the epoch.
pub fn now() -> u64 {
    seconds(Timestamp::now())
}

/// An instant as eero writes one, `2026-11-07T22:54:43Z`.
pub fn parse(said: &str) -> Option<u64> {
    said.parse::<Timestamp>().ok().map(seconds)
}

/// A device's clock reading, in the form given, as if its clock kept UTC;
/// a clock that doesn't is put right by the offset `clock_offset` finds.
pub fn civil(said: &str, form: &str) -> Option<i64> {
    let civil = DateTime::strptime(form, said.trim()).ok()?;
    Some(civil.to_zoned(TimeZone::UTC).ok()?.timestamp().as_second())
}

/// How far a device's clock, read at `now`, sits from UTC, to the nearest
/// quarter hour, since a zone is whole quarter hours from UTC and the
/// read's own delay is seconds.
pub fn clock_offset(clock: i64, now: u64) -> Option<i64> {
    const QUARTER: i64 = 15 * 60;
    let now = i64::try_from(now).ok()?;
    let off = clock - now;
    Some((off + off.signum() * QUARTER / 2) / QUARTER * QUARTER)
}

fn seconds(instant: Timestamp) -> u64 {
    u64::try_from(instant.as_second()).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_eero_instant_reads_as_seconds_since_the_epoch() {
        assert_eq!(
            parse("2026-11-07T22:54:43Z"),
            Some(1_794_092_083),
            "eero's instant misread"
        );
    }

    #[test]
    fn a_clock_a_few_seconds_off_keeps_utc() {
        assert_eq!(
            clock_offset(1_000_000 + 7, 1_000_000),
            Some(0),
            "a slow read moved the zone"
        );
    }

    #[test]
    fn a_clock_four_hours_behind_is_put_right() {
        let behind = 1_000_000 - 4 * 3600 + 20;
        assert_eq!(
            clock_offset(behind, 1_000_000),
            Some(-4 * 3600),
            "the zone wasn't found"
        );
    }

    #[test]
    fn a_modem_clock_reads_in_its_own_form() {
        assert_eq!(
            civil("Sun Nov 15, 2026, 00:20:28", "%a %b %d, %Y, %H:%M:%S"),
            Some(1_794_702_028),
            "the modem's clock misread"
        );
    }
}

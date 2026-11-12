//! The local clock, by glibc's own reckoning of the zone, as the
//! backlight's evening reads it (modules/backlight/src/evening.rs), and
//! the civil calendar's days, for the dates pi writes and the sun's days.

/// A moment by the local clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    /// Sunday 0 to Saturday 6.
    pub weekday: u32,
    /// Minutes past midnight.
    pub minute: u32,
}

/// glibc's broken-down time.
#[repr(C)]
#[derive(Default)]
struct Tm {
    sec: i32,
    min: i32,
    hour: i32,
    mday: i32,
    mon: i32,
    year: i32,
    wday: i32,
    yday: i32,
    isdst: i32,
    gmtoff: i64,
    zone: usize,
}

unsafe extern "C" {
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
}

/// The local clock at `seconds` since 1970 began in UTC, or none where
/// glibc can't say it.
pub fn local(seconds: i64) -> Option<Local> {
    let mut tm = Tm::default();
    // SAFETY: both pointers are to values alive across the call.
    if unsafe { localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return None;
    }
    Some(Local {
        year: tm.year + 1900,
        month: u32::try_from(tm.mon + 1).ok()?,
        day: u32::try_from(tm.mday).ok()?,
        weekday: u32::try_from(tm.wday).ok()?,
        minute: u32::try_from(tm.hour * 60 + tm.min).ok()?,
    })
}

/// Days since 1970-01-01 for a civil date, by Howard Hinnant's reckoning.
pub fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let month = i64::from(month);
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_era_days - 719_468
}

/// The civil date of a day counted from 1970-01-01, the other way round,
/// for the tests' clock.
#[cfg(test)]
pub fn civil_from_days(days: i64) -> (i32, u32, u32) {
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
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    // Every count here is far inside each type, for any day a log holds.
    (
        i32::try_from(year).unwrap_or(i32::MAX),
        u32::try_from(month).unwrap_or(1),
        u32::try_from(day).unwrap_or(1),
    )
}

/// Seconds since 1970 for a UTC moment written as pi writes it,
/// `2026-11-09T20:01:55.906Z`, read to the second.
pub fn iso(text: &str) -> Option<i64> {
    let number = |range: std::ops::Range<usize>| text.get(range)?.parse::<i64>().ok();
    let year = i32::try_from(number(0..4)?).ok()?;
    let month = u32::try_from(number(5..7)?).ok()?;
    let day = u32::try_from(number(8..10)?).ok()?;
    let (hour, minute, second) = (number(11..13)?, number(14..16)?, number(17..19)?);
    let shaped = text.get(4..5) == Some("-") && text.get(10..11) == Some("T");
    (shaped && (1..=12).contains(&month) && (1..=31).contains(&day) && hour < 24)
        .then(|| days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second)
}

/// A clock that's UTC moved by `offset` minutes, for the tests, which
/// can't lean on the zone the machine running them is in.
#[cfg(test)]
pub fn fixed(offset: i64) -> impl Fn(i64) -> Option<Local> {
    move |seconds| {
        let shifted = seconds + offset * 60;
        let days = shifted.div_euclid(86_400);
        let (year, month, day) = civil_from_days(days);
        Some(Local {
            year,
            month,
            day,
            // 1970-01-01 was a Thursday.
            weekday: u32::try_from((days + 4).rem_euclid(7)).ok()?,
            minute: u32::try_from(shifted.rem_euclid(86_400) / 60).ok()?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_date_counts_its_days_both_ways() {
        assert_eq!(days_from_civil(1970, 1, 1), 0, "the epoch is day nought");
        assert_eq!(days_from_civil(2026, 11, 12), 20_769, "12 November 2026");
        assert_eq!(civil_from_days(20_769), (2026, 11, 12), "and back");
        assert_eq!(
            civil_from_days(-1),
            (1969, 12, 31),
            "the day before the epoch"
        );
    }

    #[test]
    fn pi_s_moments_read_to_the_second() {
        assert_eq!(
            iso("2026-11-09T20:01:55.906Z"),
            Some(20_766 * 86_400 + 20 * 3600 + 60 + 55),
            "a moment pi wrote"
        );
        assert_eq!(iso("2026-13-09T20:01:55Z"), None, "no thirteenth month");
        assert_eq!(iso("yesterday"), None, "not a moment");
    }

    #[test]
    fn a_fixed_clock_says_the_weekday_and_minute() {
        let toronto = fixed(-300);
        // 03:30 UTC on Thursday 12 November is 22:30 on the Wednesday.
        let said = toronto(20_769 * 86_400 + 3 * 3600 + 1800).expect("a moment");
        assert_eq!(
            (said.day, said.weekday, said.minute),
            (11, 3, 22 * 60 + 30),
            "Wednesday 22:30 in Toronto"
        );
    }

    #[test]
    fn the_local_clock_reads_a_moment() {
        let said = local(0).expect("the epoch reads locally");
        assert!(
            said.minute < 1440,
            "{} isn't a minute of a day",
            said.minute
        );
        assert!(said.weekday < 7, "{} isn't a weekday", said.weekday);
    }
}

//! A moment as a calendar and a clock read it: the machine's own, through
//! glibc's local time, for what a call's drawer says over its title, and
//! the world's, for the cases, which read the same wherever they run.
//! Nothing here reads the clock's now: a time that changes as the minute
//! does travels as the ready-row form's `{at, as}`, for the shell to word.

/// A moment's date and time of day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct When {
    pub year: i32,
    /// 1 for January.
    pub month: u32,
    pub day: u32,
    /// 0 for Sunday.
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
}

const DAYS: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

impl When {
    /// "14:00", as the launcher's clock says a time.
    pub fn clock(&self) -> String {
        format!("{:02}:{:02}", self.hour, self.minute)
    }

    /// "Tuesday 3 November · 14:00", the year only where it isn't
    /// `this_year`, as a notice's eyebrow says when it came.
    pub fn dated(&self, this_year: i32) -> String {
        let day = DAYS[self.weekday as usize % 7];
        let month = MONTHS[(self.month as usize + 11) % 12];
        let year = if self.year == this_year {
            String::new()
        } else {
            format!(" {}", self.year)
        };
        format!("{day} {} {month}{year} \u{b7} {}", self.day, self.clock())
    }
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

/// A moment in milliseconds since 1970 as the machine's local time reads
/// it, or the world's where glibc can't say.
pub fn local(ms: i64) -> When {
    let seconds = ms.div_euclid(1000);
    let mut tm = Tm::default();
    // SAFETY: both pointers are to values alive across the call.
    if unsafe { localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return utc(ms);
    }
    let whole = |value: i32| u32::try_from(value).unwrap_or(0);
    When {
        year: tm.year + 1900,
        month: whole(tm.mon + 1),
        day: whole(tm.mday),
        weekday: whole(tm.wday),
        hour: whole(tm.hour),
        minute: whole(tm.min),
    }
}

/// A moment in milliseconds since 1970 as the world's time reads it, by
/// the proleptic Gregorian calendar (Howard Hinnant's days from civil,
/// run backwards).
pub fn utc(ms: i64) -> When {
    let minutes = ms.div_euclid(60_000);
    let days = minutes.div_euclid(1440);
    let of_day = minutes.rem_euclid(1440);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * of_year + 2) / 153;
    let day = of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    let narrow = |value: i64| u32::try_from(value).unwrap_or(0);
    When {
        year: i32::try_from(year).unwrap_or(i32::MAX),
        month: narrow(month),
        day: narrow(day),
        // 1 January 1970 was a Thursday.
        weekday: narrow((days + 4).rem_euclid(7)),
        hour: narrow(of_day / 60),
        minute: narrow(of_day % 60),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_moment_reads_as_the_worlds_calendar_and_clock() {
        // 3 November 2026, 14:05 in the world's time.
        let when = utc(1_793_714_700_000);
        assert_eq!(
            when,
            When {
                year: 2026,
                month: 11,
                day: 3,
                weekday: 2,
                hour: 14,
                minute: 5
            },
            "the moment as a calendar reads it"
        );
        assert_eq!(when.clock(), "14:05", "the time of day");
        assert_eq!(
            when.dated(2026),
            "Tuesday 3 November \u{b7} 14:05",
            "a day of this year, with no year"
        );
        assert_eq!(
            utc(1_767_171_600_000).dated(2026),
            "Wednesday 31 December 2025 \u{b7} 09:00",
            "a day of another year, with its year"
        );
        assert_eq!(
            utc(0).dated(1970),
            "Thursday 1 January \u{b7} 00:00",
            "the epoch itself"
        );
    }
}

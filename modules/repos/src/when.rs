//! Moments as a person reads them, as design.md's When writes them: today
//! its bare time ("15:20"), then "yesterday 23:40", a weekday within six
//! days ("Wed"), then its date ("28 Oct"), and forward the same ("tomorrow
//! 00:10"), all in the machine's own time zone, which glibc reads. A view
//! takes a `Clock` so its tests read in UTC, the same wherever they run.
//! Times kept or answered stay seconds since 1970, said in RFC 3339.

/// A moment's date and time of day.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct When {
    pub year: i64,
    /// 1 for January.
    pub month: i64,
    pub day: i64,
    /// 0 for Sunday.
    pub weekday: usize,
    pub hour: i64,
    pub minute: i64,
}

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// The time now, and how a moment reads where the person is.
#[derive(Clone, Copy)]
pub struct Clock {
    /// Seconds since 1970.
    pub now: u64,
    pub local: fn(u64) -> When,
}

impl Clock {
    /// The clock of the machine the command runs on.
    pub fn here(now: u64) -> Self {
        Self { now, local }
    }

    /// A moment as When writes it, bare: "10:21", "yesterday 23:40",
    /// "Wed", "3 Nov", "3 Nov 2025".
    pub fn moment(&self, then: u64) -> String {
        let (then, now) = ((self.local)(then), (self.local)(self.now));
        let clock = format!("{:02}:{:02}", then.hour, then.minute);
        match day_number(then) - day_number(now) {
            0 => clock,
            -1 => format!("yesterday {clock}"),
            1 => format!("tomorrow {clock}"),
            -6..=6 => DAYS[then.weekday % 7].to_owned(),
            _ => {
                let month = MONTHS[usize::try_from(then.month - 1).unwrap_or(0) % 12];
                if then.year == now.year {
                    format!("{} {month}", then.day)
                } else {
                    format!("{} {month} {}", then.day, then.year)
                }
            }
        }
    }

    /// A moment as a phrase says it, "at" before a bare time alone:
    /// "written at 10:21", "written yesterday 23:40".
    pub fn at(&self, then: u64) -> String {
        let moment = self.moment(then);
        if moment.starts_with(|c: char| c.is_ascii_digit()) && moment.contains(':') {
            format!("at {moment}")
        } else {
            moment
        }
    }
}

fn day_number(when: When) -> i64 {
    days_from_civil(when.year, when.month, when.day)
}

/// glibc's broken-down time, as `localtime_r` fills it on Linux and macOS.
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

/// A moment as the machine's own time zone reads it, or the world's where
/// the C library can't say.
pub fn local(seconds: u64) -> When {
    let at = i64::try_from(seconds).unwrap_or(i64::MAX);
    let mut tm = Tm::default();
    // SAFETY: both pointers are to values alive across the call, and Tm
    // is laid out as the C library's struct tm.
    if unsafe { localtime_r(&raw const at, &raw mut tm) }.is_null() {
        return utc(seconds);
    }
    When {
        year: i64::from(tm.year) + 1900,
        month: i64::from(tm.mon) + 1,
        day: i64::from(tm.mday),
        weekday: usize::try_from(tm.wday).unwrap_or(0),
        hour: i64::from(tm.hour),
        minute: i64::from(tm.min),
    }
}

/// A moment as the world's time reads it (Howard Hinnant's civil from
/// days).
pub fn utc(seconds: u64) -> When {
    let seconds = i64::try_from(seconds).unwrap_or(i64::MAX);
    let days = seconds.div_euclid(86_400);
    let of_day = seconds.rem_euclid(86_400);
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let index = (5 * of_year + 2) / 153;
    let month = if index < 10 { index + 3 } else { index - 9 };
    When {
        year: year_of_era + era * 400 + i64::from(month <= 2),
        month,
        day: of_year - (153 * index + 2) / 5 + 1,
        // 1 January 1970 was a Thursday.
        weekday: usize::try_from((days + 4).rem_euclid(7)).unwrap_or(0),
        hour: of_day / 3600,
        minute: of_day % 3600 / 60,
    }
}

/// The days from 1970 to a date (Howard Hinnant's algorithm, counting
/// years from March so a leap day falls last).
pub fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let of_era_days = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146_097 + of_era_days - 719_468
}

/// The seconds since 1970 of a time as GitHub says one,
/// `2026-11-15T15:10:03Z`, always in UTC; none for any other shape.
pub fn seconds_of(time: &str) -> Option<u64> {
    let time = time.strip_suffix('Z')?;
    let (date, clock) = time.split_once('T')?;
    let mut date = date.splitn(3, '-').map(str::parse::<i64>);
    let (year, month, day) = (date.next()?.ok()?, date.next()?.ok()?, date.next()?.ok()?);
    let clock = clock.split_once('.').map_or(clock, |(whole, _)| whole);
    let mut clock = clock.splitn(3, ':').map(str::parse::<i64>);
    let (hour, minute, second) = (
        clock.next()?.ok()?,
        clock.next()?.ok()?,
        clock.next()?.ok()?,
    );
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let seconds = days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from(seconds).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sunday 15 November 2026, 15:10:03 in UTC.
    const SUNDAY: u64 = 1_794_755_403;
    const DAY: u64 = 86_400;

    fn clock() -> Clock {
        Clock {
            now: SUNDAY,
            local: utc,
        }
    }

    #[test]
    fn a_moment_reads_by_how_far_its_day_is_from_today() {
        let clock = clock();
        assert_eq!(clock.moment(SUNDAY - 49 * 60), "14:21");
        assert_eq!(clock.moment(SUNDAY - 15 * 3600 - 10 * 60), "00:00");
        assert_eq!(
            clock.moment(SUNDAY - 15 * 3600 - 11 * 60),
            "yesterday 23:59"
        );
        assert_eq!(clock.moment(SUNDAY - 4 * DAY), "Wed");
        assert_eq!(clock.moment(SUNDAY - 6 * DAY), "Mon");
        assert_eq!(clock.moment(SUNDAY - 7 * DAY), "8 Nov");
        assert_eq!(clock.moment(SUNDAY - 365 * DAY), "15 Nov 2025");
        assert_eq!(clock.moment(SUNDAY + DAY), "tomorrow 15:10");
    }

    #[test]
    fn at_goes_only_before_a_bare_time() {
        let clock = clock();
        assert_eq!(clock.at(SUNDAY), "at 15:10");
        assert_eq!(clock.at(SUNDAY - DAY), "yesterday 15:10");
        assert_eq!(clock.at(SUNDAY - 4 * DAY), "Wed");
    }

    #[test]
    fn the_world_s_calendar_holds_across_leap_days_and_eras() {
        for (seconds, said) in [
            (0, (1970, 1, 1, 4)),
            (951_782_400, (2000, 2, 29, 2)),
            (SUNDAY, (2026, 11, 15, 0)),
        ] {
            let when = utc(seconds);
            assert_eq!(
                (when.year, when.month, when.day, when.weekday),
                said,
                "{seconds}"
            );
            assert_eq!(days_from_civil(when.year, when.month, when.day) * 86_400, {
                i64::try_from(seconds - seconds % DAY).unwrap_or(0)
            });
        }
    }

    #[test]
    fn the_machine_s_clock_reads_a_moment_within_a_day_of_the_world_s() {
        let (here, world) = (local(SUNDAY), utc(SUNDAY));
        let apart = days_from_civil(here.year, here.month, here.day)
            - days_from_civil(world.year, world.month, world.day);
        assert!(apart.abs() <= 1, "{here:?} against {world:?}");
    }

    #[test]
    fn github_s_times_read_as_seconds() {
        assert_eq!(seconds_of("2026-11-15T15:10:03Z"), Some(SUNDAY));
        assert_eq!(seconds_of("2024-02-29T23:59:59Z"), Some(1_709_251_199));
        assert_eq!(seconds_of("1970-01-01T00:00:00.250Z"), Some(0));
        assert_eq!(seconds_of("2026-11-15T15:10:03+01:00"), None);
        assert_eq!(seconds_of("2026-13-01T00:00:00Z"), None);
        assert_eq!(seconds_of("yesterday"), None);
    }
}

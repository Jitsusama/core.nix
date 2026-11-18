//! The time where the machine is: a moment as its own zone reads it, and
//! a local date and time back to a moment, through the C library's
//! `localtime_r` and `mktime`, so the zone and its daylight saving are read
//! as everything else on the machine reads them. One copy here serves every
//! command that says a local time, rather than each declaring the C
//! library's `struct tm` for itself. It's declared here, not taken from the
//! `libc` crate, as the kit depends on `serde_json` and `unicode-width`
//! alone; glibc and macOS lay `struct tm` out alike on 64-bit machines,
//! by their headers, though the kit's own tests run on Linux alone.

use std::ffi::{c_char, c_int, c_long};
use std::time::{SystemTime, UNIX_EPOCH};

/// A moment as the machine's clock reads it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Local {
    pub year: i32,
    /// 1 for January to 12 for December.
    pub month: u32,
    /// The day of the month, from 1.
    pub day: u32,
    /// 0 for Sunday to 6 for Saturday.
    pub weekday: u32,
    pub hour: u32,
    pub minute: u32,
    /// 0 to 60, since a leap second reads as the 60th.
    pub second: u32,
}

impl Local {
    /// Its date, as `2026-11-17`.
    pub fn date(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }

    /// Minutes past its midnight, 0 to 1439.
    pub fn minute_of_day(&self) -> u32 {
        self.hour * 60 + self.minute
    }
}

/// The C library's `struct tm`, as glibc and macOS lay it out.
#[repr(C)]
struct Tm {
    second: c_int,
    minute: c_int,
    hour: c_int,
    day: c_int,
    month: c_int,
    year: c_int,
    weekday: c_int,
    yearday: c_int,
    summer: c_int,
    offset: c_long,
    zone: *const c_char,
}

impl Tm {
    fn zeroed() -> Self {
        Self {
            second: 0,
            minute: 0,
            hour: 0,
            day: 0,
            month: 0,
            year: 0,
            weekday: 0,
            yearday: 0,
            summer: 0,
            offset: 0,
            zone: std::ptr::null(),
        }
    }
}

// time_t is 64 bits and struct tm nine ints, a long and a pointer, on the
// machines this builds for; a target where they aren't stops the build here
// rather than having the C library write past the struct.
const _: () = assert!(
    size_of::<c_long>() == 8,
    "struct tm's offset is a 64-bit long"
);
const _: () = assert!(
    size_of::<Tm>() == 56,
    "struct tm is nine ints, a long and a pointer"
);

unsafe extern "C" {
    fn localtime_r(time: *const i64, out: *mut Tm) -> *mut Tm;
    fn mktime(time: *mut Tm) -> i64;
    fn tzset();
}

/// The moment `seconds` after 1970 began as the machine's zone reads it;
/// none where the C library can't place it. The zone is read again each
/// time, so a daemon follows the machine to another.
pub fn at(seconds: i64) -> Option<Local> {
    let mut tm = Tm::zeroed();
    // glibc's localtime_r reads the zone only the first time, where mktime
    // reads it every time; tzset reads it again if it has changed.
    // SAFETY: tzset takes nothing and only reads the zone.
    unsafe { tzset() };
    // SAFETY: both pointers are to values alive across the call, and Tm is
    // laid out as the C library's struct tm.
    if unsafe { localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return None;
    }
    let whole = |value: c_int| u32::try_from(value).ok();
    Some(Local {
        year: tm.year.checked_add(1900)?,
        month: whole(tm.month)? + 1,
        day: whole(tm.day)?,
        weekday: whole(tm.weekday)?,
        hour: whole(tm.hour)?,
        minute: whole(tm.minute)?,
        second: whole(tm.second)?,
    })
}

/// Now, as the machine's zone reads it; none where the clock is before
/// 1970 or the C library can't place it.
pub fn now() -> Option<Local> {
    let since = SystemTime::now().duration_since(UNIX_EPOCH).ok()?;
    at(i64::try_from(since.as_secs()).ok()?)
}

/// The moment a local date and time names, in seconds since 1970 began,
/// with month 1 for January; none where the C library can't say, though
/// the second before 1970 began, which `mktime` answers as its failure
/// does, is told apart by the weekday it fills in. A field
/// past its end rolls into the next, as `mktime` normalizes it, so day 32
/// of a month is the next month's first, and whether daylight saving
/// applies is for it to work out.
pub fn moment(year: i32, month: i32, day: i32, hour: i32, minute: i32, second: i32) -> Option<i64> {
    let mut tm = Tm::zeroed();
    tm.year = year.checked_sub(1900)?;
    tm.month = month.checked_sub(1)?;
    tm.day = day;
    tm.hour = hour;
    tm.minute = minute;
    tm.second = second;
    tm.summer = -1;
    tm.weekday = -1;
    // SAFETY: the pointer is to a value alive across the call, and Tm is
    // laid out as the C library's struct tm.
    let seconds = unsafe { mktime(&raw mut tm) };
    (seconds != -1 || tm.weekday != -1).then_some(seconds)
}

// The tests read whatever zone the machine runs in, since setting TZ in
// one would race the others, so each holds in every zone: a moment read
// back is the one written, at noon in July, which no zone's daylight
// saving skips. One runs this binary again under Eastern time, which the
// sandbox's UTC can't stand in for, to pin each field and summer's hour.
#[cfg(test)]
mod tests {
    use super::*;

    /// Eastern time by its POSIX rule, which glibc and macOS read without a
    /// zone database.
    const EASTERN: &str = "EST5EDT,M3.2.0,M11.1.0";

    #[test]
    fn eastern_moments_are_read_and_written_as_its_clocks_say_them() {
        if std::env::var_os("TZ").is_none_or(|zone| zone != EASTERN) {
            let name = "local::tests::eastern_moments_are_read_and_written_as_its_clocks_say_them";
            let ran = std::process::Command::new(std::env::current_exe().expect("its own path"))
                .args(["--exact", name])
                .env("TZ", EASTERN)
                .output()
                .expect("the case runs again under Eastern time");
            let said = String::from_utf8_lossy(&ran.stdout);
            assert!(
                ran.status.success() && said.contains("test result: ok. 1 passed"),
                "the case failed or didn't run under Eastern time:\n{said}{}",
                String::from_utf8_lossy(&ran.stderr)
            );
            return;
        }
        let local = |year, month, day, weekday, hour, minute, second| Local {
            year,
            month,
            day,
            weekday,
            hour,
            minute,
            second,
        };
        assert_eq!(
            at(1_794_556_333),
            Some(local(2026, 11, 13, 5, 2, 52, 13)),
            "07:52:13 UTC on 13 November is 02:52:13 on Friday in Eastern standard time"
        );
        assert_eq!(
            at(1_784_133_296),
            Some(local(2026, 7, 15, 3, 12, 34, 56)),
            "16:34:56 UTC on 15 July is 12:34:56 on Wednesday in Eastern summer time"
        );
        assert_eq!(
            at(0),
            Some(local(1969, 12, 31, 3, 19, 0, 0)),
            "1970 began at 19:00 on Wednesday, the last of 1969, in Eastern time"
        );
        assert_eq!(
            moment(2026, 7, 15, 12, 34, 56),
            Some(1_784_133_296),
            "12:34:56 on 15 July in Eastern time is in summer, four hours behind UTC"
        );
        assert_eq!(
            moment(2026, 11, 13, 2, 52, 13),
            Some(1_794_556_333),
            "02:52:13 on 13 November in Eastern time is five hours behind UTC"
        );
        assert_eq!(
            moment(1969, 12, 31, 18, 59, 59),
            Some(-1),
            "the second before 1970 began is a moment, though mktime fails with its number"
        );
    }

    #[test]
    fn a_moment_past_any_year_is_not_placed() {
        assert_eq!(
            at(i64::MAX),
            None,
            "a moment whose year an int can't hold isn't placed"
        );
        assert_eq!(
            moment(i32::MIN, 1, 1, 0, 0, 0),
            None,
            "a year 1900 can't be taken from isn't a moment"
        );
    }

    fn noon() -> i64 {
        moment(2026, 7, 15, 12, 34, 56).expect("noon in July is a moment in every zone")
    }

    #[test]
    fn a_local_time_read_back_is_the_one_written() {
        let local = at(noon()).expect("a moment in July is placed");
        assert_eq!(
            local,
            Local {
                year: 2026,
                month: 7,
                day: 15,
                weekday: 3,
                hour: 12,
                minute: 34,
                second: 56,
            },
            "15 July 2026 is a Wednesday, read back as written"
        );
    }

    #[test]
    fn a_date_and_its_minute_are_said_from_its_fields() {
        let local = at(noon()).expect("a moment in July is placed");
        assert_eq!(
            local.date(),
            "2026-07-15",
            "the date pads its month and day"
        );
        assert_eq!(
            local.minute_of_day(),
            12 * 60 + 34,
            "minutes count from midnight"
        );
        let new_year = at(moment(2027, 1, 2, 0, 5, 0).expect("a moment in January"));
        assert_eq!(
            new_year.map(|local| local.date()),
            Some("2027-01-02".to_owned()),
            "January is month 1 and its second day 02"
        );
    }

    #[test]
    fn a_day_past_its_month_rolls_into_the_next() {
        let first = moment(2026, 2, 1, 12, 0, 0).expect("noon on 1 February is a moment");
        assert_eq!(
            moment(2026, 1, 32, 12, 0, 0),
            Some(first),
            "day 32 of January is the first of February"
        );
        assert_eq!(
            moment(2026, 7, 15, 13, 34, 56).map(|later| later - noon()),
            Some(3600),
            "an hour on is 3600 seconds on"
        );
    }

    #[test]
    fn now_is_placed() {
        assert!(now().is_some(), "now, after 1970, wasn't placed");
    }
}

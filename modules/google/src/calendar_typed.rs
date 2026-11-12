//! A time typed inside Move the Meeting, read as the shell's Calendar.js
//! read one before the daemon answered what's typed there (ready-rows.md's
//! `within`): its hour, a colon and its minutes when it has them, and "am"
//! or "pm" when typed, after a day when one is: "14:30", "9", "tomorrow
//! 10:00", "thursday 9", "thu 2pm". The days are named in English, as the
//! shell's Repeats.js names them.

use jiff::tz::TimeZone;
use jiff::{Timestamp, ToSpan};

/// The days of the week, Sunday first, as a zoned date numbers them.
const DAYS: [&str; 7] = [
    "sunday",
    "monday",
    "tuesday",
    "wednesday",
    "thursday",
    "friday",
    "saturday",
];
/// How few letters of a day's name name it: "thu" is Thursday.
const DAY_LETTERS: usize = 3;
/// Hours on a clock, and in half a day.
const HOURS: i8 = 24;
const HALF_DAY: i8 = 12;
const MINUTES: i8 = 60;

/// When a typed time would start a meeting lasting `lasts` milliseconds,
/// and end, at `now` on Joel's clock in `zone`; none for text that isn't a
/// time, or one gone by. A day of the week named as today, at a time gone
/// by, is next week's.
#[must_use]
pub fn typed(text: &str, lasts: i64, now: i64, zone: &TimeZone) -> Option<(i64, i64)> {
    let text = text.trim().to_lowercase();
    let (day, (hour, minute)) = read(&text)?;
    let today = Timestamp::from_millisecond(now)
        .ok()?
        .to_zoned(zone.clone());
    let weekday = today.weekday().to_sunday_zero_offset();
    let days = day_named(day, weekday)?;
    let at = |days: i8| {
        today
            .date()
            .checked_add(i64::from(days).days())
            .ok()?
            .at(hour, minute, 0, 0)
            .to_zoned(zone.clone())
            .ok()
            .map(|zoned| zoned.timestamp().as_millisecond())
    };
    let mut start = at(days)?;
    let named = !matches!(day, None | Some("today" | "tomorrow"));
    if start <= now && named {
        start = at(days + 7)?;
    }
    (start > now).then_some((start, start + lasts))
}

/// The day named, if any, and the hour and minute on a 24-hour clock, of
/// text already trimmed and lowercase; none where it isn't a time.
fn read(text: &str) -> Option<(Option<&str>, (i8, i8))> {
    // A first word is a day only where what follows is a time, so "9 pm"
    // is nine at night, not a day named "9".
    if let Some((first, rest)) = text.split_once(char::is_whitespace)
        && let Some(time) = clock(rest.trim_start())
    {
        return Some((Some(first), time));
    }
    clock(text).map(|time| (None, time))
}

/// The hour and minute of "9", "14:30", "2pm" or "9:15 am", on a 24-hour
/// clock; none for anything else, or an hour or minute no clock reads.
fn clock(text: &str) -> Option<(i8, i8)> {
    let (text, half) = match (text.strip_suffix("am"), text.strip_suffix("pm")) {
        (Some(rest), _) => (rest.trim_end(), Some(false)),
        (_, Some(rest)) => (rest.trim_end(), Some(true)),
        _ => (text, None),
    };
    let (hour, minute) = text.split_once(':').unwrap_or((text, "00"));
    let digits = |part: &str, fewest: usize| {
        (fewest..=2).contains(&part.len()) && part.bytes().all(|byte| byte.is_ascii_digit())
    };
    if !digits(hour, 1) || !digits(minute, 2) {
        return None;
    }
    let (mut hour, minute): (i8, i8) = (hour.parse().ok()?, minute.parse().ok()?);
    if let Some(after_noon) = half {
        if !(1..=HALF_DAY).contains(&hour) {
            return None;
        }
        hour = hour % HALF_DAY + if after_noon { HALF_DAY } else { 0 };
    }
    (hour < HOURS && minute < MINUTES).then_some((hour, minute))
}

/// How many days from today a word names, today being `weekday` (Sunday
/// 0): none or "today" is today, "tomorrow" tomorrow, and a day of the
/// week by its name or its first three letters or more the next on or
/// after today; none for a word that names no day.
fn day_named(word: Option<&str>, weekday: i8) -> Option<i8> {
    match word {
        None | Some("today") => Some(0),
        Some("tomorrow") => Some(1),
        Some(word) if word.len() >= DAY_LETTERS => {
            let named = DAYS.iter().position(|day| day.starts_with(word))?;
            let named = i8::try_from(named).ok()?;
            Some((named - weekday + 7) % 7)
        }
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use jiff::civil::date;

    /// Joel's clock four hours behind UTC, as the crate's other cases keep
    /// it, so the build's sandbox needs no tzdata.
    fn zone() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-4))
    }

    /// Thursday 12 November 2026 at 14:00 on that clock, as milliseconds.
    fn now(zone: &TimeZone) -> i64 {
        date(2026, 11, 12)
            .at(14, 0, 0, 0)
            .to_zoned(zone.clone())
            .expect("a time the clock has")
            .timestamp()
            .as_millisecond()
    }

    /// When a typed time starts, as that clock says it.
    fn said(at: i64, zone: &TimeZone) -> String {
        Timestamp::from_millisecond(at)
            .expect("a time")
            .to_zoned(zone.clone())
            .strftime("%a %Y-%m-%d %H:%M")
            .to_string()
    }

    const HALF_HOUR: i64 = 30 * 60_000;

    fn typed_as(text: &str) -> Option<String> {
        let zone = zone();
        typed(text, HALF_HOUR, now(&zone), &zone).map(|(start, end)| {
            assert_eq!(end - start, HALF_HOUR, "{text} lasts as the meeting does");
            said(start, &zone)
        })
    }

    #[test]
    fn a_time_alone_is_today_and_a_day_named_its_next() {
        assert_eq!(
            typed_as("14:30").as_deref(),
            Some("Thu 2026-11-12 14:30"),
            "today"
        );
        assert_eq!(
            typed_as("9pm").as_deref(),
            Some("Thu 2026-11-12 21:00"),
            "pm"
        );
        assert_eq!(
            typed_as("9 pm").as_deref(),
            Some("Thu 2026-11-12 21:00"),
            "pm apart"
        );
        assert_eq!(
            typed_as("Tomorrow 10:00").as_deref(),
            Some("Fri 2026-11-13 10:00"),
            "tomorrow, in any case"
        );
        assert_eq!(
            typed_as("mon 9").as_deref(),
            Some("Mon 2026-11-16 09:00"),
            "a day"
        );
        assert_eq!(
            typed_as("thursday 15:15").as_deref(),
            Some("Thu 2026-11-12 15:15"),
            "today's name, later today"
        );
        assert_eq!(
            typed_as("thu 9:15 am").as_deref(),
            Some("Thu 2026-11-19 09:15"),
            "today's name, gone by today, is a week on"
        );
        assert_eq!(typed_as("12am tomorrow"), None, "the day goes first");
        assert_eq!(
            typed_as("tomorrow 12am").as_deref(),
            Some("Fri 2026-11-13 00:00"),
            "12am is midnight"
        );
        assert_eq!(
            typed_as("tomorrow 12pm").as_deref(),
            Some("Fri 2026-11-13 12:00"),
            "12pm is noon"
        );
    }

    #[test]
    fn what_no_clock_reads_or_has_gone_by_is_no_time() {
        for text in [
            "9",
            "13pm",
            "0am",
            "24:00",
            "10:60",
            "10:5",
            "th 10",
            "someday 10",
            "noon",
            "",
            "1234",
        ] {
            assert_eq!(typed_as(text), None, "{text:?} is no time to come");
        }
    }
}

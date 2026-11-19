//! When a step says it starts: a step such as F8TT8O's "Judge the choice
//! live at Thu 19 09:00" on 19 November is no step to wake its cell to, or
//! to count it idle on, until that moment passes. Only a start counts, said
//! with "at" or "after" before a weekday, a day of the month and a time; a
//! due date, "by Thu 19 12:00" or a bare "Thu 19 18:00, once hubs land", is
//! work that could be done now, so it stays a step to wake to.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

/// The words that say a step starts at the moment after them.
const STARTS: [&str; 2] = ["at", "after"];

/// How many days either side of today a weekday and day of the month are
/// looked for, since a step names no month: wide enough for a start a
/// month off, and only one date in it can match both.
const LOOKED: i64 = 40;

/// When `step` says it starts, in seconds, its weekday and day of the
/// month read as the nearest date to `now` in `tz` that has both; none
/// where it says no start or the date and time don't make one.
#[must_use]
pub fn starts(step: &str, now: i64, tz: &TimeZone) -> Option<i64> {
    let words: Vec<&str> = step.split_whitespace().collect();
    let today = Timestamp::from_second(now)
        .ok()?
        .to_zoned(tz.clone())
        .date();
    words.windows(4).find_map(|w| {
        if !STARTS.contains(&w[0].to_ascii_lowercase().as_str()) {
            return None;
        }
        let weekday = weekday(w[1])?;
        let day: i8 = w[2].parse().ok()?;
        let (hour, minute) = time(w[3])?;
        let date = nearest(today, weekday, day)?;
        let start = date.at(hour, minute, 0, 0).to_zoned(tz.clone()).ok()?;
        Some(start.timestamp().as_second())
    })
}

/// A weekday from its name or its first three letters, in any case.
fn weekday(word: &str) -> Option<Weekday> {
    let word = word.to_ascii_lowercase();
    let word = word.trim_end_matches(|c: char| !c.is_ascii_alphabetic());
    [
        ("monday", Weekday::Monday),
        ("tuesday", Weekday::Tuesday),
        ("wednesday", Weekday::Wednesday),
        ("thursday", Weekday::Thursday),
        ("friday", Weekday::Friday),
        ("saturday", Weekday::Saturday),
        ("sunday", Weekday::Sunday),
    ]
    .into_iter()
    .find(|(name, _)| word.len() >= 3 && name.starts_with(word))
    .map(|(_, day)| day)
}

/// An hour and minute from "09:00", with any punctuation after it.
fn time(word: &str) -> Option<(i8, i8)> {
    let word = word.trim_end_matches(|c: char| !c.is_ascii_digit());
    let (hour, minute) = word.split_once(':')?;
    let two = minute.len() == 2;
    let (hour, minute): (i8, i8) = (hour.parse().ok()?, minute.parse().ok()?);
    ((0..24).contains(&hour) && two && (0..60).contains(&minute)).then_some((hour, minute))
}

/// The date nearest `today` that falls on `weekday` and `day` of its month.
fn nearest(today: Date, weekday: Weekday, day: i8) -> Option<Date> {
    (0..=LOOKED)
        .flat_map(|off| [off, -off])
        .filter_map(|off| today.checked_add(Span::new().days(off)).ok())
        .find(|d| d.day() == day && d.weekday() == weekday)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toronto() -> TimeZone {
        TimeZone::fixed(jiff::tz::offset(-5))
    }

    /// 05:30 on Thursday 19 November 2026 in Toronto, in seconds.
    const THU_19_0530: i64 = 1_795_084_200;

    #[test]
    fn a_step_starting_at_a_day_and_time_starts_then() {
        let at = starts(
            "Judge the choice live at Thu 19 09:00, over the desk spell",
            THU_19_0530,
            &toronto(),
        );
        assert_eq!(
            at,
            Some(THU_19_0530 + 3 * 3600 + 30 * 60),
            "09:00 that morning in Toronto"
        );
        assert_eq!(
            starts("Look again after Friday 20 10:30.", THU_19_0530, &toronto()),
            Some(THU_19_0530 + 29 * 3600),
            "the next morning, the weekday spelt out"
        );
        assert_eq!(
            starts("Judge it at Wed 18 09:00", THU_19_0530, &toronto()),
            Some(THU_19_0530 - 20 * 3600 - 30 * 60),
            "a start already passed is still said, as passed"
        );
    }

    #[test]
    fn a_due_date_or_a_day_that_matches_no_date_says_no_start() {
        for step in [
            "Each program's page under 50 ms, by Tue 11-10 12:00.",
            "Land the hubs by Thu 19 12:00",
            "Check the day's run Thu 19 18:00, once hubs land.",
            "Judge it at Fri 19 09:00",
            "Judge it at Thu 19 25:00",
            "Judge it at Thu 19 evening",
            "Judge it at Thump 19 09:00",
        ] {
            assert_eq!(
                starts(step, THU_19_0530, &toronto()),
                None,
                "{step:?} said a start"
            );
        }
    }
}

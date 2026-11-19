//! When a step says it starts: a step such as F8TT8O's "Judge the choice
//! live at Thu 19 09:00" on 19 November is no step to wake its cell to, or
//! to count it idle on, until that moment passes. Only a start counts, said
//! with "at" or "after" before a weekday, a day of the month and a time; a
//! due date, "by Thu 19 12:00" or a bare "Thu 19 18:00, once hubs land", is
//! work that could be done now, so it stays a step to wake to. A cell
//! writes its own step, so a start honoured holds back every word of its
//! idleness: one further off than `AHEAD_AT_MOST` is no start at all.

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{Span, Timestamp};

/// The words that say a step starts at the moment after them.
const STARTS: [&str; 2] = ["at", "after"];

/// How many days either side of today a weekday and day of the month are
/// looked for, since a step names no month. Two dates with one day of the
/// month are 28 days apart at the least, from February into March, so in
/// 27 days only one can match both.
const LOOKED: i64 = 13;

/// How far ahead a start is honoured, in seconds: a weekend's, from a
/// Friday evening to a Monday morning, and no further. The starts that
/// hid cells were that day's or the next's.
const AHEAD_AT_MOST: i64 = 3 * 24 * 3600;

/// When `step` says it starts, in seconds, its weekday and day of the
/// month read as the date within `LOOKED` days of `now` in `tz` that has
/// both; none where it says no start, the date and time don't make one,
/// or it is over `AHEAD_AT_MOST` ahead.
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
        let day = day(w[2])?;
        let (hour, minute) = time(w[3])?;
        let date = nearest(today, weekday, day)?;
        let start = date.at(hour, minute, 0, 0).to_zoned(tz.clone()).ok()?;
        Some(start.timestamp().as_second()).filter(|at| *at - now <= AHEAD_AT_MOST)
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

/// A day of the month from "19", with anything after its digits: "19's",
/// "19th" and "19," are each the 19th, as a cell writes them. A word
/// that doesn't start with its digits is no day.
fn day(word: &str) -> Option<i8> {
    word.trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .ok()
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
        // 1JQPPC's step on 19 November, woken at 10:40 as idle since its
        // day was read as "19's", no number.
        assert_eq!(
            starts(
                "After Thu 19's 18:00 cadence-day run, the first with #5591",
                THU_19_0530,
                &toronto(),
            ),
            Some(THU_19_0530 + 12 * 3600 + 30 * 60),
            "a day written 19's gives that evening"
        );
        for step in ["after Thu 19th 18:00", "after Thu 19\u{2019}s 18:00"] {
            assert_eq!(
                starts(step, THU_19_0530, &toronto()),
                Some(THU_19_0530 + 12 * 3600 + 30 * 60),
                "{step:?} gave no start that evening"
            );
        }
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
            "Judge it at Thu x19 09:00",
            "Judge it at Thu th 18:00",
            "Judge it at Mon 23 09:00",
            "Judge it at Sun 22 09:00",
        ] {
            assert_eq!(
                starts(step, THU_19_0530, &toronto()),
                None,
                "{step:?} said a start"
            );
        }
        assert!(
            starts("Judge it at Sun 22 05:00", THU_19_0530, &toronto()).is_some(),
            "a start a weekend off was refused"
        );
    }

    #[test]
    fn a_start_four_weeks_back_is_read_as_passed_not_as_next_months() {
        // Noon on Thursday 4 March 2027 in Toronto: 19 February and 19
        // March 2027 are both Fridays.
        let thu_4_march = 1_804_179_600;
        assert_eq!(
            starts("Judge it at Fri 19 09:00", thu_4_march, &toronto()),
            Some(1_803_045_600),
            "read as 19 March, ahead, or not at all"
        );
    }
}

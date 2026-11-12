//! When Joel stopped each night, and what that makes usual.
//!
//! A night's stop is the last minute he was at the machine before a gap of
//! four hours or more that starts between 18:00 and 06:00 by the local
//! clock: long enough to be sleep rather than dinner. Where an evening
//! holds two such gaps, the longer is the night's. The stop must be a busy
//! minute, with others in the half hour before it, so a stray key on the
//! way to bed, or a message an agent sent in his name, doesn't move it; a
//! lone minute just before the gap gives way to the busy one before it.
//! That holds only for the minutes rhythm watched: those seeded from the
//! logs are a command or a message now and then, each typed, so a seeded
//! night stops at its last.
//!
//! What's usual is the middle of the last four weeks' stops, each week back
//! counting half as much as the one after it, so a change of habit shows
//! within a week and one late night moves nothing. After a break, where
//! the last four weeks hold too few nights, the four weeks up to the last
//! evening he was here say more than the guess, and are followed. Each weekday's is the
//! middle of the last eight weeks' stops on that weekday once there are
//! three, each fortnight back counting half, so a weekday follows its own
//! change within two. Until there are a week's nights, the evening guesses
//! a midnight stop, and says it guesses.

use crate::clock::{Local, days_from_civil};

/// The shortest gap that's a night's sleep, in seconds.
const SLEEP: i64 = 4 * 3600;
/// Where in the local day a night's gap may start: from 18:00 to 06:00.
const EVENING: u32 = 18 * 60;
const MORNING: u32 = 6 * 60;
/// How busy the half hour before a stop must be, in minutes seen.
const BUSY: usize = 3;
const BUSY_WITHIN: i64 = 30 * 60;
/// How far back a lone last minute may give way to a busy one.
const STRAY: i64 = 2 * 3600;
/// How many nights make a profile worth following, and what's guessed
/// until then: midnight, in minutes after 18:00.
pub const ENOUGH: usize = 7;
pub const GUESS: u32 = 6 * 60;
/// The weeks each reading looks back over, in days.
pub const USUAL_DAYS: i64 = 28;
const WEEKDAY_DAYS: i64 = 56;
const WEEKDAY_NIGHTS: usize = 3;
/// How many days back a night's weight halves, toward what's usual and
/// toward its weekday's, and what the newest night weighs.
pub const USUAL_HALVES: i64 = 7;
const WEEKDAY_HALVES: i64 = 14;
pub const NEWEST: u32 = 1 << 10;

/// A night Joel stopped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Night {
    /// The evening's date, as days since 1970 began, and its weekday,
    /// Sunday 0: a stop after midnight belongs to the evening before.
    pub evening: i64,
    pub weekday: u32,
    /// The last busy minute before the gap, in seconds since 1970.
    pub stop: i64,
    /// The stop's local time, in minutes after 18:00, so a night past
    /// midnight still sorts after one before it.
    pub after_six: u32,
}

/// Each night Joel stopped, oldest first, from the minutes he was seen at
/// the machine, sorted, read by `local`; those before `watched` were
/// seeded from the logs.
pub fn nights(seen: &[i64], watched: i64, local: impl Fn(i64) -> Option<Local>) -> Vec<Night> {
    let mut found: Vec<(Night, i64)> = Vec::new();
    for (index, pair) in seen.windows(2).enumerate() {
        let gap = pair[1] - pair[0];
        if gap < SLEEP {
            continue;
        }
        let stop = if pair[0] < watched {
            Some(pair[0])
        } else {
            busy_before(seen, index)
        };
        let Some(stop) = stop else {
            continue;
        };
        let Some(clock) = local(stop) else {
            continue;
        };
        if (MORNING..EVENING).contains(&clock.minute) {
            continue;
        }
        let night = night_of(stop, clock);
        match found.last_mut() {
            Some((last, longest)) if last.evening == night.evening => {
                if gap > *longest {
                    *last = night;
                    *longest = gap;
                }
            }
            _ => found.push((night, gap)),
        }
    }
    found.into_iter().map(|(night, _)| night).collect()
}

/// The last busy minute at or before `seen[index]`, within `STRAY` of it.
fn busy_before(seen: &[i64], index: usize) -> Option<i64> {
    let last = seen[index];
    (0..=index)
        .rev()
        .map(|at| seen[at])
        .take_while(|minute| last - minute <= STRAY)
        .find(|&minute| {
            let from = seen.partition_point(|seen| *seen < minute - BUSY_WITHIN);
            let to = seen.partition_point(|seen| *seen <= minute);
            to - from >= BUSY
        })
}

/// The night a kept stop belongs to, read by `local`.
pub fn night_at(stop: i64, local: impl Fn(i64) -> Option<Local>) -> Option<Night> {
    local(stop).map(|clock| night_of(stop, clock))
}

/// The night a stop at this local time belongs to.
fn night_of(stop: i64, clock: Local) -> Night {
    let today = days_from_civil(clock.year, clock.month, clock.day);
    if clock.minute >= EVENING {
        Night {
            evening: today,
            weekday: clock.weekday,
            stop,
            after_six: clock.minute - EVENING,
        }
    } else {
        Night {
            evening: today - 1,
            weekday: (clock.weekday + 6) % 7,
            stop,
            after_six: clock.minute + MORNING,
        }
    }
}

/// What the nights make usual, as of the evening `today`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Profile {
    /// How many nights the last four weeks hold.
    pub nights: usize,
    /// The middle of their stops, the latest weighing most, in minutes
    /// after 18:00.
    pub usual: Option<u32>,
    /// Each weekday's, Sunday first, where it has three nights, the latest
    /// weighing most.
    pub weekdays: [Option<u32>; 7],
    /// The evening the readings end at, where it isn't today's: the last
    /// he was here, after a break.
    pub from: Option<i64>,
    /// How far the day's light moves tonight's stop from what the weeks
    /// say, in minutes, earlier below zero (season.rs).
    pub drift: i32,
}

impl Profile {
    /// The nights' profile as of the evening `today`, as days since 1970,
    /// or, where the last four weeks hold too few nights but the four weeks
    /// up to the last evening he was here don't, as of that evening.
    pub fn of(nights: &[Night], today: i64) -> Profile {
        let profile = Profile::as_of(nights, today);
        if profile.nights >= ENOUGH {
            return profile;
        }
        let Some(last) = nights
            .iter()
            .map(|night| night.evening)
            .filter(|evening| *evening < today)
            .max()
        else {
            return profile;
        };
        let then = Profile::as_of(nights, last);
        if then.nights < ENOUGH {
            return profile;
        }
        Profile {
            from: Some(last),
            ..then
        }
    }

    /// The nights' profile as of the evening `today`.
    fn as_of(nights: &[Night], today: i64) -> Profile {
        let within = |days: i64| {
            nights
                .iter()
                .filter(move |night| night.evening <= today && today - night.evening < days)
        };
        let weighed = |night: &Night, halves: i64| {
            (
                night.after_six,
                NEWEST >> ((today - night.evening) / halves),
            )
        };
        let recent: Vec<(u32, u32)> = within(USUAL_DAYS)
            .map(|night| weighed(night, USUAL_HALVES))
            .collect();
        let weekdays = std::array::from_fn(|weekday| {
            let stops: Vec<(u32, u32)> = within(WEEKDAY_DAYS)
                .filter(|night| night.weekday as usize == weekday)
                .map(|night| weighed(night, WEEKDAY_HALVES))
                .collect();
            (stops.len() >= WEEKDAY_NIGHTS)
                .then(|| middle(stops))
                .flatten()
        });
        Profile {
            nights: recent.len(),
            usual: middle(recent),
            weekdays,
            from: None,
            drift: 0,
        }
    }

    /// When he likely stops on an evening of this weekday, in minutes
    /// after 18:00, and whether that's learned or the guess.
    pub fn tonight(&self, weekday: u32) -> (u32, bool) {
        if self.nights < ENOUGH {
            return (GUESS, false);
        }
        let learned = self.weekdays[weekday as usize % 7].or(self.usual);
        let moved = i64::from(learned.unwrap_or(GUESS)) + i64::from(self.drift);
        (u32::try_from(moved.clamp(0, 1439)).unwrap_or(GUESS), true)
    }
}

/// The middle of some stops by their weights: the first, earliest to
/// latest, at which half the weight is reached, so of two equal halves the
/// earlier.
fn middle(mut stops: Vec<(u32, u32)>) -> Option<u32> {
    stops.sort_unstable();
    let total: u32 = stops.iter().map(|(_, weight)| weight).sum();
    let mut reached = 0;
    stops
        .into_iter()
        .find(|&(_, weight)| {
            reached += weight;
            2 * reached >= total
        })
        .map(|(stop, _)| stop)
}

/// A time in minutes after 18:00 as the clock says it, `00:40`.
pub fn clock_of(after_six: u32) -> String {
    let minute = (EVENING + after_six) % 1440;
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

/// The evening a moment belongs to: today's from noon, else yesterday's.
pub fn evening_of(clock: Local) -> i64 {
    let today = days_from_civil(clock.year, clock.month, clock.day);
    if clock.minute >= 12 * 60 {
        today
    } else {
        today - 1
    }
}

/// A weekday's name.
pub fn weekday_name(weekday: u32) -> &'static str {
    [
        "Sunday",
        "Monday",
        "Tuesday",
        "Wednesday",
        "Thursday",
        "Friday",
        "Saturday",
    ][weekday as usize % 7]
}

/// An evening's weekday, Sunday 0.
pub fn weekday_of(evening: i64) -> u32 {
    // 1970-01-01 was a Thursday.
    u32::try_from((evening + 4).rem_euclid(7)).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::fixed;

    /// Toronto's winter clock, five hours behind UTC.
    const TORONTO: i64 = -300;
    /// Wednesday 11 November 2026, as days since 1970.
    const WEDNESDAY: i64 = 20_768;

    /// Seconds since 1970 for a Toronto evening's local hour and minute,
    /// past midnight carried into the next day.
    fn at(evening: i64, hour: i64, minute: i64) -> i64 {
        evening * 86_400 + (hour * 60 + minute - TORONTO) * 60
    }

    /// Joel at the machine every minute from one time to another.
    fn busy(from: i64, to: i64) -> Vec<i64> {
        (from / 60..=to / 60).map(|minute| minute * 60).collect()
    }

    /// An evening of work from 19:00 until `hour:minute`, then sleep until
    /// 09:00, as minutes seen.
    fn evening(evening: i64, hour: i64, minute: i64) -> Vec<i64> {
        let mut seen = busy(at(evening, 19, 0), at(evening, hour, minute));
        seen.extend(busy(at(evening, 33, 0), at(evening, 33, 30)));
        seen
    }

    #[test]
    fn a_night_stops_at_the_last_busy_minute_before_sleep() {
        let found = nights(&evening(WEDNESDAY, 24, 40), 0, fixed(TORONTO));
        assert_eq!(found.len(), 1, "one night in {found:?}");
        assert_eq!(found[0].evening, WEDNESDAY, "the evening before midnight");
        assert_eq!(found[0].weekday, 3, "a Wednesday");
        assert_eq!(clock_of(found[0].after_six), "00:40", "stopped at 00:40");
    }

    #[test]
    fn dinner_is_no_night() {
        let mut seen = busy(at(WEDNESDAY, 9, 0), at(WEDNESDAY, 13, 0));
        seen.extend(busy(at(WEDNESDAY, 17, 30), at(WEDNESDAY, 18, 30)));
        seen.extend(busy(at(WEDNESDAY, 21, 0), at(WEDNESDAY, 23, 50)));
        seen.extend(busy(at(WEDNESDAY, 33, 0), at(WEDNESDAY, 33, 30)));
        let found = nights(&seen, 0, fixed(TORONTO));
        assert_eq!(found.len(), 1, "one night in {found:?}");
        assert_eq!(clock_of(found[0].after_six), "23:50", "not the 18:30 break");
    }

    #[test]
    fn a_lunch_gap_is_no_night() {
        let mut seen = busy(at(WEDNESDAY, 9, 0), at(WEDNESDAY, 11, 0));
        seen.extend(busy(at(WEDNESDAY, 15, 30), at(WEDNESDAY, 16, 0)));
        assert_eq!(
            nights(&seen, 0, fixed(TORONTO)),
            vec![],
            "a day's gap isn't sleep"
        );
    }

    #[test]
    fn a_stray_minute_before_bed_moves_no_stop() {
        let mut seen = busy(at(WEDNESDAY, 19, 0), at(WEDNESDAY, 23, 10));
        seen.push(at(WEDNESDAY, 24, 20));
        seen.extend(busy(at(WEDNESDAY, 33, 0), at(WEDNESDAY, 33, 30)));
        let found = nights(&seen, 0, fixed(TORONTO));
        assert_eq!(
            clock_of(found[0].after_six),
            "23:10",
            "the busy minute, not the stray"
        );
    }

    #[test]
    fn a_seeded_night_stops_at_its_last_minute() {
        // The logs hold a command or a message now and then, never a
        // stray key, so a lone late one is where the night stopped.
        let mut seen = busy(at(WEDNESDAY, 19, 0), at(WEDNESDAY, 23, 10));
        seen.push(at(WEDNESDAY, 24, 20));
        seen.extend(busy(at(WEDNESDAY, 33, 0), at(WEDNESDAY, 33, 30)));
        let watched = at(WEDNESDAY, 40, 0);
        let found = nights(&seen, watched, fixed(TORONTO));
        assert_eq!(
            clock_of(found[0].after_six),
            "00:20",
            "the lone message, since it was seeded"
        );
    }

    #[test]
    fn of_two_night_gaps_the_longer_is_sleep() {
        let mut seen = busy(at(WEDNESDAY, 17, 0), at(WEDNESDAY, 18, 30));
        seen.extend(busy(at(WEDNESDAY, 22, 45), at(WEDNESDAY, 25, 15)));
        seen.extend(busy(at(WEDNESDAY, 33, 0), at(WEDNESDAY, 33, 30)));
        let found = nights(&seen, 0, fixed(TORONTO));
        assert_eq!(found.len(), 1, "one night in {found:?}");
        assert_eq!(
            clock_of(found[0].after_six),
            "01:15",
            "the long sleep, not the evening off"
        );
    }

    /// Four weeks of nights, Fridays later than the rest.
    fn four_weeks() -> Vec<Night> {
        let mut seen = Vec::new();
        for back in (0..28).rev() {
            let evening = WEDNESDAY - back;
            let late = if weekday_of(evening) == 5 { 26 } else { 24 };
            seen.extend(self::evening(evening, late, 30));
        }
        nights(&seen, 0, fixed(TORONTO))
    }

    #[test]
    fn the_usual_stop_is_the_middle_of_four_weeks() {
        let profile = Profile::of(&four_weeks(), WEDNESDAY);
        assert_eq!(profile.nights, 28, "four weeks of nights");
        assert_eq!(
            profile.usual.map(clock_of).as_deref(),
            Some("00:30"),
            "the middle"
        );
        assert_eq!(
            profile.tonight(3),
            (6 * 60 + 30, true),
            "a Wednesday, learned"
        );
    }

    #[test]
    fn a_weekday_with_its_own_habit_keeps_it() {
        let profile = Profile::of(&four_weeks(), WEDNESDAY);
        assert_eq!(
            profile.weekdays[5].map(clock_of).as_deref(),
            Some("02:30"),
            "Fridays"
        );
        assert_eq!(
            clock_of(profile.tonight(5).0),
            "02:30",
            "a Friday stays late"
        );
    }

    /// The nights of these evenings, each a number of days back from
    /// Wednesday, stopping at `hour:minute`.
    fn stopping(backs: impl Iterator<Item = i64>, hour: i64, minute: i64) -> Vec<i64> {
        backs
            .flat_map(|back| evening(WEDNESDAY - back, hour, minute))
            .collect()
    }

    #[test]
    fn a_change_of_habit_shows_within_a_week() {
        let mut seen = stopping((7..28).rev(), 24, 30);
        seen.extend(stopping((0..7).rev(), 25, 30));
        let profile = Profile::of(&nights(&seen, 0, fixed(TORONTO)), WEDNESDAY);
        assert_eq!(profile.nights, 28, "four weeks of nights");
        assert_eq!(
            profile.usual.map(clock_of).as_deref(),
            Some("01:30"),
            "this week's 01:30 outweighs three older weeks at 00:30"
        );
    }

    #[test]
    fn a_weekday_follows_its_own_change_within_two_weeks() {
        // Wednesday's last eight Fridays, five days back and every week
        // before: the six older at 02:30, the two latest at 00:30.
        let fridays = |weeks: std::ops::Range<i64>| weeks.rev().map(|week| 5 + 7 * week);
        let mut seen = stopping(fridays(2..8), 26, 30);
        seen.extend(stopping(fridays(0..2), 24, 30));
        let profile = Profile::of(&nights(&seen, 0, fixed(TORONTO)), WEDNESDAY);
        assert_eq!(
            profile.weekdays[5].map(clock_of).as_deref(),
            Some("00:30"),
            "the last two Fridays outweigh six older ones"
        );
    }

    #[test]
    fn after_a_break_the_last_four_weeks_he_was_here_are_followed() {
        // Four weeks of nights stopping at 01:00, the last forty days ago.
        let seen = stopping((40..68).rev(), 25, 0);
        let profile = Profile::of(&nights(&seen, 0, fixed(TORONTO)), WEDNESDAY);
        assert_eq!(
            profile.from,
            Some(WEDNESDAY - 40),
            "from the last evening he was here"
        );
        assert_eq!(
            profile.tonight(3),
            (7 * 60, true),
            "his habit then, learned, not the guess"
        );
    }

    #[test]
    fn a_night_is_read_again_from_its_stop() {
        let found = nights(&evening(WEDNESDAY, 24, 40), 0, fixed(TORONTO));
        assert_eq!(
            night_at(found[0].stop, fixed(TORONTO)),
            Some(found[0]),
            "the same night"
        );
    }

    #[test]
    fn the_day_s_light_moves_a_learned_stop_and_not_the_guess() {
        let profile = Profile {
            drift: -30,
            ..Profile::of(&four_weeks(), WEDNESDAY)
        };
        let (stop, learned) = Profile::of(&four_weeks(), WEDNESDAY).tonight(3);
        assert_eq!(
            profile.tonight(3),
            (stop - 30, learned),
            "half an hour earlier"
        );
        let few: Vec<Night> = four_weeks().into_iter().rev().take(ENOUGH - 1).collect();
        let guessed = Profile {
            drift: -30,
            ..Profile::of(&few, WEDNESDAY)
        };
        assert_eq!(
            guessed.tonight(3),
            (GUESS, false),
            "the guess stays midnight"
        );
    }

    #[test]
    fn too_few_nights_guess_midnight() {
        let few: Vec<Night> = four_weeks().into_iter().rev().take(ENOUGH - 1).collect();
        let profile = Profile::of(&few, WEDNESDAY);
        assert_eq!(profile.tonight(3), (GUESS, false), "the guess, said as one");
        assert_eq!(clock_of(GUESS), "00:00", "midnight");
    }

    #[test]
    fn an_evening_runs_from_noon_to_noon() {
        let toronto = fixed(TORONTO);
        let late = toronto(at(WEDNESDAY, 25, 0)).expect("a moment");
        let noon = toronto(at(WEDNESDAY, 12, 0)).expect("a moment");
        assert_eq!(
            evening_of(late),
            WEDNESDAY,
            "01:00 Thursday is Wednesday's evening"
        );
        assert_eq!(evening_of(noon), WEDNESDAY, "noon starts it");
        assert_eq!(weekday_name(weekday_of(WEDNESDAY)), "Wednesday", "its name");
    }
}

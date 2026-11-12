//! How Joel's stop moves with the length of the day, and how far that
//! moves tonight's.
//!
//! The usual stop is the middle of four weeks, the latest weighing most,
//! so through a season's turn it lags the day by a week or two. Over the
//! kept year each night's stop is set against that day's daylight where
//! he is now, and a straight line fitted through them, in whole numbers,
//! says how many minutes a minute of daylight moves it. Tonight's stop
//! then moves by that much for each minute today's daylight differs from
//! the four weeks' own, weighed as the usual stop weighs them. A line is
//! only read from nights spread over two months and an hour and a half of
//! daylight, since a few weeks of one season say nothing of another, and
//! the shift stays within an hour, so the line can't run off with a habit.

use crate::nights::{NEWEST, USUAL_DAYS, USUAL_HALVES};

/// How many nights a line needs.
const FIT_NIGHTS: usize = 30;
/// How far apart, in days, the first and last of them must be.
const FIT_SPAN: i64 = 60;
/// How far apart, in minutes, their days' daylight must be.
const FIT_DAYLIGHT: u32 = 90;
/// The most the line moves tonight's stop, in minutes either way.
const MOST: i64 = 60;

/// A night's stop against its day's light.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    /// The evening, as days since 1970.
    pub evening: i64,
    /// Minutes from sunrise to sunset that day.
    pub daylight: u32,
    /// When he stopped, in minutes after 18:00.
    pub after_six: u32,
}

/// How many minutes tonight's stop moves, as of the evening `today` with
/// `daylight` minutes of it: earlier below zero, later above.
#[must_use]
pub fn drift(year: &[Point], today: i64, daylight: u32) -> i32 {
    let _ = (year, today, daylight);
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nights from `first` to `last` days before `today`, the daylight
    /// growing two minutes a day back from 600, each stopping half a
    /// minute later for each minute more of it.
    fn shortening(today: i64, last: i64, first: i64) -> Vec<Point> {
        (last..=first)
            .map(|back| {
                let more = u32::try_from(back - last).expect("back is past last");
                Point {
                    evening: today - back,
                    daylight: 600 + 2 * more,
                    after_six: 390 + more,
                }
            })
            .collect()
    }

    #[test]
    fn a_shortening_day_brings_the_stop_earlier() {
        // The four weeks before today all had 600 minutes of daylight,
        // stopping at 00:30; before them the days were longer, and the
        // stops later. Today has an hour less daylight.
        let mut year = shortening(1000, 29, 200);
        year.extend((1..=28).map(|back| Point {
            evening: 1000 - back,
            daylight: 600,
            after_six: 390,
        }));
        assert_eq!(
            drift(&year, 1000, 540),
            -30,
            "half a minute for each of the sixty"
        );
    }

    #[test]
    fn the_shift_stays_within_an_hour() {
        let year = shortening(1000, 1, 200);
        assert_eq!(
            drift(&year, 1000, 300),
            -60,
            "an hour at most, however short the day"
        );
    }

    #[test]
    fn a_few_weeks_of_one_season_say_nothing() {
        // Fifty-five nights and 108 minutes of daylight, but under two
        // months.
        let year = shortening(1000, 1, 55);
        assert_eq!(drift(&year, 1000, 540), 0, "54 days aren't two months");
        let flat: Vec<Point> = shortening(1000, 1, 200)
            .into_iter()
            .map(|point| Point {
                daylight: 600 + (point.daylight - 600) / 5,
                ..point
            })
            .collect();
        assert_eq!(
            drift(&flat, 1000, 540),
            0,
            "eighty minutes of daylight aren't enough"
        );
    }

    #[test]
    fn too_few_nights_say_nothing() {
        let year: Vec<Point> = shortening(1000, 1, 200).into_iter().step_by(8).collect();
        assert!(year.len() < FIT_NIGHTS, "the case holds too few");
        assert_eq!(drift(&year, 1000, 540), 0, "a line needs thirty nights");
    }
}

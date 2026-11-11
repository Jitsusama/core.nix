//! The night light: from sunset to sunrise where the laptop is, the built-in
//! screen warms to a night white whatever its sensor sees, easing from its
//! own white over the half hour after sunset and back over the half hour
//! before sunrise (VKCF3Z's ruling, 2026-11-08: 4,000 K by default, gentle
//! enough to keep code's colours readable, warmer by an option). This file
//! only reckons the white the night wants and when to look again; warmth.rs
//! takes the warmer of it and the room's, and answering.rs sets it.
//!
//! Where the laptop is comes from whereabouts' file, its latitude and
//! longitude; with none known there's no schedule, and the shell says the
//! night light waits for where Joel is rather than guessing. The sun's
//! times are NOAA's reckoning in the sun crate (modules/sun), the same
//! one the launcher's calendars reckon their sunsets by, so the night falls
//! here when Date says the day ends.

use crate::warmth::WHITE;
use sun::Crossing;

/// How long the screen takes to ease into the night and back out of it.
pub const EASE: i64 = 30 * MINUTE;
/// The night's whites are steps of the warmth's own.
const STEP: u16 = 50;
/// Where the laptop is may change while it's day or night, so it's looked
/// at again within the hour, whatever the sun.
const LOOK_AGAIN: i64 = 60 * MINUTE;
/// While no place is known it's looked for again this often, since
/// whereabouts often says it a moment after the daemon starts: on
/// 2026-11-08 27 seconds later, which waiting the hour turned into an
/// hour without the night light.
pub const UNPLACED: i64 = 10 * SECOND;

/// The shortest wait between looks while easing, so a night white close to
/// the screen's own still eases rather than spinning.
const SOONEST: i64 = SECOND;

const SECOND: i64 = 1000;
const MINUTE: i64 = 60 * SECOND;
const DAY: i64 = 24 * 60 * MINUTE;

/// Where the laptop is, in degrees north and east.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub latitude: f64,
    pub longitude: f64,
}

impl Place {
    /// Where whereabouts' file says the laptop is, or none when it says no
    /// place. The file is one JSON object of plain numbers and words, so
    /// only the two figures needed are read from it.
    pub fn read(text: &str) -> Option<Place> {
        let latitude = figure(text, "latitude")?;
        let longitude = figure(text, "longitude")?;
        ((-90.0..=90.0).contains(&latitude) && (-180.0..=180.0).contains(&longitude)).then_some(
            Place {
                latitude,
                longitude,
            },
        )
    }
}

/// The number a JSON object gives a key, read as far as the number goes.
fn figure(text: &str, key: &str) -> Option<f64> {
    let after = text.split_once(&format!("\"{key}\""))?.1;
    let value = after.trim_start().strip_prefix(':')?.trim_start();
    let end = value
        .find(|letter: char| !(letter.is_ascii_digit() || "+-.eE".contains(letter)))
        .unwrap_or(value.len());
    value[..end]
        .parse()
        .ok()
        .filter(|number: &f64| number.is_finite())
}

/// A night: its sunset and the sunrise after it, in milliseconds since
/// 1970 began in UTC.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Night {
    pub sunset: i64,
    pub sunrise: i64,
}

/// What the night light wants now: the white for the screen, the night
/// under way or the next, and when to look again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Plan {
    pub kelvin: u16,
    pub night: Option<Night>,
    pub next: i64,
}

/// The civil date of a day counted from 1970's first in UTC, by Howard
/// Hinnant's `civil_from_days`.
fn civil(days: i64) -> (i32, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let of_era = shifted.rem_euclid(146_097);
    let year_of_era = (of_era - of_era / 1460 + of_era / 36_524 - of_era / 146_096) / 365;
    let of_year = of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * of_year + 2) / 153;
    let day = of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a month is 1 to 12, a day 1 to 31, and the year a sunset's is far within i32"
    )]
    (year as i32, month as u32, day as u32)
}

/// When the sun rises or sets on a day counted from 1970's first, in
/// milliseconds since 1970 began, or how it doesn't.
fn crossing(day: i64, place: Place, rising: bool) -> Crossing {
    let (year, month, date) = civil(day);
    match sun::crosses(year, month, date, place.latitude, place.longitude, rising) {
        #[expect(
            clippy::cast_precision_loss,
            reason = "days since 1970 are far within what a float holds whole"
        )]
        Crossing::At(minutes) => Crossing::At((day as f64 * sun::DAY + minutes) * 60_000.0),
        other => other,
    }
}

/// The nights around a moment, each sunset with the first sunrise after
/// it, oldest first.
fn nights(now: i64, place: Place) -> Vec<Night> {
    let today = now.div_euclid(DAY);
    let at = |day, rising| match crossing(day, place, rising) {
        #[expect(
            clippy::cast_possible_truncation,
            reason = "milliseconds since 1970 are far within i64"
        )]
        Crossing::At(ms) => Some(ms.round() as i64),
        _ => None,
    };
    let sunrises: Vec<i64> = (today - 1..=today + 3)
        .filter_map(|day| at(day, true))
        .collect();
    (today - 2..=today + 1)
        .filter_map(|day| at(day, false))
        .filter_map(|sunset| {
            let sunrise = sunrises.iter().copied().find(|&rise| rise > sunset)?;
            Some(Night { sunset, sunrise })
        })
        .collect()
}

/// A white between the night's and the screen's own, `part` of the way
/// from the screen's own, to the nearest step.
fn between(white: u16, part: f64) -> u16 {
    let span = f64::from(WHITE.saturating_sub(white));
    let kelvin = f64::from(WHITE) - span * part.clamp(0.0, 1.0);
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the white lies between the night's and 6,500 K, so its steps fit"
    )]
    let steps = (kelvin / f64::from(STEP)).round() as u16;
    steps * STEP
}

/// What the night light wants at `now` for a place, its night white
/// `white`.
pub fn plan(now: i64, place: Place, white: u16) -> Plan {
    let soon = now + LOOK_AGAIN;
    let all = nights(now, place);
    let under_way = all
        .iter()
        .copied()
        .find(|night| night.sunset <= now && now < night.sunrise);
    let coming = all.iter().copied().find(|night| night.sunset > now);
    let Some(night) = under_way else {
        // By day, or where the sun neither rises nor sets today: a polar
        // night is a night throughout, a midnight sun a day.
        let dark = matches!(
            crossing(now.div_euclid(DAY), place, false),
            Crossing::Unrisen
        );
        return Plan {
            kelvin: if dark { white.min(WHITE) } else { WHITE },
            night: coming,
            next: coming.map_or(soon, |night| night.sunset).min(soon),
        };
    };
    let ease = EASE.min((night.sunrise - night.sunset) / 2).max(1);
    let steps = i64::from(WHITE.saturating_sub(white) / STEP).max(1);
    let easing = (ease / steps).max(SOONEST);
    #[expect(
        clippy::cast_precision_loss,
        reason = "milliseconds within half an hour are held exactly"
    )]
    let part = |from: i64| (now - from) as f64 / ease as f64;
    let (kelvin, next) = if now < night.sunset + ease {
        (between(white, part(night.sunset)), now + easing)
    } else if now < night.sunrise - ease {
        (between(white, 1.0), night.sunrise - ease)
    } else {
        (
            between(white, 1.0 - part(night.sunrise - ease)),
            (now + easing).min(night.sunrise),
        )
    };
    Plan {
        kelvin,
        night: Some(night),
        next: next.min(soon),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Toronto, and the 8th of November 2026 at noon there, 17:00 UTC.
    const TORONTO: Place = Place {
        latitude: 43.6532,
        longitude: -79.3832,
    };
    const NOON: i64 = 1_794_157_200_000;
    const TROMSO: Place = Place {
        latitude: 69.6492,
        longitude: 18.9553,
    };

    #[test]
    fn a_date_is_counted_from_1970() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(NOON.div_euclid(DAY)), (2026, 11, 8));
        assert_eq!(civil(-1), (1969, 12, 31));
        assert_eq!(civil(11_016), (2000, 2, 29));
    }

    #[test]
    fn by_day_the_screen_keeps_its_own_white_until_sunset() {
        let day = plan(NOON, TORONTO, 4000);
        assert_eq!(day.kelvin, WHITE);
        let night = day.night.expect("a night comes");
        // NOAA's solar calculator gives 16:59 and 07:04 the next morning, in
        // Toronto's standard time, five hours behind UTC.
        let sunset = 1_794_175_140_000;
        let sunrise = 1_794_225_840_000;
        assert!((night.sunset - sunset).abs() <= 2 * MINUTE, "{night:?}");
        assert!((night.sunrise - sunrise).abs() <= 2 * MINUTE, "{night:?}");
        assert_eq!(day.next, (NOON + LOOK_AGAIN).min(night.sunset));
    }

    #[test]
    fn after_sunset_it_eases_to_the_night_white_and_holds_it() {
        let night = plan(NOON, TORONTO, 4000).night.expect("a night comes");
        assert_eq!(plan(night.sunset, TORONTO, 4000).kelvin, WHITE);
        assert_eq!(plan(night.sunset + EASE / 2, TORONTO, 4000).kelvin, 5250);
        let dark = plan(night.sunset + EASE, TORONTO, 4000);
        assert_eq!(dark.kelvin, 4000);
        assert_eq!(
            dark.next,
            (night.sunrise - EASE).min(night.sunset + EASE + LOOK_AGAIN),
            "nothing moves until morning but where the laptop is"
        );
    }

    #[test]
    fn before_sunrise_it_eases_back_to_the_screens_own_white() {
        let night = plan(NOON, TORONTO, 4000).night.expect("a night comes");
        assert_eq!(plan(night.sunrise - EASE, TORONTO, 4000).kelvin, 4000);
        assert_eq!(plan(night.sunrise - EASE / 2, TORONTO, 4000).kelvin, 5250);
        assert_eq!(plan(night.sunrise, TORONTO, 4000).kelvin, WHITE);
    }

    #[test]
    fn while_easing_it_looks_again_for_each_step() {
        let night = plan(NOON, TORONTO, 4000).night.expect("a night comes");
        let mut now = night.sunset;
        let mut seen = vec![];
        while now < night.sunset + EASE {
            let step = plan(now, TORONTO, 4000);
            seen.push(step.kelvin);
            now = step.next;
        }
        assert!(
            seen.windows(2).all(|pair| pair[0] - pair[1] <= STEP),
            "no step past one of the warmth's: {seen:?}"
        );
        assert_eq!(seen.len(), 50, "one look for each of the 50 steps");
    }

    #[test]
    fn a_night_white_as_white_as_the_screens_never_moves_it() {
        let night = plan(NOON, TORONTO, WHITE).night.expect("a night comes");
        assert_eq!(plan(night.sunset + EASE, TORONTO, WHITE).kelvin, WHITE);
    }

    #[test]
    fn a_polar_night_is_night_throughout_and_a_midnight_sun_day() {
        // 21 December and 21 June 2026 at noon UTC.
        assert_eq!(plan(1_797_854_400_000, TROMSO, 4000).kelvin, 4000);
        assert_eq!(plan(1_782_043_200_000, TROMSO, 4000).kelvin, WHITE);
    }

    #[test]
    fn where_the_laptop_is_comes_from_whereabouts() {
        let said = r#"{"v":2,"latitude":43.6748,"longitude":-79.3794,"accuracy":26000}"#;
        assert_eq!(
            Place::read(said),
            Some(Place {
                latitude: 43.6748,
                longitude: -79.3794
            })
        );
        assert_eq!(Place::read(r#"{"v":2}"#), None, "no place yet");
        assert_eq!(Place::read(r#"{"latitude": 91, "longitude": 0}"#), None);
        assert_eq!(Place::read("{} half written"), None);
    }
}

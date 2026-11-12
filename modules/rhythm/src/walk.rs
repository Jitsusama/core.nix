//! When to suggest a few minutes outside, and what the suggestion says.
//!
//! Daylight and fewer long stretches of sitting are the strongest signals
//! the light research found (RSCH-20261108-HGB8JZ, change 11), and Joel
//! asked for the prompts "only in daylight, fewer in poor weather, at most
//! three a day" (2026-11-11). So one comes only between sunrise and a
//! little before sunset, after about forty minutes at the machine without
//! a break, at a pause in his typing or at forty-five minutes whatever he's
//! doing; never within an hour and a half of the last, never during a call
//! or while Quiet holds notices, and at most three a day, one when the
//! weather is poor. Weather that can't be had restricts nothing.
//!
//! This file decides; keep.rs looks each minute and does what it says.

use crate::read::{Sun, clock};

/// Minutes at the machine without a break before a walk is suggested at a
/// pause in the typing.
pub const STEADY: i64 = 40;

/// Minutes at the machine after which it's suggested whatever he's doing.
pub const LONGEST: i64 = 45;

/// Minutes away that are a break already, so the stretch starts over.
pub const BREAK: i64 = 10;

/// Minutes between one suggestion and the next.
pub const APART: i64 = 90;

/// Suggestions in a day, and in a day of poor weather.
pub const A_DAY: usize = 3;
pub const A_POOR_DAY: usize = 1;

/// Minutes of daylight a walk needs left.
pub const LEFT: u32 = 20;

/// Minutes since the last one seen past which nobody is there to ask.
const GONE: i64 = 3;

/// How long Joel has been at the machine without a break, in minutes, to
/// the last minute seen, counting from no earlier than `since`. `seen` is
/// whole minutes in seconds since 1970, sorted.
pub fn stretch(seen: &[i64], since: i64) -> i64 {
    let Some(&last) = seen.last() else {
        return 0;
    };
    let mut first = last;
    for &moment in seen.iter().rev().skip(1) {
        if moment < since || first - moment >= BREAK * 60 {
            break;
        }
        first = moment;
    }
    (last - first) / 60 + 1
}

/// What a minute's look sees.
pub struct Look<'a> {
    /// Seconds since 1970.
    pub now: i64,
    /// Minutes past the local midnight.
    pub minute: u32,
    /// Whether someone is at the keys this minute.
    pub at_keys: bool,
    /// The minutes he's been at the machine, sorted.
    pub seen: &'a [i64],
    /// Today's suggestions so far, in seconds since 1970, sorted.
    pub walks: &'a [i64],
    pub sun: Option<&'a Sun>,
}

/// Whether a walk is due, before the weather, a call and Quiet are asked.
pub fn due(look: &Look) -> bool {
    let Some(sun) = look.sun else {
        return false;
    };
    if look.minute < sun.rise || look.minute + LEFT > sun.set || look.walks.len() >= A_DAY {
        return false;
    }
    let last_walk = look.walks.last().copied();
    if last_walk.is_some_and(|walk| look.now - walk < APART * 60) {
        return false;
    }
    let Some(&last) = look.seen.last() else {
        return false;
    };
    if look.now - last > GONE * 60 {
        return false;
    }
    let long = stretch(look.seen, last_walk.unwrap_or(i64::MIN));
    long >= LONGEST || (long >= STEADY && !look.at_keys)
}

/// Whether the day leaves room for another walk in this weather.
pub fn room(walks: usize, weather: Option<&Weather>) -> bool {
    let most = if weather.is_some_and(Weather::poor) {
        A_POOR_DAY
    } else {
        A_DAY
    };
    walks < most
}

/// The weather outside now, as Open-Meteo gives it.
#[derive(Clone, Debug, PartialEq)]
pub struct Weather {
    /// The air's temperature, and how it feels, in degrees Celsius.
    pub degrees: f64,
    pub feels: f64,
    /// Rain, showers and snow of the last quarter hour, in millimetres.
    pub wet: f64,
    /// WMO's code for what the sky is doing.
    pub code: u32,
    /// The strongest gusts, in kilometres an hour.
    pub gusts: f64,
}

/// What's asked of Open-Meteo for a place, the forecast the launcher's
/// weather reads too.
pub fn asked(latitude: f64, longitude: f64) -> String {
    format!(
        "https://api.open-meteo.com/v1/forecast?latitude={latitude}&longitude={longitude}\
         &current=temperature_2m,apparent_temperature,precipitation,weather_code,wind_gusts_10m"
    )
}

/// A number given for a key in JSON, read without a parser.
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

impl Weather {
    /// The weather from Open-Meteo's answer, read from its current values
    /// rather than their units, which come first under the same names.
    pub fn read(answer: &str) -> Option<Weather> {
        let current = answer.split_once("\"current\":")?.1;
        let code = figure(current, "weather_code")
            .filter(|code| (0.0..=99.0).contains(code) && code.fract() == 0.0)?;
        Some(Weather {
            degrees: figure(current, "temperature_2m")?,
            feels: figure(current, "apparent_temperature")?,
            wet: figure(current, "precipitation")?,
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "a code checked whole, from 0 to 99"
            )]
            code: code as u32,
            gusts: figure(current, "wind_gusts_10m")?,
        })
    }

    /// Whether it's poor weather for a walk: falling wet, a storm, gusts
    /// that push, or a cold or heat that bites.
    pub fn poor(&self) -> bool {
        self.wet > 0.0
            || self.code >= 51
            || self.gusts >= 50.0
            || !(-15.0..=32.0).contains(&self.feels)
    }

    /// What the sky is doing, in a word.
    fn sky(&self) -> &'static str {
        match self.code {
            45 | 48 => "foggy",
            51..=57 => "drizzling",
            61..=67 | 80..=82 => "raining",
            71..=77 | 85 | 86 => "snowing",
            95.. => "stormy",
            _ if self.wet > 0.0 => "wet",
            _ if self.gusts >= 50.0 => "windy",
            _ => "dry",
        }
    }
}

/// A temperature in whole degrees, never "-0".
fn degrees(celsius: f64) -> String {
    let whole = format!("{celsius:.0}");
    if whole == "-0" { "0".to_owned() } else { whole }
}

/// The suggestion's two lines: a question, and what it's like out.
pub fn said(weather: Option<&Weather>, sunset: u32) -> (String, String) {
    let out = "A few minutes outside?".to_owned();
    match weather {
        Some(weather) if weather.poor() => (
            "A few minutes on your feet?".to_owned(),
            format!(
                "{}° and {}, so maybe indoors",
                degrees(weather.degrees),
                weather.sky()
            ),
        ),
        Some(weather) => (
            out,
            format!(
                "{}° and {}, sun until {}",
                degrees(weather.degrees),
                weather.sky(),
                clock(sunset)
            ),
        ),
        None => (out, format!("Sun until {}", clock(sunset))),
    }
}

/// Whether Quiet holds notices now, as the shell writes its file:
/// "<since> <until>" in milliseconds, until 0 for until Joel ends it, or
/// nothing while he doesn't (modules/notices reads it the same way).
pub fn quiet(text: Option<&str>, now_ms: i64) -> bool {
    let Some(text) = text else {
        return false;
    };
    let mut said = text.split_whitespace();
    let (Some(_since), Some(until)) = (said.next(), said.next()) else {
        return false;
    };
    let until = until.parse::<i64>().unwrap_or(0);
    until == 0 || until > now_ms
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A sunny November day in southern Ontario, by the local clock.
    const SUN: Sun = Sun {
        rise: 7 * 60 + 5,
        set: 16 * 60 + 52,
    };

    /// Noon on some day, in seconds since 1970, with its local minute.
    const NOON: i64 = 20_000 * 86_400 + 12 * 3600;

    /// Every minute from `from` minutes before `now` to `to` before it.
    fn minutes(now: i64, from: i64, to: i64) -> Vec<i64> {
        (to..=from).rev().map(|back| now - back * 60).collect()
    }

    fn look<'a>(seen: &'a [i64], walks: &'a [i64], at_keys: bool) -> Look<'a> {
        Look {
            now: NOON,
            minute: 12 * 60,
            at_keys,
            seen,
            walks,
            sun: Some(&SUN),
        }
    }

    #[test]
    fn a_stretch_runs_back_to_the_last_break() {
        let mut seen = minutes(NOON, 90, 60);
        seen.extend(minutes(NOON, 45, 0));
        assert_eq!(
            stretch(&seen, i64::MIN),
            46,
            "fifteen minutes away start it over"
        );
        let mut seen = minutes(NOON, 50, 30);
        seen.extend(minutes(NOON, 25, 0));
        assert_eq!(
            stretch(&seen, i64::MIN),
            51,
            "five minutes away is no break"
        );
        assert_eq!(
            stretch(&seen, NOON - 20 * 60),
            21,
            "nor counted from before the last walk"
        );
    }

    #[test]
    fn a_walk_is_due_at_a_pause_after_forty_minutes() {
        let seen = minutes(NOON, 41, 1);
        assert!(due(&look(&seen, &[], false)), "a pause after 41 minutes");
        assert!(!due(&look(&seen, &[], true)), "not while typing, until 45");
        let seen = minutes(NOON, 30, 0);
        assert!(!due(&look(&seen, &[], false)), "nor after 31");
    }

    #[test]
    fn a_walk_is_due_at_forty_five_minutes_whatever_he_s_doing() {
        let seen = minutes(NOON, 45, 0);
        assert!(due(&look(&seen, &[], true)), "46 minutes typing");
    }

    #[test]
    fn nobody_is_asked_once_gone() {
        let seen = minutes(NOON, 60, 5);
        assert!(!due(&look(&seen, &[], false)), "five minutes gone");
    }

    #[test]
    fn walks_come_only_in_daylight_with_time_left() {
        let seen = minutes(NOON, 50, 0);
        let mut early = look(&seen, &[], true);
        early.minute = 6 * 60 + 50;
        assert!(!due(&early), "before sunrise");
        let mut late = look(&seen, &[], true);
        late.minute = 16 * 60 + 40;
        assert!(!due(&late), "twelve minutes before sunset");
        let mut nowhere = look(&seen, &[], true);
        nowhere.sun = None;
        assert!(!due(&nowhere), "with no sun known, no daylight is known");
    }

    #[test]
    fn walks_stay_apart_and_three_a_day() {
        let seen = minutes(NOON, 100, 0);
        let recent = [NOON - 60 * 60];
        assert!(!due(&look(&seen, &recent, true)), "an hour after the last");
        let earlier = [NOON - 100 * 60];
        assert!(due(&look(&seen, &earlier, true)), "100 minutes after it");
        let three = [NOON - 400 * 60, NOON - 300 * 60, NOON - 200 * 60];
        assert!(!due(&look(&seen, &three, true)), "a fourth in a day");
    }

    #[test]
    fn poor_weather_leaves_room_for_one() {
        let poor = Weather {
            degrees: 3.0,
            feels: 0.0,
            wet: 0.4,
            code: 61,
            gusts: 20.0,
        };
        let fine = Weather {
            wet: 0.0,
            code: 2,
            ..poor.clone()
        };
        assert!(room(0, Some(&poor)), "the first in the rain");
        assert!(!room(1, Some(&poor)), "not a second");
        assert!(room(2, Some(&fine)), "a third when it's fine");
        assert!(room(2, None), "weather unknown restricts nothing");
    }

    #[test]
    fn open_meteo_s_current_weather_is_read_past_its_units() {
        let answer = r#"{"latitude":43.67,"current_units":{"time":"iso8601",
            "temperature_2m":"°C","apparent_temperature":"°C","precipitation":"mm",
            "weather_code":"wmo code","wind_gusts_10m":"km/h"},"current":{"time":
            "2026-11-12T12:00","interval":900,"temperature_2m":-0.4,
            "apparent_temperature":-5.2,"precipitation":0.00,"weather_code":3,
            "wind_gusts_10m":31.7}}"#;
        assert_eq!(
            Weather::read(answer),
            Some(Weather {
                degrees: -0.4,
                feels: -5.2,
                wet: 0.0,
                code: 3,
                gusts: 31.7
            }),
            "the current values, not the units"
        );
        assert_eq!(Weather::read("{}"), None, "nothing to read");
    }

    #[test]
    fn what_poor_weather_is() {
        let fine = Weather {
            degrees: 6.0,
            feels: 3.0,
            wet: 0.0,
            code: 1,
            gusts: 25.0,
        };
        assert!(!fine.poor(), "cool and dry");
        for poor in [
            Weather {
                wet: 0.2,
                ..fine.clone()
            },
            Weather {
                code: 71,
                ..fine.clone()
            },
            Weather {
                gusts: 60.0,
                ..fine.clone()
            },
            Weather {
                feels: -22.0,
                ..fine.clone()
            },
            Weather {
                feels: 35.0,
                ..fine.clone()
            },
        ] {
            assert!(poor.poor(), "{poor:?} is poor");
        }
    }

    #[test]
    fn the_suggestion_says_the_weather_and_the_sunset() {
        let fine = Weather {
            degrees: 6.3,
            feels: 3.0,
            wet: 0.0,
            code: 1,
            gusts: 25.0,
        };
        assert_eq!(
            said(Some(&fine), SUN.set),
            (
                "A few minutes outside?".to_owned(),
                "6° and dry, sun until 16:52".to_owned()
            ),
            "fine weather"
        );
        let wet = Weather {
            degrees: -0.2,
            wet: 1.0,
            code: 63,
            ..fine
        };
        assert_eq!(
            said(Some(&wet), SUN.set),
            (
                "A few minutes on your feet?".to_owned(),
                "0° and raining, so maybe indoors".to_owned()
            ),
            "poor weather, never minus nought"
        );
        assert_eq!(said(None, SUN.set).1, "Sun until 16:52", "weather unknown");
    }

    #[test]
    fn quiet_holds_while_its_file_says_so() {
        assert!(!quiet(None, 5_000), "no file");
        assert!(!quiet(Some(""), 5_000), "an empty file");
        assert!(quiet(Some("1000 0"), 5_000), "until Joel ends it");
        assert!(quiet(Some("1000 9000"), 5_000), "until a time to come");
        assert!(!quiet(Some("1000 4000"), 5_000), "a time gone");
    }
}

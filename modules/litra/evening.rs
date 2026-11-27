//! The evening's light: when tonight's wind-down is, by rhythm's profile and
//! the clock, and each light's own setting, kept by its serial, beside the
//! warmer and lower one a call is lit at after it.

use std::ffi::{c_char, c_int, c_long};
use std::fs;
use std::path::Path;

use crate::light::{DIMMEST, State, WARMEST};

/// Where the evening's light comes from: rhythm's profile, which says when
/// Joel winds down tonight, and the file each light's own setting is kept
/// in.
pub(crate) struct Evening<'a> {
    pub(crate) profile: &'a Path,
    pub(crate) kept: &'a Path,
}

/// The share of its own brightness, in percent, a light keeps on a call
/// after Joel's wind-down.
const EVENING_SHARE: u32 = 70;
/// When the evening ends, in minutes after midnight, as rhythm's does.
const MORNING: u16 = 6 * 60;

/// A light's brightness, in its own units, and its warmth.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Setting {
    pub(crate) units: u16,
    pub(crate) kelvin: u16,
}

impl Setting {
    /// What a light that has said its brightness and warmth is set to.
    pub(crate) fn of(state: &State) -> Option<Self> {
        Some(Self {
            units: state.units?,
            kelvin: state.kelvin?,
        })
    }

    /// The setting a call after the wind-down lights it at: warm, and a
    /// little lower, never below the least it gives.
    pub(crate) fn for_evening(self) -> Self {
        let lower = u32::from(self.units) * EVENING_SHARE / 100;
        Self {
            units: u16::try_from(lower).unwrap_or(self.units).max(DIMMEST),
            kelvin: WARMEST,
        }
    }
}

/// A light's own setting, from what it shows as a call lights it and what
/// was kept of it: the kept one while the light still shows the evening's
/// of it, since the evening left it there, and otherwise what it shows,
/// which Joel set.
pub(crate) fn own(seen: Setting, kept: Option<Setting>) -> Setting {
    match kept {
        Some(kept) if kept.for_evening() == seen => kept,
        _ => seen,
    }
}

/// The own settings kept, by each light's serial, one a line: its serial,
/// its units and its kelvin. A line short of any is passed over.
pub(crate) fn read_kept(text: &str) -> Vec<(String, Setting)> {
    text.lines()
        .filter_map(|line| {
            let mut words = line.trim().rsplitn(3, ' ');
            let kelvin = words.next()?.parse().ok()?;
            let units = words.next()?.parse().ok()?;
            let serial = words.next()?.trim();
            (!serial.is_empty()).then(|| (serial.to_owned(), Setting { units, kelvin }))
        })
        .collect()
}

/// The own settings kept, as `read_kept` reads them.
pub(crate) fn written_kept(kept: &[(String, Setting)]) -> String {
    use std::fmt::Write as _;
    kept.iter().fold(String::new(), |mut text, (serial, own)| {
        let _ = writeln!(text, "{serial} {} {}", own.units, own.kelvin);
        text
    })
}

/// A time of day, `HH:MM`, in minutes after midnight.
fn minutes(text: &str) -> Option<u16> {
    let (hour, minute) = text.split_once(':')?;
    let (hour, minute): (u16, u16) = (hour.parse().ok()?, minute.parse().ok()?);
    (hour < 24 && minute < 60).then_some(hour * 60 + minute)
}

/// Tonight's wind-down, in minutes after midnight, from rhythm's profile,
/// which writes it as `"windDown": "HH:MM"`.
fn wind_down(profile: &str) -> Option<u16> {
    let (_, after) = profile.split_once("\"windDown\"")?;
    let value = after.trim_start().strip_prefix(':')?.trim_start();
    let (time, _) = value.strip_prefix('"')?.split_once('"')?;
    minutes(time)
}

/// Whether a minute of the day is after the wind-down and before morning,
/// for a wind-down in the evening or one past midnight.
fn evening(now: u16, wind_down: u16) -> bool {
    if wind_down < MORNING {
        now >= wind_down && now < MORNING
    } else {
        now >= wind_down || now < MORNING
    }
}

/// glibc's `struct tm`, which it fills whole; only the hour and minute
/// are read.
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

unsafe extern "C" {
    fn time(now: *mut i64) -> i64;
    fn localtime_r(now: *const i64, tm: *mut Tm) -> *mut Tm;
}

/// The minute of the day here, by the machine's time zone.
fn local_minute() -> Option<u16> {
    // SAFETY: a plain call, and a `struct tm` of our own for it to fill,
    // all of whose fields may be zero.
    let tm = unsafe {
        let now = time(std::ptr::null_mut());
        let mut tm: Tm = std::mem::zeroed();
        if localtime_r(&raw const now, &raw mut tm).is_null() {
            return None;
        }
        tm
    };
    u16::try_from(tm.hour * 60 + tm.minute).ok()
}

/// Whether it's evening by tonight's wind-down in rhythm's profile; never
/// while there's no profile to say when that is.
pub(crate) fn tonight(profile: &Path) -> bool {
    let down = fs::read_to_string(profile)
        .ok()
        .and_then(|text| wind_down(&text));
    matches!((down, local_minute()), (Some(down), Some(now)) if evening(now, down))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_evening_call_lights_it_warm_and_a_little_lower() {
        let own = Setting {
            units: 200,
            kelvin: 5000,
        };
        assert_eq!(
            own.for_evening(),
            Setting {
                units: 140,
                kelvin: WARMEST
            }
        );
        let dim = Setting {
            units: 25,
            kelvin: 4000,
        };
        assert_eq!(
            dim.for_evening().units,
            DIMMEST,
            "the evening took it below the least it gives"
        );
    }

    #[test]
    fn a_light_tells_its_setting_once_it_has_said_everything() {
        let said = State {
            on: Some(false),
            units: Some(120),
            kelvin: Some(4500),
        };
        assert_eq!(
            Setting::of(&said),
            Some(Setting {
                units: 120,
                kelvin: 4500
            })
        );
        let short = State {
            kelvin: None,
            ..said
        };
        assert_eq!(Setting::of(&short), None);
    }

    #[test]
    fn its_own_setting_survives_the_evening_but_not_joels_hand() {
        let mine = Setting {
            units: 200,
            kelvin: 5000,
        };
        let evenings = Setting {
            units: 140,
            kelvin: WARMEST,
        };
        assert_eq!(
            own(evenings, Some(mine)),
            mine,
            "the evening's was taken for his"
        );
        let picked = Setting {
            units: 90,
            kelvin: 3500,
        };
        assert_eq!(own(picked, Some(mine)), picked, "his own pick was undone");
        assert_eq!(own(picked, None), picked);
    }

    #[test]
    fn own_settings_are_kept_by_serial() {
        let kept = vec![
            (
                "2241FE30".to_owned(),
                Setting {
                    units: 200,
                    kelvin: 5000,
                },
            ),
            (
                "9A1C".to_owned(),
                Setting {
                    units: 20,
                    kelvin: 2700,
                },
            ),
        ];
        assert_eq!(read_kept(&written_kept(&kept)), kept);
        assert_eq!(
            read_kept("2241FE30 200\nnonsense\n\n9A1C 20 2700\n"),
            kept[1..],
            "a line short of a setting was taken"
        );
    }

    #[test]
    fn tonights_wind_down_is_read_from_rhythms_profile() {
        let profile = concat!(
            r#"{"v": 1, "basis": "learned", "tonight": "22:45", "#,
            r#""windDown": "20:15", "sunset": "17:05"}"#
        );
        assert_eq!(wind_down(profile), Some(20 * 60 + 15));
        assert_eq!(wind_down(r#"{"windDown":"00:30"}"#), Some(30));
        assert_eq!(wind_down(r#"{"windDown": "25:00"}"#), None);
        assert_eq!(wind_down(r#"{"windDown": null}"#), None);
        assert_eq!(wind_down(""), None);
    }

    #[test]
    fn the_clock_reads_a_minute_of_the_day() {
        let now = local_minute();
        assert!(
            now.is_some_and(|now| now < 24 * 60),
            "the clock read {now:?}"
        );
        assert!(
            !tonight(Path::new("/nonexistent/profile.json")),
            "no profile, yet the evening"
        );
    }

    #[test]
    fn the_evening_runs_from_the_wind_down_to_morning() {
        let at = |hour: u16, minute: u16| hour * 60 + minute;
        let down = at(20, 15);
        for (now, is) in [
            (at(12, 0), false),
            (at(20, 14), false),
            (at(20, 15), true),
            (at(23, 59), true),
            (at(0, 30), true),
            (at(5, 59), true),
            (at(6, 0), false),
        ] {
            assert_eq!(
                evening(now, down),
                is,
                "at {now} after a wind-down at 20:15"
            );
        }
        let late = at(0, 30);
        for (now, is) in [
            (at(23, 0), false),
            (at(0, 29), false),
            (at(0, 30), true),
            (at(5, 0), true),
            (at(6, 0), false),
        ] {
            assert_eq!(
                evening(now, late),
                is,
                "at {now} after a wind-down at 00:30"
            );
        }
    }
}

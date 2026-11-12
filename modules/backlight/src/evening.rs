//! The evening's dimming: from a time of Joel's evening to a time of his
//! morning, the built-in screen eases to half the brightness the curve asks
//! for, over half an hour, and eases back over the half hour after.
//!
//! Why: the light that sets the body clock falls as the screen's brightness
//! does, and brightness can take far more of it away than warmth can (the
//! light research, RSCH-20261108-HGB8JZ, change 2: brightness spans about
//! eightfold, while warming to 4000 K takes off about half). It's tied to
//! the clock, not
//! the sun, since the evening that matters is the one before sleep, and in
//! November the sun sets five hours before it.
//!
//! A brightness Joel picks in the evening is learned as what he'd want by
//! day in that light, so a pick both sets the screen now and is still
//! dimmed the next evening. The dimming never takes the screen under
//! `FLOOR`, about 20 nits, so text stays easy to read; a screen the curve
//! already sets dimmer than that is left as it is.

/// What the evening leaves of the brightness, once it's fully under way.
pub const SHARE: f64 = 0.5;

/// How long, in minutes, the evening takes to ease in, and the morning to
/// ease out.
pub const EASE: f64 = 30.0;

/// The least the dimming takes the screen to, as a share of its brightest:
/// about 20 nits on a 400-nit panel.
pub const FLOOR: f64 = 0.05;

/// Minutes in a day.
const DAY: f64 = 1440.0;

/// An evening, from its start to its end, each in minutes past midnight by
/// the local clock; the end can fall the next day.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Evening {
    start: f64,
    end: f64,
}

impl Evening {
    /// An evening as the command line gives it, "21:30-05:00".
    pub fn read(said: &str) -> Option<Self> {
        let (start, end) = said.split_once('-')?;
        Some(Evening {
            start: minute(start)?,
            end: minute(end)?,
        })
    }

    /// What the evening leaves of the brightness at `minute` past midnight:
    /// all of it by day, easing to `SHARE` over the half hour from its start,
    /// and back over the half hour from its end.
    pub fn share(&self, minute: f64) -> f64 {
        let into = (minute - self.start).rem_euclid(DAY);
        let length = (self.end - self.start).rem_euclid(DAY);
        let depth = if into < length {
            (into / EASE).min(1.0)
        } else if into < length + EASE {
            1.0 - (into - length) / EASE
        } else {
            0.0
        };
        1.0 - (1.0 - SHARE) * depth.clamp(0.0, 1.0)
    }
}

/// "21:30" as minutes past midnight.
fn minute(said: &str) -> Option<f64> {
    let (hour, minute) = said.split_once(':')?;
    let hour: u8 = hour.parse().ok().filter(|hour| *hour < 24)?;
    let minute: u8 = minute.parse().ok().filter(|minute| *minute < 60)?;
    Some(f64::from(hour) * 60.0 + f64::from(minute))
}

/// The brightness the screen goes to when the curve asks for `level` and
/// the evening leaves `share` of it.
pub fn dimmed(level: f64, share: f64) -> f64 {
    (level * share).max(FLOOR.min(level))
}

/// The brightness by day that `dimmed` takes to `level`, so a pick in the
/// evening teaches the curve what Joel would pick in that light by day.
pub fn undimmed(level: f64, share: f64) -> f64 {
    if level <= FLOOR {
        level
    } else {
        (level / share).clamp(FLOOR, 1.0)
    }
}

/// glibc's broken-down time.
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

/// Minutes past midnight by the local clock, for `wall` milliseconds since
/// 1970 began.
pub fn local_minute(wall: i64) -> Option<f64> {
    let seconds = wall.div_euclid(1000);
    let mut tm = Tm::default();
    // SAFETY: both pointers are to values alive across the call.
    if unsafe { localtime_r(&raw const seconds, &raw mut tm) }.is_null() {
        return None;
    }
    Some(f64::from(tm.hour) * 60.0 + f64::from(tm.min) + f64::from(tm.sec) / 60.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evening() -> Evening {
        Evening::read("21:30-05:00").expect("a well-formed evening")
    }

    fn at(hour: f64, minute: f64) -> f64 {
        evening().share(hour * 60.0 + minute)
    }

    #[test]
    fn the_day_keeps_all_of_the_brightness() {
        for (hour, minute) in [(5.0, 30.0), (12.0, 0.0), (21.0, 29.0)] {
            assert!(
                (at(hour, minute) - 1.0).abs() < 1e-9,
                "at {hour}:{minute} the day dimmed to {}",
                at(hour, minute)
            );
        }
    }

    #[test]
    fn the_evening_eases_in_over_half_an_hour() {
        assert!((at(21.0, 45.0) - 0.75).abs() < 1e-9, "a quarter hour in");
        assert!((at(22.0, 0.0) - SHARE).abs() < 1e-9, "half an hour in");
        assert!((at(2.0, 0.0) - SHARE).abs() < 1e-9, "past midnight");
    }

    #[test]
    fn the_morning_eases_out_over_half_an_hour() {
        assert!((at(5.0, 0.0) - SHARE).abs() < 1e-9, "at its end");
        assert!((at(5.0, 15.0) - 0.75).abs() < 1e-9, "a quarter hour on");
    }

    #[test]
    fn an_evening_that_cant_be_is_refused() {
        for said in ["21:30", "25:00-05:00", "21:30-05:60", "evening"] {
            assert_eq!(Evening::read(said), None, "{said} read as an evening");
        }
    }

    #[test]
    fn dimming_halves_down_to_the_floor_and_no_further() {
        assert!((dimmed(0.6, SHARE) - 0.3).abs() < 1e-9, "halved");
        assert!(
            (dimmed(0.08, SHARE) - FLOOR).abs() < 1e-9,
            "held at the floor"
        );
        assert!((dimmed(0.03, SHARE) - 0.03).abs() < 1e-9, "left under it");
        assert!((dimmed(0.6, 1.0) - 0.6).abs() < 1e-9, "by day");
    }

    #[test]
    fn an_evening_pick_teaches_what_the_day_would_want() {
        assert!((undimmed(0.3, SHARE) - 0.6).abs() < 1e-9, "doubled");
        assert!(
            (undimmed(0.7, SHARE) - 1.0).abs() < 1e-9,
            "held at the brightest"
        );
        assert!(
            (undimmed(0.03, SHARE) - 0.03).abs() < 1e-9,
            "left under the floor"
        );
    }

    #[test]
    fn the_local_clock_reads_a_minute_of_the_day() {
        let minute = local_minute(0).expect("the epoch reads locally");
        assert!(
            (0.0..DAY).contains(&minute),
            "{minute} isn't a minute of a day"
        );
    }
}

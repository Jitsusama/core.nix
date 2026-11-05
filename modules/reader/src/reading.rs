//! How a row too long for gO's list reads on while it's chosen, as
//! docs/design.md's "A long name reads on" has the launcher's rows do
//! (its Reading.js and ReadsOn.qml): after half a second the words slide
//! until their end shows, at a steady twelve figures a second with soft
//! ends, rest a second there, glide back to their start and rest again,
//! for as long as the row stays chosen.

use std::time::{Duration, Instant};

/// How long the words rest at their start, to read how they begin, and
/// so a held arrow, choosing a row each repeat, never sets one going.
const REST_FIRST: f64 = 0.5;
/// How long they rest at their end, to read it.
const REST_LAST: f64 = 1.0;
/// The longest each soft end of the slide lasts, in seconds; a slide the
/// pace alone would take less than two ramps for ramps for half that
/// time, so the pace still holds between them.
const RAMP: f64 = 0.2;
/// How many figures a second the slide reads at.
pub const FIGURES_A_SECOND: f64 = 12.0;

/// Words reading on: how far past their room they run and the pace they
/// slide at, both in pixels, and when their row was chosen.
pub struct Reading {
    over: f64,
    pace: f64,
    from: Instant,
}

/// Where the words are at a moment: how far slid from their start, and
/// whether they're moving, which needs every frame, or resting until a
/// moment, which needs none until then.
#[derive(Debug, PartialEq)]
pub struct At {
    pub slid: f64,
    pub resting_until: Option<Instant>,
}

impl Reading {
    pub fn new(over: f64, pace: f64, from: Instant) -> Reading {
        Reading {
            over: over.max(0.0),
            pace: pace.max(1.0),
            from,
        }
    }

    /// How long the slide out lasts, and each of its soft ends: the pace
    /// gathered evenly from rest covers half the ground the pace would, so
    /// it lasts a ramp longer than the pace alone would take.
    fn slide(&self) -> (f64, f64) {
        let steady = self.over / self.pace;
        let ramp = RAMP.min(steady / 2.0);
        (steady + ramp, ramp)
    }

    /// How far the slide out has gone `t` seconds in: speeding up evenly,
    /// the pace held, then slowing evenly, the pieces meeting at one speed.
    fn slid_out(&self, t: f64) -> f64 {
        let (seconds, ramp) = self.slide();
        if ramp <= 0.0 {
            return self.over;
        }
        let t = t.clamp(0.0, seconds);
        if t < ramp {
            self.pace * t * t / (2.0 * ramp)
        } else if t < seconds - ramp {
            self.pace * ramp / 2.0 + self.pace * (t - ramp)
        } else {
            let left = seconds - t;
            self.over - self.pace * left * left / (2.0 * ramp)
        }
    }

    /// Where the words are at `now`.
    pub fn at(&self, now: Instant) -> At {
        let (seconds, _) = self.slide();
        let glide = crate::motion::ARRIVING.as_secs_f64();
        let period = REST_FIRST + seconds + REST_LAST + glide;
        let since = now.saturating_duration_since(self.from).as_secs_f64();
        let round = (since / period).floor();
        let t = since - round * period;
        let began = self.from + Duration::from_secs_f64(round * period);
        let until = |s: f64| Some(began + Duration::from_secs_f64(s));
        if t < REST_FIRST {
            At {
                slid: 0.0,
                resting_until: until(REST_FIRST),
            }
        } else if t < REST_FIRST + seconds {
            At {
                slid: self.slid_out(t - REST_FIRST),
                resting_until: None,
            }
        } else if t < REST_FIRST + seconds + REST_LAST {
            At {
                slid: self.over,
                resting_until: until(REST_FIRST + seconds + REST_LAST),
            }
        } else {
            let back = (t - REST_FIRST - seconds - REST_LAST) / glide;
            At {
                slid: self.over * (1.0 - crate::motion::eased(back)),
                resting_until: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(reading: &Reading, seconds: f64) -> At {
        reading.at(reading.from + Duration::from_secs_f64(seconds))
    }

    #[test]
    fn the_words_rest_at_their_start_for_half_a_second() {
        let reading = Reading::new(120.0, 84.0, Instant::now());
        for seconds in [0.0, 0.25, 0.49] {
            let at = after(&reading, seconds);
            assert!(at.slid.abs() < 1e-9, "slid {} at {seconds} s", at.slid);
            assert_eq!(
                at.resting_until,
                Some(reading.from + Duration::from_secs_f64(REST_FIRST))
            );
        }
    }

    #[test]
    fn the_slide_holds_its_pace_between_soft_ends() {
        let reading = Reading::new(300.0, 84.0, Instant::now());
        let (seconds, ramp) = reading.slide();
        let step = 0.001;
        let speed = |t: f64| (reading.slid_out(t + step) - reading.slid_out(t)) / step;
        assert!(speed(0.0) < 1.0, "it starts from rest");
        assert!(
            (speed(seconds / 2.0) - 84.0).abs() < 0.5,
            "it holds the pace"
        );
        assert!(speed(seconds - step) < 1.0, "it settles back to rest");
        let mut t = 0.0;
        while t < seconds {
            assert!(speed(t) <= 84.0 + 0.5, "never past the pace, at {t} s");
            t += step;
        }
        assert!((reading.slid_out(seconds) - 300.0).abs() < 1e-6);
        assert!((ramp - RAMP).abs() < 1e-9);
    }

    #[test]
    fn a_short_slide_ramps_for_half_its_time() {
        let reading = Reading::new(14.0, 84.0, Instant::now());
        let (seconds, ramp) = reading.slide();
        assert!(ramp < RAMP);
        assert!((reading.slid_out(seconds) - 14.0).abs() < 1e-6);
        assert!(reading.slid_out(seconds / 2.0) > 0.0);
    }

    #[test]
    fn the_words_rest_a_second_at_their_end_then_glide_back_and_rest_again() {
        let reading = Reading::new(84.0, 84.0, Instant::now());
        let (seconds, _) = reading.slide();
        let end = REST_FIRST + seconds;
        let rest = after(&reading, end + 0.5);
        assert!((rest.slid - 84.0).abs() < 1e-9);
        assert!(rest.resting_until.is_some());
        let gliding = after(&reading, end + REST_LAST + 0.05);
        assert!(gliding.slid > 0.0 && gliding.slid < 84.0);
        assert_eq!(gliding.resting_until, None);
        let period = end + REST_LAST + crate::motion::ARRIVING.as_secs_f64();
        let again = after(&reading, period + 0.1);
        assert!(again.slid.abs() < 1e-9, "back at its start, resting");
        assert!(again.resting_until.is_some());
        assert!(after(&reading, period + REST_FIRST + 0.3).slid > 0.0);
    }
}

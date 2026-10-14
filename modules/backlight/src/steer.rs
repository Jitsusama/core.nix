//! When the screen follows the light, and how it moves when it does.
//!
//! Not for every flicker: a change has to be worth seeing, and the light has
//! to hold. Brighter light is followed after a second, since a dim screen in
//! sunlight is hard to read; dimmer light after four, so a hand or a passing
//! shadow over the sensor changes nothing.

use std::time::{Duration, Instant};

/// A change smaller than this, as a share of the brightest, isn't worth a
/// move.
pub const SLIGHT: f64 = 0.03;

/// How long brighter light has to hold before the screen follows it.
pub const BRIGHTENING: Duration = Duration::from_secs(1);

/// How long dimmer light has to hold.
pub const DIMMING: Duration = Duration::from_secs(4);

/// A move the light asked for, waiting to see whether the light holds.
#[derive(Clone, Copy, Debug)]
struct Pending {
    level: f64,
    brighter: bool,
    due: Instant,
}

#[derive(Debug, Default)]
pub struct Steer {
    pending: Option<Pending>,
}

impl Steer {
    /// The light now wants the screen at `wanted`, and it's at `shown`. A
    /// change too slight to see forgets any move waiting, since the light
    /// came back. A change the same way as the one waiting only updates where
    /// it goes, so light that keeps on changing is still followed in time.
    pub fn heard(&mut self, wanted: f64, shown: f64, now: Instant) {
        if (wanted - shown).abs() < SLIGHT {
            self.pending = None;
            return;
        }
        let brighter = wanted > shown;
        match &mut self.pending {
            Some(pending) if pending.brighter == brighter => pending.level = wanted,
            _ => {
                let wait = if brighter { BRIGHTENING } else { DIMMING };
                self.pending = Some(Pending {
                    level: wanted,
                    brighter,
                    due: now + wait,
                });
            }
        }
    }

    /// When the move waiting is due, if one is.
    pub fn next(&self) -> Option<Instant> {
        self.pending.map(|pending| pending.due)
    }

    /// The level to move to, once the light has held long enough.
    pub fn due(&mut self, now: Instant) -> Option<f64> {
        let pending = self.pending.filter(|pending| pending.due <= now)?;
        self.pending = None;
        Some(pending.level)
    }

    /// Forgets the move waiting, when Joel picks a brightness himself.
    pub fn forget(&mut self) {
        self.pending = None;
    }
}

/// A move from one brightness to another, eased out the way the desktop's
/// surfaces move: quick to start, settling gently. A longer way takes longer,
/// from a quarter of a second to a second.
#[derive(Clone, Copy, Debug)]
pub struct Ramp {
    from: f64,
    to: f64,
    start: Instant,
    length: Duration,
}

impl Ramp {
    pub fn new(from: f64, to: f64, start: Instant) -> Self {
        let length = Duration::from_millis(250) + Duration::from_secs_f64(0.75 * (to - from).abs());
        Ramp {
            from,
            to,
            start,
            length,
        }
    }

    /// Where the move goes.
    pub fn to(&self) -> f64 {
        self.to
    }

    /// The brightness the move has reached by now.
    pub fn at(&self, now: Instant) -> f64 {
        let share = (now.saturating_duration_since(self.start).as_secs_f64()
            / self.length.as_secs_f64())
        .min(1.0);
        let eased = 1.0 - (1.0 - share).powi(3);
        self.from + (self.to - self.from) * eased
    }

    /// Whether the move has arrived.
    pub fn done(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.start) >= self.length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MOMENT: Duration = Duration::from_millis(100);

    #[test]
    fn a_slight_change_moves_nothing() {
        let mut steer = Steer::default();
        steer.heard(0.51, 0.5, Instant::now());
        assert_eq!(steer.next(), None);
    }

    #[test]
    fn brighter_light_is_followed_once_it_has_held_a_second() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.8, 0.5, start);
        assert_eq!(steer.due(start + BRIGHTENING - MOMENT), None);
        assert_eq!(steer.due(start + BRIGHTENING), Some(0.8));
        assert_eq!(steer.next(), None);
    }

    #[test]
    fn dimmer_light_waits_longer() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.2, 0.5, start);
        assert_eq!(steer.due(start + BRIGHTENING), None);
        assert_eq!(steer.due(start + DIMMING), Some(0.2));
    }

    #[test]
    fn a_shadow_that_passes_changes_nothing() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.2, 0.5, start);
        steer.heard(0.5, 0.5, start + Duration::from_secs(1));
        assert_eq!(steer.due(start + DIMMING), None);
    }

    #[test]
    fn light_that_keeps_changing_the_same_way_is_still_followed_in_time() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.6, 0.5, start);
        steer.heard(0.7, 0.5, start + MOMENT * 5);
        steer.heard(0.8, 0.5, start + MOMENT * 9);
        assert_eq!(steer.due(start + BRIGHTENING), Some(0.8));
    }

    #[test]
    fn light_that_turns_back_waits_afresh() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.8, 0.5, start);
        let turned = start + MOMENT * 5;
        steer.heard(0.2, 0.5, turned);
        assert_eq!(steer.due(start + BRIGHTENING), None);
        assert_eq!(steer.due(turned + DIMMING), Some(0.2));
    }

    #[test]
    fn a_pick_forgets_the_move_waiting() {
        let start = Instant::now();
        let mut steer = Steer::default();
        steer.heard(0.8, 0.5, start);
        steer.forget();
        assert_eq!(steer.due(start + DIMMING), None);
    }

    #[test]
    fn a_ramp_starts_where_it_was_and_arrives_where_it_goes() {
        let start = Instant::now();
        let ramp = Ramp::new(0.2, 0.6, start);
        assert!((ramp.at(start) - 0.2).abs() < 1e-9);
        let end = start + Duration::from_secs(1);
        assert!(ramp.done(end));
        assert!((ramp.at(end) - 0.6).abs() < 1e-9);
    }

    #[test]
    fn a_ramp_is_past_halfway_at_half_its_time() {
        let start = Instant::now();
        let ramp = Ramp::new(0.0, 1.0, start);
        let half = start + ramp.length / 2;
        assert!(!ramp.done(half));
        assert!(ramp.at(half) > 0.5);
    }

    #[test]
    fn a_longer_way_takes_longer() {
        let start = Instant::now();
        let short = Ramp::new(0.5, 0.55, start);
        let long = Ramp::new(0.0, 1.0, start);
        assert_eq!(
            short.length,
            Duration::from_millis(250) + Duration::from_secs_f64(0.0375)
        );
        assert_eq!(long.length, Duration::from_secs(1));
    }
}

//! The units the shaper's rates are kept in, and what each direction of a
//! link may be held to.
//!
//! The rates themselves come from the one-way controller (src/oneway.rs,
//! run live by src/live.rs), from each direction's own delay as the
//! recorder measures it. The controller before it found them from pings
//! to distant hosts, and went on 18 November: five faults in four days
//! came from guessing whose queue a round trip shows, and on 3 November it
//! held a 312 Mbit/s download at 81.

/// A count, a rate in kbit/s or a time in microseconds, as a float. Exact:
/// every one here is far below 2⁵³.
#[expect(clippy::cast_precision_loss, reason = "every one is far below 2⁵³")]
pub fn float(n: u64) -> f64 {
    n as f64
}

/// A rate worked out as a float, to the nearest whole kbit/s. A negative
/// rate is none, and every caller holds the result within its bounds.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "rounded, held at zero or more, and within its bounds by every caller"
)]
pub fn whole(kbit: f64) -> u64 {
    kbit.round().max(0.0) as u64
}

/// Rates are in kbit/s, as tc takes them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: u64,
    pub max: u64,
}

/// One direction of a link: the rate the shaper holds it to now, within
/// what it may be held to.
#[derive(Clone, Debug, PartialEq)]
pub struct Direction {
    pub bounds: Bounds,
    pub rate: u64,
}

impl Direction {
    /// A direction held at `rate`, kept within its bounds.
    pub fn new(bounds: Bounds, rate: u64) -> Self {
        Direction {
            bounds,
            rate: rate.clamp(bounds.min, bounds.max),
        }
    }

    /// Holds the direction under a link's new most, `max`, never under its
    /// floor, answering whether the most moved. A slower link brings the
    /// rate down with it; a faster one leaves it where it is, for the
    /// controller to find.
    pub fn bound(&mut self, max: u64) -> bool {
        let max = max.max(self.bounds.min);
        if max == self.bounds.max {
            return false;
        }
        self.bounds.max = max;
        self.rate = self.rate.min(max);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Bounds = Bounds {
        min: 5_000,
        max: 900_000,
    };

    #[test]
    fn a_rate_starts_within_its_bounds() {
        assert_eq!(
            Direction::new(BOUNDS, 1_000_000).rate,
            900_000,
            "over the most"
        );
        assert_eq!(Direction::new(BOUNDS, 1_000).rate, 5_000, "under the floor");
        assert_eq!(Direction::new(BOUNDS, 50_000).rate, 50_000, "between");
    }

    #[test]
    fn a_link_slowed_brings_down_what_was_held_over_its_new_most() {
        let mut d = Direction::new(BOUNDS, 200_000);
        assert!(d.bound(93_000), "a slower link's most moved");
        assert_eq!(d.rate, 93_000, "the rate held over the new most");
        assert!(!d.bound(93_000), "the same most moves nothing");
        assert!(d.bound(930_000), "a faster link's most moved");
        assert_eq!(d.bounds.max, 930_000, "the new most");
        assert_eq!(d.rate, 93_000, "a faster link is found, not assumed");
    }

    #[test]
    fn a_most_under_the_floor_is_held_at_it() {
        let mut d = Direction::new(BOUNDS, 50_000);
        assert!(d.bound(10), "the most moved, to the floor");
        assert_eq!((d.bounds.max, d.rate), (5_000, 5_000), "held at the floor");
    }
}

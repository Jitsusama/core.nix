//! How the view moves on a key, by design.md's Motion: one ease-out curve,
//! (0.23, 1, 0.32, 1), 150 ms when arriving. A jump first holds the old view
//! until the bands it lands on are drawn, for 50 ms at most, so it never
//! shows bare ground and never feels stuck.

use std::time::{Duration, Instant};

/// How long a move takes to arrive.
pub const ARRIVING: Duration = Duration::from_millis(150);
/// How long a jump waits for the bands it lands on, at most.
pub const HOLD_AT_MOST: Duration = Duration::from_millis(50);

/// The ease-out curve at `x`, from 0 to 1: the cubic Bézier through (0, 0),
/// (0.23, 1), (0.32, 1) and (1, 1), solved for its x by Newton's method.
pub fn eased(x: f64) -> f64 {
    let (x1, y1, x2, y2) = (0.23, 1.0, 0.32, 1.0);
    let bezier = |t: f64, a: f64, b: f64| {
        let u = 1.0 - t;
        3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t
    };
    let slope = |t: f64, a: f64, b: f64| {
        let u = 1.0 - t;
        3.0 * u * u * a + 6.0 * u * t * (b - a) + 3.0 * t * t * (1.0 - b)
    };
    let x = x.clamp(0.0, 1.0);
    let mut t = x;
    for _ in 0..8 {
        let d = slope(t, x1, x2);
        if d.abs() < 1e-6 {
            break;
        }
        t = (t - (bezier(t, x1, x2) - x) / d).clamp(0.0, 1.0);
    }
    bezier(t, y1, y2)
}

/// A jump under way: where it leaves from, where it lands, and when it was
/// asked for and when it began to move.
pub struct Jump {
    pub from: f64,
    pub to: f64,
    pub asked: Instant,
    pub moving: Option<Instant>,
}

impl Jump {
    /// A jump from `now` to `to`. One farther than two screens starts its
    /// move a screen short of where it lands, since the bands between were
    /// never drawn.
    pub fn new(now: f64, to: f64, screen: f64, asked: Instant) -> Jump {
        let from = if (to - now).abs() > 2.0 * screen {
            to - screen * (to - now).signum()
        } else {
            now
        };
        Jump {
            from,
            to,
            asked,
            moving: None,
        }
    }

    /// Where the view is at `at`, once moving, and whether it has landed.
    pub fn at(&self, at: Instant, cut: bool) -> (f64, bool) {
        let Some(moving) = self.moving else {
            return (self.from, false);
        };
        let x = if cut {
            1.0
        } else {
            (at - moving).as_secs_f64() / ARRIVING.as_secs_f64()
        };
        (self.from + (self.to - self.from) * eased(x), x >= 1.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_curve_starts_and_ends_where_it_should() {
        assert!(eased(0.0).abs() < 1e-9);
        assert!((eased(1.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn the_curve_is_mostly_there_at_once() {
        assert!(eased(0.25) > 0.75, "{}", eased(0.25));
        assert!(eased(0.5) < 1.0);
    }

    #[test]
    fn a_far_jump_moves_its_last_screen() {
        let jump = Jump::new(0.0, 100_000.0, 1800.0, Instant::now());
        assert!((jump.from - 98_200.0).abs() < 1e-9);
        let back = Jump::new(100_000.0, 0.0, 1800.0, Instant::now());
        assert!((back.from - 1800.0).abs() < 1e-9);
    }

    #[test]
    fn a_near_jump_moves_all_the_way() {
        let jump = Jump::new(1000.0, 2800.0, 1800.0, Instant::now());
        assert!((jump.from - 1000.0).abs() < 1e-9);
    }
}

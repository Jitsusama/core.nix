//! What the daemon decides about the light: when the screen follows it, what
//! the curve learns, and what a shut lid stops.
//!
//! It's given the brightness shown and the time, and says what to do, so it
//! reads no screen, writes no file and speaks no D-Bus; main.rs does those.

use std::time::{Duration, Instant};

use crate::curve::Curve;
use crate::steer::{Ramp, Steer};

/// How often a move steps while it's under way.
pub const STEP: Duration = Duration::from_millis(16);

#[derive(Debug)]
pub struct Follow {
    curve: Curve,
    /// Whether the curve has learned anything yet.
    knows_joel: bool,
    steer: Steer,
    ramp: Option<Ramp>,
    /// The light last heard, once there's been any.
    lux: Option<f64>,
    /// Whether the lid is shut, so the light heard is the lid's.
    shut: bool,
}

impl Follow {
    /// Follows the light along this curve, with the lid open and nothing
    /// heard yet. Unless `knows_joel`, the first light heard teaches the
    /// curve the brightness the screen already has.
    pub fn new(curve: Curve, knows_joel: bool) -> Self {
        Follow {
            curve,
            knows_joel,
            steer: Steer::default(),
            ramp: None,
            lux: None,
            shut: false,
        }
    }

    /// The curve, as it has learned to be.
    pub fn curve(&self) -> &Curve {
        &self.curve
    }

    /// The light last heard, once there's been any.
    pub fn lux(&self) -> Option<f64> {
        self.lux
    }

    /// The light changed to `lux`, with the screen at `shown`. Light that
    /// can't be, or that the shut lid hears, is passed over. It says whether
    /// the curve learned, as the first light heard teaches it the brightness
    /// shown.
    #[must_use]
    pub fn heard(&mut self, lux: f64, shown: f64, now: Instant) -> bool {
        if !lux.is_finite() || lux < 0.0 || self.shut {
            return false;
        }
        self.lux = Some(lux);
        let learned = !self.knows_joel;
        if learned {
            self.learn(lux, shown);
        }
        let shown = self.ramp.map_or(shown, |ramp| ramp.to());
        self.steer.heard(self.curve.level_at(lux), shown, now);
        learned
    }

    /// The lid opened or shut. Shut, any move the light asked for stops; the
    /// light heard once it opens is followed again. It says whether that's a
    /// change.
    #[must_use]
    pub fn lid(&mut self, shut: bool) -> bool {
        if shut == self.shut {
            return false;
        }
        self.shut = shut;
        if shut {
            self.halt();
        }
        true
    }

    /// Joel picked `level`: any move stops, and unless the lid is shut the
    /// curve learns it for the light there is, which it gives.
    #[must_use]
    pub fn chose(&mut self, level: f64) -> Option<f64> {
        self.halt();
        let lux = self.lux.filter(|_| !self.shut)?;
        self.learn(lux, level);
        Some(lux)
    }

    /// Stops any move, under way or waiting on the light.
    pub fn halt(&mut self) {
        self.ramp = None;
        self.steer.forget();
    }

    /// Gives up the move under way, when the screen can't be read or set;
    /// light heard since it started still asks for its own.
    pub fn abandon(&mut self) {
        self.ramp = None;
    }

    /// When to step next: soon while a move is under way, or when the move
    /// waiting on the light is due.
    pub fn wake(&self, now: Instant) -> Option<Instant> {
        if self.ramp.is_some() {
            Some(now + STEP)
        } else {
            self.steer.next()
        }
    }

    /// Starts the move the light asked for, from `shown`, once it's due and
    /// none is under way, and gives where it goes.
    pub fn start(&mut self, shown: f64, now: Instant) -> Option<f64> {
        if self.ramp.is_some() {
            return None;
        }
        let to = self.steer.due(now)?;
        self.ramp = Some(Ramp::new(shown, to, now));
        Some(to)
    }

    /// Where the move under way has reached by now, if one is; the last step
    /// ends it.
    pub fn step(&mut self, now: Instant) -> Option<f64> {
        let ramp = self.ramp?;
        if ramp.done(now) {
            self.ramp = None;
        }
        Some(ramp.at(now))
    }

    fn learn(&mut self, lux: f64, level: f64) {
        self.curve.learn(lux, level);
        self.knows_joel = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Point;
    use crate::steer::{BRIGHTENING, DIMMING};

    /// A while longer than any light is waited on before it's followed.
    const LATER: Duration = Duration::from_secs(60);

    /// Half as bright in a dim room as in a bright one.
    fn curve() -> Curve {
        Curve::through([
            Point {
                lux: 10.0,
                level: 0.2,
            },
            Point {
                lux: 1000.0,
                level: 0.8,
            },
        ])
    }

    /// Following a curve that already knows Joel, the light long heard.
    fn following(lux: f64, shown: f64, start: Instant) -> Follow {
        let mut follow = Follow::new(curve(), true);
        let _ = follow.heard(lux, shown, start);
        follow
    }

    /// Lets the move the light asked for run to its end, and gives where it
    /// ended, or none when there was none to make.
    fn moved(follow: &mut Follow, shown: f64, now: Instant) -> Option<f64> {
        follow.start(shown, now)?;
        let mut at = None;
        while let Some(level) = follow.step(now + LATER) {
            at = Some(level);
        }
        at
    }

    #[test]
    fn dimmer_light_is_followed_once_it_has_held() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        let _ = follow.heard(10.0, 0.8, start);
        assert_eq!(follow.wake(start), Some(start + DIMMING));
        let arrived = moved(&mut follow, 0.8, start + DIMMING).unwrap();
        assert!((arrived - 0.2).abs() < 1e-9, "arrived at {arrived}");
    }

    #[test]
    fn with_the_lid_shut_its_dark_moves_nothing() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        assert!(follow.lid(true));
        let _ = follow.heard(0.0, 0.8, start);
        assert_eq!(follow.wake(start), None);
        assert_eq!(moved(&mut follow, 0.8, start + LATER), None);
    }

    #[test]
    fn shutting_the_lid_stops_a_move_waiting_on_the_light() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        let _ = follow.heard(10.0, 0.8, start);
        assert!(follow.lid(true));
        assert_eq!(follow.wake(start), None);
        assert_eq!(moved(&mut follow, 0.8, start + LATER), None);
    }

    #[test]
    fn shutting_the_lid_stops_a_move_under_way() {
        let start = Instant::now();
        let mut follow = following(10.0, 0.8, start);
        follow.start(0.8, start + DIMMING).unwrap();
        assert!(follow.lid(true));
        assert_eq!(follow.step(start + DIMMING), None);
    }

    #[test]
    fn opened_the_light_is_followed_again() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        assert!(follow.lid(true));
        let _ = follow.heard(0.0, 0.8, start);
        assert!(follow.lid(false));
        let _ = follow.heard(10.0, 0.8, start);
        assert!(moved(&mut follow, 0.8, start + DIMMING).is_some());
    }

    #[test]
    fn a_move_given_up_leaves_the_light_heard_since_asking() {
        let start = Instant::now();
        let mut follow = following(10.0, 0.8, start);
        follow.start(0.8, start + DIMMING).unwrap();
        let _ = follow.heard(1000.0, 0.5, start + DIMMING);
        follow.abandon();
        assert_eq!(follow.step(start + DIMMING), None);
        let due = start + DIMMING + BRIGHTENING;
        assert_eq!(follow.start(0.5, due), Some(0.8));
    }

    #[test]
    fn the_lid_said_again_is_no_change() {
        let mut follow = Follow::new(curve(), true);
        assert!(!follow.lid(false));
        assert!(follow.lid(true));
        assert!(!follow.lid(true));
    }

    #[test]
    fn a_pick_is_learned_for_the_light_there_is() {
        let start = Instant::now();
        let mut follow = following(10.0, 0.2, start);
        assert_eq!(follow.chose(0.5), Some(10.0));
        assert!((follow.curve().level_at(10.0) - 0.5).abs() < 1e-9);
    }

    #[test]
    fn with_the_lid_shut_a_pick_teaches_nothing() {
        let start = Instant::now();
        let mut follow = following(10.0, 0.2, start);
        assert!(follow.lid(true));
        assert_eq!(follow.chose(0.5), None);
        assert_eq!(follow.curve(), &curve());
    }

    #[test]
    fn a_pick_stops_the_move_the_light_asked_for() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        let _ = follow.heard(10.0, 0.8, start);
        let _ = follow.chose(0.6);
        assert_eq!(moved(&mut follow, 0.6, start + LATER), None);
    }

    #[test]
    fn the_first_light_teaches_the_brightness_the_screen_has() {
        let start = Instant::now();
        let mut follow = Follow::new(curve(), false);
        assert!(follow.heard(100.0, 0.7, start));
        assert!((follow.curve().level_at(100.0) - 0.7).abs() < 1e-9);
        assert_eq!(follow.wake(start), None, "starting moved the screen");
        assert!(!follow.heard(200.0, 0.7, start), "it learned twice");
    }

    #[test]
    fn light_that_cant_be_is_passed_over() {
        let start = Instant::now();
        let mut follow = Follow::new(curve(), false);
        assert!(!follow.heard(f64::NAN, 0.5, start));
        assert!(!follow.heard(-1.0, 0.5, start));
        assert_eq!(follow.lux(), None);
    }

    #[test]
    fn light_heard_mid_move_is_weighed_against_where_the_move_goes() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        let _ = follow.heard(10.0, 0.8, start);
        follow.start(0.8, start + DIMMING).unwrap();
        // Still as dim as the move is going, though the screen is only on its
        // way down, so no second move waits once this one ends.
        let _ = follow.heard(10.0, 0.75, start + DIMMING);
        while follow.step(start + LATER).is_some() {}
        assert_eq!(follow.start(0.2, start + LATER + BRIGHTENING), None);
    }
}

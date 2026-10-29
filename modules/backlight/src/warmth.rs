//! The screen's warmth, following the warmth of the room's light.
//!
//! An eye takes the light around it for white, so a screen at its own white
//! reads blue under a lamp. The screen leans part of the way toward the
//! room's light, as True Tone does: two fifths of the way, so under the
//! warmest lamp, about 2700 K, it sits near 5000 K, and it never goes cooler
//! than its own 6500 K, whatever the sky. The sensor can't see colour in
//! the dark, so below a dim light the warmth last seen holds, and while the
//! lid is shut the light is the lid's, so nothing follows it.
//!
//! A move glides, 50 K a fifth of a second, so it's never seen as a jump,
//! and one smaller than 150 K isn't started, so the screen doesn't wander
//! with every flicker of the light. This file only decides; main.rs sets
//! what it decides.

use std::time::{Duration, Instant};

/// The screen's own white, and the coolest it's set.
pub const WHITE: u16 = 6500;
/// The warmest room light followed; a candle is taken for a lamp.
const WARMEST_ROOM: f64 = 2700.0;
/// How far toward the room's light the screen leans.
const LEAN: f64 = 0.4;
/// Below this many lux the sensor's colour is taken for noise.
const DIM: f64 = 10.0;
/// One step of a glide, in kelvin, and how often one is taken.
const STEP: u16 = 50;
const EVERY: Duration = Duration::from_millis(200);
/// The least move worth starting, in kelvin.
const WORTH: u16 = 150;

/// The screen's warmth for a room's light, to the nearest step.
pub fn for_room(room: f64) -> u16 {
    let white = f64::from(WHITE);
    let room = room.clamp(WARMEST_ROOM, white);
    let kelvin = white - LEAN * (white - room);
    let steps = (kelvin / f64::from(STEP)).round();
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the kelvin is held between 2700 and 6500, so its steps fit"
    )]
    let steps = steps as u16;
    steps * STEP
}

pub struct Warmth {
    /// The warmth the screen is set to, as far as this daemon knows.
    shown: u16,
    /// Where a glide under way is going.
    aim: Option<u16>,
    /// When its next step is due.
    next: Option<Instant>,
    /// The room's warmth and light, last heard.
    room: Option<f64>,
    lux: Option<f64>,
    shut: bool,
}

impl Warmth {
    /// Starting from the warmth the screen already has.
    pub fn new(shown: u16) -> Self {
        Warmth {
            shown,
            aim: None,
            next: None,
            room: None,
            lux: None,
            shut: false,
        }
    }

    /// The room's light changed warmth.
    pub fn room(&mut self, kelvin: f64, now: Instant) {
        self.room = Some(kelvin);
        self.follow(now);
    }

    /// The room's light changed brightness, which says whether its colour
    /// can be trusted.
    pub fn light(&mut self, lux: f64, now: Instant) {
        self.lux = Some(lux);
        self.follow(now);
    }

    /// The lid opened or shut. Shut, a glide under way stops where it is.
    pub fn lid(&mut self, shut: bool, now: Instant) {
        self.shut = shut;
        if shut {
            self.abandon();
        } else {
            self.follow(now);
        }
    }

    /// Whatever sets the warmth started again, at its own white.
    pub fn reset(&mut self, now: Instant) {
        self.shown = WHITE;
        self.abandon();
        self.follow(now);
    }

    /// The warmth couldn't be set, so the glide stops.
    pub fn abandon(&mut self) {
        self.aim = None;
        self.next = None;
    }

    /// Where a glide under way is going.
    pub fn going(&self) -> Option<u16> {
        self.aim
    }

    fn follow(&mut self, now: Instant) {
        let (Some(room), Some(lux)) = (self.room, self.lux) else {
            return;
        };
        if self.shut || lux < DIM {
            return;
        }
        let to = for_room(room);
        match self.aim {
            Some(aim) if aim == to => {}
            Some(_) => self.aim = Some(to),
            None if to.abs_diff(self.shown) < WORTH => {}
            None => {
                self.aim = Some(to);
                self.next = Some(now);
            }
        }
    }

    /// When the next step is due, if a glide is under way.
    pub fn wake(&self) -> Option<Instant> {
        self.next
    }

    /// The warmth to set now, when a step is due.
    pub fn step(&mut self, now: Instant) -> Option<u16> {
        if self.next? > now {
            return None;
        }
        let aim = self.aim?;
        let at = if aim > self.shown {
            self.shown.saturating_add(STEP).min(aim)
        } else {
            self.shown.saturating_sub(STEP).max(aim)
        };
        self.shown = at;
        if at == aim {
            self.abandon();
        } else {
            self.next = Some(now + EVERY);
        }
        Some(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every step a glide takes, from now until it's done.
    fn glide(warmth: &mut Warmth, now: Instant) -> Vec<u16> {
        let mut steps = Vec::new();
        let mut at = now;
        while let Some(due) = warmth.wake() {
            at = at.max(due);
            if let Some(kelvin) = warmth.step(at) {
                steps.push(kelvin);
            }
        }
        steps
    }

    #[test]
    fn the_screen_leans_two_fifths_of_the_way_to_the_room() {
        assert_eq!(for_room(6500.0), 6500);
        assert_eq!(for_room(2700.0), 5000, "4980, to the nearest step");
        assert_eq!(for_room(4000.0), 5500);
    }

    #[test]
    fn it_warms_no_further_than_a_lamp_and_cools_no_further_than_white() {
        assert_eq!(for_room(1800.0), for_room(2700.0));
        assert_eq!(for_room(9000.0), WHITE);
    }

    #[test]
    fn a_warm_room_glides_the_screen_warmer_a_step_at_a_time() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(18.0, now);
        warmth.room(2609.0, now);
        let steps = glide(&mut warmth, now);
        assert_eq!(steps.first(), Some(&6450));
        assert_eq!(steps.last(), Some(&5000));
        assert!(steps.windows(2).all(|pair| pair[0] - pair[1] == STEP));
        assert_eq!(warmth.wake(), None, "done once there");
    }

    #[test]
    fn steps_are_a_fifth_of_a_second_apart() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(4000.0, now);
        assert_eq!(warmth.step(now), Some(6450));
        assert_eq!(warmth.step(now), None, "not yet due");
        assert_eq!(warmth.wake(), Some(now + EVERY));
    }

    #[test]
    fn nothing_moves_in_the_dark_or_with_the_lid_shut() {
        let now = Instant::now();
        let mut dark = Warmth::new(WHITE);
        dark.light(3.0, now);
        dark.room(2700.0, now);
        assert_eq!(dark.wake(), None);

        let mut shut = Warmth::new(WHITE);
        shut.lid(true, now);
        shut.light(100.0, now);
        shut.room(2700.0, now);
        assert_eq!(shut.wake(), None);
        shut.lid(false, now);
        assert_eq!(shut.wake(), Some(now), "follows once the lid opens");
    }

    #[test]
    fn a_small_change_in_the_room_moves_nothing() {
        let now = Instant::now();
        let mut warmth = Warmth::new(5500);
        warmth.light(100.0, now);
        warmth.room(4200.0, now);
        assert_eq!(warmth.wake(), None, "5600 is only 100 K away");
    }

    #[test]
    fn a_glide_turns_when_the_room_does() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        assert_eq!(warmth.step(now), Some(6450));
        warmth.room(6500.0, now);
        assert_eq!(glide(&mut warmth, now), vec![6500]);
    }

    #[test]
    fn a_restarted_setter_is_brought_back_to_the_room() {
        let now = Instant::now();
        let mut warmth = Warmth::new(5000);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        assert_eq!(warmth.wake(), None, "already there");
        warmth.reset(now);
        assert_eq!(glide(&mut warmth, now).last(), Some(&5000));
    }
}

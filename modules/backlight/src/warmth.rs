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
//! A sensor's colour flickers: at night on 2026-11-11 optimus's said 2700,
//! 5200 and 8000 K within seconds. So no one reading is believed. Each
//! counts for the time it held, and the room's colour settles toward it
//! over about 20 seconds, so a flicker is taken for the light between and
//! a lamp switched on is followed within a minute. The settled colour is
//! said for the shell only once it moves 15 mireds from what was last said,
//! and not at all while the screen doesn't follow the room, since each
//! change rewrites the note the shell wakes for.
//!
//! A move glides, 50 K a fifth of a second, so it's never seen as a jump,
//! and one smaller than 150 K isn't started, so the screen doesn't wander
//! with every flicker of the light. The night comes slower: a move to the
//! night's white that isn't one of its evening steps, as when the laptop
//! starts or opens after sunset, glides 50 K every 1.2 seconds, so its
//! whole warmth comes over about a minute rather than in ten seconds.
//! This file only decides; answering.rs sets what it decides.
//!
//! Joel can say the screen shouldn't follow the room. Then it glides back
//! to its own white, however near it already is, and holds there until he
//! says it should follow again.
//!
//! At night the screen warms too, whatever its sensor sees (night.rs): the
//! night light says the white it wants now, which eases from the screen's
//! own after sunset and back before sunrise, and the screen takes the
//! warmer of the room's and the night's. Where no sensor sees colour, the
//! night is all that warms it.

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
/// How often a step is taken when the night arrives all at once.
const ARRIVING: Duration = Duration::from_millis(1200);
/// The least move worth starting, in kelvin.
const WORTH: u16 = 150;
/// How long, in seconds, a reading of the room's colour takes to settle:
/// the settled colour goes about two thirds of the way toward a reading
/// that holds this long.
const SETTLING: f64 = 20.0;
/// How far, in mireds, the settled colour moves before it's said again.
const BAND: f64 = 15.0;

/// A light's colour in mireds, the scale on which equal steps look equal.
fn mireds(kelvin: f64) -> f64 {
    1e6 / kelvin.max(1.0)
}

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
    /// The room's colour and light, last heard, and when the colour last
    /// settled toward what was heard.
    room: Option<f64>,
    lux: Option<f64>,
    since: Option<Instant>,
    /// The room's colour settled from what was heard, in mireds, and the
    /// kelvin last said of it.
    settled: Option<f64>,
    said: Option<u16>,
    shut: bool,
    /// Whether the screen follows the room, as Joel last said.
    follows: bool,
    /// The room's warmth last trusted, which holds below a dim light.
    held: Option<u16>,
    /// The white the night light wants now, the screen's own by day, and
    /// whether the screen is warmer than its own for the night, so it's
    /// brought back to it by morning whatever the room says.
    night: u16,
    by_night: bool,
    /// How often the glide under way takes a step.
    pace: Duration,
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
            since: None,
            settled: None,
            said: None,
            shut: false,
            follows: true,
            held: None,
            night: WHITE,
            by_night: false,
            pace: EVERY,
        }
    }

    /// The night light wants this white now (night.rs).
    pub fn night(&mut self, kelvin: u16, now: Instant) {
        if self.night != kelvin {
            self.night = kelvin;
            self.follow(now);
        }
    }

    /// Joel said whether the screen should follow the room.
    pub fn follow_the_room(&mut self, follows: bool, now: Instant) {
        if self.follows != follows {
            self.follows = follows;
            self.follow(now);
        }
    }

    /// Whether the screen follows the room.
    pub fn follows(&self) -> bool {
        self.follows
    }

    /// The room's light's colour as last said, in kelvin to the nearest
    /// hundred, while the screen follows the room; none while it doesn't,
    /// or while no light bright enough to trust has been heard.
    pub fn room_white(&self) -> Option<u16> {
        self.said.filter(|_| self.follows)
    }

    /// The warmth the screen is at, or going to.
    pub fn stands(&self) -> u16 {
        self.aim.unwrap_or(self.shown)
    }

    /// The room's light changed warmth.
    pub fn room(&mut self, kelvin: f64, now: Instant) {
        self.settle(now);
        self.room = Some(kelvin);
        self.heard(now);
    }

    /// The room's light changed brightness, which says whether its colour
    /// can be trusted.
    pub fn light(&mut self, lux: f64, now: Instant) {
        self.settle(now);
        self.lux = Some(lux);
        self.heard(now);
    }

    /// Whether the room's light is bright enough for its colour to count.
    fn trusted(&self) -> bool {
        self.lux.is_some_and(|lux| lux >= DIM)
    }

    /// Counts the room's last reading for the time it held, if its light
    /// was bright enough to trust, settling the room's colour toward it.
    fn settle(&mut self, now: Instant) {
        let since = self.since.replace(now);
        let (Some(room), Some(since), Some(settled)) = (self.room, since, self.settled) else {
            return;
        };
        if self.trusted() {
            let held = now.saturating_duration_since(since).as_secs_f64();
            let toward = 1.0 - (-held / SETTLING).exp();
            self.settled = Some(settled + (mireds(room) - settled) * toward);
        }
    }

    /// After a reading: the first trusted colour is taken as it is, the
    /// settled colour is said once it has moved far enough, and the screen
    /// follows.
    fn heard(&mut self, now: Instant) {
        if self.settled.is_none() && self.trusted() {
            self.settled = self.room.map(mireds);
        }
        if let Some(settled) = self.settled {
            let moved = self
                .said
                .is_none_or(|said| (mireds(f64::from(said)) - settled).abs() >= BAND);
            if moved {
                #[expect(
                    clippy::cast_possible_truncation,
                    clippy::cast_sign_loss,
                    reason = "a light's colour, held between 1,000 and 40,000 K, fits"
                )]
                let kelvin = ((1e6 / settled).clamp(1000.0, 40_000.0) / 100.0).round() * 100.0;
                self.said = Some(kelvin as u16);
            }
        }
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

    /// The room's settled warmth while its light is bright enough to trust.
    fn lit(&self) -> Option<u16> {
        let settled = self.settled?;
        self.trusted().then(|| for_room(1e6 / settled))
    }

    /// Where the screen should be, if it should move: the warmer of the
    /// room's warmth, while it follows the room, else its own white, and
    /// the night's. The room's last trusted warmth holds in the dark; with
    /// nothing known of the room, the night alone moves it, and by day only
    /// back from where the night took it.
    fn wanted(&self) -> Option<u16> {
        if self.shut {
            return None;
        }
        let room = if self.follows {
            self.lit().or(self.held)
        } else {
            Some(WHITE)
        };
        match room {
            Some(room) => Some(room.min(self.night)),
            None => (self.night < WHITE || self.by_night).then_some(self.night),
        }
    }

    fn follow(&mut self, now: Instant) {
        if let Some(lit) = self.lit() {
            self.held = Some(lit);
        }
        let Some(to) = self.wanted() else {
            return;
        };
        // A small move isn't worth starting for the room, but going back to
        // white is always finished, and so is each of the night's steps,
        // which are small by design.
        let nightly = self.night < WHITE && to == self.night;
        if nightly {
            self.by_night = true;
        }
        let slight = self.follows && !nightly && to != WHITE && to.abs_diff(self.shown) < WORTH;
        self.pace = if nightly && to.abs_diff(self.shown) > STEP {
            ARRIVING
        } else {
            EVERY
        };
        match self.aim {
            Some(aim) if aim == to => {}
            Some(_) => self.aim = Some(to),
            None if to == self.shown || slight => {}
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

    /// The warmth to set now, when a step is due. The screen is at it only
    /// once [`Warmth::set`] says so, since a set that fails leaves it where
    /// it was, and the next glide must start from there.
    pub fn step(&self, now: Instant) -> Option<u16> {
        if self.next? > now {
            return None;
        }
        let aim = self.aim?;
        Some(if aim > self.shown {
            self.shown.saturating_add(STEP).min(aim)
        } else {
            self.shown.saturating_sub(STEP).max(aim)
        })
    }

    /// The screen took a step to `kelvin`.
    pub fn set(&mut self, kelvin: u16, now: Instant) {
        self.shown = kelvin;
        if kelvin == WHITE {
            self.by_night = false;
        }
        if self.aim == Some(kelvin) {
            self.abandon();
        } else {
            self.next = Some(now + self.pace);
        }
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
                warmth.set(kelvin, at);
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
        warmth.set(6450, now);
        assert_eq!(warmth.step(now), None, "not yet due");
        assert_eq!(warmth.wake(), Some(now + EVERY));
    }

    #[test]
    fn a_step_that_fails_leaves_the_screen_where_it_was() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        assert_eq!(warmth.step(now), Some(6450));
        warmth.abandon();
        assert_eq!(warmth.stands(), WHITE, "the note says the warmth it's at");
        warmth.room(2690.0, now);
        assert_eq!(
            glide(&mut warmth, now).first(),
            Some(&6450),
            "starts again from white"
        );
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
        warmth.set(6450, now);
        warmth.room(6500.0, now);
        let later = now + Duration::from_secs(120);
        warmth.light(100.0, later);
        assert_eq!(glide(&mut warmth, later), vec![6500]);
    }

    #[test]
    fn a_flickering_sensor_settles_before_it_is_said_or_followed() {
        // At night on 2026-11-11 optimus's sensor said 2700, 5200 and
        // 8000 K within seconds, and each was written for the shell.
        let start = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, start);
        warmth.room(5200.0, start);
        glide(&mut warmth, start);
        let mut said = vec![warmth.room_white()];
        let mut aims = Vec::new();
        let mut at = start;
        for kelvin in [2700.0, 5200.0, 8000.0, 5200.0].iter().cycle().take(100) {
            at += Duration::from_millis(500);
            warmth.room(*kelvin, at);
            if said.last() != Some(&warmth.room_white()) {
                said.push(warmth.room_white());
            }
            aims.extend(warmth.going());
        }
        assert!(
            said.len() <= 3,
            "the room's colour was said {} times in 50 s: {said:?}",
            said.len()
        );
        assert!(
            aims.iter().all(|&aim| aim >= 5500),
            "the screen aimed for a lamp it never had: {aims:?}"
        );
    }

    #[test]
    fn a_room_whose_colour_holds_is_followed_within_a_minute() {
        let start = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, start);
        warmth.room(6500.0, start);
        warmth.room(2700.0, start + Duration::from_secs(1));
        let mut at = start + Duration::from_secs(1);
        for _ in 0..60 {
            at += Duration::from_secs(1);
            warmth.light(100.0, at);
        }
        assert_eq!(warmth.stands(), 5000, "the lamp's warmth, a minute on");
    }

    #[test]
    fn the_rooms_colour_isnt_said_while_the_screen_doesnt_follow_it() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        assert_eq!(warmth.room_white(), Some(2700));
        warmth.follow_the_room(false, now);
        assert_eq!(warmth.room_white(), None);
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

    #[test]
    fn told_not_to_follow_it_glides_back_to_white_and_holds() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        glide(&mut warmth, now);
        warmth.follow_the_room(false, now);
        assert_eq!(warmth.stands(), WHITE);
        assert_eq!(glide(&mut warmth, now).last(), Some(&WHITE));
        warmth.room(3000.0, now);
        assert_eq!(warmth.wake(), None, "the room is no longer followed");
    }

    #[test]
    fn going_back_to_white_finishes_however_near_it_is() {
        let now = Instant::now();
        let mut warmth = Warmth::new(6400);
        warmth.follow_the_room(false, now);
        assert_eq!(glide(&mut warmth, now), vec![6450, 6500]);
    }

    #[test]
    fn a_lid_shut_on_the_way_back_finishes_it_once_open() {
        let now = Instant::now();
        let mut warmth = Warmth::new(5000);
        warmth.follow_the_room(false, now);
        warmth.lid(true, now);
        assert_eq!(warmth.wake(), None);
        warmth.lid(false, now);
        assert_eq!(glide(&mut warmth, now).last(), Some(&WHITE));
    }

    #[test]
    fn told_to_follow_again_it_goes_back_to_the_room() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.follow_the_room(false, now);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        assert_eq!(warmth.wake(), None);
        warmth.follow_the_room(true, now);
        assert!(warmth.follows());
        assert_eq!(warmth.stands(), 5000);
    }

    #[test]
    fn the_night_warms_a_screen_whose_sensor_sees_no_colour() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.night(4000, now);
        assert_eq!(glide(&mut warmth, now).last(), Some(&4000));
    }

    #[test]
    fn a_night_arriving_all_at_once_comes_over_a_minute() {
        let start = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.night(4000, start);
        let mut at = start;
        while let Some(due) = warmth.wake() {
            at = at.max(due);
            if let Some(kelvin) = warmth.step(at) {
                warmth.set(kelvin, at);
            }
        }
        assert_eq!(warmth.stands(), 4000);
        let took = at - start;
        assert!(
            took >= Duration::from_secs(55) && took <= Duration::from_secs(65),
            "the night's 2500 K came in {took:?}"
        );
    }

    #[test]
    fn the_room_still_glides_at_its_own_pace() {
        let start = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, start);
        warmth.room(2700.0, start);
        let first = warmth.step(start).expect("the room's glide started");
        warmth.set(first, start);
        assert_eq!(warmth.wake(), Some(start + EVERY));
    }

    #[test]
    fn each_of_the_nights_small_steps_is_taken() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.night(6450, now);
        assert_eq!(glide(&mut warmth, now), vec![6450]);
        warmth.night(6400, now);
        assert_eq!(glide(&mut warmth, now), vec![6400]);
    }

    #[test]
    fn by_morning_the_screen_comes_back_to_its_own_white() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.night(4000, now);
        glide(&mut warmth, now);
        warmth.night(6450, now);
        glide(&mut warmth, now);
        warmth.night(WHITE, now);
        assert_eq!(glide(&mut warmth, now), vec![WHITE]);
        warmth.night(WHITE, now);
        assert_eq!(warmth.wake(), None, "white by day holds");
    }

    #[test]
    fn the_screen_takes_the_warmer_of_the_room_and_the_night() {
        let now = Instant::now();
        let mut warmth = Warmth::new(WHITE);
        warmth.light(100.0, now);
        warmth.room(2700.0, now);
        glide(&mut warmth, now);
        warmth.night(4000, now);
        assert_eq!(glide(&mut warmth, now).last(), Some(&4000));
        warmth.night(WHITE, now);
        assert_eq!(
            glide(&mut warmth, now).last(),
            Some(&5000),
            "the room's by day"
        );
    }
}

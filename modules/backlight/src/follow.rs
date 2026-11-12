//! What the daemon decides about the light: when the screen follows it, what
//! the curve learns, what a shut lid stops, and when the keyboard's light
//! comes on and goes out.
//!
//! It's given the brightness shown and the time, and says what to do, so it
//! reads no screen, writes no file and speaks no D-Bus; the daemon does
//! those (answering.rs and dbus.rs).
//!
//! Long because its tests, half the file, walk each case of the light, the
//! lid, the evening and the keyboard in turn, and one decision is easier to
//! follow beside its cases than split from them.

use std::time::{Duration, Instant};

use crate::curve::Curve;
use crate::evening::{dimmed, eased};
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
    /// What the evening leaves of the brightness the curve asks for, all
    /// of it by day, eased toward what the clock says (evening.rs).
    share: f64,
    /// What the clock says the evening leaves now.
    clock: f64,
    /// How the share moves toward the clock's.
    hold: Hold,
    /// What Joel set in each light in the evening, which the evening never
    /// dims past again.
    floor: Option<Curve>,
}

/// How the evening's share moves toward what the clock says.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Hold {
    /// Not at all until the screen's been seen with some light, when the
    /// share is taken from it, so a start deep in the evening eases from
    /// where the screen is rather than jumping to the clock's.
    Unseen,
    /// A minute's ease at a time.
    Easing,
    /// Not at all: Joel picked a brightness this evening, which holds until
    /// the morning.
    Held,
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
            share: 1.0,
            clock: 1.0,
            hold: Hold::Unseen,
            floor: None,
        }
    }

    /// Never dims the evening past what Joel set in it, light by light, as
    /// kept from evenings before.
    pub fn floored(mut self, floor: Option<Curve>) -> Self {
        self.floor = floor;
        self
    }

    /// What Joel set in each light in the evening, once he has.
    pub fn floor(&self) -> Option<&Curve> {
        self.floor.as_ref()
    }

    /// Whether a brightness Joel picked holds for the rest of the evening.
    pub fn holding(&self) -> bool {
        self.hold == Hold::Held
    }

    /// The curve, as it has learned to be.
    pub fn curve(&self) -> &Curve {
        &self.curve
    }

    /// The light last heard, once there's been any.
    pub fn lux(&self) -> Option<f64> {
        self.lux
    }

    /// What the clock last said the evening leaves, though the share may
    /// still wait for light to ease from.
    pub fn clock(&self) -> f64 {
        self.clock
    }

    /// The brightness the screen goes to in this light: what the curve
    /// asks for, dimmed in the evening, but never past what Joel set in
    /// that light in an evening, nor brighter for it than the day's.
    pub fn asks(&self, lux: f64) -> f64 {
        let level = self.curve.level_at(lux);
        let dimmed = dimmed(level, self.share);
        match &self.floor {
            Some(floor) if self.share < 1.0 => dimmed.max(floor.level_at(lux).min(level)),
            _ => dimmed,
        }
    }

    /// Whether the screen follows the light now: some heard, and the lid
    /// open, since a shut lid hears only itself.
    pub fn following(&self) -> bool {
        self.lux.is_some() && !self.shut
    }

    /// Whether a move is under way.
    pub fn moving(&self) -> bool {
        self.ramp.is_some()
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
        if self.hold == Hold::Unseen {
            self.hold = Hold::Easing;
            if self.clock < 1.0 {
                self.share = shown / self.curve.level_at(lux);
            }
        }
        let learned = !self.knows_joel;
        if learned {
            self.learn(lux, shown);
        }
        let shown = self.ramp.map_or(shown, |ramp| ramp.to());
        self.steer.heard(self.asks(lux), shown, now);
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
    /// curve learns it for the light there is, which it gives. In the
    /// evening it holds instead until the morning, and is kept as what the
    /// evening never dims past in that light, leaving the day's curve be.
    #[must_use]
    pub fn chose(&mut self, level: f64) -> Option<f64> {
        self.halt();
        let lux = self.lux.filter(|_| !self.shut)?;
        if self.clock < 1.0 || self.share < 1.0 {
            self.hold = Hold::Held;
            self.share = level / self.curve.level_at(lux);
            self.floor
                .get_or_insert_with(|| Curve::through([]))
                .learn(lux, level);
        } else {
            self.learn(lux, level);
        }
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

    /// A minute's look at the clock, which says the evening leaves `clock`
    /// of the brightness the curve asks for, with the screen at `shown`: the
    /// share eases a minute's way toward it (evening.rs), unless Joel's pick
    /// holds it, which the morning ends. Until the screen's been seen with
    /// some light, nothing moves, so a start eases from where it is.
    /// The light heard is asked about again, as a move it waits on like any
    /// other, so the evening comes in the same small steps the light's do.
    pub fn dims(&mut self, clock: f64, shown: f64, now: Instant) {
        self.clock = clock;
        if clock >= 1.0 {
            self.hold = match self.hold {
                Hold::Held => Hold::Easing,
                hold => hold,
            };
        }
        if self.hold != Hold::Easing {
            return;
        }
        let share = eased(self.share, clock);
        if (share - self.share).abs() < f64::EPSILON {
            return;
        }
        self.share = share;
        let Some(lux) = self.lux.filter(|_| !self.shut) else {
            return;
        };
        let shown = self.ramp.map_or(shown, |ramp| ramp.to());
        self.steer.heard(self.asks(lux), shown, now);
    }

    fn learn(&mut self, lux: f64, level: f64) {
        self.curve.learn(lux, level);
        self.knows_joel = true;
    }
}

/// Below this light, in lux, the room is dark, and the keyboard is lit:
/// a desk lamp at night, or a dim room in the morning, reads under it.
pub const DARK: f64 = 25.0;

/// Above this light the room is light, and the keyboard goes out. It's four
/// times `DARK`, so light between the two leaves the keyboard as it is, and
/// light that wavers near either never turns it on and off.
pub const LIGHT: f64 = 100.0;

/// How far the light has to move from where Joel set the keyboard, as a
/// ratio either way, before the room sets it again.
pub const WELL_AWAY: f64 = 4.0;

/// How long the room's light has to hold before the keyboard follows it, so
/// a hand over the sensor changes nothing.
pub const KEYS_HOLD: Duration = Duration::from_secs(4);

/// What the room says of the keyboard: lit in the dark, out in the light,
/// and nothing in between.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Room {
    Dark,
    Light,
}

fn room(lux: f64) -> Option<Room> {
    if lux < DARK {
        Some(Room::Dark)
    } else if lux > LIGHT {
        Some(Room::Light)
    } else {
        None
    }
}

/// When the keyboard's light follows the room. It's given the level read
/// and the time, and says what level to set, so it reads and writes nothing.
///
/// The level Joel sets with the light key holds until the light moves well
/// away from where he set it and the room says otherwise there. The level
/// the keyboard has as the daemon starts counts as his, so starting never
/// changes it. In the dark it's lit to the last level he lit it to.
#[derive(Debug)]
pub struct Keys {
    /// The level the dark lights it to.
    lit: u32,
    /// The level last read or set.
    level: u32,
    /// Joel's level holds for the light he set it in, once that's heard.
    joel: Joel,
    /// The level the room asks for, and when it's due.
    pending: Option<(u32, Instant)>,
    /// Whether the lid is shut, so the light heard is the lid's.
    shut: bool,
}

/// Whether Joel's level holds, and for what light.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Joel {
    /// He set it, and the light hasn't been heard since.
    Set,
    /// He set it in this light.
    At(f64),
    /// The room sets it.
    Room,
}

impl Keys {
    /// Follows the room from `level`, which holds as Joel's for the first
    /// light heard. Unlit, the dark lights it to its first step.
    pub fn new(level: u32) -> Self {
        Keys {
            lit: level.max(1),
            level,
            joel: Joel::Set,
            pending: None,
            shut: false,
        }
    }

    /// The level was read as `level`. Unless that's the level last read or
    /// set, Joel set it with the key, and it holds; it says whether he did.
    #[must_use]
    pub fn saw(&mut self, level: u32) -> bool {
        if level == self.level {
            return false;
        }
        self.level = level;
        if level > 0 {
            self.lit = level;
        }
        self.joel = Joel::Set;
        self.pending = None;
        true
    }

    /// The light changed to `lux`. Joel's level holds until the light is
    /// well away from where he set it and the room says otherwise there;
    /// then the dark asks for the keyboard lit and the light for it out, once
    /// that holds.
    pub fn heard(&mut self, lux: f64, now: Instant) {
        if !lux.is_finite() || lux < 0.0 || self.shut {
            return;
        }
        match self.joel {
            Joel::Set => {
                self.joel = Joel::At(lux);
                return;
            }
            Joel::At(at) => {
                let away = lux > at.max(1.0) * WELL_AWAY || lux * WELL_AWAY < at;
                let says = room(lux);
                if !away || says.is_none() || says == room(at) {
                    return;
                }
                self.joel = Joel::Room;
            }
            Joel::Room => {}
        }
        let wanted = match room(lux) {
            Some(Room::Dark) => self.lit,
            Some(Room::Light) => 0,
            None => {
                self.pending = None;
                return;
            }
        };
        if wanted == self.level {
            self.pending = None;
        } else if self.pending.map(|(level, _)| level) != Some(wanted) {
            self.pending = Some((wanted, now + KEYS_HOLD));
        }
    }

    /// The lid opened or shut. Shut, any change the light asked for stops.
    pub fn lid(&mut self, shut: bool) {
        self.shut = shut;
        if shut {
            self.pending = None;
        }
    }

    /// When the change the light asked for is due, if one is waiting.
    pub fn wake(&self) -> Option<Instant> {
        self.pending.map(|(_, due)| due)
    }

    /// Whether the change the light asked for is due by `now`, so the
    /// level is worth reading. On a Dell each read is a firmware call that
    /// stops every core while it runs, and the screen steps every 16 ms.
    pub fn is_due(&self, now: Instant) -> bool {
        self.wake().is_some_and(|due| due <= now)
    }

    /// The level to set, once the light has held long enough.
    pub fn due(&mut self, now: Instant) -> Option<u32> {
        let (level, _) = self.pending.filter(|&(_, due)| due <= now)?;
        self.pending = None;
        self.level = level;
        Some(level)
    }

    /// Lets go of the change the light asked for, when the level can't be
    /// read.
    pub fn forget(&mut self) {
        self.pending = None;
    }

    /// Setting it failed, so it's still at `level`.
    pub fn unset(&mut self, level: u32) {
        self.level = level;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::curve::Point;
    use crate::evening::SHARE;
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

    /// The clock's evening looked at once a minute for `minutes`, as the
    /// daemon does, with the screen at `shown`.
    fn evening_for(follow: &mut Follow, minutes: u32, shown: f64, start: Instant) {
        for minute in 0..minutes {
            follow.dims(SHARE, shown, start + Duration::from_mins(minute.into()));
        }
    }

    #[test]
    fn a_start_inside_the_evening_eases_from_where_the_screen_is() {
        // 12 Nov, 00:00: a restart deep in the evening took Joel's screen
        // from 85% to 43% within seconds.
        let start = Instant::now();
        let mut follow = Follow::new(curve(), true);
        follow.dims(SHARE, 0.8, start);
        let _ = follow.heard(1000.0, 0.8, start);
        let first = follow.asks(1000.0);
        assert!(
            (first - 0.8).abs() < 0.01,
            "a start in the evening asked {first} at once of a screen at 0.8"
        );
        evening_for(&mut follow, 31, 0.8, start);
        let eased = follow.asks(1000.0);
        assert!(
            (eased - 0.8 * SHARE).abs() < 1e-9,
            "half an hour on it asked {eased}"
        );
    }

    #[test]
    fn a_pick_in_the_evening_holds_for_the_rest_of_it() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        evening_for(&mut follow, 31, 0.8, start);
        assert_eq!(follow.chose(0.75), Some(1000.0));
        evening_for(&mut follow, 120, 0.75, start + LATER);
        let asks = follow.asks(1000.0);
        assert!(
            (asks - 0.75).abs() < 1e-9,
            "0.75 picked in the evening came to ask {asks}"
        );
        let learned = follow.curve().level_at(1000.0);
        assert!(
            (learned - 0.8).abs() < 1e-9,
            "a pick in the evening moved the day's curve to {learned}"
        );
    }

    #[test]
    fn the_evening_never_dims_past_what_joel_set_at_that_light() {
        // 12 Nov: 43% at 18 lux was too dark; a pick up says so, and a
        // restart keeps it.
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        evening_for(&mut follow, 31, 0.8, start);
        let _ = follow.chose(0.7);
        let floor = follow.floor().cloned();
        let mut again = Follow::new(curve(), true).floored(floor);
        let _ = again.heard(1000.0, 0.7, start);
        evening_for(&mut again, 120, 0.7, start);
        let asks = again.asks(1000.0);
        assert!(
            asks >= 0.7 - 1e-9,
            "after a restart the evening dimmed past 0.7 to {asks}"
        );
    }

    #[test]
    fn the_clock_s_evening_is_kept_while_no_light_is_heard() {
        // 12 Nov, 01:25: with the lid shut and no light heard, the journal
        // said the evening eased in at every minute's look.
        let start = Instant::now();
        let mut follow = Follow::new(curve(), true);
        follow.dims(0.85, 0.8, start);
        assert!(
            (follow.clock() - 0.85).abs() < 1e-9,
            "the clock's 0.85 was kept as {}",
            follow.clock()
        );
    }

    #[test]
    fn the_evening_dims_the_light_already_followed() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        evening_for(&mut follow, 31, 0.8, start);
        let at = moved(&mut follow, 0.8, start + Duration::from_mins(40));
        assert!(
            at.is_some_and(|level| (level - 0.8 * SHARE).abs() < 1e-9),
            "the evening took 0.8 to {at:?}"
        );
    }

    #[test]
    fn light_heard_in_the_evening_asks_for_it_dimmed() {
        let start = Instant::now();
        let mut follow = following(10.0, 0.2, start);
        evening_for(&mut follow, 31, 0.2, start);
        let dimmed = 0.2 * SHARE;
        let _ = moved(&mut follow, 0.2, start + Duration::from_mins(40));
        let _ = follow.heard(1000.0, dimmed, start + Duration::from_mins(41));
        let at = moved(&mut follow, dimmed, start + Duration::from_mins(42));
        assert!(
            at.is_some_and(|level| (level - 0.8 * SHARE).abs() < 1e-9),
            "brighter light in the evening went to {at:?}"
        );
    }

    #[test]
    fn a_pick_by_day_still_teaches_the_curve() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        follow.dims(1.0, 0.8, start);
        assert_eq!(follow.chose(0.6), Some(1000.0));
        assert!(!follow.holding(), "a pick by day held");
        let learned = follow.curve().level_at(1000.0);
        assert!(
            (learned - 0.6).abs() < 1e-9,
            "0.6 by day was learned as {learned}"
        );
    }

    #[test]
    fn morning_gives_the_brightness_back() {
        let start = Instant::now();
        let mut follow = following(1000.0, 0.8, start);
        evening_for(&mut follow, 31, 0.8, start);
        let dimmed = 0.8 * SHARE;
        let _ = moved(&mut follow, 0.8, start + Duration::from_mins(40));
        let _ = follow.chose(dimmed);
        for minute in 41..72 {
            follow.dims(1.0, dimmed, start + Duration::from_mins(minute));
        }
        assert!(!follow.holding(), "the morning kept the evening's pick");
        let at = moved(&mut follow, dimmed, start + Duration::from_mins(80));
        assert!(
            at.is_some_and(|level| (level - 0.8).abs() < 1e-9),
            "the morning took {dimmed} to {at:?}"
        );
    }

    /// The keyboard following the room, unlit at the start in daylight and
    /// lit to `lit` in the dark, the room having set it since.
    fn keys_following(lit: u32, start: Instant) -> Keys {
        let mut keys = Keys::new(lit);
        keys.heard(20.0, start);
        assert!(keys.saw(0));
        keys.heard(20.0, start);
        keys.heard(500.0, start);
        keys.heard(500.0, start);
        keys
    }

    /// What the keyboard is set to once the light has held, if anything.
    fn keys_after_the_hold(keys: &mut Keys, now: Instant) -> Option<u32> {
        keys.due(now + KEYS_HOLD)
    }

    #[test]
    fn starting_leaves_the_keyboard_as_it_is() {
        let start = Instant::now();
        let mut keys = Keys::new(2);
        keys.heard(500.0, start);
        assert_eq!(keys.wake(), None, "starting changed the keyboard");
    }

    #[test]
    fn a_dark_room_lights_the_keyboard_and_a_light_one_puts_it_out() {
        let start = Instant::now();
        let mut keys = Keys::new(0);
        keys.heard(500.0, start);
        keys.heard(5.0, start);
        assert_eq!(keys.wake(), Some(start + KEYS_HOLD));
        assert_eq!(keys_after_the_hold(&mut keys, start), Some(1));
        keys.heard(500.0, start + LATER);
        assert_eq!(keys_after_the_hold(&mut keys, start + LATER), Some(0));
    }

    #[test]
    fn nothing_is_due_until_the_light_has_held() {
        let start = Instant::now();
        let mut keys = Keys::new(0);
        keys.heard(500.0, start);
        assert!(!keys.is_due(start + LATER), "due with nothing asked for");
        keys.heard(5.0, start);
        assert!(!keys.is_due(start), "due before the light held");
        assert!(keys.is_due(start + KEYS_HOLD), "not due once it held");
    }

    #[test]
    fn a_hand_over_the_sensor_lights_nothing() {
        let start = Instant::now();
        let mut keys = Keys::new(0);
        keys.heard(500.0, start);
        keys.heard(0.0, start);
        keys.heard(500.0, start + KEYS_HOLD / 2);
        assert_eq!(keys.wake(), None);
        assert_eq!(keys_after_the_hold(&mut keys, start), None);
    }

    #[test]
    fn light_between_dark_and_light_leaves_the_keyboard_as_it_is() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        keys.heard(5.0, start);
        assert_eq!(keys_after_the_hold(&mut keys, start), Some(1));
        for lux in [30.0, 60.0, 95.0, 40.0] {
            keys.heard(lux, start + LATER);
            assert_eq!(keys.wake(), None, "{lux} lux changed it");
        }
    }

    #[test]
    fn a_level_set_by_the_key_holds_until_the_light_moves_well_away() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        // Joel lights it in daylight, and it stays lit while the light
        // wavers about where he lit it, even past where daylight would
        // otherwise put it out.
        assert!(keys.saw(2));
        keys.heard(300.0, start);
        for lux in [250.0, 500.0, 1100.0, 150.0, 80.0] {
            keys.heard(lux, start);
            assert_eq!(keys.wake(), None, "{lux} lux undid Joel's level");
        }
        // Dark, well away from where he lit it, it's lit at his level still.
        keys.heard(5.0, start);
        assert_eq!(keys.wake(), None);
        // So the room has it again: daylight puts it out, and the dark
        // lights it to the level he last picked.
        keys.heard(500.0, start + LATER);
        assert_eq!(keys_after_the_hold(&mut keys, start + LATER), Some(0));
        keys.heard(5.0, start + LATER * 2);
        assert_eq!(keys_after_the_hold(&mut keys, start + LATER * 2), Some(2));
    }

    #[test]
    fn a_level_set_at_dusk_outlasts_light_just_past_daylights_threshold() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        // Lit by the key at dusk, between dark and light, it stays lit in
        // light that's daylight but not yet four times where he lit it.
        assert!(keys.saw(1));
        keys.heard(30.0, start);
        keys.heard(110.0, start);
        assert_eq!(keys.wake(), None, "light just past the threshold undid it");
        keys.heard(500.0, start);
        assert_eq!(keys_after_the_hold(&mut keys, start), Some(0));
    }

    #[test]
    fn the_keyboard_put_out_in_the_dark_stays_out_while_its_dark() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        keys.heard(5.0, start);
        assert_eq!(keys_after_the_hold(&mut keys, start), Some(1));
        assert!(keys.saw(0));
        for lux in [2.0, 0.0, 9.0, 20.0] {
            keys.heard(lux, start + LATER);
            assert_eq!(keys.wake(), None, "{lux} lux lit it again");
        }
    }

    #[test]
    fn with_the_lid_shut_its_dark_lights_nothing() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        keys.lid(true);
        keys.heard(0.0, start);
        assert_eq!(keys.wake(), None);
        keys.lid(false);
        keys.heard(500.0, start);
        assert_eq!(keys.wake(), None);
    }

    #[test]
    fn a_level_that_couldnt_be_set_isnt_taken_for_joels() {
        let start = Instant::now();
        let mut keys = keys_following(1, start);
        keys.heard(5.0, start);
        assert_eq!(keys_after_the_hold(&mut keys, start), Some(1));
        keys.unset(0);
        assert!(!keys.saw(0));
    }
}

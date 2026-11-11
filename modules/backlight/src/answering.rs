//! How the daemon answers what it hears: the light and the warmth read,
//! the switch and the night, the relay starting, a wake, the lid, a screen
//! coming or going, and the steps each level takes toward what it wants.

use std::fs;
use std::io;
use std::time::{Duration, Instant};

use tokio::sync::watch;
use zbus::Connection;

use crate::dbus::RelayProxy;
use crate::night::Place;
use crate::warmth::{WHITE, Warmth};
use crate::{Backlight, RELAY_RESTS, ask, kept_say, night, switch, wall, write_whole};

impl Backlight<'_> {
    /// The light changed. The first light heard teaches the curve the
    /// brightness the screen already has. The keyboard's level isn't read
    /// here: on a Dell each read is a firmware call, which fails under a
    /// build's memory pressure, and the light sensor reports ten times a
    /// second. It's read when a step is due (`keyboard_step`), which is where
    /// a level Joel set with the key has to win.
    pub(crate) fn heard(&mut self, lux: f64) {
        let shown = self.screen.level_of(self.seen);
        if self.follow.heard(lux, shown, Instant::now()) {
            self.keep();
        }
        if let Some((keys, _)) = &mut self.keyboard {
            keys.heard(lux, Instant::now());
        }
        self.warm(|warmth, now| warmth.light(lux, now));
    }

    /// The room's light changed warmth.
    pub(crate) fn warmed(&mut self, kelvin: f64) {
        self.warm(|warmth, now| warmth.room(kelvin, now));
    }

    /// Serves Joel's say over the warmth, where there's a warmth to set,
    /// starting from what he last said. Gives the bus it's served on and
    /// what he says through it: whether it follows the room, and whether
    /// the night light is on.
    pub(crate) async fn serve_switch(
        &mut self,
    ) -> (
        Option<Connection>,
        Option<watch::Receiver<bool>>,
        Option<watch::Receiver<bool>>,
    ) {
        if self.warmth.is_none() {
            return (None, None, None);
        }
        let follows = kept_say(&self.switch);
        if !follows {
            eprintln!("<6>the screen keeps its own white, as Joel said");
        }
        self.night.on = kept_say(&self.night.switch);
        if !self.night.on {
            eprintln!("<6>the night light is off, as Joel said");
        }
        self.warm(|warmth, now| warmth.follow_the_room(follows, now));
        match switch::serve(follows, self.night.on).await {
            Ok((bus, said)) => (Some(bus), Some(said.follows), Some(said.night)),
            Err(error) => {
                eprintln!("<4>can't take Joel's say over the screen's warmth: {error}");
                (None, None, None)
            }
        }
    }

    /// Joel said whether the night light is on: it's kept, and the night
    /// is looked at again at once.
    pub(crate) fn night_switched(&mut self, on: bool) {
        if self.night.on == on {
            return;
        }
        self.night.on = on;
        if on {
            eprintln!("<6>the night light is on, as Joel said");
        } else {
            eprintln!("<6>the night light is off, as Joel said");
        }
        if let Err(error) = write_whole(&self.night.switch, switch::kept(on)) {
            eprintln!("<4>can't keep Joel's say over the night light: {error}");
        }
        self.look_at_night();
    }

    /// What the night light wants now, where whereabouts says the laptop
    /// is, given to the warmth, with when to look again; with no place
    /// known, or the light off, it wants the screen's own white.
    fn look_at_night(&mut self) {
        let place = fs::read_to_string(&self.night.whereabouts)
            .ok()
            .and_then(|text| Place::read(&text));
        let now = wall();
        let plan = place.map(|place| night::plan(now, place, self.night.white));
        if plan.is_some() != self.night.plan.is_some() {
            if plan.is_some() {
                eprintln!("<6>the night light follows the sun where the laptop is");
            } else {
                eprintln!("<5>the night light waits to know where the laptop is");
            }
        }
        self.night.plan = plan;
        let next = plan.map_or(now + night::UNPLACED, |plan| plan.next);
        let wait = u64::try_from(next - now).unwrap_or_default();
        self.night.due = Some(Instant::now() + Duration::from_millis(wait));
        let kelvin = match plan {
            Some(plan) if self.night.on => plan.kelvin,
            _ => WHITE,
        };
        // Said as the evening's ease begins and the morning's ends, rather
        // than for each of its fifty steps.
        if (kelvin < WHITE) != (self.night.wants < WHITE) {
            if kelvin < WHITE {
                eprintln!(
                    "<6>the night light warms the screen toward {} K",
                    self.night.white
                );
            } else {
                eprintln!("<6>the night light gives the screen back its own white");
            }
        }
        self.night.wants = kelvin;
        if let Some((warmth, _)) = &mut self.warmth {
            warmth.night(kelvin, Instant::now());
        }
        // Said even when the warmth holds, since the night's times moved.
        self.say_warmth();
    }

    /// Joel said whether the warmth follows the room: it's kept, and the
    /// screen goes to the room's warmth or back to its own white.
    pub(crate) fn switched(&mut self, follows: bool) {
        if self
            .warmth
            .as_ref()
            .is_some_and(|(warmth, _)| warmth.follows() == follows)
        {
            return;
        }
        if follows {
            eprintln!("<6>the screen's warmth follows the room again, as Joel said");
        } else {
            eprintln!("<6>the screen goes back to its own white, as Joel said");
        }
        if let Err(error) = write_whole(&self.switch, switch::kept(follows)) {
            eprintln!("<4>can't keep Joel's say over the warmth: {error}");
        }
        self.warm(|warmth, now| warmth.follow_the_room(follows, now));
    }

    /// Tells the shell how the warmth stands, when that changed: the
    /// warmth, whether it follows the room, whether a sensor sees colour
    /// and the room's settled colour while the screen follows it (warmth.rs
    /// says why it's settled first), and the night light, on or
    /// off, its white, and the sunset and sunrise of the night under way
    /// or the next, or null for both while where
    /// the laptop is isn't known.
    fn say_warmth(&mut self) {
        let Some((warmth, _)) = &self.warmth else {
            return;
        };
        let (sunset, sunrise) = self
            .night
            .plan
            .and_then(|plan| plan.night)
            .map_or(("null".into(), "null".into()), |night| {
                (night.sunset.to_string(), night.sunrise.to_string())
            });
        let room = warmth
            .room_white()
            .map_or_else(|| "null".into(), |kelvin| kelvin.to_string());
        let text = format!(
            "{{\"kelvin\": {}, \"followsTheRoom\": {}, \"seesColour\": {}, \"roomWhite\": {room}, \
             \"nightLight\": {}, \"nightWhite\": {}, \"sunset\": {sunset}, \
             \"sunrise\": {sunrise}}}\n",
            warmth.stands(),
            warmth.follows(),
            self.sees,
            self.night.on,
            self.night.white,
        );
        if self.noted.as_deref() == Some(text.as_str()) {
            return;
        }
        match write_whole(&self.note, &text) {
            Ok(()) => self.noted = Some(text),
            Err(error) => eprintln!("<4>can't tell the shell the screen's warmth: {error}"),
        }
    }

    /// What sets the warmth started again, at its own white.
    pub(crate) fn relay_started(&mut self) {
        if self.warmth.is_some() {
            eprintln!("<6>the screens' warmth started again at {WHITE} K");
        }
        self.relay_down = None;
        self.warm(Warmth::reset);
    }

    /// Tells the warmth of a change, and says when that starts a glide.
    fn warm(&mut self, change: impl FnOnce(&mut Warmth, Instant)) {
        let Some((warmth, _)) = &mut self.warmth else {
            return;
        };
        let idle = warmth.going().is_none();
        change(warmth, Instant::now());
        // Going back to white when Joel asked is said where he asked.
        if let (true, Some(to), true) = (idle, warmth.going(), warmth.follows()) {
            eprintln!("<6>warms the screens toward {to} K for the room's light");
        }
        self.say_warmth();
    }

    /// Takes the next step of the warmth's glide, once it's due.
    async fn warmth_step(&mut self, now: Instant) {
        if self.relay_down.is_some_and(|until| now < until) {
            return;
        }
        let Some((warmth, relay)) = &mut self.warmth else {
            return;
        };
        let Some(kelvin) = warmth.step(now) else {
            return;
        };
        let relay: &RelayProxy = relay;
        let refused = match ask::twice(ask::MOMENT, || relay.set_temperature(kelvin)).await {
            Some(Ok(())) => {
                warmth.set(kelvin, now);
                None
            }
            Some(Err(error)) => Some(error.to_string()),
            None => Some("wl-gammarelay-rs didn't answer".to_owned()),
        };
        // Where the compositor can't hold gamma, every one of the night's
        // fifty steps would ask again and fail: what sets the warmth is let
        // be until it starts again by itself, or a while has passed, since
        // nothing but a set starts it on the bus.
        if let Some(why) = refused {
            eprintln!("<4>can't set the screens' warmth, so it's let be a while: {why}");
            warmth.abandon();
            self.relay_down = Some(now + RELAY_RESTS);
        }
    }

    /// Reads the keyboard's level, which its key may have changed unheard,
    /// says when Joel set it, and gives it.
    fn keyboard_read(&mut self) -> Option<u32> {
        let (keys, keyboard) = self.keyboard.as_mut()?;
        match keyboard.level() {
            Ok(level) => {
                if keys.saw(level) {
                    eprintln!(
                        "<6>the keyboard light is at {level} of {}, as set",
                        keyboard.max
                    );
                }
                Some(level)
            }
            Err(error) => {
                eprintln!("<4>can't read {}: {error}", keyboard.name);
                None
            }
        }
    }

    /// Sets the keyboard's light once the light has held (follow.rs).
    async fn keyboard_step(&mut self, now: Instant) {
        // The level is read only when there's a change to make, since each
        // read is a firmware call and this runs with every screen step.
        if !self
            .keyboard
            .as_ref()
            .is_some_and(|(keys, _)| keys.is_due(now))
        {
            return;
        }
        // Unread, the level can't be known to be anybody's but Joel's, so
        // what the light asked for is let go.
        let Some(was) = self.keyboard_read() else {
            if let Some((keys, _)) = &mut self.keyboard {
                keys.forget();
            }
            return;
        };
        let lux = self.follow.lux().unwrap_or_default();
        let Some((keys, keyboard)) = &mut self.keyboard else {
            return;
        };
        let Some(level) = keys.due(now) else {
            return;
        };
        eprintln!("<6>follows {lux:.0} lux with the keyboard light at {level}");
        if let Err(error) = self
            .session
            .set_brightness("leds", &keyboard.name, level)
            .await
        {
            eprintln!("<4>can't set the keyboard light: {error}");
            keys.unset(was);
        }
    }

    /// The machine woke. The night's next look is counted on a clock that
    /// stops while it sleeps, so the sun is looked at again now rather
    /// than up to an hour late.
    pub(crate) fn woke(&mut self) {
        self.night.due = Some(Instant::now());
    }

    /// The lid opened or shut, and the light heard is the lid's until it
    /// opens (follow.rs).
    pub(crate) fn lid(&mut self, shut: bool) {
        if let Some((keys, _)) = &mut self.keyboard {
            keys.lid(shut);
        }
        self.warm(|warmth, now| warmth.lid(shut, now));
        // A lid opening is most often a wake, heard before logind says so.
        if !shut {
            self.woke();
        }
        if !self.follow.lid(shut) {
            return;
        }
        if shut {
            eprintln!("<6>the lid is shut, so the light it hears is its own");
        } else {
            eprintln!("<6>the lid is open");
        }
        self.say_level(self.screen.level_of(self.seen));
    }

    /// The brightness was written. Unless this daemon wrote it, Joel picked
    /// it.
    pub(crate) fn screen_changed(&mut self) -> io::Result<()> {
        let steps = self.screen.brightness()?;
        if steps != self.seen {
            self.seen = steps;
            if Some(steps) != self.wrote {
                self.chose(steps);
            }
        }
        Ok(())
    }

    /// Joel picked this brightness: any move stops, the curve learns it for
    /// the light there is, unless the lid is shut, and the shell is told.
    fn chose(&mut self, steps: u32) {
        self.wrote = None;
        let level = self.screen.level_of(steps);
        if let Some(lux) = self.follow.chose(level) {
            self.keep();
            eprintln!("<6>learns {:.0}% for {lux:.0} lux", level * 100.0);
        }
        if let Err(error) = write_whole(&self.chosen, &format!("{level:.3}\n")) {
            eprintln!("<4>can't tell the shell about the pick: {error}");
        }
        self.say_level(level);
    }

    /// Tells the shell the level the screen settled at, the light it was
    /// heard in, what the curve asks for that light, and whether the screen
    /// follows the light now, a fact a line, its name and its value apart
    /// by a tab, as the desk's facts are (`$XDG_RUNTIME_DIR/backlight-now`,
    /// read by Screens' Brightness).
    fn say_level(&mut self, level: f64) {
        let heard = self.follow.lux().map_or_else(String::new, |lux| {
            let asks = self.follow.curve().level_at(lux);
            format!("lux\t{lux:.0}\nasks\t{asks:.3}\n")
        });
        let following = if self.follow.following() { "yes" } else { "no" };
        let text = format!("level\t{level:.3}\n{heard}following\t{following}\n");
        if self.level_said.as_deref() == Some(text.as_str()) {
            return;
        }
        match write_whole(&self.now_file, &text) {
            Ok(()) => self.level_said = Some(text),
            Err(error) => eprintln!("<4>can't tell the shell the screen's level: {error}"),
        }
    }

    /// Keeps what the curve has learned, for the next time the daemon starts.
    fn keep(&self) {
        if let Err(error) = write_whole(&self.learned, &self.follow.curve().write()) {
            eprintln!("<4>can't keep what was learned: {error}");
        }
    }

    /// When to step next (follow.rs).
    pub(crate) fn wake(&self) -> Option<Instant> {
        let keys = self.keyboard.as_ref().and_then(|(keys, _)| keys.wake());
        let warmth = self.warmth.as_ref().and_then(|(warmth, _)| warmth.wake());
        // A glide waits on what sets the warmth while it's let be.
        let warmth = match self.relay_down {
            Some(until) => warmth.map(|due| due.max(until)),
            None => warmth,
        };
        let night = self.warmth.as_ref().and(self.night.due);
        [self.follow.wake(Instant::now()), keys, warmth, night]
            .into_iter()
            .flatten()
            .min()
    }

    /// Starts the move the light asked for once it's due, and takes the next
    /// step of the move under way.
    pub(crate) async fn step(&mut self) {
        let now = Instant::now();
        if self.warmth.is_some() && self.night.due.is_some_and(|due| due <= now) {
            self.look_at_night();
        }
        self.keyboard_step(now).await;
        self.warmth_step(now).await;
        let from = self.screen.level_of(self.seen);
        if let Some(to) = self.follow.start(from, now) {
            let lux = self.follow.lux().unwrap_or_default();
            eprintln!(
                "<6>follows {lux:.0} lux from {:.0}% to {:.0}%",
                from * 100.0,
                to * 100.0
            );
        }
        let Some(level) = self.follow.step(now) else {
            return;
        };
        // A pick that landed since the last step, and isn't heard yet, stops
        // the move rather than being stepped over.
        match self.screen.brightness() {
            Ok(steps) if steps != self.seen && Some(steps) != self.wrote => {
                self.seen = steps;
                self.chose(steps);
                return;
            }
            Ok(steps) => self.seen = steps,
            Err(error) => {
                eprintln!("<4>can't read the backlight: {error}");
                self.follow.abandon();
                return;
            }
        }
        let steps = self.screen.steps_for(level);
        if steps == self.seen {
            return;
        }
        self.wrote = Some(steps);
        if let Err(error) = self
            .session
            .set_brightness("backlight", &self.screen.name, steps)
            .await
        {
            eprintln!("<4>can't set the backlight: {error}");
            self.follow.abandon();
            return;
        }
        // The move's last step is where the level settles.
        if !self.follow.moving() {
            self.say_level(self.screen.level_of(steps));
        }
    }
}

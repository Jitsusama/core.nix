//! backlight: sets the screen's brightness for the light in the room, and
//! learns from the brightness Joel picks.
//!
//! The light comes from the ambient light sensor's buffer, which wakes the
//! daemon only when the light changes (light.rs). The curve says how bright
//! the screen should be in that light (curve.rs), and once the light holds
//! (steer.rs) the brightness moves there through logind, which lets the
//! session set its own backlight.
//!
//! The sensor sits in the lid, so while the lid is shut the light it hears
//! is the lid's: nothing follows it and nothing is learned from it until
//! the lid opens, as logind says it does.
//!
//! What to do about the light, the lid and a pick is decided apart from the
//! screen, the files and D-Bus (follow.rs), so it's tested without them;
//! this file reads and writes what it decides.
//!
//! Whatever else writes the brightness, which is Joel with the keys, teaches
//! the curve: that brightness, in this light, is the one he wants. The first
//! light heard teaches it the brightness the screen already has, so starting
//! never changes the screen. Every pick is also written to
//! $XDG_RUNTIME_DIR/backlight-chosen, which the shell's brightness bar
//! watches, so the bar shows for Joel's changes and never for the light's.
//!
//! The panel's brightness is the one brightness: the keys, the light and the
//! lid all end in it. Each monitor plugged in follows it over DDC/CI, set to
//! the same share of its own brightest (monitor.rs).
//!
//! The keyboard's light follows the room too (keyboard.rs): lit in the dark,
//! out in daylight, and left as Joel sets it with its key until the light
//! moves well away from where he did (follow.rs).
//!
//! Usage: backlight <standard curve>. The standard curve is where learning
//! starts, a point to a line: the lux, a space and the share of the brightest.
//! What it learns is kept in `$XDG_STATE_HOME/backlight/learned`.

mod curve;
mod facts;
mod follow;
mod keyboard;
mod light;
mod monitor;
mod screen;
mod steer;

use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::ExitCode;
use std::time::Instant;

use zbus::export::futures_core::Stream;
use zbus::proxy::PropertyStream;
use zbus::{Connection, proxy};

use curve::Curve;
use follow::{Follow, Keys};
use keyboard::Keyboard;
use light::{Readings, Sensor};
use monitor::Monitors;
use screen::Screen;

/// The session this daemon runs in, which logind lets set its backlight.
#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait Session {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
}

/// logind itself, which says whether the lid is shut and when that changes.
#[proxy(
    interface = "org.freedesktop.login1.Manager",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1"
)]
trait Manager {
    #[zbus(property)]
    fn lid_closed(&self) -> zbus::Result<bool>;
}

/// Waits for the lid to open or shut, or forever once logind stops saying.
async fn lid_moved(lid: &mut Option<Pin<Box<PropertyStream<'_, bool>>>>) -> zbus::Result<bool> {
    let Some(stream) = lid else {
        return std::future::pending().await;
    };
    if let Some(changed) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
        changed.get().await
    } else {
        *lid = None;
        std::future::pending().await
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// The light sensor's readings and the light now, or none when there's no
/// sensor to hear, which leaves the daemon telling the shell of picks.
fn listen() -> Option<(Readings, f64)> {
    let sensor = match Sensor::find(Path::new("/sys/bus/iio/devices"), Path::new("/dev")) {
        Ok(Some(sensor)) => sensor,
        Ok(None) => {
            eprintln!("<5>no light sensor to follow");
            return None;
        }
        Err(error) => {
            eprintln!("<4>can't look for a light sensor: {error}");
            return None;
        }
    };
    match sensor.open() {
        Ok(opened) => Some(opened),
        Err(error) => {
            eprintln!("<4>can't hear {}: {error}", sensor.name());
            None
        }
    }
}

/// Waits for the light to change, or forever when there's no sensor.
async fn light(readings: &mut Option<Readings>) -> io::Result<f64> {
    match readings {
        Some(readings) => readings.changed().await,
        None => std::future::pending().await,
    }
}

/// A folder from an XDG variable, or its fallback under the home folder.
fn xdg(variable: &str, fallback: &str) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(dir) = env::var_os(variable).filter(|dir| !dir.is_empty()) {
        return Ok(PathBuf::from(dir));
    }
    let home = env::var_os("HOME").ok_or("neither XDG nor HOME says where to keep things")?;
    Ok(Path::new(&home).join(fallback))
}

/// Writes a file whole or not at all, so a reader never sees half of it.
fn write_whole(path: &Path, text: &str) -> io::Result<()> {
    let writing = path.with_extension("writing");
    fs::write(&writing, text)?;
    fs::rename(&writing, path)
}

async fn run() -> Result<(), Box<dyn Error>> {
    let standard = env::args_os()
        .nth(1)
        .ok_or("usage: backlight <standard curve>")?;
    let standard = Curve::read(&fs::read_to_string(&standard)?)
        .ok_or_else(|| format!("{} holds no curve", Path::new(&standard).display()))?;

    let Some(screen) = Screen::find(Path::new("/sys/class/backlight"))? else {
        eprintln!("<5>this machine has no backlight to set");
        return Ok(());
    };

    let kept = xdg("XDG_STATE_HOME", ".local/state")?.join("backlight");
    fs::create_dir_all(&kept)?;
    let learned = kept.join("learned");
    let (curve, knows_joel) = match fs::read_to_string(&learned) {
        Ok(text) => {
            if let Some(curve) = Curve::read(&text) {
                (curve, true)
            } else {
                eprintln!(
                    "<4>{} holds no curve, so learning starts again",
                    learned.display()
                );
                (standard, false)
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => (standard, false),
        Err(error) => return Err(error.into()),
    };
    // Beside the runtime folder's other files rather than in a folder of
    // its own, which the shell could start watching before it existed.
    let chosen = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR isn't set")?
        .join("backlight-chosen");

    let bus = Connection::system().await?;
    let session = SessionProxy::new(&bus).await?;
    let manager = ManagerProxy::new(&bus).await?;
    // Heard before it's read, so a change between the two isn't missed.
    let mut lid = Some(Box::pin(manager.receive_lid_closed_changed().await));
    let shut = manager.lid_closed().await?;
    let mut watch = screen.watch()?;
    let seen = screen.brightness()?;
    let keyboard = match Keyboard::find(Path::new("/sys/class/leds")) {
        Ok(Some(keyboard)) => match keyboard.level() {
            Ok(level) => Some((Keys::new(level), keyboard)),
            Err(error) => {
                eprintln!("<4>can't read {}: {error}", keyboard.name);
                None
            }
        },
        Ok(None) => None,
        Err(error) => {
            eprintln!("<4>can't look for a keyboard light: {error}");
            None
        }
    };
    let mut backlight = Backlight {
        screen,
        session,
        follow: Follow::new(curve, knows_joel),
        learned,
        chosen,
        seen,
        wrote: None,
        keyboard,
    };
    backlight.lid(shut);
    let mut monitors = Monitors::find(backlight.screen.level_of(seen));
    let (mut readings, now) = listen().unzip();
    if let Some(lux) = now {
        eprintln!("<6>hears {lux:.0} lux");
        backlight.heard(lux);
    }

    loop {
        let wake = backlight.wake();
        tokio::select! {
            // The keys come first: a pick heard late could be overwritten by
            // a step of a move it should have stopped.
            biased;
            changed = watch.changed() => {
                changed?;
                backlight.screen_changed()?;
            }
            lux = light(&mut readings) => backlight.heard(lux?),
            shut = lid_moved(&mut lid) => backlight.lid(shut?),
            () = until(wake) => backlight.step().await,
            changed = monitors.changed() => {
                changed?;
                monitors.look();
            }
        }
        monitors.follow(backlight.screen.level_of(backlight.seen));
    }
}

/// Waits until a moment, or forever when there's none.
async fn until(moment: Option<Instant>) {
    match moment {
        Some(moment) => tokio::time::sleep_until(moment.into()).await,
        None => std::future::pending().await,
    }
}

struct Backlight<'a> {
    screen: Screen,
    session: SessionProxy<'a>,
    follow: Follow,
    learned: PathBuf,
    chosen: PathBuf,
    /// The brightness last read, in steps.
    seen: u32,
    /// The brightness this daemon last set, in steps.
    wrote: Option<u32>,
    /// The keyboard's light, when there is one, and when it follows the room.
    keyboard: Option<(Keys, Keyboard)>,
}

impl Backlight<'_> {
    /// The light changed. The first light heard teaches the curve the
    /// brightness the screen already has.
    fn heard(&mut self, lux: f64) {
        let shown = self.screen.level_of(self.seen);
        if self.follow.heard(lux, shown, Instant::now()) {
            self.keep();
        }
        self.keyboard_read();
        if let Some((keys, _)) = &mut self.keyboard {
            keys.heard(lux, Instant::now());
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

    /// The lid opened or shut, and the light heard is the lid's until it
    /// opens (follow.rs).
    fn lid(&mut self, shut: bool) {
        if let Some((keys, _)) = &mut self.keyboard {
            keys.lid(shut);
        }
        if !self.follow.lid(shut) {
            return;
        }
        if shut {
            eprintln!("<6>the lid is shut, so the light it hears is its own");
        } else {
            eprintln!("<6>the lid is open");
        }
    }

    /// The brightness was written. Unless this daemon wrote it, Joel picked
    /// it.
    fn screen_changed(&mut self) -> io::Result<()> {
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
    }

    /// Keeps what the curve has learned, for the next time the daemon starts.
    fn keep(&self) {
        if let Err(error) = write_whole(&self.learned, &self.follow.curve().write()) {
            eprintln!("<4>can't keep what was learned: {error}");
        }
    }

    /// When to step next (follow.rs).
    fn wake(&self) -> Option<Instant> {
        let keys = self.keyboard.as_ref().and_then(|(keys, _)| keys.wake());
        match (self.follow.wake(Instant::now()), keys) {
            (Some(screen), Some(keys)) => Some(screen.min(keys)),
            (screen, keys) => screen.or(keys),
        }
    }

    /// Starts the move the light asked for once it's due, and takes the next
    /// step of the move under way.
    async fn step(&mut self) {
        let now = Instant::now();
        self.keyboard_step(now).await;
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
        }
    }
}

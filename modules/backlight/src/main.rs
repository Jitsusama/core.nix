//! backlight: sets the screen's brightness for the light in the room, and
//! learns from the brightness Joel picks.
//!
//! The light comes from the ambient light sensor's buffer, which wakes the
//! daemon only when the light changes (light.rs). The curve says how bright
//! the screen should be in that light (curve.rs), and once the light holds
//! (steer.rs) the brightness moves there through logind, which lets the
//! session set its own backlight.
//!
//! Whatever else writes the brightness, which is Joel with the keys, teaches
//! the curve: that brightness, in this light, is the one he wants. The first
//! light heard teaches it the brightness the screen already has, so starting
//! never changes the screen. Every pick is also written to
//! $XDG_RUNTIME_DIR/backlight-chosen, which the shell's brightness bar
//! watches, so the bar shows for Joel's changes and never for the light's.
//!
//! Usage: backlight <standard curve>. The standard curve is where learning
//! starts, a point to a line: the lux, a space and the share of the brightest.
//! What it learns is kept in $XDG_STATE_HOME/backlight/learned.

mod curve;
mod light;
mod screen;
mod steer;

use std::env;
use std::error::Error;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use zbus::{Connection, proxy};

use curve::Curve;
use light::{Readings, Sensor};
use screen::Screen;
use steer::{Ramp, Steer};

/// How often a move steps while it's under way.
const STEP: Duration = Duration::from_millis(16);

/// The session this daemon runs in, which logind lets set its backlight.
#[proxy(
    interface = "org.freedesktop.login1.Session",
    default_service = "org.freedesktop.login1",
    default_path = "/org/freedesktop/login1/session/auto"
)]
trait Session {
    fn set_brightness(&self, subsystem: &str, name: &str, brightness: u32) -> zbus::Result<()>;
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
        Ok(text) => match Curve::read(&text) {
            Some(curve) => (curve, true),
            None => {
                eprintln!(
                    "<4>{} holds no curve, so learning starts again",
                    learned.display()
                );
                (standard, false)
            }
        },
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
    let mut watch = screen.watch()?;
    let seen = screen.brightness()?;
    let mut backlight = Backlight {
        screen,
        session,
        curve,
        knows_joel,
        learned,
        chosen,
        steer: Steer::default(),
        ramp: None,
        lux: None,
        seen,
        wrote: None,
    };
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
            () = until(wake) => backlight.step().await,
        }
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
    curve: Curve,
    /// Whether the curve has learned anything yet.
    knows_joel: bool,
    learned: PathBuf,
    chosen: PathBuf,
    steer: Steer,
    ramp: Option<Ramp>,
    /// The light last heard, once there's been any.
    lux: Option<f64>,
    /// The brightness last read, in steps.
    seen: u32,
    /// The brightness this daemon last set, in steps.
    wrote: Option<u32>,
}

impl Backlight<'_> {
    /// The light changed. The first light heard teaches the curve the
    /// brightness the screen already has.
    fn heard(&mut self, lux: f64) {
        if !lux.is_finite() || lux < 0.0 {
            return;
        }
        self.lux = Some(lux);
        if !self.knows_joel {
            self.learn(lux, self.screen.level_of(self.seen));
        }
        let shown = self
            .ramp
            .map_or_else(|| self.screen.level_of(self.seen), |ramp| ramp.to());
        self.steer
            .heard(self.curve.level_at(lux), shown, Instant::now());
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
    /// the light there is, and the shell is told.
    fn chose(&mut self, steps: u32) {
        self.ramp = None;
        self.steer.forget();
        self.wrote = None;
        let level = self.screen.level_of(steps);
        if let Some(lux) = self.lux {
            self.learn(lux, level);
            eprintln!("<6>learns {:.0}% for {lux:.0} lux", level * 100.0);
        }
        if let Err(error) = write_whole(&self.chosen, &format!("{level:.3}\n")) {
            eprintln!("<4>can't tell the shell about the pick: {error}");
        }
    }

    fn learn(&mut self, lux: f64, level: f64) {
        self.curve.learn(lux, level);
        self.knows_joel = true;
        if let Err(error) = write_whole(&self.learned, &self.curve.write()) {
            eprintln!("<4>can't keep what was learned: {error}");
        }
    }

    /// When to step next: soon while a move is under way, or when the move
    /// waiting on the light is due.
    fn wake(&self) -> Option<Instant> {
        if self.ramp.is_some() {
            Some(Instant::now() + STEP)
        } else {
            self.steer.next()
        }
    }

    /// Starts the move the light asked for once it's due, and takes the next
    /// step of the move under way.
    async fn step(&mut self) {
        let now = Instant::now();
        if self.ramp.is_none() {
            let Some(to) = self.steer.due(now) else {
                return;
            };
            let from = self.screen.level_of(self.seen);
            let lux = self.lux.unwrap_or_default();
            eprintln!(
                "<6>follows {lux:.0} lux from {:.0}% to {:.0}%",
                from * 100.0,
                to * 100.0
            );
            self.ramp = Some(Ramp::new(from, to, now));
        }
        let Some(ramp) = self.ramp else {
            return;
        };
        if ramp.done(now) {
            self.ramp = None;
        }
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
                self.ramp = None;
                return;
            }
        }
        let steps = self.screen.steps_for(ramp.at(now));
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
            self.ramp = None;
        }
    }
}

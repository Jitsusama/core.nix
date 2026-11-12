//! backlight: sets the screen's brightness for the light in the room, and
//! learns from the brightness Joel picks.
//!
//! The light comes from the ambient light sensor's buffer, which wakes the
//! daemon only when the light changes (light.rs), or from a colour sensor's
//! window of thresholds where no sensor has a buffer for it (windowed.rs).
//! The curve says how bright
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
//! the same share of its own brightest (monitor/).
//!
//! The keyboard's light follows the room too (keyboard.rs): lit in the dark,
//! out in daylight, and left as Joel sets it with its key until the light
//! moves well away from where he did (follow.rs).
//!
//! Where a sensor sees the light's colour, the built-in panel's warmth
//! follows the room's light part of the way (warmth.rs), set through
//! wl-gammarelay-rs, which holds niri's gamma for each screen. When that
//! program starts again, at its own white, the room's warmth is set again.
//! Joel says whether it follows the room through the session's bus
//! (switch.rs), and his say is kept in `$XDG_STATE_HOME/backlight/warmth`.
//! From sunset to sunrise where the laptop is, the night light warms the
//! built-in panel too, whatever its sensor sees (night.rs), through the
//! same program; where it is comes from whereabouts'
//! `$XDG_RUNTIME_DIR/whereabouts/here.json`, what it keeps of where here
//! is, looked at again within the hour,
//! or every ten seconds until it says a place,
//! and Joel's say over it is kept in `$XDG_STATE_HOME/backlight/night`.
//! Where the warmth stands, whether it follows the room, whether a sensor
//! sees colour, and the night light's white, sunset and sunrise are written
//! to `$XDG_RUNTIME_DIR/backlight-warmth` for the shell's Warmth, on every
//! machine with a panel to warm. The daemon's stop leaves `{}` there, which
//! shows no Warmth.
//! Every call to wl-gammarelay-rs is given a moment to answer (ask.rs), so
//! the warmth never holds the brightness, the keys or the lid.
//!
//! Usage: backlight <standard curve> [night white]. The standard curve is
//! where learning starts, a point to a line: the lux, a space and the share
//! of the brightest. What it learns is kept in
//! `$XDG_STATE_HOME/backlight/learned`. The night white is the night light's
//! warmth in kelvin, 4000 unless it's given.
//!
//! Usage: backlight --keeps-the-open-level <saver>, as root. It keeps the
//! panel's saved level current while the lid is open (keep.rs), running the
//! saver with the backlight's systemd-backlight instance each time the
//! brightness holds after a change.

mod answering;
mod ask;
mod curve;
mod dbus;
mod facts;
mod follow;
mod keep;
mod keyboard;
mod light;
mod monitor;
mod night;
mod screen;
mod steer;
mod switch;
mod uevent;
mod warmth;
mod windowed;

use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::io::unix::AsyncFd;
use zbus::Connection;

use curve::Curve;
use dbus::{
    ManagerProxy, RelayProxy, SessionProxy, lid_moved, relay, relay_started, switched, woke,
};
use follow::{Follow, Keys};
use keyboard::Keyboard;
use light::{Heard, Readings, Sensor};
use monitor::Monitors;
use night::Plan;
use screen::Screen;
use warmth::{WHITE, Warmth};

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    let ran = if env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--keeps-the-open-level")
    {
        keep_the_open_level().await
    } else {
        run().await
    };
    match ran {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("<3>{error}");
            ExitCode::FAILURE
        }
    }
}

/// A light sensor's readings and the reading now, or none when there's no
/// sensor to hear. Without the light the daemon only tells the shell of
/// picks, and without the warmth the screens keep their own white. Only the
/// first look says it found none. The light comes from a colour sensor's
/// window where no sensor has a buffer for it.
fn listen(heard: Heard, first: bool) -> Option<(Readings, f64)> {
    let (root, dev) = (Path::new("/sys/bus/iio/devices"), Path::new("/dev"));
    let found = Sensor::find(root, dev, heard);
    if heard == Heard::Light && matches!(found, Ok(None)) {
        match windowed::Sensor::find(root, dev) {
            Ok(Some(sensor)) => {
                let name = sensor.name();
                return match sensor.open() {
                    Ok((window, lux)) => Some((Readings::Windowed(window), lux)),
                    Err(error) => {
                        eprintln!("<4>can't hear {name}: {error}");
                        None
                    }
                };
            }
            Ok(None) => {}
            Err(error) => eprintln!("<4>can't look for a colour sensor: {error}"),
        }
    }
    let sensor = match found {
        Ok(Some(sensor)) => sensor,
        Ok(None) => {
            match heard {
                _ if !first => {}
                Heard::Light => eprintln!("<5>no light sensor to follow"),
                Heard::Warmth => eprintln!("<5>no light sensor sees the light's colour yet"),
            }
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

/// The light sensor may be set up only after the daemon starts, as at a
/// boot where the sensor hub comes late, so until it's heard it's looked
/// for again each time udev sets up an IIO device. Only the first look says
/// it found none.
fn look_for_light(readings: &mut Option<Readings>, backlight: &mut Backlight, first: bool) {
    if readings.is_some() {
        return;
    }
    if let Some((heard, lux)) = listen(Heard::Light, first) {
        eprintln!("<6>hears {lux:.0} lux");
        *readings = Some(heard);
        backlight.heard(lux);
    }
}

/// Whether a sensor the daemon could hear is still to be found: the light,
/// or the warmth where there's a warmth to set.
fn missing(readings: Option<&Readings>, warm: Option<&Readings>, backlight: &Backlight) -> bool {
    readings.is_none() || (warm.is_none() && backlight.warmth.is_some())
}

/// udev's uevents, which say when it has set up a light sensor, or none
/// when they can't be heard.
fn hear_sensors() -> Option<AsyncFd<File>> {
    uevent::socket_for(uevent::UDEV)
        .inspect_err(|error| eprintln!("<4>can't hear light sensors come: {error}"))
        .ok()
}

/// udev said it set up an IIO device, or it couldn't be heard: looks again
/// for a missing sensor, and stops hearing udev once none is missing, or
/// once it can't be heard, since only a late sensor is lost then.
fn sensors_came(
    came: io::Result<bool>,
    sensors: &mut Option<AsyncFd<File>>,
    readings: &mut Option<Readings>,
    warm: &mut Option<Readings>,
    backlight: &mut Backlight,
) {
    match came {
        Ok(true) => {
            look_for_light(readings, backlight, false);
            look_for_warmth(warm, backlight);
        }
        Ok(false) => {}
        Err(error) => {
            eprintln!("<4>can't hear light sensors come: {error}");
            *sensors = None;
        }
    }
    if !missing(readings.as_ref(), warm.as_ref(), backlight) {
        *sensors = None;
    }
}

/// The sensor's colour may be set up only after the daemon starts, as at a
/// switch or a boot where the sensor hub comes late, so until it's heard
/// it's looked for again with each light, and each time udev sets up an
/// IIO device.
fn look_for_warmth(warm: &mut Option<Readings>, backlight: &mut Backlight) {
    if warm.is_some() || backlight.warmth.is_none() {
        return;
    }
    if let Some((heard, kelvin)) = listen(Heard::Warmth, false) {
        eprintln!("<6>hears the room's light at {kelvin:.0} K");
        *warm = Some(heard);
        backlight.sees = true;
        backlight.warmed(kelvin);
    }
}

/// Whether Joel's say kept in a file is on, as switch.rs reads it; on
/// while nothing is kept yet, or what's kept can't be read.
fn kept_say(path: &Path) -> bool {
    match fs::read_to_string(path) {
        Ok(text) => switch::says(&text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(error) => {
            eprintln!("<4>can't read {}: {error}", path.display());
            true
        }
    }
}

/// The keyboard's light and its level, where there is one to set.
fn keyboard() -> Option<(Keys, Keyboard)> {
    match Keyboard::find(Path::new("/sys/class/leds")) {
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
    }
}

/// Waits for a reading to change, or forever when there's no sensor.
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

/// Keeps the panel's saved level current while the lid is open, through the
/// saver the command line names (keep.rs).
async fn keep_the_open_level() -> Result<(), Box<dyn Error>> {
    let saver = env::args_os()
        .nth(2)
        .ok_or("usage: backlight --keeps-the-open-level <saver>")?;
    let Some(screen) = Screen::find(Path::new("/sys/class/backlight"))? else {
        eprintln!("<5>this machine has no backlight to keep");
        return Ok(());
    };
    let instance = format!("backlight:{}", screen.name);
    let mut watch = screen.watch()?;
    keep::keep(
        keep::HOLD,
        async || watch.changed().await,
        || match Command::new(&saver).arg(&instance).status() {
            Ok(status) if status.success() => (),
            Ok(status) => eprintln!("<4>saving {instance} ended {status}"),
            Err(error) => eprintln!("<4>can't save {instance}: {error}"),
        },
    )
    .await?;
    Ok(())
}

/// The night white the command line gives, after the standard curve, or
/// the night light's own.
fn night_white() -> Result<u16, String> {
    let Some(said) = env::args().nth(2) else {
        return Ok(NIGHT_WHITE);
    };
    said.parse::<u16>()
        .ok()
        .filter(|kelvin| (1000..=WHITE).contains(kelvin))
        .ok_or_else(|| format!("{said} is no night white between 1000 and {WHITE} K"))
}

/// What learning goes on from: the curve kept in `learned`, and that it's
/// Joel's, or the standard curve while none is kept or what's kept holds
/// none.
fn learned_curve(standard: Curve, learned: &Path) -> io::Result<(Curve, bool)> {
    match fs::read_to_string(learned) {
        Ok(text) => Ok(if let Some(curve) = Curve::read(&text) {
            (curve, true)
        } else {
            eprintln!(
                "<4>{} holds no curve, so learning starts again",
                learned.display()
            );
            (standard, false)
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok((standard, false)),
        Err(error) => Err(error),
    }
}

async fn run() -> Result<(), Box<dyn Error>> {
    let standard = env::args_os()
        .nth(1)
        .ok_or("usage: backlight <standard curve> [night white]")?;
    let standard = Curve::read(&fs::read_to_string(&standard)?)
        .ok_or_else(|| format!("{} holds no curve", Path::new(&standard).display()))?;
    let night_white = night_white()?;

    let Some(screen) = Screen::find(Path::new("/sys/class/backlight"))? else {
        eprintln!("<5>this machine has no backlight to set");
        return Ok(());
    };

    let kept = xdg("XDG_STATE_HOME", ".local/state")?.join("backlight");
    fs::create_dir_all(&kept)?;
    let learned = kept.join("learned");
    let (curve, knows_joel) = learned_curve(standard, &learned)?;
    // Beside the runtime folder's other files rather than in a folder of
    // its own, which the shell could start watching before it existed.
    let runtime = env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .ok_or("XDG_RUNTIME_DIR isn't set")?;

    let bus = Connection::system().await?;
    let session = SessionProxy::new(&bus).await?;
    let manager = ManagerProxy::new(&bus).await?;
    // Heard before it's read, so a change between the two isn't missed.
    let mut lid = Box::pin(manager.receive_lid_closed_changed().await);
    let mut sleeps = Box::pin(manager.receive_prepare_for_sleep().await?);
    let shut = manager.lid_closed().await?;
    let mut watch = screen.watch()?;
    let seen = screen.brightness()?;
    let (warmth, mut started) = relay(&screen).await;
    let mut backlight = Backlight {
        screen,
        session,
        follow: Follow::new(curve, knows_joel),
        learned,
        chosen: runtime.join("backlight-chosen"),
        now_file: runtime.join("backlight-now"),
        level_said: None,
        seen,
        wrote: None,
        keyboard: keyboard(),
        warmth,
        switch: kept.join("warmth"),
        note: runtime.join("backlight-warmth"),
        noted: None,
        sees: false,
        night: Nightly::new(
            night_white,
            kept.join("night"),
            runtime.join("whereabouts/here.json"),
        ),
        relay_down: None,
    };
    // Held for as long as the daemon runs, since it holds the switch's name.
    let (_switch_bus, mut said, mut said_night) = backlight.serve_switch().await;
    // Heard before they're looked for, so a sensor set up between isn't
    // missed.
    let mut sensors = hear_sensors();
    let (mut warm, room) = listen(Heard::Warmth, true).unzip();
    backlight.sees = warm.is_some();
    backlight.lid(shut);
    let mut monitors = Monitors::find(backlight.screen.level_of(seen));
    let mut readings = None;
    look_for_light(&mut readings, &mut backlight, true);
    if let (Some(kelvin), Some(_)) = (room, &backlight.warmth) {
        eprintln!("<6>hears the room's light at {kelvin:.0} K");
        backlight.warmed(kelvin);
    }
    sensors = sensors.filter(|_| missing(readings.as_ref(), warm.as_ref(), &backlight));

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
            lux = light(&mut readings) => {
                backlight.heard(lux?);
                look_for_warmth(&mut warm, &mut backlight);
            }
            kelvin = light(&mut warm) => backlight.warmed(kelvin?),
            shut = lid_moved(&mut lid) => backlight.lid(shut?),
            resumed = woke(&mut sleeps) => {
                resumed?;
                backlight.woke();
            }
            follows = switched(&mut said) => backlight.switched(follows),
            on = switched(&mut said_night) => backlight.night_switched(on),
            again = relay_started(&mut started) => if again {
                backlight.relay_started();
            },
            () = until(wake) => backlight.step().await,
            changed = monitors.changed() => {
                changed?;
                monitors.look();
            }
            came = uevent::heard(sensors.as_ref(), light::sets_up_a_sensor) => {
                sensors_came(came, &mut sensors, &mut readings, &mut warm, &mut backlight);
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
    /// Where the shell is told the level each time it settles, and what it
    /// was last told.
    now_file: PathBuf,
    level_said: Option<String>,
    /// The brightness last read, in steps.
    seen: u32,
    /// The brightness this daemon last set, in steps.
    wrote: Option<u32>,
    /// The keyboard's light, when there is one, and when it follows the room.
    keyboard: Option<(Keys, Keyboard)>,
    /// The screens' warmth and what sets it, where a sensor sees colour.
    warmth: Option<(Warmth, RelayProxy<'a>)>,
    /// Where Joel's say over the warmth is kept.
    switch: PathBuf,
    /// Where the shell is told how the warmth stands, and what it was last
    /// told.
    note: PathBuf,
    noted: Option<String>,
    /// Whether a sensor sees the light's colour.
    sees: bool,
    /// The night light.
    night: Nightly,
    /// Until when what sets the warmth is let be, after it couldn't.
    relay_down: Option<Instant>,
}

/// How long what sets the warmth is let be after it couldn't, unless it
/// starts again by itself sooner.
const RELAY_RESTS: Duration = Duration::from_mins(10);

/// The night light's white unless the command line gives another: GNOME's
/// Night Light's, gentle enough that code's colours still read (VKCF3Z,
/// 2026-11-08).
const NIGHT_WHITE: u16 = 4000;

/// The night light: its white, whether Joel has it on and where that's
/// kept, where whereabouts says the laptop is, what it wants now, and when
/// to look again.
struct Nightly {
    white: u16,
    on: bool,
    switch: PathBuf,
    whereabouts: PathBuf,
    plan: Option<Plan>,
    wants: u16,
    due: Option<Instant>,
}

impl Nightly {
    /// On, as it starts until Joel's kept say is read, knowing no night yet,
    /// and due to look at once.
    fn new(white: u16, switch: PathBuf, whereabouts: PathBuf) -> Self {
        Nightly {
            white,
            on: true,
            switch,
            whereabouts,
            plan: None,
            wants: WHITE,
            due: Some(Instant::now()),
        }
    }
}

/// Milliseconds since 1970 began in UTC, by the wall clock, which the sun
/// keeps to.
fn wall() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since| i64::try_from(since.as_millis()).ok())
        .unwrap_or_default()
}

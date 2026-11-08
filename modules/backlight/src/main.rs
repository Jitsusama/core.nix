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
//! Where a sensor sees the light's colour, the built-in panel's warmth
//! follows the room's light part of the way (warmth.rs), set through
//! wl-gammarelay-rs, which holds niri's gamma for each screen. When that
//! program starts again, at its own white, the room's warmth is set again.
//! Joel says whether it follows the room through the session's bus
//! (switch.rs), and his say is kept in `$XDG_STATE_HOME/backlight/warmth`.
//! From sunset to sunrise where the laptop is, the night light warms the
//! built-in panel too, whatever its sensor sees (night.rs), through the
//! same program; where it is comes from whereabouts'
//! `$XDG_RUNTIME_DIR/whereabouts.json`, looked at again within the hour,
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

mod ask;
mod curve;
mod facts;
mod follow;
mod keep;
mod keyboard;
mod light;
mod monitor;
mod night;
mod screen;
mod steer;
// The sun's reckoning the launcher's calendars use, one file for both, so
// the night falls here when Date says the day ends.
#[path = "../../quickshell/timekeeping/src/sun.rs"]
mod sun;
mod switch;
mod uevent;
mod warmth;

use std::env;
use std::error::Error;
use std::fs;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::process::{Command, ExitCode};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use tokio::io::unix::AsyncFd;
use tokio::sync::watch;
use zbus::export::futures_core::Stream;
use zbus::fdo::DBusProxy;
use zbus::proxy::{CacheProperties, OwnerChangedStream, PropertyStream};
use zbus::{Connection, proxy};

use curve::Curve;
use follow::{Follow, Keys};
use keyboard::Keyboard;
use light::{Heard, Readings, Sensor};
use monitor::Monitors;
use night::{Place, Plan};
use screen::Screen;
use warmth::{WHITE, Warmth};

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

    /// Said as the machine goes to sleep, `start` true, and as it wakes.
    #[zbus(signal)]
    fn prepare_for_sleep(&self, start: bool) -> zbus::Result<()>;
}

/// wl-gammarelay-rs, which sets every screen's warmth through niri.
#[proxy(
    interface = "rs.wl.gammarelay",
    default_service = "rs.wl-gammarelay",
    default_path = "/"
)]
trait Relay {
    #[zbus(property)]
    fn temperature(&self) -> zbus::Result<u16>;
    #[zbus(property)]
    fn set_temperature(&self, kelvin: u16) -> zbus::Result<()>;
}

/// Waits for the screens' warmth to be set by a program that just started,
/// or forever once the bus stops saying.
async fn relay_started(owners: &mut Option<Pin<Box<OwnerChangedStream<'_>>>>) -> bool {
    let Some(stream) = owners else {
        return std::future::pending().await;
    };
    if let Some(owner) = std::future::poll_fn(|cx| stream.as_mut().poll_next(cx)).await {
        owner.is_some()
    } else {
        *owners = None;
        std::future::pending().await
    }
}

/// Waits for Joel to say whether the warmth follows the room, or forever
/// once nothing can.
async fn switched(said: &mut Option<watch::Receiver<bool>>) -> bool {
    let Some(heard) = said else {
        return std::future::pending().await;
    };
    if heard.changed().await.is_ok() {
        *heard.borrow_and_update()
    } else {
        *said = None;
        std::future::pending().await
    }
}

/// Waits for the machine to wake from sleep, however it slept: an idle
/// suspend or a suspend asked for wakes with no lid moving. Once logind
/// stops saying, as when the system bus goes, it fails, so the daemon
/// stops and systemd starts it again hearing it afresh.
async fn woke(sleeps: &mut Pin<Box<PrepareForSleepStream>>) -> zbus::Result<()> {
    loop {
        let Some(said) = std::future::poll_fn(|cx| sleeps.as_mut().poll_next(cx)).await else {
            return Err(zbus::Error::Failure(
                "logind stopped saying when it sleeps".into(),
            ));
        };
        if said.args().is_ok_and(|args| !*args.start()) {
            return Ok(());
        }
    }
}

/// Waits for the lid to open or shut. Once logind stops saying, as when
/// the system bus goes, it fails, as `woke` does.
async fn lid_moved(lid: &mut Pin<Box<PropertyStream<'_, bool>>>) -> zbus::Result<bool> {
    match std::future::poll_fn(|cx| lid.as_mut().poll_next(cx)).await {
        Some(changed) => changed.get().await,
        None => Err(zbus::Error::Failure(
            "logind stopped saying where the lid is".into(),
        )),
    }
}

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
/// first look says it found none.
fn listen(heard: Heard, first: bool) -> Option<(Readings, f64)> {
    let sensor = match Sensor::find(Path::new("/sys/bus/iio/devices"), Path::new("/dev"), heard) {
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

/// Each time what sets the screens' warmth starts.
type Started = Option<Pin<Box<OwnerChangedStream<'static>>>>;

/// What sets the built-in panel's warmth, on the session's bus, with the
/// warmth it has and its starts heard. Only that panel is warmed, as True
/// Tone does: the sensor sits beside it, and another screen's white is its
/// own. None when the panel can't be named or the bus can't be reached.
async fn relay(screen: &Screen) -> (Option<(Warmth, RelayProxy<'static>)>, Started) {
    let Some(connector) = screen.connector() else {
        eprintln!("<5>{} names no panel, so no screen is warmed", screen.name);
        return (None, None);
    };
    // wl-gammarelay-rs names each screen's object for its connector, with
    // a dash turned into an underscore.
    let path = format!("/outputs/{}", connector.replace('-', "_"));
    let bus = match Connection::session().await {
        Ok(bus) => bus,
        Err(error) => {
            eprintln!("<4>can't reach the session's bus to warm the screens: {error}");
            return (None, None);
        }
    };
    // Uncached, since the program may start after the daemon, or again.
    let built = match RelayProxy::builder(&bus).path(path) {
        Ok(builder) => builder.cache_properties(CacheProperties::No).build().await,
        Err(error) => Err(error),
    };
    let relay = match built {
        Ok(relay) => relay,
        Err(error) => {
            eprintln!("<4>can't ask for the screens' warmth: {error}");
            return (None, None);
        }
    };
    // Heard before it's read, so a start between the two isn't missed.
    let started = relay
        .inner()
        .receive_owner_changed()
        .await
        .ok()
        .map(Box::pin);
    // Read only from a relay already running: asking would start one, which
    // the bus does on the first warmth set, and one starts at the screen's
    // own white.
    let running = match DBusProxy::new(&bus).await {
        Ok(owners) => owners
            .name_has_owner(relay.inner().destination().clone())
            .await
            .unwrap_or(false),
        Err(_) => false,
    };
    let shown = if running {
        ask::twice(ask::MOMENT, || relay.temperature())
            .await
            .and_then(Result::ok)
            .unwrap_or(WHITE)
    } else {
        WHITE
    };
    (Some((Warmth::new(shown), relay)), started)
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
            runtime.join("whereabouts.json"),
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
            woke = woke(&mut sleeps) => {
                woke?;
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

impl Backlight<'_> {
    /// The light changed. The first light heard teaches the curve the
    /// brightness the screen already has. The keyboard's level isn't read
    /// here: on a Dell each read is a firmware call, which fails under a
    /// build's memory pressure, and the light sensor reports ten times a
    /// second. It's read when a step is due (`keyboard_step`), which is where
    /// a level Joel set with the key has to win.
    fn heard(&mut self, lux: f64) {
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
    fn warmed(&mut self, kelvin: f64) {
        self.warm(|warmth, now| warmth.room(kelvin, now));
    }

    /// Serves Joel's say over the warmth, where there's a warmth to set,
    /// starting from what he last said. Gives the bus it's served on and
    /// what he says through it: whether it follows the room, and whether
    /// the night light is on.
    async fn serve_switch(
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
    fn night_switched(&mut self, on: bool) {
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
        let next = plan.map_or(now + night::EASE * 2, |plan| plan.next);
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
    fn switched(&mut self, follows: bool) {
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
    /// and the room's colour it last heard, and the night light, on or
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
    fn relay_started(&mut self) {
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
    fn woke(&mut self) {
        self.night.due = Some(Instant::now());
    }

    /// The lid opened or shut, and the light heard is the lid's until it
    /// opens (follow.rs).
    fn lid(&mut self, shut: bool) {
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
    fn wake(&self) -> Option<Instant> {
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
    async fn step(&mut self) {
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

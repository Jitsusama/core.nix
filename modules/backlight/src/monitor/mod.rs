//! The monitors beside the panel, whose brightness follows the panel's.
//!
//! A monitor takes its brightness over DDC/CI, a small protocol on its
//! cable's I2C bus, which the kernel offers as /dev/i2c-N. Only the buses of
//! DisplayPort connectors, and those a dock's MST hub makes for the monitors
//! behind it, are tried; the panel's own and the machine's other buses are
//! left alone. udev lets the person at the seat open those buses. A
//! connector DRM says has nothing on it is passed over, so an empty port
//! isn't tried and warned about at every start.
//!
//! A connector's bus passes through to the hub behind it, so a monitor on a
//! dock is reached twice: by its hub's bus and by the connector's. Two
//! threads on one monitor would break the gap it wants between commands, so
//! a connector's bus whose monitor's EDID matches one a hub's bus reads is
//! left to the hub's, which is the monitor's own. Hub buses are never matched
//! with each other, since two of one model can share an EDID that has no
//! serial.
//!
//! A monitor answers slowly, a second or so for a read, so each is read once,
//! to learn its brightest, and after that only written. Each has a thread of
//! its own that owns its bus, so the daemon never waits on one, and it writes
//! only the latest level it was asked for: a move of many steps becomes as
//! many writes as the monitor takes, never a queue of them.
//!
//! Monitors come and go as their buses do, which a watch on /dev hears, or
//! as a connector's cable does, which the kernel's uevents say, since a
//! connector's bus stays whether anything is on it. Nothing is looked for
//! until one of them does.
//!
//! HP's Z27k G3 shows nothing during a boot with the laptop's lid shut unless
//! its DisplayPort hot-plug detection is set to Always on, and a reset or a
//! firmware update would quietly put it back to Low power. So each time one
//! is found, its setting is read, and set to Always on only if it isn't.
//!
//! What a monitor says of itself goes to a file for the launcher (facts.rs):
//! its capabilities string, then the contrast, colour preset, panel,
//! controller and firmware level it lists, read by its thread only while
//! no brightness is waiting to be set, so a key press never waits behind
//! them.
//!
//! Both need to know which monitor a bus reaches, from its EDID. A monitor
//! coming back after a sleep or a replug can answer for its brightness
//! before its EDID answers, so its thread starts knowing none, and is told
//! at the next look that reads it.

mod buses;
mod ddc;

use std::collections::{HashMap, VecDeque};
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::{env, thread};

use tokio::io::unix::AsyncFd;

use crate::facts::{self, Told};
use crate::uevent;
use buses::{Reach, Seen, buses, doubled, names_a_bus, says_a_cable_moved, watch};
use ddc::{
    BRIGHTNESS, Bus, CONTRAST, CONTROLLER, FIRMWARE, Identity, LONGEST_CAPABILITIES, PANEL, PRESET,
    Reading, hold_hot_plug, holds_hot_plug, hot_plug_named, reached, value_for,
};

/// What a monitor's thread is asked for: the latest level, which monitor it
/// reaches once that's learned after it started, and whether its bus has
/// gone.
#[derive(Default)]
struct Wanted {
    level: Option<f64>,
    identity: Option<Identity>,
    gone: bool,
}

impl Wanted {
    /// Asks for a level, in place of any not yet taken.
    fn want(&mut self, level: f64) {
        self.level = Some(level);
    }

    /// The level asked for since the last one taken, if any.
    fn take(&mut self) -> Option<f64> {
        self.level.take()
    }
}

/// What the daemon and a monitor's thread share.
struct Shared {
    wanted: Mutex<Wanted>,
    woken: Condvar,
}

impl Shared {
    fn wanted(&self) -> MutexGuard<'_, Wanted> {
        // A thread that panicked holding it left a level, which is still one.
        self.wanted.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Waits for a level to be asked for, the bus to go, or, while there's
    /// other work, gives the thread a turn for it; a level comes first.
    fn next(&self, busy: bool) -> Next {
        let mut wanted = self.wanted();
        loop {
            if wanted.gone {
                return Next::Gone;
            }
            if let Some(level) = wanted.take() {
                return Next::Level(level);
            }
            if let Some(identity) = wanted.identity.take() {
                return Next::Identified(identity);
            }
            if busy {
                return Next::Idle;
            }
            wanted = self
                .woken
                .wait(wanted)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// What a monitor's thread does next.
#[derive(Debug, PartialEq)]
enum Next {
    Level(f64),
    Identified(Identity),
    Idle,
    Gone,
}

/// Work a monitor's thread does while no level is waiting.
enum Chore {
    /// Read its capabilities string from an offset, with what's read so far
    /// and how many times this piece has failed.
    Capabilities {
        offset: u16,
        read: Vec<u8>,
        failed: u8,
    },
    /// Read a VCP code it lists, for a fact.
    Read(u8),
}

/// Does one chore, giving those it leads to.
fn chore(name: &str, bus: &mut Bus, told: &mut Told, chore: Chore) -> Vec<Chore> {
    match chore {
        Chore::Capabilities {
            offset,
            mut read,
            failed,
        } => match bus.read_capabilities(offset) {
            Ok(piece) if !piece.is_empty() && offset < LONGEST_CAPABILITIES => {
                read.extend_from_slice(&piece);
                let offset = offset + u16::try_from(piece.len()).unwrap_or(u16::MAX);
                vec![Chore::Capabilities {
                    offset,
                    read,
                    failed: 0,
                }]
            }
            Ok(_) => {
                let capabilities = String::from_utf8_lossy(&read);
                if let Some(version) = facts::mccs_version(&capabilities) {
                    told.set("MCCS", version);
                }
                let codes = facts::listed(&capabilities);
                [CONTRAST, PRESET, PANEL, CONTROLLER, FIRMWARE]
                    .into_iter()
                    .filter(|code| codes.contains(code))
                    .map(Chore::Read)
                    .collect()
            }
            Err(_) if failed < 2 => vec![Chore::Capabilities {
                offset,
                read,
                failed: failed + 1,
            }],
            Err(error) => {
                eprintln!("<5>the monitor on {name} doesn't say what it takes: {error}");
                Vec::new()
            }
        },
        Chore::Read(code) => {
            // One that fails is left out, rather than failing the rest.
            if let Ok(Reading { max, now }) = bus.read_vcp(code) {
                match code {
                    CONTRAST => told.set("contrast", share(now, max)),
                    PRESET => told.set("colour preset", facts::preset(now)),
                    PANEL => told.set("panel", facts::panel(now)),
                    CONTROLLER => told.set("controller", facts::controller(now)),
                    FIRMWARE => told.set("firmware level", facts::firmware(now)),
                    _ => {}
                }
            }
            Vec::new()
        }
    }
}

/// A monitor's brightness, as a share of its brightest.
fn share(value: u16, max: u16) -> String {
    format!(
        "{}%",
        (u32::from(value) * 100 + u32::from(max) / 2) / u32::from(max.max(1))
    )
}

/// A monitor's thread, which ends once this is dropped, and whether it
/// knows which monitor it reaches.
struct Monitor {
    shared: Arc<Shared>,
    identified: bool,
}

impl Monitor {
    fn start(name: &str, bus: Bus, identity: Option<Identity>, level: f64) -> io::Result<Monitor> {
        let shared = Arc::new(Shared {
            wanted: Mutex::new(Wanted {
                level: Some(level),
                identity: None,
                gone: false,
            }),
            woken: Condvar::new(),
        });
        let owned = name.to_owned();
        let theirs = Arc::clone(&shared);
        thread::Builder::new()
            .name(format!("monitor on {name}"))
            .spawn(move || follow(&owned, bus, identity, &theirs))?;
        Ok(Monitor {
            shared,
            identified: identity.is_some(),
        })
    }

    fn want(&self, level: f64) {
        self.shared.wanted().want(level);
        self.shared.woken.notify_one();
    }

    /// Tells a thread that started before its monitor's EDID answered
    /// which monitor it reaches, once; one that knows is told nothing.
    fn identify(&mut self, identity: Option<Identity>) {
        let Some(identity) = identity.filter(|_| !self.identified) else {
            return;
        };
        self.identified = true;
        self.shared.wanted().identity = Some(identity);
        self.shared.woken.notify_one();
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.shared.wanted().gone = true;
        self.shared.woken.notify_one();
    }
}

/// What a monitor's thread does once it knows which monitor it reaches:
/// holds a Z27k G3's hot-plug detection, and keeps its facts in a file of
/// their own, starting with its brightness and capabilities.
fn identified(
    name: &str,
    bus: &mut Bus,
    identity: Option<Identity>,
    brightness: String,
) -> (Told, VecDeque<Chore>) {
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let mut told = Told::new(runtime.as_deref(), identity.as_ref().map(|each| &each[..]));
    told.set("brightness", brightness);
    if identity.as_ref().is_some_and(holds_hot_plug)
        && let Some(held) = hold_hot_plug(name, bus)
    {
        told.set("hot-plug detection", hot_plug_named(held).to_owned());
    }
    let mut chores = VecDeque::new();
    if told.kept() {
        chores.push_back(Chore::Capabilities {
            offset: 0,
            read: Vec::new(),
            failed: 0,
        });
    }
    (told, chores)
}

/// A monitor's thread: learns its brightest, holds a Z27k G3's hot-plug
/// detection, then sets each level asked for, unless it already shows it.
/// A bus can answer for brightness before its EDID answers, as a monitor
/// comes back after a sleep, so the thread may learn its monitor later.
fn follow(name: &str, mut bus: Bus, identity: Option<Identity>, shared: &Shared) {
    let found = bus
        .read_vcp(BRIGHTNESS)
        .or_else(|_| bus.read_vcp(BRIGHTNESS));
    let Reading { max, now } = match found {
        Ok(found) => found,
        Err(_) if shared.wanted().gone => return,
        Err(error) => {
            eprintln!(
                "<4>nothing on {name} answers for its brightness, so it's left alone: {error}"
            );
            return;
        }
    };
    eprintln!("<6>the monitor on {name} follows the panel, {now} of {max}");
    let (mut told, mut chores) = identified(name, &mut bus, identity, share(now, max));
    let mut shown = now;
    let mut failing = false;
    loop {
        let busy = !chores.is_empty() || told.unsaid();
        let level = match shared.next(busy) {
            Next::Gone => break,
            Next::Idle => {
                match chores.pop_front() {
                    Some(work) => chores.extend(chore(name, &mut bus, &mut told, work)),
                    None => told.say(),
                }
                continue;
            }
            Next::Identified(identity) => {
                eprintln!("<6>the monitor on {name} has answered with its EDID, so it's known now");
                (told, chores) = identified(name, &mut bus, Some(identity), share(shown, max));
                continue;
            }
            Next::Level(level) => level,
        };
        let value = value_for(level, max);
        if value == shown {
            continue;
        }
        match bus.write_vcp(BRIGHTNESS, value) {
            Ok(()) => {
                shown = value;
                failing = false;
                told.set("brightness", share(shown, max));
            }
            // Once, rather than for every step of a move.
            Err(error) if !failing => {
                eprintln!("<4>can't set the monitor on {name}: {error}");
                failing = true;
            }
            Err(_) => {}
        }
    }
    eprintln!("<6>the monitor on {name} has gone");
}

/// The monitors there are, each following the panel's level.
pub struct Monitors {
    class: PathBuf,
    dev: PathBuf,
    /// The panel's level, as a share of its brightest.
    level: f64,
    /// Each bus found, by name.
    buses: HashMap<String, Found>,
    watch: Option<AsyncFd<File>>,
    /// The kernel's uevents, which say when a connector's cable comes or
    /// goes: its bus stays in /dev either way, so the watch can't hear it.
    cables: Option<AsyncFd<File>>,
}

/// A bus that reaches a monitor.
struct Found {
    reach: Reach,
    /// Its monitor, once its EDID has been read.
    identity: Option<Identity>,
    /// Its monitor's thread, or none while it can't be opened, as before
    /// udev has opened it to the seat, or while it's left to another bus.
    monitor: Option<Monitor>,
    /// Whether it's been tried and couldn't be opened, said once.
    tried: bool,
    /// The bus its monitor is left to, said once.
    left_to: Option<String>,
}

impl Monitors {
    /// The monitors there are now, set to the panel's level.
    pub fn find(level: f64) -> Monitors {
        let dev = PathBuf::from("/dev");
        // Watched before they're looked for, so one that comes between isn't
        // missed.
        let watch = watch(&dev)
            .inspect_err(|error| eprintln!("<4>can't hear monitors come and go: {error}"))
            .ok();
        let cables = uevent::socket_for(uevent::KERNEL)
            .inspect_err(|error| eprintln!("<4>can't hear cables come and go: {error}"))
            .ok();
        let mut monitors = Monitors {
            class: PathBuf::from("/sys/class/i2c-dev"),
            dev,
            level,
            buses: HashMap::new(),
            watch,
            cables,
        };
        monitors.look();
        monitors
    }

    /// The panel's level is now this.
    pub fn follow(&mut self, level: f64) {
        if level.to_bits() == self.level.to_bits() {
            return;
        }
        self.level = level;
        for monitor in self
            .buses
            .values()
            .filter_map(|found| found.monitor.as_ref())
        {
            monitor.want(level);
        }
    }

    /// Waits for a bus to come, go or be opened to the seat, or a
    /// connector's cable to come or go, or forever when neither can be heard.
    pub async fn changed(&mut self) -> io::Result<()> {
        loop {
            // Each waits only on its source's readiness, so the one that
            // loses the race has read nothing, and loses nothing.
            let moved = tokio::select! {
                moved = uevent::heard(self.watch.as_ref(), names_a_bus) => moved?,
                moved = uevent::heard(self.cables.as_ref(), says_a_cable_moved) => {
                    let moved = moved?;
                    if moved {
                        eprintln!("<7>a connector's cable came or went, so monitors are looked for");
                    }
                    moved
                }
            };
            if moved {
                return Ok(());
            }
        }
    }

    /// Looks for the monitors again: a new one's thread starts, at the
    /// panel's level, and a gone one's ends.
    pub fn look(&mut self) {
        let found = match buses(&self.class, &self.dev) {
            Ok(found) => found,
            Err(error) => {
                eprintln!("<4>can't look for monitors: {error}");
                return;
            }
        };
        self.buses
            .retain(|name, _| found.iter().any(|(seen, _)| seen == name));
        for (name, reach) in found {
            let bus = self.buses.entry(name).or_insert_with_key(|_| Found {
                reach,
                identity: None,
                monitor: None,
                tried: false,
                left_to: None,
            });
            bus.reach = reach;
        }
        // Read until it answers, since a bus not yet open to the seat can't
        // say which monitor it reaches.
        for (name, bus) in &mut self.buses {
            if bus.identity.is_none() {
                bus.identity = reached(&self.dev.join(name)).ok();
            }
        }
        let seen: Vec<Seen> = self
            .buses
            .iter()
            .map(|(name, bus)| Seen {
                name,
                reach: bus.reach,
                identity: bus.identity,
            })
            .collect();
        let doubled: HashMap<String, String> = doubled(&seen)
            .into_iter()
            .map(|(name, hub)| (name.to_owned(), hub.to_owned()))
            .collect();
        let mut names: Vec<String> = self.buses.keys().cloned().collect();
        names.sort();
        for name in names {
            let Some(bus) = self.buses.get_mut(&name) else {
                continue;
            };
            if let Some(hub) = doubled.get(&name) {
                if bus.left_to.as_ref() != Some(hub) {
                    eprintln!("<6>{name} reaches the monitor {hub} does, so it's left to {hub}");
                    bus.left_to = Some(hub.clone());
                }
                bus.monitor = None;
                continue;
            }
            bus.left_to = None;
            if let Some(monitor) = &mut bus.monitor {
                monitor.identify(bus.identity);
                continue;
            }
            let identity = bus.identity;
            let monitor = Bus::open(&self.dev.join(&name))
                .and_then(|opened| Monitor::start(&name, opened, identity, self.level));
            match monitor {
                Ok(monitor) => bus.monitor = Some(monitor),
                // udev opens a bus to the seat a moment after it comes, and
                // the watch hears it then.
                Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                    if !bus.tried {
                        eprintln!("<5>{name} isn't open to the seat yet");
                    }
                    bus.tried = true;
                }
                Err(error) => {
                    if !bus.tried {
                        eprintln!("<4>can't open {name}: {error}");
                    }
                    bus.tried = true;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

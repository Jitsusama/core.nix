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
//! Monitors come and go as their buses do, which a watch on /dev hears, so
//! nothing is looked for until one does.
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

use std::collections::{HashMap, VecDeque};
use std::env;
use std::ffi::{CString, c_char, c_int, c_ulong};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::time::Duration;

use tokio::io::Interest;
use tokio::io::unix::AsyncFd;

use crate::facts::{self, Told};

// std links libc, so these come from it.
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

/// The ioctl that says which address on the bus reads and writes go to.
const I2C_SLAVE: c_ulong = 0x0703;
/// Where a monitor answers DDC/CI.
const DDC_CI: u8 = 0x37;
/// Where a monitor gives its EDID.
const EDID: u8 = 0x50;
/// The VCP code that holds a monitor's brightness.
const BRIGHTNESS: u8 = 0x10;
/// The VCP code, in the maker's own range, where HP's Z27k G3 keeps its
/// DisplayPort hot-plug detection: its low byte is 0 for Low power and 1 for
/// Always on. Read from one before and after the setting was changed on its
/// own menu; no other code changed.
const HOT_PLUG: u8 = 0xE7;
const ALWAYS_ON: u16 = 0x0001;
/// HP's maker code and the Z27k G3's product code, as its EDID gives them
/// and an `Identity` begins; its EDID's name reads "HP Z27k G3".
const Z27K_G3: [u8; 4] = [0x22, 0x0E, 0xD3, 0x36];
/// How long a monitor takes to have an answer ready.
const ANSWERING: Duration = Duration::from_millis(40);
/// How long a monitor wants between one command and the next.
const BETWEEN: Duration = Duration::from_millis(50);
/// How long a monitor takes to have a piece of its capabilities ready.
const DESCRIBING: Duration = Duration::from_millis(50);
/// The capabilities request, and the longest string read for it.
const CAPABILITIES: u8 = 0xF3;
const LONGEST_CAPABILITIES: u16 = 4096;
/// The VCP codes read for a monitor's facts, if it lists them: its contrast,
/// colour preset, panel, display controller and firmware level.
const CONTRAST: u8 = 0x12;
const PRESET: u8 = 0x14;
const PANEL: u8 = 0xB6;
const CONTROLLER: u8 = 0xC8;
const FIRMWARE: u8 = 0xC9;

const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
/// The bus's permissions changed, as when udev opens it to the seat.
const IN_ATTRIB: u32 = 0x4;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
/// Events were lost, so anything could have changed.
const IN_Q_OVERFLOW: u32 = 0x4000;

/// A message from the host: who it's from, how long it is, what it says, and
/// a checksum that counts the monitor's address too.
fn message(payload: &[u8]) -> Vec<u8> {
    let length = u8::try_from(payload.len()).expect("a DDC/CI message says at most 32 bytes");
    let mut message = vec![0x51, 0x80 | length];
    message.extend_from_slice(payload);
    message.push(message.iter().fold(0x6E, |sum, byte| sum ^ byte));
    message
}

/// Asks what a VCP code holds.
fn get_vcp(vcp: u8) -> Vec<u8> {
    message(&[0x01, vcp])
}

/// Sets a VCP code.
fn set_vcp(vcp: u8, value: u16) -> Vec<u8> {
    let [high, low] = value.to_be_bytes();
    message(&[0x03, vcp, high, low])
}

/// Asks for a monitor's capabilities string from an offset.
fn capabilities_request(offset: u16) -> Vec<u8> {
    let [high, low] = offset.to_be_bytes();
    message(&[CAPABILITIES, high, low])
}

/// What a monitor says a VCP code holds.
#[derive(Debug, PartialEq)]
struct Reading {
    max: u16,
    now: u16,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// A monitor's answer to `get_vcp`, whose checksum counts the host's address.
fn reading(reply: &[u8], vcp: u8) -> io::Result<Reading> {
    let [
        0x6E,
        0x88,
        0x02,
        result,
        code,
        _kind,
        max_high,
        max_low,
        now_high,
        now_low,
        sum,
    ] = *reply
    else {
        return Err(refused(format!("it answered {reply:02x?}")));
    };
    if reply[..10].iter().fold(0x50, |sum, byte| sum ^ byte) != sum {
        return Err(refused(format!(
            "its answer's checksum is wrong: {reply:02x?}"
        )));
    }
    if result != 0 {
        return Err(refused(format!("it has no VCP code {vcp:#04x}")));
    }
    if code != vcp {
        return Err(refused(format!(
            "it answered for VCP code {code:#04x}, not {vcp:#04x}"
        )));
    }
    let max = u16::from_be_bytes([max_high, max_low]);
    if max == 0 && vcp == BRIGHTNESS {
        return Err(refused("its brightest is 0".into()));
    }
    Ok(Reading {
        max,
        now: u16::from_be_bytes([now_high, now_low]),
    })
}

/// The value nearest a share of a monitor's brightest.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "a share of the brightest is between 0 and max"
)]
fn value_for(level: f64, max: u16) -> u16 {
    (level.clamp(0.0, 1.0) * f64::from(max)).round() as u16
}

/// Which monitor an EDID is: its maker, model, serial and when it was made.
type Identity = [u8; 10];

/// The monitor an EDID's first block names, if it is one.
fn identity(edid: &[u8]) -> Option<Identity> {
    let block = edid.get(..128)?;
    let whole = block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) == 0;
    if block[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] || !whole {
        return None;
    }
    block[8..18].try_into().ok()
}

/// Whether a monitor is one whose hot-plug detection is held at Always on.
fn holds_hot_plug(identity: &Identity) -> bool {
    identity[..4] == Z27K_G3
}

/// What to set a Z27k G3's hot-plug detection to, given what it holds: Always
/// on, keeping whatever else it says, but only if it's at Low power.
fn hot_plug_to_set(now: u16) -> Option<u16> {
    let [_, low] = now.to_be_bytes();
    (low == 0).then_some(now | ALWAYS_ON)
}

/// Reads a Z27k G3's hot-plug detection, and sets it to Always on if it
/// isn't, giving what it holds after, if that's known.
fn hold_hot_plug(name: &str, bus: &mut Bus) -> Option<u16> {
    let now = match bus.read_vcp(HOT_PLUG) {
        Ok(Reading { now, .. }) => now,
        Err(error) => {
            eprintln!("<4>can't read the hot-plug detection of the monitor on {name}: {error}");
            return None;
        }
    };
    let Some(value) = hot_plug_to_set(now) else {
        eprintln!("<6>the monitor on {name} keeps hot-plug detection always on ({now:#06x})");
        return Some(now);
    };
    if let Err(error) = bus.write_vcp(HOT_PLUG, value) {
        eprintln!(
            "<4>the monitor on {name} has hot-plug detection at low power ({now:#06x}), \
             and it can't be set: {error}"
        );
        return Some(now);
    }
    match bus.read_vcp(HOT_PLUG) {
        Ok(Reading { now: after, .. }) => {
            eprintln!(
                "<5>the monitor on {name} had hot-plug detection at low power ({now:#06x}), \
                 and now has {after:#06x}"
            );
            Some(after)
        }
        Err(error) => {
            eprintln!(
                "<4>the monitor on {name} had hot-plug detection at low power ({now:#06x}), \
                 and was set to {value:#06x}, but can't be read back: {error}"
            );
            None
        }
    }
}

/// Hot-plug detection, as the monitor's menu names it.
fn hot_plug_named(value: u16) -> &'static str {
    let [_, low] = value.to_be_bytes();
    if low == 0 { "low power" } else { "always on" }
}

/// An I2C bus, its reads and writes going to one address.
fn open_at(path: &Path, address: u8) -> io::Result<File> {
    let file = OpenOptions::new().read(true).write(true).open(path)?;
    // SAFETY: the descriptor is the file's, open for the whole call, and
    // I2C_SLAVE takes the address as a plain number.
    if unsafe { ioctl(file.as_raw_fd(), I2C_SLAVE, c_ulong::from(address)) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

/// The monitor a bus reaches, by its EDID. A block is read in a few
/// milliseconds, and only as a bus comes.
fn reached(path: &Path) -> io::Result<Identity> {
    let mut file = open_at(path, EDID)?;
    file.write_all(&[0])?;
    let mut edid = [0; 128];
    file.read_exact(&mut edid)?;
    identity(&edid).ok_or_else(|| refused("its EDID isn't one".into()))
}

/// A monitor's bus, opened to talk to it.
struct Bus {
    file: File,
}

impl Bus {
    fn open(path: &Path) -> io::Result<Bus> {
        Ok(Bus {
            file: open_at(path, DDC_CI)?,
        })
    }

    fn send(&mut self, message: &[u8]) -> io::Result<()> {
        let sent = self.file.write(message)?;
        if sent != message.len() {
            return Err(io::Error::new(
                io::ErrorKind::WriteZero,
                format!("only {sent} of {} bytes went", message.len()),
            ));
        }
        Ok(())
    }

    fn read_vcp(&mut self, vcp: u8) -> io::Result<Reading> {
        self.send(&get_vcp(vcp))?;
        thread::sleep(ANSWERING);
        let mut reply = [0; 11];
        let read = self.file.read(&mut reply);
        thread::sleep(BETWEEN);
        reading(&reply[..read?], vcp)
    }

    fn write_vcp(&mut self, vcp: u8, value: u16) -> io::Result<()> {
        let sent = self.send(&set_vcp(vcp, value));
        thread::sleep(BETWEEN);
        sent
    }

    /// One piece of the capabilities string, from an offset: none once it's
    /// ended.
    fn read_capabilities(&mut self, offset: u16) -> io::Result<Vec<u8>> {
        self.send(&capabilities_request(offset))?;
        thread::sleep(DESCRIBING);
        let mut reply = [0; 38];
        let read = self.file.read(&mut reply);
        thread::sleep(BETWEEN);
        facts::fragment(&reply[..read?], offset)
            .map(<[u8]>::to_vec)
            .map_err(refused)
    }
}

/// What a monitor's thread is asked for: the latest level, and whether its
/// bus has gone.
#[derive(Default)]
struct Wanted {
    level: Option<f64>,
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

/// A monitor's thread, which ends once this is dropped.
struct Monitor {
    shared: Arc<Shared>,
}

impl Monitor {
    fn start(name: &str, bus: Bus, identity: Option<Identity>, level: f64) -> io::Result<Monitor> {
        let shared = Arc::new(Shared {
            wanted: Mutex::new(Wanted {
                level: Some(level),
                gone: false,
            }),
            woken: Condvar::new(),
        });
        let owned = name.to_owned();
        let theirs = Arc::clone(&shared);
        thread::Builder::new()
            .name(format!("monitor on {name}"))
            .spawn(move || follow(&owned, bus, identity, &theirs))?;
        Ok(Monitor { shared })
    }

    fn want(&self, level: f64) {
        self.shared.wanted().want(level);
        self.shared.woken.notify_one();
    }
}

impl Drop for Monitor {
    fn drop(&mut self) {
        self.shared.wanted().gone = true;
        self.shared.woken.notify_one();
    }
}

/// A monitor's thread: learns its brightest, holds a Z27k G3's hot-plug
/// detection, then sets each level asked for, unless it already shows it.
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
    let runtime = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from);
    let mut told = Told::new(runtime.as_deref(), identity.as_ref().map(|each| &each[..]));
    told.set("brightness", share(now, max));
    if identity.as_ref().is_some_and(holds_hot_plug)
        && let Some(held) = hold_hot_plug(name, &mut bus)
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

/// Whether a DRM connector is a DisplayPort one, not the panel's eDP.
fn is_displayport(connector: &str) -> bool {
    connector
        .strip_prefix("card")
        .and_then(|rest| rest.split_once('-'))
        .is_some_and(|(_, kind)| kind.starts_with("DP-"))
}

/// How a bus reaches a monitor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Reach {
    /// An MST hub made it for the one monitor behind it.
    ThroughAHub,
    /// It's a DisplayPort connector's, and reaches what's on the cable,
    /// through a hub if there is one.
    ByItsConnector,
}

/// How a bus in /sys/class/i2c-dev reaches a monitor, if it does.
///
/// A connector that DRM says has nothing on it reaches none. A dock's hub is
/// one of those too, since its monitors are on connectors of their own, which
/// its hub's buses reach.
fn reach(bus: &Path) -> Option<Reach> {
    let name = fs::read_to_string(bus.join("name")).unwrap_or_default();
    if name.trim() == "DPMST" {
        return Some(Reach::ThroughAHub);
    }
    let device = fs::canonicalize(bus.join("device")).ok()?;
    let connector = device.parent()?;
    let displayport = connector
        .file_name()
        .is_some_and(|connector| is_displayport(&connector.to_string_lossy()));
    let status = fs::read_to_string(connector.join("status")).unwrap_or_default();
    (displayport && status.trim() != "disconnected").then_some(Reach::ByItsConnector)
}

/// A bus that reaches a monitor, and which monitor once its EDID is read.
struct Seen<'a> {
    name: &'a str,
    reach: Reach,
    identity: Option<Identity>,
}

/// The connector buses that reach a monitor a hub's bus reaches too, each
/// with the hub's bus it's left to.
fn doubled<'a>(seen: &[Seen<'a>]) -> HashMap<&'a str, &'a str> {
    let hubs: HashMap<Identity, &str> = seen
        .iter()
        .filter(|bus| bus.reach == Reach::ThroughAHub)
        .filter_map(|bus| Some((bus.identity?, bus.name)))
        .collect();
    seen.iter()
        .filter(|bus| bus.reach == Reach::ByItsConnector)
        .filter_map(|bus| Some((bus.name, *hubs.get(&bus.identity?)?)))
        .collect()
}

/// The buses that reach a monitor, by name, and have a node in `dev`.
fn buses(class: &Path, dev: &Path) -> io::Result<Vec<(String, Reach)>> {
    let entries = match fs::read_dir(class) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error),
    };
    let mut found = Vec::new();
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        // The node goes before the class's entry does, so a bus going is
        // known from the node.
        if !name.starts_with("i2c-") || !dev.join(&name).exists() {
            continue;
        }
        if let Some(reach) = reach(&entry.path()) {
            found.push((name, reach));
        }
    }
    found.sort();
    Ok(found)
}

/// Whether inotify's events name a bus, or lost some that might have.
fn names_a_bus(events: &[u8]) -> bool {
    // Each is a descriptor, a mask, a cookie, a length, then a name that
    // long, padded with NULs.
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let mask = word(4);
        let length = word(12) as usize;
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        if mask & IN_Q_OVERFLOW != 0 || name.starts_with(b"i2c-") {
            return true;
        }
        rest = after;
    }
    false
}

/// A watch on `dev` that wakes when a node is made, removed, or opened to
/// someone.
fn watch(dev: &Path) -> io::Result<AsyncFd<File>> {
    // SAFETY: it takes only flags.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just made, and nothing else holds it.
    let file = File::from(unsafe { OwnedFd::from_raw_fd(fd) });
    let path = CString::new(dev.as_os_str().as_bytes())?;
    // SAFETY: the descriptor is the file's, and the path a C string that
    // outlives the call.
    let mask = IN_CREATE | IN_DELETE | IN_ATTRIB;
    if unsafe { inotify_add_watch(file.as_raw_fd(), path.as_ptr(), mask) } < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the File is moved into the AsyncFd, which owns it until it's
    // dropped, and a File keeps one descriptor open for its life.
    unsafe { AsyncFd::register_with_interest(file, Interest::READABLE) }.map_err(io::Error::from)
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
        let mut monitors = Monitors {
            class: PathBuf::from("/sys/class/i2c-dev"),
            dev,
            level,
            buses: HashMap::new(),
            watch,
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

    /// Waits for a bus to come, go or be opened to the seat, or forever when
    /// that can't be heard.
    pub async fn changed(&mut self) -> io::Result<()> {
        let Some(watch) = &self.watch else {
            return std::future::pending().await;
        };
        let mut events = [0; 4096];
        loop {
            let mut ready = watch.readable().await?;
            let mut heard = false;
            loop {
                let mut file = ready.get_inner();
                match file.read(&mut events) {
                    Ok(read) => heard |= names_a_bus(&events[..read]),
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        ready.clear_ready();
                        break;
                    }
                    Err(error) => return Err(error),
                }
            }
            if heard {
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
            if bus.monitor.is_some() {
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
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn asking_for_a_vcp_code_is_framed_and_summed() {
        assert_eq!(get_vcp(0x10), [0x51, 0x82, 0x01, 0x10, 0xAC]);
    }

    #[test]
    fn setting_a_vcp_code_is_framed_and_summed() {
        let sent = set_vcp(0x10, 0x0140);
        assert_eq!(sent[..6], [0x51, 0x84, 0x03, 0x10, 0x01, 0x40]);
        let sum = [0x6E, 0x51, 0x84, 0x03, 0x10, 0x01, 0x40]
            .iter()
            .fold(0, |sum, byte| sum ^ byte);
        assert_eq!(sent[6], sum);
        assert_eq!(sent.len(), 7);
    }

    /// A monitor's answer, summed as one sends it.
    fn answer(result: u8, vcp: u8, max: u16, now: u16) -> Vec<u8> {
        let [max_high, max_low] = max.to_be_bytes();
        let [now_high, now_low] = now.to_be_bytes();
        let mut reply = vec![
            0x6E, 0x88, 0x02, result, vcp, 0x00, max_high, max_low, now_high, now_low,
        ];
        reply.push(reply.iter().fold(0x50, |sum, byte| sum ^ byte));
        reply
    }

    #[test]
    fn a_good_answer_gives_the_brightest_and_the_brightness_now() {
        assert_eq!(
            reading(&answer(0, 0x10, 100, 40), 0x10).unwrap(),
            Reading { max: 100, now: 40 }
        );
    }

    #[test]
    fn an_answer_with_a_wrong_checksum_is_refused() {
        let mut reply = answer(0, 0x10, 100, 40);
        reply[10] ^= 1;
        assert!(reading(&reply, 0x10).is_err());
    }

    #[test]
    fn an_answer_for_another_vcp_code_is_refused() {
        assert!(reading(&answer(0, 0x12, 100, 40), 0x10).is_err());
    }

    #[test]
    fn an_answer_that_says_no_such_code_is_refused() {
        assert!(reading(&answer(1, 0x10, 100, 40), 0x10).is_err());
    }

    #[test]
    fn a_short_answer_is_refused() {
        assert!(reading(&[0x6E, 0x80, 0xBE], 0x10).is_err());
    }

    #[test]
    fn the_panel_s_share_becomes_the_monitor_s_nearest_value() {
        assert_eq!(value_for(0.0, 100), 0);
        assert_eq!(value_for(1.0, 100), 100);
        assert_eq!(value_for(1.0 / 3.0, 100), 33);
        assert_eq!(value_for(2.0 / 3.0, 100), 67);
        assert_eq!(value_for(0.5, 101), 51);
        assert_eq!(value_for(2.0, 100), 100);
    }

    #[test]
    fn a_z27k_g3_has_its_hot_plug_detection_held_and_other_monitors_do_not() {
        let z27k_g3 = [0x22, 0x0E, 0xD3, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
        let another_hp = [0x22, 0x0E, 0xD4, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
        let another_maker = [0x10, 0xAC, 0xD3, 0x36, 0, 0, 0, 0, 0x1C, 0x1F];
        assert!(holds_hot_plug(&z27k_g3));
        assert!(!holds_hot_plug(&another_hp));
        assert!(!holds_hot_plug(&another_maker));
    }

    #[test]
    fn hot_plug_detection_at_low_power_is_set_to_always_on() {
        assert_eq!(hot_plug_to_set(0x0000), Some(0x0001));
        assert_eq!(hot_plug_to_set(0x0100), Some(0x0101));
    }

    #[test]
    fn hot_plug_detection_always_on_is_left_alone() {
        assert_eq!(hot_plug_to_set(0x0001), None);
        assert_eq!(hot_plug_to_set(0x0101), None);
    }

    #[test]
    fn what_a_z27k_g3_says_for_its_hot_plug_detection_is_read() {
        assert_eq!(
            reading(&answer(0, HOT_PLUG, 1, 0), HOT_PLUG).unwrap(),
            Reading { max: 1, now: 0 }
        );
    }

    #[test]
    fn setting_hot_plug_detection_to_always_on_is_framed_as_the_menu_sets_it() {
        assert_eq!(
            set_vcp(HOT_PLUG, ALWAYS_ON)[..6],
            [0x51, 0x84, 0x03, 0xE7, 0x00, 0x01]
        );
    }

    #[test]
    fn only_the_latest_level_asked_for_is_taken() {
        let mut wanted = Wanted::default();
        assert_eq!(wanted.take(), None);
        for level in [0.1, 0.2, 0.3] {
            wanted.want(level);
        }
        assert_eq!(wanted.take(), Some(0.3));
        assert_eq!(wanted.take(), None);
    }

    #[test]
    fn a_gone_monitor_s_thread_is_asked_for_nothing_more() {
        let shared = Shared {
            wanted: Mutex::new(Wanted {
                level: Some(0.5),
                gone: true,
            }),
            woken: Condvar::new(),
        };
        assert_eq!(shared.next(true), Next::Gone);
    }

    #[test]
    fn a_level_waiting_comes_before_other_work_and_other_work_before_waiting() {
        let shared = Shared {
            wanted: Mutex::new(Wanted {
                level: Some(0.5),
                gone: false,
            }),
            woken: Condvar::new(),
        };
        assert_eq!(shared.next(true), Next::Level(0.5));
        assert_eq!(shared.next(true), Next::Idle);
    }

    #[test]
    fn asking_for_capabilities_is_framed_and_summed() {
        let sent = capabilities_request(0x0020);
        assert_eq!(sent[..5], [0x51, 0x83, 0xF3, 0x00, 0x20]);
        let sum = [0x6E, 0x51, 0x83, 0xF3, 0x00, 0x20]
            .iter()
            .fold(0, |sum, byte| sum ^ byte);
        assert_eq!(sent[5], sum);
    }

    #[test]
    fn a_firmware_level_with_no_brightest_is_still_read() {
        assert_eq!(
            reading(&answer(0, FIRMWARE, 0, 0x0100), FIRMWARE).unwrap(),
            Reading {
                max: 0,
                now: 0x0100
            }
        );
        assert!(reading(&answer(0, BRIGHTNESS, 0, 0), BRIGHTNESS).is_err());
    }

    #[test]
    fn a_monitor_s_brightness_and_hot_plug_detection_are_named() {
        assert_eq!(share(40, 100), "40%");
        assert_eq!(share(1, 3), "33%");
        assert_eq!(share(0, 0), "0%");
        assert_eq!(hot_plug_named(0x0001), "always on");
        assert_eq!(hot_plug_named(0x0100), "low power");
    }

    /// A stand-in sysfs and /dev, gone when the test is.
    struct Tree(PathBuf);

    impl Tree {
        fn new(test: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("backlight-{}-{test}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            for dir in ["class", "dev", "devices"] {
                fs::create_dir_all(root.join(dir)).unwrap();
            }
            Tree(root)
        }

        /// A bus of this name, whose device sits at `device` under devices.
        fn add(&self, bus: &str, name: &str, device: &str) {
            let device = self.0.join("devices").join(device);
            fs::create_dir_all(&device).unwrap();
            let dir = self.0.join("class").join(bus);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("name"), format!("{name}\n")).unwrap();
            symlink(&device, dir.join("device")).unwrap();
            fs::write(self.0.join("dev").join(bus), "").unwrap();
        }

        /// What a connector under devices says is on it, as DRM's status.
        fn status(&self, connector: &str, status: &str) {
            let dir = self.0.join("devices").join(connector);
            fs::write(dir.join("status"), format!("{status}\n")).unwrap();
        }

        fn buses(&self) -> Vec<(String, Reach)> {
            buses(&self.0.join("class"), &self.0.join("dev")).unwrap()
        }

        fn names(&self) -> Vec<String> {
            self.buses().into_iter().map(|(name, _)| name).collect()
        }
    }

    impl Drop for Tree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_bus_behind_an_mst_hub_reaches_a_monitor() {
        let tree = Tree::new("mst");
        tree.add("i2c-13", "DPMST", "i2c-13");
        assert_eq!(tree.buses(), [("i2c-13".into(), Reach::ThroughAHub)]);
    }

    #[test]
    fn a_displayport_connector_s_bus_reaches_a_monitor_and_the_panel_s_does_not() {
        let tree = Tree::new("connectors");
        tree.add(
            "i2c-7",
            "AUX USBC1/DDI TC1/PHY TC1",
            "card1/card1-DP-2/i2c-7",
        );
        tree.add("i2c-3", "AUX A/DDI A/PHY A", "card1/card1-eDP-1/i2c-3");
        tree.add("i2c-5", "i915 gmbus dpb", "card1/card1-HDMI-A-1/i2c-5");
        assert_eq!(tree.buses(), [("i2c-7".into(), Reach::ByItsConnector)]);
    }

    #[test]
    fn a_connector_with_nothing_on_it_is_passed_over() {
        let tree = Tree::new("disconnected");
        tree.add(
            "i2c-10",
            "AUX USBC1/DDI TC1/PHY TC1",
            "card0/card0-DP-1/i2c-10",
        );
        tree.add(
            "i2c-11",
            "AUX USBC2/DDI TC2/PHY TC2",
            "card0/card0-DP-2/i2c-11",
        );
        tree.status("card0/card0-DP-1", "disconnected");
        tree.status("card0/card0-DP-2", "connected");
        assert_eq!(tree.buses(), [("i2c-11".into(), Reach::ByItsConnector)]);
    }

    #[test]
    fn the_machine_s_other_buses_are_left_alone() {
        let tree = Tree::new("others");
        tree.add(
            "i2c-0",
            "SMBus I801 adapter at efa0",
            "pci0000:00/0000:00:1f.4/i2c-0",
        );
        tree.add(
            "i2c-1",
            "Synopsys DesignWare I2C adapter",
            "pci0000:00/i2c-1",
        );
        assert!(tree.buses().is_empty());
    }

    #[test]
    fn a_bus_whose_node_has_gone_is_gone() {
        let tree = Tree::new("gone");
        tree.add("i2c-13", "DPMST", "i2c-13");
        tree.add("i2c-14", "DPMST", "i2c-14");
        fs::remove_file(tree.0.join("dev/i2c-14")).unwrap();
        assert_eq!(tree.names(), ["i2c-13"]);
    }

    /// An EDID's first block for a monitor, summed as a monitor sums it.
    fn edid(serial: u8) -> Vec<u8> {
        let mut block = vec![0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00];
        // HP's maker code, a model, the serial, the week and year made.
        block.extend_from_slice(&[0x22, 0xF0, 0x4E, 0x37, serial, 0, 0, 0, 12, 34]);
        block.resize(127, 0);
        block.push(0u8.wrapping_sub(block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte))));
        block
    }

    #[test]
    fn an_edid_names_its_monitor_by_maker_model_serial_and_making() {
        assert_eq!(
            identity(&edid(7)),
            Some([0x22, 0xF0, 0x4E, 0x37, 7, 0, 0, 0, 12, 34])
        );
    }

    #[test]
    fn what_isn_t_a_whole_edid_names_no_monitor() {
        let mut summed_wrong = edid(7);
        summed_wrong[127] ^= 1;
        let mut no_header = edid(7);
        no_header[0] = 0xFF;
        assert_eq!(identity(&summed_wrong), None);
        assert_eq!(identity(&no_header), None);
        assert_eq!(identity(&edid(7)[..100]), None);
        assert_eq!(identity(&[0; 128]), None);
    }

    fn seen(name: &str, reach: Reach, serial: Option<u8>) -> Seen<'_> {
        Seen {
            name,
            reach,
            identity: serial.and_then(|serial| identity(&edid(serial))),
        }
    }

    #[test]
    fn a_connector_reaching_a_hub_s_monitor_is_left_to_the_hub_s_bus() {
        let found = [
            seen("i2c-10", Reach::ByItsConnector, Some(1)),
            seen("i2c-13", Reach::ThroughAHub, Some(1)),
        ];
        assert_eq!(doubled(&found), HashMap::from([("i2c-10", "i2c-13")]));
    }

    #[test]
    fn monitors_behind_a_hub_are_each_followed_though_their_edids_match() {
        let found = [
            seen("i2c-10", Reach::ByItsConnector, Some(1)),
            seen("i2c-13", Reach::ThroughAHub, Some(1)),
            seen("i2c-14", Reach::ThroughAHub, Some(1)),
        ];
        assert_eq!(doubled(&found).len(), 1);
        assert!(doubled(&found[1..]).is_empty());
    }

    #[test]
    fn a_connector_is_followed_unless_a_hub_s_bus_is_known_to_reach_its_monitor() {
        assert!(
            doubled(&[
                seen("i2c-7", Reach::ByItsConnector, Some(1)),
                seen("i2c-8", Reach::ByItsConnector, Some(1)),
                seen("i2c-13", Reach::ThroughAHub, Some(2)),
                seen("i2c-10", Reach::ByItsConnector, None),
                seen("i2c-14", Reach::ThroughAHub, None),
            ])
            .is_empty()
        );
    }

    #[test]
    fn no_class_of_buses_is_none_found() {
        let tree = Tree::new("no-class");
        assert!(
            buses(&tree.0.join("missing"), &tree.0.join("dev"))
                .unwrap()
                .is_empty()
        );
    }

    /// An inotify event for a name, padded as the kernel pads it.
    fn event(mask: u32, name: &str) -> Vec<u8> {
        let mut padded = name.as_bytes().to_vec();
        if !name.is_empty() {
            padded.resize((name.len() / 16 + 1) * 16, 0);
        }
        let mut event = Vec::new();
        event.extend_from_slice(&1i32.to_ne_bytes());
        event.extend_from_slice(&mask.to_ne_bytes());
        event.extend_from_slice(&0u32.to_ne_bytes());
        event.extend_from_slice(&u32::try_from(padded.len()).unwrap().to_ne_bytes());
        event.extend_from_slice(&padded);
        event
    }

    #[test]
    fn a_bus_coming_is_heard_among_other_nodes() {
        let mut events = event(IN_CREATE, "tty7");
        events.extend(event(IN_CREATE, "i2c-13"));
        assert!(names_a_bus(&events));
    }

    #[test]
    fn other_nodes_coming_and_going_are_not_a_bus() {
        let mut events = event(IN_CREATE, "hidraw3");
        events.extend(event(IN_DELETE, "input/event9"));
        assert!(!names_a_bus(&events));
    }

    #[test]
    fn lost_events_might_have_named_a_bus() {
        assert!(names_a_bus(&event(IN_Q_OVERFLOW, "")));
    }
}

//! lid-hinge: gives logind and the compositor a lid that shuts only when the hinge agrees.
//!
//! A laptop's lid switch is a magnet sensor its embedded controller reads, and pressure on
//! the chassis can make it say shut with the lid wide open. Shut on battery, logind then puts
//! the laptop to sleep under the hands typing on it, and the compositor turns off its only
//! screen. udev hides that switch from both (nixos.nix), and this offers them a switch of its
//! own, through uinput, that follows it with one exception.
//!
//! A laptop with a hinge sensor reports the lid's angle as it moves, but its sensor hub reads
//! nothing once the switch says shut, true or not, so the angle can't be asked after the
//! switch. What tells a real close from a false one is the angle read before it: a lid coming
//! down passes through the last few degrees, and one wide open hasn't moved. So a shut switch
//! is believed at once when the hinge read under `NEAR_SHUT` degrees within `MOVING` of a
//! reading over it. A reading under it alone says nothing: the hub sends a stale one as the
//! lid opens, and on the laptop this was written for one came in the same second as a false
//! close, with the lid open and typed on.
//!
//! A lid slammed from wide open is faster than the hub reads, so that alone would doubt it
//! too. A doubted close is believed once a light sensor in the lid that saw light at the switch
//! reads dark, as a shut lid covers it, or where none saw light, in a dark room or a laptop
//! without one. The hub reads no light either once the switch says shut, so a dark reading
//! counts only once the laptop's own keys and touchpad have gone untouched for `HANDS_OFF`:
//! nobody types on a shut lid. A doubt lasts as long as the switch says shut, up to
//! `DOUBTED_AT_MOST` after they were last used. A believed close is doubted again once they
//! are used, from `SETTLING` after the switch; whatever else is plugged in can be used with
//! the lid shut. An opened switch is always believed.
//!
//! The hinge is heard from its IIO buffer, which udev turns on, so it wakes this only when the
//! angle changes. The light, and the interrupts of the laptop's keyboard and its i2c-hid
//! devices, are read only while the lid is shut or a close is doubted, once a second.
//! `LID_HINGE_IIO` and `LID_HINGE_NODES` name where IIO's devices and their nodes are, for a
//! test's stand-ins.

use std::env;
use std::f64::consts::PI;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_short, c_ulong};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// The angle under which a hinge's last reading says the lid was coming down. A lid left
/// nearly shut on a desk read about 30°, and one in use reads 90° to 115°.
const NEAR_SHUT: f64 = 45.0;

/// How soon before a reading under `NEAR_SHUT` one over it must have come, for the lid to be
/// coming down rather than the hub sending a stale reading.
const MOVING: Duration = Duration::from_secs(10);

/// How long the laptop's own keys and touchpad must go untouched before a dark light sensor
/// is believed.
const HANDS_OFF: Duration = Duration::from_secs(30);

/// How long after a switch shuts the keys and touchpad say nothing, while a palm lifts off.
const SETTLING: Duration = Duration::from_secs(2);

/// Readings under this are the hub's zero for a shut switch, not an angle.
const NO_ANGLE: f64 = 1.0;

/// What a light sensor must fall to, of what it read at the switch, to read dark.
const DARK_SHARE: f64 = 0.05;

/// How often the light and the hands are read while the lid is shut or a close doubted.
const LOOK_AGAIN: Duration = Duration::from_secs(1);

/// How often the hinge is looked for while there's none: the sensor hub's devices come a
/// moment after this starts at boot, and again after it resets, and the angles before a shut
/// switch are only heard once the hinge is.
const LOOK_FOR_HINGE: Duration = Duration::from_secs(2);

/// How long a close is doubted, after the laptop's own keys and touchpad were last used, before
/// it's believed anyway. A false close seen on the laptop this was written for lasted two
/// minutes, and a lid shut in a lit room should sleep in a bag before long.
const DOUBTED_AT_MOST: Duration = Duration::from_mins(10);

/// The name of the switch this offers, which udev's rule leaves showing.
const NAME: &str = "Lid Switch (Hinge)";

// Linux's input and uinput numbers, from linux/input.h, input-event-codes.h and uinput.h.
const EV_SYN: u16 = 0x00;
const EV_SW: u16 = 0x05;
const SW_LID: u16 = 0x00;
const SYN_REPORT: u16 = 0;
const SYN_DROPPED: u16 = 3;
const BUS_VIRTUAL: u16 = 0x06;
const EVIOCGSW: c_ulong = 0x8008_451b;
const EVIOCGBIT_SW: c_ulong = 0x8008_4525;
const EVIOCGNAME: c_ulong = 0x8100_4506;
const UI_SET_EVBIT: c_ulong = 0x4004_5564;
const UI_SET_SWBIT: c_ulong = 0x4004_556d;
const UI_DEV_SETUP: c_ulong = 0x405c_5503;
const UI_DEV_CREATE: c_ulong = 0x5501;
/// The size of an `input_event` on a 64-bit machine: a timeval, a type, a code, a value.
const EVENT_SIZE: usize = 24;

/// Linux's `O_NONBLOCK`, the same on every architecture core builds for.
const O_NONBLOCK: i32 = 0o4000;
const POLLIN: c_short = 0x001;
const POLLERR: c_short = 0x008;
const POLLHUP: c_short = 0x010;

/// One file `poll` waits on, as the kernel's `pollfd` lays it out.
#[repr(C)]
struct PollFd {
    fd: c_int,
    events: c_short,
    revents: c_short,
}

/// A device's name and number for uinput, as `uinput_setup` lays it out.
#[repr(C)]
struct Setup {
    bustype: u16,
    vendor: u16,
    product: u16,
    version: u16,
    name: [u8; 80],
    ff_effects_max: u32,
}

// std links libc, so these come from it.
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn poll(fds: *mut PollFd, count: c_ulong, timeout: c_int) -> c_int;
}

/// What the switch offered should say, and why, for the journal.
#[derive(Debug, PartialEq)]
enum Verdict {
    /// Shut, believed: the hinge was coming down, or the light went dark.
    Shut(&'static str),
    /// Open, as the lid's own switch now says.
    Open,
    /// The lid's switch says shut, and this doesn't believe it yet.
    Doubted,
    /// The switch offered says what it already did.
    Unchanged,
    /// Open, though the lid's switch says shut: the laptop's own keys or touchpad were used.
    Used,
}

/// A close not believed yet: when it came, and what each light sensor read at it, or none
/// where the light can't tell, as when the keys opened a close already believed.
struct Doubt {
    since: Instant,
    light: Option<Vec<Option<f64>>>,
}

/// Whether the lid is shut, from its switch, the hinge's angle, the light and the hands on the
/// laptop's own keys and touchpad.
struct Judge {
    switch_shut: bool,
    offered_shut: bool,
    /// The hinge's last angle, for the journal.
    hinge: Option<f64>,
    /// When the hinge last read over `NEAR_SHUT`, since the switch last opened.
    open_at: Option<Instant>,
    /// Whether the hinge was seen coming down past `NEAR_SHUT`.
    coming_down: bool,
    /// When the switch last said shut.
    shut_at: Option<Instant>,
    /// The laptop's own keys and touchpad: how many presses so far, and when the last came.
    presses: Option<u64>,
    pressed: Option<Instant>,
    doubt: Option<Doubt>,
}

impl Judge {
    /// As the lid's switch says when this starts: nothing came before it to doubt it by.
    fn new(shut: bool) -> Judge {
        Judge {
            switch_shut: shut,
            offered_shut: shut,
            hinge: None,
            open_at: None,
            coming_down: false,
            shut_at: None,
            presses: None,
            pressed: None,
            doubt: None,
        }
    }

    /// A reading of the hinge's angle. Only those while the switch reads open say how the
    /// lid came down.
    /// Says whether the lid has just come down past `NEAR_SHUT`.
    fn hinge(&mut self, degrees: f64, now: Instant) -> bool {
        if self.switch_shut || degrees < NO_ANGLE {
            return false;
        }
        self.hinge = Some(degrees);
        let was = self.coming_down;
        if degrees >= NEAR_SHUT {
            self.open_at = Some(now);
            self.coming_down = false;
        } else if !was {
            self.coming_down = self
                .open_at
                .is_some_and(|at| now.saturating_duration_since(at) <= MOVING);
        }
        self.coming_down && !was
    }

    /// The lid's own switch, with the light each sensor reads as it changes.
    fn switch(&mut self, shut: bool, light: Vec<Option<f64>>, now: Instant) -> Verdict {
        if shut == self.switch_shut {
            return Verdict::Unchanged;
        }
        self.switch_shut = shut;
        if !shut {
            self.doubt = None;
            self.shut_at = None;
            // Opening, it reads its way up again from the first degrees.
            self.hinge = None;
            self.open_at = None;
            self.coming_down = false;
            return self.offer(false, Verdict::Open);
        }
        self.shut_at = Some(now);
        if self.coming_down {
            return self.offer(true, Verdict::Shut("the hinge was nearly shut"));
        }
        self.doubt = Some(Doubt {
            since: now,
            light: Some(light),
        });
        Verdict::Doubted
    }

    /// The light each sensor reads now, while a close is doubted.
    fn light(&mut self, light: &[Option<f64>], now: Instant) -> Verdict {
        let Some(doubt) = &self.doubt else {
            return Verdict::Unchanged;
        };
        let used = self.pressed.map_or(doubt.since, |at| at.max(doubt.since));
        let untouched = now.saturating_duration_since(used);
        if let Some(then) = doubt.light.as_ref().filter(|_| untouched >= HANDS_OFF) {
            if !then.iter().flatten().any(|&lux| lux > 0.0) {
                self.doubt = None;
                return self.offer(true, Verdict::Shut("no light sensor saw light to tell by"));
            }
            let dark = then.iter().zip(light).any(|(then, now)| match (then, now) {
                (Some(then), Some(now)) => *then > 0.0 && *now <= then * DARK_SHARE,
                _ => false,
            });
            if dark {
                self.doubt = None;
                return self.offer(true, Verdict::Shut("the light went dark"));
            }
        }
        if untouched >= DOUBTED_AT_MOST {
            self.doubt = None;
            return self.offer(true, Verdict::Shut("it stayed shut too long to doubt"));
        }
        Verdict::Unchanged
    }

    fn doubting(&self) -> bool {
        self.doubt.is_some()
    }

    /// Whether the light and the hands need reading: while the lid is offered shut, or doubted.
    fn watching(&self) -> bool {
        self.offered_shut || self.doubting()
    }

    /// How many presses the laptop's own keys and touchpad have had so far. Any since the last
    /// count are a hand on the laptop, so a shut lid it believed is open again, and doubted.
    fn keys(&mut self, count: u64, now: Instant) -> Verdict {
        let pressed = self.presses.is_some_and(|before| before != count);
        self.presses = Some(count);
        if !pressed {
            return Verdict::Unchanged;
        }
        self.pressed = Some(now);
        let settled = self
            .shut_at
            .is_none_or(|at| now.saturating_duration_since(at) >= SETTLING);
        if !(self.switch_shut && self.offered_shut && settled) {
            return Verdict::Unchanged;
        }
        self.doubt = Some(Doubt {
            since: now,
            light: None,
        });
        self.offer(false, Verdict::Used)
    }

    fn offer(&mut self, shut: bool, verdict: Verdict) -> Verdict {
        if shut == self.offered_shut {
            return Verdict::Unchanged;
        }
        self.offered_shut = shut;
        verdict
    }
}

/// Where one channel sits in a buffered sample, and how it's written there, from a scan
/// element's type as the kernel writes it: le:s16/32>>0 is a little-endian signed 16-bit
/// number in 32 bits, not shifted.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Field {
    signed: bool,
    big_endian: bool,
    bits: u32,
    bytes: usize,
    shift: u32,
}

impl Field {
    fn read(text: &str) -> Option<Field> {
        let (endian, rest) = text.trim().split_once(':')?;
        let big_endian = match endian {
            "le" => false,
            "be" => true,
            _ => return None,
        };
        let (bits, rest) = rest.split_once('/')?;
        let (signed, bits) = match bits.split_at_checked(1)? {
            ("s", bits) => (true, bits),
            ("u", bits) => (false, bits),
            _ => return None,
        };
        let (storage, shift) = rest.split_once(">>")?;
        let field = Field {
            signed,
            big_endian,
            bits: bits.parse().ok()?,
            bytes: storage.parse::<usize>().ok()? / 8,
            shift: shift.parse().ok()?,
        };
        let fits = matches!(field.bytes, 1 | 2 | 4 | 8)
            && field.bits > 0
            && u64::from(field.bits + field.shift) <= 8 * field.bytes as u64;
        fits.then_some(field)
    }

    fn value(self, bytes: &[u8]) -> i64 {
        let mut word = [0; 8];
        let raw = if self.big_endian {
            word[8 - self.bytes..].copy_from_slice(bytes);
            u64::from_be_bytes(word)
        } else {
            word[..self.bytes].copy_from_slice(bytes);
            u64::from_le_bytes(word)
        };
        let unused = 64 - self.bits;
        let raw = raw >> self.shift << unused;
        if self.signed {
            i64::from_ne_bytes(raw.to_ne_bytes()) >> unused
        } else {
            i64::try_from(raw >> unused).unwrap_or(i64::MAX)
        }
    }
}

/// Where the hinge's angle sits in each sample of its buffer, and how it turns into degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout {
    size: usize,
    at: usize,
    field: Field,
    degrees_per_step: f64,
    offset: f64,
}

impl Layout {
    /// From the channels turned on, as their index, field and whether it's the hinge's own.
    /// Each sits after the one before at a multiple of its own size, and a sample is a
    /// multiple of the largest.
    fn of(mut channels: Vec<(u32, Field, bool)>, radians: f64, offset: f64) -> Option<Layout> {
        channels.sort_by_key(|&(index, _, _)| index);
        let mut size: usize = 0;
        let mut largest: usize = 1;
        let mut hinge = None;
        for (_, field, is_hinge) in channels {
            size = size.next_multiple_of(field.bytes);
            if is_hinge {
                hinge = Some((size, field));
            }
            size += field.bytes;
            largest = largest.max(field.bytes);
        }
        let (at, field) = hinge?;
        Some(Layout {
            size: size.next_multiple_of(largest),
            at,
            field,
            degrees_per_step: radians * 180.0 / PI,
            offset,
        })
    }

    /// Each whole sample's angle, in degrees, and how many bytes they took.
    fn angles(self, bytes: &[u8]) -> (Vec<f64>, usize) {
        let samples = bytes.len() / self.size;
        let angles = (0..samples)
            .map(|sample| {
                let at = sample * self.size + self.at;
                #[expect(
                    clippy::cast_precision_loss,
                    reason = "a hinge's readings are far below 2⁵³, so the float is exact"
                )]
                let raw = self.field.value(&bytes[at..at + self.field.bytes]) as f64;
                (raw + self.offset) * self.degrees_per_step
            })
            .collect();
        (angles, samples * self.size)
    }
}

fn text(path: &Path) -> io::Result<String> {
    fs::read_to_string(path).map(|text| text.trim().to_owned())
}

fn on(path: &Path) -> bool {
    text(path).is_ok_and(|text| text == "1")
}

/// The IIO devices under `root` that a sensor named `name` is.
fn devices(root: &Path, name: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|dir| text(&dir.join("name")).is_ok_and(|text| text == name))
        .collect();
    found.sort();
    found
}

/// The hinge's buffer, as the channels its device has turned on lay it out.
fn hinge_layout(dir: &Path) -> Option<Layout> {
    if !on(&dir.join("buffer/enable")) || !on(&dir.join("scan_elements/in_angl0_en")) {
        return None;
    }
    let mut channels = Vec::new();
    for entry in fs::read_dir(dir.join("scan_elements")).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(channel) = name.strip_suffix("_en") else {
            continue;
        };
        if !on(&entry.path()) {
            continue;
        }
        let scan = dir.join("scan_elements");
        let field = Field::read(&text(&scan.join(format!("{channel}_type"))).ok()?)?;
        let index = text(&scan.join(format!("{channel}_index")))
            .ok()?
            .parse()
            .ok()?;
        channels.push((index, field, channel == "in_angl0"));
    }
    let number = |name: &str, default: f64| {
        text(&dir.join(name)).map_or(Some(default), |text| text.parse().ok())
    };
    Layout::of(
        channels,
        number("in_angl_scale", 1.0)?,
        number("in_angl_offset", 0.0)?,
    )
}

/// The hinge's buffer, opened, and how its samples read.
struct Hinge {
    node: File,
    layout: Layout,
    held: Vec<u8>,
}

impl Hinge {
    fn find(root: &Path, nodes: &Path) -> Option<Hinge> {
        devices(root, "hinge").into_iter().find_map(|dir| {
            let layout = hinge_layout(&dir)?;
            let node = OpenOptions::new()
                .read(true)
                .custom_flags(O_NONBLOCK)
                .open(nodes.join(dir.file_name()?))
                .ok()?;
            Some(Hinge {
                node,
                layout,
                held: Vec::new(),
            })
        })
    }

    /// The angles that came since the last read, oldest first.
    fn angles(&mut self) -> io::Result<Vec<f64>> {
        let mut chunk = [0; 256];
        loop {
            match self.node.read(&mut chunk) {
                Ok(0) => break,
                Ok(read) => self.held.extend_from_slice(&chunk[..read]),
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => break,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        let (angles, used) = self.layout.angles(&self.held);
        self.held.drain(..used);
        Ok(angles)
    }
}

/// What each light sensor under `root` reads now, in its own raw units.
fn light(root: &Path) -> Vec<Option<f64>> {
    devices(root, "als")
        .iter()
        .map(|dir| text(&dir.join("in_illuminance_raw")).ok()?.parse().ok())
        .collect()
}

/// The presses so far of the devices `names` names, from `/proc/interrupts`, which counts each
/// line's interrupts on every CPU and ends it with the devices on it; none when none is there.
fn presses(interrupts: &str, names: &[String]) -> Option<u64> {
    interrupts
        .lines()
        .skip(1)
        .filter(|line| {
            line.split_whitespace()
                .last()
                .is_some_and(|name| names.iter().any(|wanted| wanted == name))
        })
        .map(|line| {
            line.split_whitespace()
                .skip(1)
                .map_while(|field| field.parse::<u64>().ok())
                .sum::<u64>()
        })
        .reduce(|all, one| all + one)
}

/// The presses so far of the laptop's own keyboard, on i8042, and its i2c-hid devices, the
/// touchpad among them, which each name their interrupt line after themselves.
fn hands() -> Option<u64> {
    let mut names = vec!["i8042".to_owned()];
    if let Ok(entries) = fs::read_dir("/sys/bus/i2c/drivers/i2c_hid_acpi") {
        names.extend(entries.flatten().filter_map(|entry| {
            Some(entry.file_name().to_str()?.strip_prefix("i2c-")?.to_owned())
        }));
    }
    presses(&fs::read_to_string("/proc/interrupts").ok()?, &names)
}

fn checked(result: c_int) -> io::Result<c_int> {
    if result < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(result)
    }
}

/// The lid's own switch: the first input device that has a lid's switch and isn't this one's.
fn find_lid() -> io::Result<(File, String)> {
    let mut nodes: Vec<PathBuf> = fs::read_dir("/dev/input")?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("event"))
        })
        .collect();
    nodes.sort();
    for path in nodes {
        let Ok(file) = OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK)
            .open(&path)
        else {
            continue;
        };
        let mut switches = [0u8; 8];
        if checked(unsafe { ioctl(file.as_raw_fd(), EVIOCGBIT_SW, switches.as_mut_ptr()) }).is_err()
            || switches[0] & (1 << SW_LID) == 0
        {
            continue;
        }
        let mut name = [0u8; 256];
        let length =
            checked(unsafe { ioctl(file.as_raw_fd(), EVIOCGNAME, name.as_mut_ptr()) }).unwrap_or(0);
        let length = usize::try_from(length).unwrap_or(0).min(name.len());
        let name = String::from_utf8_lossy(&name[..length])
            .trim_end_matches('\0')
            .to_owned();
        if name != NAME {
            return Ok((file, name));
        }
    }
    Err(io::Error::new(io::ErrorKind::NotFound, "no lid switch"))
}

/// Whether the lid's own switch reads shut now.
fn lid_shut(lid: &File) -> io::Result<bool> {
    let mut switches = [0u8; 8];
    checked(unsafe { ioctl(lid.as_raw_fd(), EVIOCGSW, switches.as_mut_ptr()) })?;
    Ok(switches[0] & (1 << SW_LID) != 0)
}

/// Reads what the lid's switch sent, saying whether it touched the lid or lost events.
fn lid_events(lid: &mut File) -> io::Result<bool> {
    let mut events = [0u8; EVENT_SIZE * 16];
    let mut touched = false;
    loop {
        match lid.read(&mut events) {
            Ok(0) => return Ok(touched),
            Ok(read) => {
                for event in events[..read].as_chunks::<EVENT_SIZE>().0 {
                    let kind = u16::from_ne_bytes([event[16], event[17]]);
                    let code = u16::from_ne_bytes([event[18], event[19]]);
                    touched |= (kind == EV_SW && code == SW_LID)
                        || (kind == EV_SYN && code == SYN_DROPPED);
                }
            }
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(touched),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) => return Err(error),
        }
    }
}

/// The switch this offers, through uinput.
struct Offered(File);

impl Offered {
    fn create() -> io::Result<Offered> {
        let file = OpenOptions::new()
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open("/dev/uinput")?;
        let fd = file.as_raw_fd();
        checked(unsafe { ioctl(fd, UI_SET_EVBIT, c_int::from(EV_SW)) })?;
        checked(unsafe { ioctl(fd, UI_SET_SWBIT, c_int::from(SW_LID)) })?;
        let mut setup = Setup {
            bustype: BUS_VIRTUAL,
            vendor: 0,
            product: 0,
            version: 1,
            name: [0; 80],
            ff_effects_max: 0,
        };
        setup.name[..NAME.len()].copy_from_slice(NAME.as_bytes());
        checked(unsafe { ioctl(fd, UI_DEV_SETUP, &raw const setup) })?;
        checked(unsafe { ioctl(fd, UI_DEV_CREATE) })?;
        Ok(Offered(file))
    }

    fn say(&mut self, shut: bool) -> io::Result<()> {
        let mut events = [0u8; 2 * EVENT_SIZE];
        for (event, (kind, code, value)) in events
            .as_chunks_mut::<EVENT_SIZE>()
            .0
            .iter_mut()
            .zip([(EV_SW, SW_LID, i32::from(shut)), (EV_SYN, SYN_REPORT, 0)])
        {
            event[16..18].copy_from_slice(&kind.to_ne_bytes());
            event[18..20].copy_from_slice(&code.to_ne_bytes());
            event[20..24].copy_from_slice(&value.to_ne_bytes());
        }
        self.0.write_all(&events)
    }
}

/// Gives the judge the hinge's angles that came since the last.
fn hear(hinge: Option<&mut Hinge>, judge: &mut Judge) -> io::Result<()> {
    let Some(hinge) = hinge else {
        return Ok(());
    };
    for degrees in hinge.angles()? {
        if judge.hinge(degrees, Instant::now()) {
            eprintln!("lid-hinge: the hinge is nearly shut, at {degrees:.0}°");
        }
    }
    Ok(())
}

fn run() -> io::Result<()> {
    let root =
        env::var_os("LID_HINGE_IIO").map_or_else(|| "/sys/bus/iio/devices".into(), PathBuf::from);
    let nodes = env::var_os("LID_HINGE_NODES").map_or_else(|| "/dev".into(), PathBuf::from);
    let (mut lid, name) = find_lid()?;
    let mut hinge = Hinge::find(&root, &nodes);
    eprintln!(
        "lid-hinge: following {name}, {}",
        if hinge.is_some() {
            "with the hinge"
        } else {
            "with no hinge, by the light alone"
        }
    );
    let mut offered = Offered::create()?;
    let mut judge = Judge::new(lid_shut(&lid)?);
    offered.say(judge.offered_shut)?;
    hear(hinge.as_mut(), &mut judge)?;
    loop {
        let mut fds = vec![PollFd {
            fd: lid.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        if let Some(found) = &hinge {
            fds.push(PollFd {
                fd: found.node.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            });
        }
        let timeout = [
            judge.watching().then_some(LOOK_AGAIN),
            hinge.is_none().then_some(LOOK_FOR_HINGE),
        ]
        .into_iter()
        .flatten()
        .min()
        .map_or(-1, |wait| {
            c_int::try_from(wait.as_millis()).unwrap_or(c_int::MAX)
        });
        let count = c_ulong::try_from(fds.len()).unwrap_or(1);
        match checked(unsafe { poll(fds.as_mut_ptr(), count, timeout) }) {
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            result => result?,
        };
        if fds[0].revents & (POLLERR | POLLHUP) != 0 {
            return Err(io::Error::other("the lid's switch went away"));
        }
        if fds
            .get(1)
            .is_some_and(|fd| fd.revents & (POLLERR | POLLHUP) != 0)
        {
            eprintln!("lid-hinge: the hinge went away, so the light alone tells");
            hinge = None;
        } else if hinge.is_none() {
            hinge = Hinge::find(&root, &nodes);
            if hinge.is_some() {
                eprintln!("lid-hinge: found the hinge");
            }
        }
        // Before the switch, since the angles came before it.
        hear(hinge.as_mut(), &mut judge)?;
        let now = Instant::now();
        // Every wake, so presses long before a close aren't taken for a hand on a shut lid.
        if let Some(count) = hands()
            && judge.keys(count, now) == Verdict::Used
        {
            eprintln!(
                "lid-hinge: the lid is open: its switch says shut, but the laptop's own keys \
                 or touchpad were used"
            );
            offered.say(false)?;
        }
        let verdict = if fds[0].revents & POLLIN != 0 && lid_events(&mut lid)? {
            let shut = lid_shut(&lid)?;
            let seen = judge
                .hinge
                .map_or_else(|| "unread".to_owned(), |degrees| format!("{degrees:.0}°"));
            let verdict = judge.switch(shut, if shut { light(&root) } else { Vec::new() }, now);
            if verdict == Verdict::Doubted {
                eprintln!(
                    "lid-hinge: the lid's switch says shut with the hinge last at {seen}, \
                     so the lid stays open until the light goes dark with no hands on it"
                );
            }
            verdict
        } else if judge.doubting() {
            judge.light(&light(&root), now)
        } else {
            Verdict::Unchanged
        };
        match verdict {
            Verdict::Shut(why) => {
                eprintln!("lid-hinge: the lid is shut: {why}");
                offered.say(true)?;
            }
            Verdict::Open => {
                eprintln!("lid-hinge: the lid is open");
                offered.say(false)?;
            }
            Verdict::Doubted | Verdict::Unchanged | Verdict::Used => {}
        }
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("lid-hinge: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
#[path = "lid-hinge_tests.rs"]
mod tests;

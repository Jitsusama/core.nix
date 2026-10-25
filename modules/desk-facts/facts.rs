//! Writes what the launcher can't learn for itself about the desk's displays,
//! sound cards, cameras and Bluetooth input devices, for it to show behind
//! each one's row.
//!
//! ```text
//! desk-facts FOLDER [LOOPBACKS]
//! ```
//!
//! Each kind goes to a file of its own in FOLDER, as JSON: an object of
//! devices, each an ordered list of `{"name": …, "value": …}`, the way the
//! launcher lists them. `displays.json` is keyed by the connector's name as
//! niri gives it ("DP-4", "eDP-1"), with what the display's EDID says of
//! itself: who made it and when, its size, native mode, refresh range, colour
//! depth and connection, its colours against sRGB's, and whether it takes
//! HDR. The one external display, when exactly one Type-C partner shows
//! DisplayPort, also gets its cable's facts: the USB mode beside the
//! picture, the power the machine charges at, and the supplies the monitor
//! offers. After those come what a monitor says of itself over DDC/CI, its
//! brightness, contrast, colour preset, panel, controller, hot-plug
//! detection, firmware level and MCCS version, which the backlight daemon
//! keeps in FOLDER as `ddc-` and the hex of the EDID's bytes 8 to 17
//! (modules/backlight's facts.rs), since the bus it reads behind an MST hub
//! names no connector. `sound.json` is keyed by a USB sound card's ALSA id
//! and direction ("MV7:in", "Plus:out"), PipeWire carrying the id on each
//! node as `alsa.id`, with what the card offers, how its clock keeps time,
//! its USB speed and firmware, and, while it plays or records, the format,
//! period and buffer it runs at, in frames and milliseconds, and the rate
//! the kernel measures it taking.
//! The Pebble X Plus's lighting comes from the file `pebble-lights` keeps
//! beside the folder. A card not on USB, a laptop's own, is keyed by each
//! of its devices as PipeWire's path to it names it ("sofsoundwire,2:out"
//! for `hw:sofsoundwire,2`), with what it runs at while it plays or
//! records, what its jack has plugged in, and the chip behind it, from the
//! parts its driver names and the `SoundWire` devices themselves.
//! `cameras.json` is keyed by a camera's name as its holders file in
//! /run/camera-holders is named. A USB camera's says what it streams while
//! it streams, the largest and the fastest mode of each format it offers,
//! its field of view where a Logitech Brio says it, and its USB speed and
//! firmware. A loopback camera LOOPBACKS lists, a line each of its holders
//! file's name, its own name and its sensor's I2C device, apart by tabs,
//! says what it streams while held and its one mode, from v4l2loopback's
//! sysfs, and its sensor, which way that faces and whether it's powered,
//! from the sensor's; neither is ever opened. `bluetooth.json` is keyed by a connected Bluetooth input
//! device's address, as bluetoothd gives it to the launcher, with how it's
//! carried (LE's HID over GATT or Classic's HIDP), its buttons and wheels
//! or its keys, and its ids and release, from the kernel's HID devices.
//!
//! A file is written only when what it says changes, by writing a copy and
//! renaming it over the old one, and all are removed when this is stopped,
//! so a fact is never left behind its device. It hears the kernel's uevents
//! for displays, cards, cameras and HID devices coming, going and changing,
//! inotify for a playback or capture device opening and closing, the
//! lighting's file being written, a camera's holders changing and a
//! monitor's DDC/CI facts being kept or removed, and a built-in card's
//! control device for its jacks. Sound and cameras are read again a few
//! times as they settle; otherwise nothing is looked at between, and each
//! kind is read again only when what it hears is about it.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

const AF_NETLINK: c_int = 16;
const SOCK_DGRAM: c_int = 2;
const SOCK_NONBLOCK: c_int = 0o4000;
const SOCK_CLOEXEC: c_int = 0o2_000_000;
const NETLINK_KOBJECT_UEVENT: c_int = 15;
const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_CLOSE_WRITE: u32 = 0x008;
const IN_CLOSE_NOWRITE: u32 = 0x010;
const IN_OPEN: u32 = 0x020;
const IN_MOVED_TO: u32 = 0x080;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
const IN_Q_OVERFLOW: u32 = 0x4000;
const POLLIN: i16 = 0x001;
const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;
const O_NONBLOCK: i32 = 0o4000;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

/// The kinds of fact, as bits, so a change reads again only its own.
const DISPLAYS: u8 = 1;
const SOUND: u8 = 2;
const CAMERAS: u8 = 4;
const BLUETOOTH: u8 = 8;
/// The kernel's uevents that change a fact, and the kinds each changes.
const HEARD: [(&[u8], u8); 7] = [
    (b"SUBSYSTEM=drm", DISPLAYS),
    (b"SUBSYSTEM=sound", SOUND),
    (b"SUBSYSTEM=usb", SOUND | CAMERAS),
    (b"SUBSYSTEM=video4linux", CAMERAS),
    (b"SUBSYSTEM=typec", DISPLAYS),
    (b"SUBSYSTEM=hid", BLUETOOTH),
    (b"SUBSYSTEM=input", BLUETOOTH),
];
/// Where each camera's holders are kept (modules/usb-camera-holders).
const HOLDERS: &str = "/run/camera-holders";
/// The Pebble X Plus, as its USB device names itself.
const PEBBLE: &str = "041e:329a";

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

impl PollFd {
    /// Polls a descriptor for something to read.
    fn reading(fd: c_int) -> PollFd {
        PollFd {
            fd,
            events: POLLIN,
            revents: 0,
        }
    }
}

#[repr(C)]
struct SockaddrNl {
    family: u16,
    pad: u16,
    pid: u32,
    groups: u32,
}

/// glibc's `sigset_t`, 1024 bits.
#[repr(C)]
struct SigSet([u64; 16]);

unsafe extern "C" {
    fn socket(domain: c_int, kind: c_int, protocol: c_int) -> c_int;
    fn bind(fd: c_int, address: *const c_void, length: u32) -> c_int;
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
    fn ioctl(fd: c_int, request: u64, ...) -> c_int;
}

/// A device's facts, in the order they're shown.
type Facts = Vec<(String, String)>;

fn fact(facts: &mut Facts, name: &str, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() {
        facts.push((name.to_owned(), value));
    }
}

// --- Displays, from their EDID ---

/// The makers a desk is likely to have, by the three letters an EDID gives;
/// any other is shown as its letters.
const MAKERS: [(&str, &str); 14] = [
    ("ACR", "Acer"),
    ("AUO", "AU Optronics"),
    ("BOE", "BOE"),
    ("DEL", "Dell"),
    ("ENC", "EIZO"),
    ("GSM", "LG"),
    ("HPN", "HP"),
    ("LEN", "Lenovo"),
    ("LGD", "LG Display"),
    ("SAM", "Samsung"),
    ("SDC", "Samsung Display"),
    ("SHP", "Sharp"),
    ("BNQ", "BenQ"),
    ("AUS", "ASUS"),
];

/// sRGB's primaries, red, green then blue, as CIE 1931 xy.
const SRGB: [(f64, f64); 3] = [(0.64, 0.33), (0.30, 0.60), (0.15, 0.06)];

/// The three letters an EDID's maker field packs into two bytes.
fn maker(high: u8, low: u8) -> String {
    let code = u16::from_be_bytes([high, low]);
    [10, 5, 0]
        .iter()
        .map(|shift| char::from(b'@' + u8::try_from((code >> shift) & 0x1f).unwrap_or(0)))
        .collect()
}

/// A display descriptor's text: up to 13 bytes, ended by a line feed.
fn text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).trim().to_owned()
}

/// The area of a triangle of xy points.
fn area(points: [(f64, f64); 3]) -> f64 {
    let [(ax, ay), (bx, by), (cx, cy)] = points;
    ((bx - ax) * (cy - ay) - (cx - ax) * (by - ay)).abs() / 2.0
}

/// The red, green, blue and white points an EDID gives, each 10 bits a
/// coordinate.
fn chromaticity(edid: &[u8]) -> [(f64, f64); 4] {
    let low = |byte: u8, shift: u8| u16::from((byte >> shift) & 0b11);
    let value = |high: u8, low: u16| f64::from(u16::from(high) << 2 | low) / 1024.0;
    let (rg, bw) = (edid[25], edid[26]);
    [
        (value(edid[27], low(rg, 6)), value(edid[28], low(rg, 4))),
        (value(edid[29], low(rg, 2)), value(edid[30], low(rg, 0))),
        (value(edid[31], low(bw, 6)), value(edid[32], low(bw, 4))),
        (value(edid[33], low(bw, 2)), value(edid[34], low(bw, 0))),
    ]
}

/// How its colours compare with sRGB's: the same primaries, or how much
/// wider or narrower its triangle is.
fn gamut(edid: &[u8]) -> String {
    let [red, green, blue, _] = chromaticity(edid);
    let primaries = [red, green, blue];
    let same = primaries
        .iter()
        .zip(SRGB)
        .all(|((x, y), (sx, sy))| (x - sx).abs() < 0.005 && (y - sy).abs() < 0.005);
    if same {
        return "sRGB's primaries".to_owned();
    }
    format!("{:.2} \u{d7} sRGB's area", area(primaries) / area(SRGB))
}

/// Its native mode, from the first detailed timing.
fn native(timing: &[u8]) -> Option<String> {
    let clock = u32::from(u16::from_le_bytes([timing[0], timing[1]])) * 10_000;
    if clock == 0 {
        return None;
    }
    let wide = u32::from(timing[2]) | u32::from(timing[4] >> 4) << 8;
    let blank_wide = u32::from(timing[3]) | u32::from(timing[4] & 0xf) << 8;
    let high = u32::from(timing[5]) | u32::from(timing[7] >> 4) << 8;
    let blank_high = u32::from(timing[6]) | u32::from(timing[7] & 0xf) << 8;
    let total = (wide + blank_wide) * (high + blank_high);
    let hertz = f64::from(clock) / f64::from(total.max(1));
    Some(format!("{wide} \u{d7} {high} at {hertz:.0} Hz"))
}

/// What a range limits descriptor says of the refresh rates it takes.
fn refresh_range(descriptor: &[u8]) -> String {
    let flags = descriptor[4];
    let offset = |set: bool| if set { 255 } else { 0 };
    let lowest = u16::from(descriptor[5]) + offset(flags & 0b11 == 0b11);
    let highest = u16::from(descriptor[6]) + offset(flags & 0b10 != 0);
    format!("{lowest} to {highest} Hz")
}

/// A digital input's bits a colour and its interface, from EDID 1.4's input
/// byte, each empty where the EDID doesn't say. An analogue input says no
/// depth.
fn input(byte: u8) -> (&'static str, &'static str) {
    if byte & 0x80 == 0 {
        return ("", "analogue");
    }
    let depth = match (byte >> 4) & 0b111 {
        1 => "6 bit",
        2 => "8 bit",
        3 => "10 bit",
        4 => "12 bit",
        5 => "14 bit",
        6 => "16 bit",
        _ => "",
    };
    let interface = match byte & 0xf {
        1 => "DVI",
        2 | 3 => "HDMI",
        5 => "DisplayPort",
        _ => "",
    };
    (depth, interface)
}

/// Whether a CTA-861 extension's data blocks hold HDR static metadata, and
/// which transfer functions it takes.
fn hdr(extension: &[u8]) -> Option<String> {
    if extension.first() != Some(&0x02) {
        return None;
    }
    let end = usize::from(*extension.get(2)?).min(extension.len());
    let mut at = 4;
    while at < end {
        let header = extension[at];
        let length = usize::from(header & 0x1f);
        let block = extension.get(at + 1..=at + length)?;
        // An extended block, tag 7, whose own tag 6 is HDR static metadata.
        if header >> 5 == 7 && block.first() == Some(&0x06) {
            let eotf = block.get(1).copied().unwrap_or(0);
            let mut takes = Vec::new();
            if eotf & 0b100 != 0 {
                takes.push("HDR10");
            }
            if eotf & 0b1000 != 0 {
                takes.push("HLG");
            }
            return Some(if takes.is_empty() {
                "metadata only".to_owned()
            } else {
                takes.join(" and ")
            });
        }
        at += 1 + length;
    }
    Some("none".to_owned())
}

/// What a display's EDID says of it, or nothing for one that isn't whole.
fn display(edid: &[u8]) -> Option<Facts> {
    let block = edid.get(..128)?;
    if block[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00]
        || block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) != 0
    {
        return None;
    }
    let mut facts = Facts::new();
    let code = maker(block[8], block[9]);
    let maker = MAKERS
        .iter()
        .find(|(letters, _)| *letters == code)
        .map_or(code.clone(), |(_, name)| (*name).to_owned());
    let mut model = String::new();
    let mut serial = String::new();
    let mut range = String::new();
    for descriptor in block[54..126].as_chunks::<18>().0.iter().skip(1) {
        if descriptor[..3] != [0, 0, 0] {
            continue;
        }
        match descriptor[3] {
            0xFC => model = text(&descriptor[5..]),
            0xFF => serial = text(&descriptor[5..]),
            0xFD => range = refresh_range(descriptor),
            _ => {}
        }
    }
    if serial.is_empty() {
        let number = u32::from_le_bytes([block[12], block[13], block[14], block[15]]);
        if number != 0 {
            serial = number.to_string();
        }
    }
    if model.is_empty() {
        // A panel often names nothing but its product code.
        model = format!("product {:04X}", u16::from_le_bytes([block[10], block[11]]));
    }
    fact(&mut facts, "maker", maker);
    fact(&mut facts, "model", model);
    fact(&mut facts, "serial", serial);
    let year = 1990 + u16::from(block[17]);
    fact(
        &mut facts,
        "made",
        match block[16] {
            0 => year.to_string(),
            0xFF => format!("{year} model"),
            week => format!("week {week}, {year}"),
        },
    );
    let (wide, high) = (f64::from(block[21]), f64::from(block[22]));
    if wide > 0.0 && high > 0.0 {
        let inches = wide.hypot(high) / 2.54;
        fact(
            &mut facts,
            "size",
            format!("{wide:.0} \u{d7} {high:.0} cm, {inches:.1} in"),
        );
    }
    fact(
        &mut facts,
        "native",
        native(&block[54..72]).unwrap_or_default(),
    );
    fact(&mut facts, "refresh", range);
    let (depth, link) = input(block[20]);
    fact(&mut facts, "depth", depth);
    fact(&mut facts, "input", link);
    fact(&mut facts, "colours", gamut(block));
    if let Some(extension) = edid.get(128..256) {
        fact(&mut facts, "HDR", hdr(extension).unwrap_or_default());
    }
    fact(&mut facts, "EDID", format!("{}.{}", block[18], block[19]));
    Some(facts)
}

/// Every connected display by niri's name for its connector, from the DRM
/// connectors `sys` lists.
fn displays(sys: &Path, told: &Path) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("class/drm")) else {
        return Vec::new();
    };
    let mut found: Vec<(String, Facts)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (_, connector) = name.strip_prefix("card")?.split_once('-')?;
            let status = fs::read_to_string(entry.path().join("status")).ok()?;
            if status.trim() != "connected" {
                return None;
            }
            let edid = fs::read(entry.path().join("edid")).ok()?;
            let mut facts = display(&edid)?;
            facts.extend(monitor_said(told, &edid));
            Some((connector.to_owned(), facts))
        })
        .collect();
    found.sort();
    // The USB-C cable's facts go to the one external display there is; with
    // more, nothing says which is the cable's.
    let mut external = found
        .iter_mut()
        .filter(|(connector, _)| !connector.starts_with("eDP"));
    if let (Some((_, facts)), None) = (external.next(), external.next()) {
        facts.extend(usb_c(sys));
    }
    found
}

/// What a monitor says of itself over DDC/CI, as the backlight daemon keeps
/// it in `told` (modules/backlight's facts.rs): a file named for the EDID's
/// bytes 8 to 17, a fact a line, its name and value split by a tab.
fn monitor_said(told: &Path, edid: &[u8]) -> Facts {
    let Some(identity) = edid.get(8..18) else {
        return Facts::new();
    };
    let name = identity
        .iter()
        .fold(String::from("ddc-"), |mut name, byte| {
            let _ = write!(name, "{byte:02x}");
            name
        });
    let text = fs::read_to_string(told.join(name)).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect()
}

/// Whether inotify's events name a monitor's DDC facts written whole, or
/// lost some that might have.
fn names_a_monitor(events: &[u8]) -> bool {
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let (mask, length) = (word(4), word(12) as usize);
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or(name);
        if mask & IN_Q_OVERFLOW != 0 || (name.starts_with(b"ddc-") && !name.ends_with(b".new")) {
            return true;
        }
        rest = after;
    }
    false
}

// --- A monitor's USB-C cable, from the Type-C class and UCSI ---

/// The SVID of the DisplayPort alternate mode.
const DISPLAYPORT_SVID: &str = "ff01";

/// The bracketed choice in a sysfs list, as "[usb2] usb3" gives "usb2".
fn chosen(list: &str) -> Option<&str> {
    let (_, rest) = list.split_once('[')?;
    Some(rest.split_once(']')?.0)
}

/// A USB mode as Type-C names it, said plainly.
fn usb_mode(mode: &str) -> String {
    match mode {
        "usb2" => "USB 2".to_owned(),
        "usb3" => "USB 3".to_owned(),
        "usb4" => "USB4".to_owned(),
        other => other.to_owned(),
    }
}

/// Volts from millivolts or microvolts, amps from milliamps or microamps,
/// to the nearest tenth, a whole number without its point.
fn tenths(thousandths: u64) -> String {
    let tenths = (thousandths + 50) / 100;
    if tenths.is_multiple_of(10) {
        format!("{}", tenths / 10)
    } else {
        format!("{}.{}", tenths / 10, tenths % 10)
    }
}

/// What a source offers, from its fixed supplies' voltages and currents in
/// millivolts and milliamps: "5, 9 and 20 V at 3 A" when they share a
/// current, and otherwise each with its own.
fn offered_power(supplies: &[(u64, u64)]) -> String {
    let Some(&(_, current)) = supplies.first() else {
        return String::new();
    };
    let volts: Vec<String> = supplies.iter().map(|(mv, _)| tenths(*mv)).collect();
    if supplies.iter().all(|(_, each)| *each == current) {
        let listed = match volts.split_last() {
            Some((last, [])) => last.clone(),
            Some((last, rest)) => format!("{} and {last}", rest.join(", ")),
            None => String::new(),
        };
        return format!("{listed} V at {} A", tenths(current));
    }
    supplies
        .iter()
        .map(|(mv, ma)| format!("{} V at {} A", tenths(*mv), tenths(*ma)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The fixed supplies a partner's PD source capabilities list, in
/// millivolts and milliamps, in their order.
fn fixed_supplies(partner: &Path) -> Vec<(u64, u64)> {
    let Ok(entries) = fs::read_dir(partner) else {
        return Vec::new();
    };
    let Some(source) = entries
        .flatten()
        .map(|entry| entry.path().join("source-capabilities"))
        .find(|path| path.is_dir())
    else {
        return Vec::new();
    };
    let mut supplies: Vec<(u32, u64, u64)> = fs::read_dir(source)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().into_string().ok()?;
            let (index, "fixed_supply") = name.split_once(':')? else {
                return None;
            };
            let read = |file: &str| {
                let text = fs::read_to_string(entry.path().join(file)).ok()?;
                text.trim()
                    .trim_end_matches(|c: char| c.is_ascii_alphabetic())
                    .parse()
                    .ok()
            };
            Some((
                index.parse().ok()?,
                read("voltage")?,
                read("maximum_current")?,
            ))
        })
        .collect();
    supplies.sort_unstable();
    supplies.into_iter().map(|(_, mv, ma)| (mv, ma)).collect()
}

/// What the USB-C cable to the one monitor carries, when exactly one
/// partner has DisplayPort's alternate mode active: how its data runs
/// beside the picture, the power it charges the machine with, and what it
/// offers. With two such partners nothing says which display is whose, so
/// none of this is said.
fn usb_c(sys: &Path) -> Facts {
    let class = sys.join("class/typec");
    let Ok(entries) = fs::read_dir(&class) else {
        return Facts::new();
    };
    let showing: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|partner| {
            partner
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with("-partner"))
                && fs::read_dir(partner)
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|mode| {
                        let read = |file: &str| {
                            fs::read_to_string(mode.path().join(file)).unwrap_or_default()
                        };
                        read("svid").trim() == DISPLAYPORT_SVID && read("active").trim() == "yes"
                    })
        })
        .collect();
    let [partner] = &showing[..] else {
        return Facts::new();
    };
    let mut facts = Facts::new();
    let mode = fs::read_to_string(partner.join("usb_mode")).unwrap_or_default();
    fact(
        &mut facts,
        "USB-C",
        chosen(&mode).map_or("DisplayPort".to_owned(), |mode| {
            format!("DisplayPort and {}", usb_mode(mode))
        }),
    );
    // UCSI names a port's supply for its device and the connector's
    // number, one more than the port's: port1 of USBC000:00 charges
    // through ucsi-source-psy-USBC000:002.
    let port = partner
        .file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.strip_suffix("-partner"))
        .unwrap_or_default();
    let index = port
        .strip_prefix("port")
        .and_then(|n| n.parse::<u32>().ok());
    let device = fs::read_link(class.join(port).join("device"))
        .ok()
        .and_then(|link| {
            link.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        });
    if let (Some(index), Some(device)) = (index, device) {
        let supply = sys
            .join("class/power_supply")
            .join(format!("ucsi-source-psy-{device}{}", index + 1));
        let read = |file: &str| {
            fs::read_to_string(supply.join(file))
                .ok()
                .and_then(|text| text.trim().parse::<u64>().ok())
        };
        if let (Some(1), Some(volts), Some(amps)) =
            (read("online"), read("voltage_now"), read("current_max"))
        {
            let watts = (volts / 1000 * (amps / 1000) + 500_000) / 1_000_000;
            fact(
                &mut facts,
                "charging",
                format!(
                    "{watts} W ({} V, {} A)",
                    tenths(volts / 1000),
                    tenths(amps / 1000)
                ),
            );
        }
    }
    fact(
        &mut facts,
        "power offered",
        offered_power(&fixed_supplies(partner)),
    );
    facts
}

// --- Sound cards, from what ALSA says of them ---

/// One way a USB card's stream can run: its format, channels and rates.
#[derive(Debug, Default, PartialEq, Eq)]
struct Setting {
    format: String,
    channels: String,
    rates: String,
}

/// One direction of a USB card, as its stream file says.
#[derive(Debug, Default, PartialEq, Eq)]
struct Stream {
    running: bool,
    momentary: String,
    clock: String,
    settings: Vec<Setting>,
}

/// A USB card's speed and its playback and capture streams, from
/// /proc/asound/cardN/stream0.
fn streams(text: &str) -> (String, Stream, Stream) {
    let speed = text
        .lines()
        .next()
        .and_then(|line| line.rsplit_once(", ")?.1.split_once(" :"))
        .map_or(String::new(), |(speed, _)| speed.to_owned());
    let (mut playback, mut capture) = (Stream::default(), Stream::default());
    let mut now: Option<&mut Stream> = None;
    for line in text.lines() {
        let line = line.trim();
        match line {
            "Playback:" => now = Some(&mut playback),
            "Capture:" => now = Some(&mut capture),
            _ => {}
        }
        let Some(stream) = now.as_deref_mut() else {
            continue;
        };
        if let Some(status) = line.strip_prefix("Status: ") {
            stream.running = status == "Running";
        } else if let Some(rate) = line.strip_prefix("Momentary freq = ") {
            rate.split(" (")
                .next()
                .unwrap_or(rate)
                .clone_into(&mut stream.momentary);
        } else if let Some(format) = line.strip_prefix("Format: ") {
            stream.settings.push(Setting {
                format: format.to_owned(),
                ..Setting::default()
            });
        } else if let Some(channels) = line.strip_prefix("Channels: ") {
            if let Some(setting) = stream.settings.last_mut() {
                channels.clone_into(&mut setting.channels);
            }
        } else if let Some(rates) = line.strip_prefix("Rates: ") {
            if let Some(setting) = stream.settings.last_mut() {
                rates.clone_into(&mut setting.rates);
            }
        } else if let Some(endpoint) = line.strip_prefix("Endpoint: ") {
            for (said, clock) in [
                ("(ASYNC)", "asynchronous"),
                ("(ADAPTIVE)", "adaptive"),
                ("(SYNC)", "synchronous"),
            ] {
                if endpoint.contains(said) {
                    clock.clone_into(&mut stream.clock);
                }
            }
        }
    }
    (speed, playback, capture)
}

/// A sample format's bits, as ALSA names it: `S24_3LE` is 24.
fn bits(format: &str) -> String {
    let digits: String = format
        .chars()
        .skip_while(|c| !c.is_ascii_digit())
        .take_while(char::is_ascii_digit)
        .collect();
    if digits.is_empty() {
        format.to_owned()
    } else {
        format!("{digits} bit")
    }
}

/// A rate in Hz, in kHz as it's said: 44100 is "44.1 kHz".
fn kilohertz(rate: &str) -> String {
    let Ok(hertz) = rate.trim().parse::<u32>() else {
        return rate.trim().to_owned();
    };
    if hertz % 1000 == 0 {
        format!("{} kHz", hertz / 1000)
    } else {
        format!("{:.1} kHz", f64::from(hertz) / 1000.0)
    }
}

fn channels(count: &str) -> String {
    match count {
        "1" => "mono".to_owned(),
        "2" => "stereo".to_owned(),
        count => format!("{count} channels"),
    }
}

/// Every way a stream can run, each once: "48 kHz, 16 bit, stereo".
fn offered(settings: &[Setting]) -> String {
    let mut said: Vec<String> = Vec::new();
    for setting in settings {
        let rates: Vec<String> = setting.rates.split(',').map(kilohertz).collect();
        let each = format!(
            "{}, {}, {}",
            rates.join(" or "),
            bits(&setting.format),
            channels(&setting.channels)
        );
        if !said.contains(&each) {
            said.push(each);
        }
    }
    said.join("; ")
}

/// What a running stream's `hw_params` say it runs at: its period and
/// buffer in frames, and the time each holds at its rate, which is the
/// card's own share of the delay.
fn running(hw_params: &str) -> String {
    let field = |name: &str| {
        hw_params
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix(": "))
            .map(|value| value.split(' ').next().unwrap_or(value).to_owned())
    };
    let (Some(format), Some(rate)) = (field("format"), field("rate")) else {
        return String::new();
    };
    let mut said = format!("{}, {}", kilohertz(&rate), bits(&format));
    let hertz = rate.parse::<u32>().ok().filter(|hertz| *hertz > 0);
    for (name, size) in [
        ("period", field("period_size")),
        ("buffer", field("buffer_size")),
    ] {
        let Some(size) = size else { continue };
        let _ = write!(said, ", {name} {size}");
        if let (Some(hertz), Ok(frames)) = (hertz, size.parse::<u32>()) {
            let _ = write!(said, " ({})", milliseconds(frames, hertz));
        }
    }
    said
}

/// The time some frames hold at a rate, to a tenth of a millisecond.
fn milliseconds(frames: u32, hertz: u32) -> String {
    let tenths = (u64::from(frames) * 10_000 + u64::from(hertz) / 2) / u64::from(hertz);
    format!("{}.{} ms", tenths / 10, tenths % 10)
}

/// A USB release number as its device gives it: 0x1310 is "13.10", and
/// 0x0021 "0.21".
fn release(bcd: &str) -> String {
    let bcd = bcd.trim();
    match (bcd.get(..2).map(str::parse::<u8>), bcd.get(2..)) {
        (Some(Ok(major)), Some(minor)) if bcd.len() == 4 => format!("{major}.{minor}"),
        _ => bcd.to_owned(),
    }
}

/// One direction of a card's facts.
fn direction(stream: &Stream, speed: &str, firmware: &str, now: &str, playing: bool) -> Facts {
    let mut facts = Facts::new();
    if stream.running {
        fact(
            &mut facts,
            if playing { "playing" } else { "recording" },
            now,
        );
        fact(&mut facts, "measured rate", stream.momentary.clone());
    }
    fact(&mut facts, "offers", offered(&stream.settings));
    fact(&mut facts, "clock", stream.clock.clone());
    fact(&mut facts, "USB", speed);
    fact(&mut facts, "firmware", firmware);
    facts
}

/// Every sound card's facts, from the cards `asound` lists and their
/// devices in `sys`: a USB card's by its ALSA id and direction, and the
/// Pebble's lighting from what its file says; another card's by each of
/// its devices (`built_in`), with what `controls` says of the card by its
/// number.
fn sound(
    asound: &Path,
    sys: &Path,
    lighting: Option<&str>,
    controls: &dyn Fn(&str) -> Control,
) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(asound) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(number) = name.strip_prefix("card") else {
            continue;
        };
        let card = entry.path();
        let Ok(id) = fs::read_to_string(card.join("id")) else {
            continue;
        };
        if !card.join("usbid").exists() {
            let parts = sound_parts(sys);
            found.extend(built_in(&card, id.trim(), &controls(number), &parts));
            continue;
        }
        let (Ok(stream0), Ok(usbid)) = (
            fs::read_to_string(card.join("stream0")),
            fs::read_to_string(card.join("usbid")),
        ) else {
            continue;
        };
        let (id, usbid) = (id.trim(), usbid.trim());
        let usb = sys.join(format!("class/sound/card{number}/device/.."));
        let firmware = fs::read_to_string(usb.join("bcdDevice"))
            .map(|bcd| format!("release {}", release(&bcd)))
            .unwrap_or_default();
        let (speed, playback, capture) = streams(&stream0);
        let now =
            |sub: &str| fs::read_to_string(card.join(sub)).map_or(String::new(), |hw| running(&hw));
        if !playback.settings.is_empty() {
            let mut facts = direction(
                &playback,
                &speed,
                &firmware,
                &now("pcm0p/sub0/hw_params"),
                true,
            );
            if usbid == PEBBLE {
                let said = match lighting {
                    Some("on") => "on",
                    Some("off") => "off while unused",
                    Some("off by hand") => "off at its button",
                    _ => "",
                };
                if !said.is_empty() {
                    facts.insert(0, ("lighting".to_owned(), said.to_owned()));
                }
            }
            found.push((format!("{id}:out"), facts));
        }
        if !capture.settings.is_empty() {
            found.push((
                format!("{id}:in"),
                direction(
                    &capture,
                    &speed,
                    &firmware,
                    &now("pcm0c/sub0/hw_params"),
                    false,
                ),
            ));
        }
    }
    found.sort();
    found
}

// --- A machine's own sound card, from ALSA's control device ---

/// ALSA's control requests, each as asound.h has it.
const SNDRV_CTL_IOCTL_CARD_INFO: u64 = 0x8178_5501;
const SNDRV_CTL_IOCTL_ELEM_LIST: u64 = 0xC050_5510;
const SNDRV_CTL_IOCTL_ELEM_READ: u64 = 0xC4C8_5512;
const SNDRV_CTL_IOCTL_SUBSCRIBE_EVENTS: u64 = 0xC004_5516;
/// An element's id, `struct snd_ctl_elem_id`: its interface at 4 and its
/// name at 16, 44 bytes.
const ELEMENT_ID: usize = 64;
/// `struct snd_ctl_elem_value`, whose first value is at 72.
const ELEMENT_VALUE: usize = 1224;
/// The card's own interface, which jacks are on.
const IFACE_CARD: u32 = 0;

/// What a card's control device says of it: its jacks, each by its
/// control's name and whether something is plugged into it, the parts its
/// driver names, and its mixer's name.
#[derive(Debug, Default, PartialEq, Eq)]
struct Control {
    jacks: Vec<(String, bool)>,
    components: String,
    mixer: String,
}

/// The text a C string in a buffer holds, up to its first NUL.
fn c_text(bytes: &[u8]) -> String {
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Every element id a card's control device has.
fn element_ids(control: &File) -> Vec<[u8; ELEMENT_ID]> {
    let mut list = [0u8; 80];
    if !ask(control, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) {
        return Vec::new();
    }
    let count = word(&list, 12);
    let mut ids = vec![[0u8; ELEMENT_ID]; count as usize];
    list[4..8].copy_from_slice(&count.to_ne_bytes());
    list[16..24].copy_from_slice(&(ids.as_mut_ptr() as usize as u64).to_ne_bytes());
    // The ids are written where the list's pointer says, which is `ids`,
    // as long as count says.
    if !ask(control, SNDRV_CTL_IOCTL_ELEM_LIST, &mut list) {
        return Vec::new();
    }
    ids.truncate(word(&list, 8) as usize);
    ids
}

/// What a card's control device says of it.
fn control(control: &File) -> Control {
    let mut said = Control::default();
    let mut info = [0u8; 376];
    if ask(control, SNDRV_CTL_IOCTL_CARD_INFO, &mut info) {
        said.mixer = c_text(&info[168..248]);
        said.components = c_text(&info[248..376]);
    }
    for id in element_ids(control) {
        let name = c_text(&id[16..60]);
        if word(&id, 4) != IFACE_CARD || !name.ends_with(" Jack") {
            continue;
        }
        let mut value = [0u8; ELEMENT_VALUE];
        value[..ELEMENT_ID].copy_from_slice(&id);
        if ask(control, SNDRV_CTL_IOCTL_ELEM_READ, &mut value) {
            said.jacks.push((name, word(&value, 72) != 0));
        }
    }
    said
}

/// Which side of a card a jack is on, from its control's name: an SDCA
/// codec says its output and input terminals ("OT 43", "IT 33"), and
/// others say what plugs in. A headset jack with neither is both.
fn jack_sides(name: &str) -> (bool, bool) {
    let words: Vec<&str> = name.split(' ').collect();
    if words.contains(&"OT") {
        return (true, false);
    }
    if words.contains(&"IT") {
        return (false, true);
    }
    let out = ["Headphone", "Line Out", "Speaker"]
        .iter()
        .any(|said| name.contains(said));
    let into = ["Mic", "Line In"].iter().any(|said| name.contains(said));
    let headset = name.contains("Headset");
    (out || headset, into || headset)
}

/// What a jack has plugged into it, by what its control says it takes.
fn plugged_in(name: &str) -> &'static str {
    if name.contains("pcm=") {
        "a display"
    } else if name.contains("Headset") {
        "a headset"
    } else if name.contains("Headphone") {
        "headphones"
    } else if name.contains("Mic") {
        "a microphone"
    } else {
        "a cable"
    }
}

/// What a device's jacks have plugged into them, or nothing where it has
/// none: a display's jack names its device ("HDMI/DP,pcm=5 Jack"), and a
/// headphone or microphone jack belongs to the device of that side that
/// says it's a jack or a headset's. A headset wins over what else is
/// plugged in, since it takes both.
fn jack(jacks: &[(String, bool)], device: u32, name: &str, playing: bool) -> String {
    let display = format!("pcm={device} ");
    let mine: Vec<&(String, bool)> = if jacks.iter().any(|(jack, _)| jack.contains(&display)) {
        jacks
            .iter()
            .filter(|(jack, _)| jack.contains(&display))
            .collect()
    } else if ["Jack", "Headphone", "Headset"]
        .iter()
        .any(|said| name.contains(said))
    {
        jacks
            .iter()
            .filter(|(jack, _)| !jack.contains("pcm="))
            .filter(|(jack, _)| {
                let (out, into) = jack_sides(jack);
                if playing { out } else { into }
            })
            .collect()
    } else {
        Vec::new()
    };
    if mine.is_empty() {
        return String::new();
    }
    let plugged = mine
        .iter()
        .filter(|(_, plugged)| *plugged)
        .map(|(jack, _)| plugged_in(jack))
        .min_by_key(|said| *said != "a headset");
    plugged.unwrap_or("nothing").to_owned()
}

/// A `SoundWire` device on the machine: its driver, maker and part, from
/// its name ("sdw:0:2:01fa:3557:01:2").
#[derive(Debug, PartialEq, Eq)]
struct Part {
    driver: String,
    maker: String,
    model: String,
}

/// Every `SoundWire` device `sys` lists.
fn sound_parts(sys: &Path) -> Vec<Part> {
    let Ok(entries) = fs::read_dir(sys.join("bus/soundwire/devices")) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let fields: Vec<&str> = name.split(':').collect();
        let ["sdw", _, _, maker, part, ..] = fields[..] else {
            continue;
        };
        let driver = fs::read_link(entry.path().join("driver"))
            .ok()
            .and_then(|link| Some(link.file_name()?.to_string_lossy().into_owned()))
            .unwrap_or_default();
        parts.push(Part {
            driver,
            maker: maker.to_owned(),
            model: part.to_owned(),
        });
    }
    parts.sort_by(|a, b| (&a.maker, &a.model).cmp(&(&b.maker, &b.model)));
    parts
}

/// A `SoundWire` part as its maker names it: Cirrus Logic's 3557 is the
/// CS35L57, Realtek's 0722 the RT722.
fn part_name(part: &Part) -> String {
    let digits = part.model.to_uppercase();
    match part.maker.as_str() {
        "01fa" if digits.len() == 4 => {
            format!("Cirrus Logic CS{}L{}", &digits[..2], &digits[2..])
        }
        "025d" => format!("Realtek RT{}", digits.trim_start_matches('0')),
        maker => format!("{maker}:{digits}"),
    }
}

/// The chips a card's driver names a part by ("cs35l56"), as the parts
/// themselves say, with how many there are where more than one: a driver
/// can name a family whose part is another of it, as cs35l56 drives the
/// CS35L57. A chip no `SoundWire` device is found for is said as it's
/// named.
fn chips(named: &str, parts: &[Part]) -> String {
    let found: Vec<String> = parts
        .iter()
        .filter(|part| {
            let model = part_name(part).to_lowercase();
            part.driver == named || model.ends_with(&format!(" {named}"))
        })
        .map(part_name)
        .collect();
    let Some(first) = found.first() else {
        return named.to_uppercase();
    };
    if found.len() > 1 && found.iter().all(|each| each == first) {
        format!("{} \u{d7} {first}", found.len())
    } else {
        found.join(", ")
    }
}

/// The chip behind a device, from the parts the card's driver names in its
/// components ("spk:cs35l56 hs:cs42l45 mic:cs42l45-dmic
/// iec61937-pcm:7,6,5"): the speakers' amplifiers, the jacks' codec, the
/// microphones' codec, and for a display's device the mixer's.
fn chip(said: &Control, parts: &[Part], device: u32, name: &str, playing: bool) -> Facts {
    let mut facts = Facts::new();
    let component = |key: &str| {
        said.components
            .split(' ')
            .find_map(|each| each.strip_prefix(key)?.strip_prefix(':'))
    };
    let displays = component("iec61937-pcm").unwrap_or_default();
    if displays.split(',').any(|each| each == device.to_string()) {
        fact(&mut facts, "codec", said.mixer.clone());
    } else if playing && name.starts_with("Speaker") {
        if let Some(amplifiers) = component("spk") {
            fact(&mut facts, "amplifiers", chips(amplifiers, parts));
        }
    } else if name.contains("Jack") {
        if let Some(codec) = component("hs") {
            fact(&mut facts, "codec", chips(codec, parts));
        }
    } else if !playing
        && name.contains("Mic")
        && let Some(codec) = component("mic")
    {
        let (chip, digital) = codec
            .strip_suffix("-dmic")
            .map_or((codec, false), |chip| (chip, true));
        let named = chips(chip, parts);
        let said = if digital {
            format!("{named}, digital microphones")
        } else {
            named
        };
        fact(&mut facts, "codec", said);
    }
    facts
}

/// A card's devices that aren't on USB, each keyed as PipeWire's path to it
/// is ("sofsoundwire,2:out" for `hw:sofsoundwire,2`), with what it runs at
/// while it plays or records, what its jack has plugged in, and its chip.
fn built_in(card: &Path, id: &str, said: &Control, parts: &[Part]) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(card) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.file_name().to_string_lossy().into_owned();
        let Some(device) = folder.strip_prefix("pcm") else {
            continue;
        };
        let (number, playing) = match device.split_at_checked(device.len().saturating_sub(1)) {
            Some((number, "p")) => (number, true),
            Some((number, "c")) => (number, false),
            _ => continue,
        };
        let Ok(device) = number.parse::<u32>() else {
            continue;
        };
        let read = |name: &str| fs::read_to_string(entry.path().join(name)).unwrap_or_default();
        let name = read("info")
            .lines()
            .find_map(|line| line.strip_prefix("id: "))
            .map(|id| id.trim_end_matches(" (*)").trim().to_owned())
            .unwrap_or_default();
        let mut facts = Facts::new();
        if read("sub0/status").starts_with("state: RUNNING") {
            fact(
                &mut facts,
                if playing { "playing" } else { "recording" },
                running(&read("sub0/hw_params")),
            );
        }
        fact(
            &mut facts,
            "plugged in",
            jack(&said.jacks, device, &name, playing),
        );
        facts.extend(chip(said, parts, device, &name, playing));
        let direction = if playing { "out" } else { "in" };
        found.push((format!("{id},{device}:{direction}"), facts));
    }
    found
}

/// Opens a card's control device to read and hear its jacks, asking it to
/// say when a value changes.
fn open_control(dev: &Path, number: &str) -> Option<File> {
    let control = fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NONBLOCK)
        .open(dev.join(format!("snd/controlC{number}")))
        .ok()?;
    let mut on = [0u8; 4];
    on.copy_from_slice(&1i32.to_ne_bytes());
    ask(&control, SNDRV_CTL_IOCTL_SUBSCRIBE_EVENTS, &mut on).then_some(control)
}

/// Whether a control device's events say a jack changed: each is 72
/// bytes, its element's id at 8 and so its name at 24.
fn a_jack_changed(events: &[u8]) -> bool {
    events
        .as_chunks::<72>()
        .0
        .iter()
        .any(|event| c_text(&event[24..68]).ends_with(" Jack"))
}

// --- Cameras, from V4L2 and their USB descriptors ---

/// V4L2's requests, each `_IOWR('V', number, size)` as videodev2.h has it.
const VIDIOC_ENUM_FMT: u64 = 0xC040_5602;
const VIDIOC_G_FMT: u64 = 0xC0D0_5604;
const VIDIOC_G_PARM: u64 = 0xC0CC_5615;
const VIDIOC_ENUM_FRAMESIZES: u64 = 0xC02C_564A;
const VIDIOC_ENUM_FRAMEINTERVALS: u64 = 0xC034_564B;
/// uvcvideo's extension unit query, `_IOWR('u', 0x21, 16)`.
const UVCIOC_CTRL_QUERY: u64 = 0xC010_7521;
const CAPTURE: u32 = 1;
const UVC_GET_CUR: u8 = 0x81;
/// The Logitech Brio's own extension unit, as its descriptor gives the GUID
/// 49e40215-f434-47fe-b158-0e885023e51b; selector 5 is its field of view.
const BRIO_UNIT: [u8; 16] = [
    0x15, 0x02, 0xE4, 0x49, 0x34, 0xF4, 0xFE, 0x47, 0xB1, 0x58, 0x0E, 0x88, 0x50, 0x23, 0xE5, 0x1B,
];

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Asks a V4L2 or UVC question of an open node, saying whether it answered.
fn ask(node: &File, request: u64, buffer: &mut [u8]) -> bool {
    // SAFETY: each request is given a buffer of the size its number says.
    unsafe { ioctl(node.as_raw_fd(), request, buffer.as_mut_ptr()) == 0 }
}

/// A pixel format's name, from its four letters.
fn format_name(fourcc: u32) -> String {
    let letters: String = fourcc
        .to_le_bytes()
        .iter()
        .map(|byte| char::from(*byte))
        .collect();
    match letters.trim() {
        "MJPG" => "MJPEG".to_owned(),
        "H264" => "H.264".to_owned(),
        "HEVC" | "H265" => "H.265".to_owned(),
        other => other.to_owned(),
    }
}

/// A mode, as its width, height and frames a second.
type Mode = (u32, u32, u32);

fn mode(width: u32, height: u32, rate: u32) -> String {
    format!("{width} \u{d7} {height} at {rate}")
}

/// A format's largest mode and its fastest, or the one when they're one.
fn summary(modes: &[Mode]) -> String {
    let pixels = |(width, height, _): &Mode| u64::from(*width) * u64::from(*height);
    let Some(largest) = modes.iter().max_by_key(|each| (pixels(each), each.2)) else {
        return String::new();
    };
    let fastest = modes
        .iter()
        .max_by_key(|each| (each.2, pixels(each)))
        .unwrap_or(largest);
    if fastest.2 == largest.2 {
        mode(largest.0, largest.1, largest.2)
    } else {
        format!(
            "{}; {}",
            mode(largest.0, largest.1, largest.2),
            mode(fastest.0, fastest.1, fastest.2)
        )
    }
}

/// Frames a second from a frame interval, rounded.
fn rate(numerator: u32, denominator: u32) -> u32 {
    denominator
        .saturating_add(numerator / 2)
        .checked_div(numerator)
        .unwrap_or(0)
}

/// The fastest a node offers a format at a size.
fn fastest(node: &File, format: u32, width: u32, height: u32) -> u32 {
    let mut best = 0;
    for index in 0..64u32 {
        let mut asked = [0u8; 52];
        for (at, value) in [(0, index), (4, format), (8, width), (12, height)] {
            asked[at..at + 4].copy_from_slice(&value.to_ne_bytes());
        }
        if !ask(node, VIDIOC_ENUM_FRAMEINTERVALS, &mut asked) {
            break;
        }
        // A discrete interval, or a range's shortest.
        best = best.max(rate(word(&asked, 20), word(&asked, 24)));
        if word(&asked, 16) != 1 {
            break;
        }
    }
    best
}

/// Every format a node offers, with each of its modes.
fn formats(node: &File) -> Vec<(u32, Vec<Mode>)> {
    let mut found = Vec::new();
    for index in 0..32u32 {
        let mut asked = [0u8; 64];
        asked[..4].copy_from_slice(&index.to_ne_bytes());
        asked[4..8].copy_from_slice(&CAPTURE.to_ne_bytes());
        if !ask(node, VIDIOC_ENUM_FMT, &mut asked) {
            break;
        }
        let format = word(&asked, 44);
        let mut modes = Vec::new();
        for size in 0..128u32 {
            let mut sizes = [0u8; 44];
            sizes[..4].copy_from_slice(&size.to_ne_bytes());
            sizes[4..8].copy_from_slice(&format.to_ne_bytes());
            if !ask(node, VIDIOC_ENUM_FRAMESIZES, &mut sizes) {
                break;
            }
            // A discrete size, or a range's largest.
            let discrete = word(&sizes, 8) == 1;
            let (width, height) = if discrete {
                (word(&sizes, 12), word(&sizes, 16))
            } else {
                (word(&sizes, 16), word(&sizes, 28))
            };
            modes.push((width, height, fastest(node, format, width, height)));
            if !discrete {
                break;
            }
        }
        found.push((format, modes));
    }
    found
}

/// What a node streams now: its size, format and rate.
fn now_streaming(node: &File) -> Option<String> {
    let mut format = [0u8; 208];
    format[..4].copy_from_slice(&CAPTURE.to_ne_bytes());
    let mut parameters = [0u8; 204];
    parameters[..4].copy_from_slice(&CAPTURE.to_ne_bytes());
    if !ask(node, VIDIOC_G_FMT, &mut format) || !ask(node, VIDIOC_G_PARM, &mut parameters) {
        return None;
    }
    Some(format!(
        "{} {}",
        format_name(word(&format, 16)),
        mode(
            word(&format, 8),
            word(&format, 12),
            rate(word(&parameters, 12), word(&parameters, 16))
        )
    ))
}

/// The id of the extension unit a camera's descriptors give this GUID.
fn unit(descriptors: &[u8], guid: &[u8; 16]) -> Option<u8> {
    let mut rest = descriptors;
    while let [length, ..] = rest {
        let length = usize::from(*length);
        if length < 2 || length > rest.len() {
            return None;
        }
        let (descriptor, after) = rest.split_at(length);
        // A class-specific interface descriptor that is an extension unit.
        if descriptor.len() >= 20
            && descriptor[1] == 0x24
            && descriptor[2] == 0x06
            && &descriptor[4..20] == guid
        {
            return Some(descriptor[3]);
        }
        rest = after;
    }
    None
}

/// A Brio's field of view, as its own unit has it set.
fn field_of_view(node: &File, unit: u8) -> Option<&'static str> {
    let mut value = [0u8; 1];
    let mut query = [0u8; 16];
    query[..3].copy_from_slice(&[unit, 5, UVC_GET_CUR]);
    query[4..6].copy_from_slice(&1u16.to_ne_bytes());
    query[8..].copy_from_slice(&(value.as_mut_ptr() as usize as u64).to_ne_bytes());
    if !ask(node, UVCIOC_CTRL_QUERY, &mut query) {
        return None;
    }
    match value[0] {
        0 => Some("90\u{b0}"),
        1 => Some("78\u{b0}"),
        2 => Some("65\u{b0}"),
        _ => None,
    }
}

/// A USB speed in Mbit/s, as it's said.
fn speed(megabits: &str) -> String {
    match megabits.trim().parse::<u32>() {
        Ok(fast) if fast >= 1000 && fast % 1000 == 0 => format!("{} Gbit/s", fast / 1000),
        Ok(speed) => format!("{speed} Mbit/s"),
        Err(_) => String::new(),
    }
}

/// A camera's name as its holders file is named: trimmed, one plain file
/// name (modules/v4l2loopback/holders' `camera_file`).
fn holders_name(name: &str) -> String {
    let called: String = name
        .trim()
        .chars()
        .map(|c| if c.is_control() { '?' } else { c })
        .collect();
    called.replace('/', "-").trim_start_matches('.').to_owned()
}

/// A loopback camera a relay fills from a sensor that isn't on USB: its
/// holders file's name, the loopback's own name, and the sensor's I2C
/// device as /sys/bus/i2c/devices names it, where it's known.
#[derive(Debug, PartialEq, Eq)]
struct Loopback {
    name: String,
    card: String,
    sensor: String,
}

/// The loopbacks a file lists, a line each: the holders file's name, the
/// loopback's name and the sensor, apart by tabs.
fn loopbacks(text: &str) -> Vec<Loopback> {
    text.lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let (name, card) = (fields.next()?, fields.next()?);
            Some(Loopback {
                name: name.to_owned(),
                card: card.to_owned(),
                sensor: fields.next().unwrap_or_default().to_owned(),
            })
        })
        .filter(|loopback| !loopback.name.is_empty() && !loopback.card.is_empty())
        .collect()
}

/// A loopback's format and mode, as v4l2loopback's `format` says them
/// ("NV12:1920x1080@300000/10000" is NV12 at 1920 \u{d7} 1080 and 30).
fn looped_mode(format: &str) -> Option<(String, String)> {
    let (fourcc, rest) = format.trim().split_once(':')?;
    let (size, rate) = rest.split_once('@')?;
    let (width, height) = size.split_once('x')?;
    let (frames, seconds) = rate.split_once('/').unwrap_or((rate, "1"));
    let number = |text: &str| text.parse::<u32>().ok();
    let rate = self::rate(number(seconds)?, number(frames)?);
    Some((
        format_name(u32::from_le_bytes(fourcc.as_bytes().try_into().ok()?)),
        mode(number(width)?, number(height)?, rate),
    ))
}

/// A sensor's maker and model: its maker from its ACPI id's first four
/// letters, its model from the name its driver gives its subdevice
/// ("ov08x40 17-0036").
fn sensor_name(sensor: &Path) -> String {
    let id = sensor
        .file_name()
        .map(|name| name.to_string_lossy().trim_start_matches("i2c-").to_owned())
        .unwrap_or_default();
    let maker = match id.get(..4) {
        Some("OVTI") => "OmniVision",
        Some("HIMX") => "Himax",
        Some("SONY") => "Sony",
        Some("GCTI") => "GalaxyCore",
        _ => "",
    };
    let model = fs::read_dir(sensor.join("video4linux"))
        .ok()
        .and_then(|mut entries| entries.next()?.ok())
        .and_then(|entry| fs::read_to_string(entry.path().join("name")).ok())
        .and_then(|name| Some(name.split(' ').next()?.trim().to_uppercase()))
        .unwrap_or_default();
    [maker, model.as_str()]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// A loopback camera's facts, from its node in `folder` and its sensor in
/// `sys`, none of which opens it: what it streams while it's held, its one
/// format and mode, its sensor, which way the sensor faces, and whether
/// it's powered.
fn looped(folder: &Path, sys: &Path, loopback: &Loopback, held: bool) -> Facts {
    let mut facts = Facts::new();
    let said = fs::read_to_string(folder.join("format")).unwrap_or_default();
    if let Some((format, mode)) = looped_mode(&said) {
        if held {
            fact(&mut facts, "streaming", format!("{format} {mode}"));
        }
        fact(&mut facts, &format, mode);
    }
    if loopback.sensor.is_empty() {
        return facts;
    }
    let sensor = sys.join("bus/i2c/devices").join(&loopback.sensor);
    let read = |name: &str| {
        fs::read_to_string(sensor.join(name))
            .unwrap_or_default()
            .trim()
            .to_owned()
    };
    fact(&mut facts, "sensor", sensor_name(&sensor));
    let facing = match read("physical_location/panel").as_str() {
        "front" => "you",
        "back" => "away from you",
        _ => "",
    };
    fact(&mut facts, "facing", facing);
    let power = match read("power/runtime_status").as_str() {
        "active" => "on",
        "suspended" => "off",
        _ => "",
    };
    fact(&mut facts, "power", power);
    facts
}

/// Every camera's facts, by its holders file's name, from the video nodes
/// `sys` lists, their nodes in `dev` and their holders in `holders`: a USB
/// camera's from the camera, and a loopback's that `looped` lists from its
/// sysfs and its sensor's.
fn cameras(
    sys: &Path,
    dev: &Path,
    holders: &Path,
    looped_from: &[Loopback],
) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("class/video4linux")) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let folder = entry.path();
        let read = |name: &str| fs::read_to_string(folder.join(name)).unwrap_or_default();
        // A UVC camera's first node is the one it streams pictures through.
        let usb = folder.join("device/..");
        if read("index").trim() != "0" {
            continue;
        }
        if !usb.join("idVendor").exists() {
            let card = read("name");
            if let Some(loopback) = looped_from.iter().find(|each| each.card == card.trim()) {
                let held = fs::read_to_string(holders.join(&loopback.name)).unwrap_or_default();
                let facts = looped(&folder, sys, loopback, !held.trim().is_empty());
                found.push((loopback.name.clone(), facts));
            }
            continue;
        }
        let name = holders_name(&read("name"));
        let Ok(node) = File::open(dev.join(entry.file_name())) else {
            continue;
        };
        let mut facts = Facts::new();
        let held = fs::read_to_string(holders.join(&name)).unwrap_or_default();
        if !held.trim().is_empty() {
            fact(
                &mut facts,
                "streaming",
                now_streaming(&node).unwrap_or_default(),
            );
        }
        for (format, modes) in formats(&node) {
            let said = summary(&modes);
            fact(&mut facts, &format_name(format), said);
        }
        let descriptors = fs::read(usb.join("descriptors")).unwrap_or_default();
        if let Some(view) = unit(&descriptors, &BRIO_UNIT).and_then(|id| field_of_view(&node, id)) {
            fact(&mut facts, "field of view", view);
        }
        let at = |name: &str| fs::read_to_string(usb.join(name)).unwrap_or_default();
        fact(&mut facts, "USB", speed(&at("speed")));
        let firmware = at("bcdDevice");
        if !firmware.trim().is_empty() {
            fact(
                &mut facts,
                "firmware",
                format!("release {}", release(&firmware)),
            );
        }
        found.push((name, facts));
    }
    found.sort();
    found
}

// --- Writing ---

fn quoted(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if u32::from(c) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Devices and their facts as the launcher reads them.
fn json(devices: &[(String, Facts)]) -> String {
    let mut out = String::from("{");
    for (at, (key, facts)) in devices.iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let _ = write!(out, "\n  {}: [", quoted(key));
        for (each, (name, value)) in facts.iter().enumerate() {
            if each > 0 {
                out.push(',');
            }
            let _ = write!(
                out,
                "\n    {{\"name\": {}, \"value\": {}}}",
                quoted(name),
                quoted(value)
            );
        }
        out.push_str("\n  ]");
    }
    out.push_str("\n}\n");
    out
}

/// Writes a file whole, through a copy renamed over it, unless it already
/// says this.
fn write(path: &Path, text: &str) -> io::Result<()> {
    if fs::read_to_string(path).is_ok_and(|said| said == text) {
        return Ok(());
    }
    let copy = path.with_extension("json.new");
    fs::write(&copy, text)?;
    fs::rename(&copy, path)
}

// --- Bluetooth input devices, from the kernel's HID and input layers ---

/// The Bluetooth bus, as the HID layer numbers it.
const BLUETOOTH_BUS: &str = "0005";
/// The kernel's mouse buttons, `BTN_LEFT` to `BTN_TASK`.
const MOUSE_BUTTONS: std::ops::RangeInclusive<usize> = 0x110..=0x117;
/// A keyboard's keys, `KEY_ESC` to the last of the first page's.
const KEYBOARD_KEYS: std::ops::RangeInclusive<usize> = 0x01..=0xFF;
/// Fewer keys than this are a mouse's or a remote's, not a keyboard's.
const FEWEST_KEYS: usize = 20;
/// The wheels, as relative axes.
const REL_HWHEEL: usize = 0x06;
const REL_WHEEL: usize = 0x08;

/// The bits a capabilities file sets: hex words, the highest first.
fn set_bits(text: &str) -> Vec<usize> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut set = Vec::new();
    for (place, word) in words.iter().rev().enumerate() {
        let Ok(value) = u64::from_str_radix(word, 16) else {
            return Vec::new();
        };
        set.extend(
            (0..64)
                .filter(|bit| value >> bit & 1 == 1)
                .map(|bit| place * 64 + bit),
        );
    }
    set
}

/// What one Bluetooth device's HID devices and their inputs have said.
#[derive(Default)]
struct Heard {
    link: String,
    id: String,
    release: String,
    keys: Vec<usize>,
    axes: Vec<usize>,
}

impl Heard {
    fn facts(mut self) -> Facts {
        self.keys.sort_unstable();
        self.keys.dedup();
        let buttons = self
            .keys
            .iter()
            .filter(|key| MOUSE_BUTTONS.contains(key))
            .count();
        let keys = self
            .keys
            .iter()
            .filter(|key| KEYBOARD_KEYS.contains(key))
            .count();
        let wheel = match (
            self.axes.contains(&REL_WHEEL),
            self.axes.contains(&REL_HWHEEL),
        ) {
            (true, true) => "vertical and horizontal",
            (true, false) => "vertical",
            (false, true) => "horizontal",
            (false, false) => "",
        };
        let mut facts = Facts::new();
        fact(&mut facts, "connection", self.link);
        if buttons > 0 {
            fact(&mut facts, "buttons", buttons.to_string());
        }
        fact(&mut facts, "wheel", wheel);
        if keys >= FEWEST_KEYS {
            fact(&mut facts, "keys", keys.to_string());
        }
        fact(&mut facts, "id", self.id);
        fact(&mut facts, "release", self.release);
        facts
    }
}

/// A HID device on the Bluetooth bus: its address, as bluetoothd writes it,
/// and how it's carried, its ids, and what its inputs take. One that
/// bluetoothd made through uhid is HID over GATT, on LE; the kernel's own
/// HIDP is Bluetooth Classic's.
fn hid_device(hid: &Path) -> Option<(String, Heard)> {
    let uevent = fs::read_to_string(hid.join("uevent")).ok()?;
    let field = |name: &str| {
        uevent
            .lines()
            .find_map(|line| line.strip_prefix(name)?.strip_prefix('='))
            .map(str::trim)
    };
    let address = field("HID_UNIQ")?.to_uppercase();
    // Its bus, vendor and product, as "0005:000025A7:0000FAA0".
    let mut ids = field("HID_ID")?.split(':');
    let (Some(BLUETOOTH_BUS), Some(vendor), Some(product)) = (ids.next(), ids.next(), ids.next())
    else {
        return None;
    };
    if address.is_empty() {
        return None;
    }
    let last = |id: &str| {
        id.get(id.len().saturating_sub(4)..)
            .unwrap_or(id)
            .to_lowercase()
    };
    let over_gatt = fs::canonicalize(hid)
        .is_ok_and(|real| real.components().any(|part| part.as_os_str() == "uhid"));
    let mut heard = Heard {
        link: if over_gatt {
            "LE, HID over GATT"
        } else {
            "Classic, HIDP"
        }
        .to_owned(),
        id: format!("{}:{}", last(vendor), last(product)),
        ..Heard::default()
    };
    let inputs = fs::read_dir(hid.join("input"))
        .into_iter()
        .flatten()
        .flatten();
    for input in inputs.map(|entry| entry.path()) {
        let read = |name: &str| fs::read_to_string(input.join(name)).unwrap_or_default();
        if heard.release.is_empty() {
            let version = read("id/version");
            if !version.trim().is_empty() {
                heard.release = release(&version);
            }
        }
        heard.keys.extend(set_bits(&read("capabilities/key")));
        heard.axes.extend(set_bits(&read("capabilities/rel")));
    }
    Some((address, heard))
}

/// Each Bluetooth input device connected, by its address, from its HID
/// devices: a device with several, as a keyboard with a touchpad has, is
/// told as one.
fn bluetooth(sys: &Path) -> Vec<(String, Facts)> {
    let Ok(entries) = fs::read_dir(sys.join("bus/hid/devices")) else {
        return Vec::new();
    };
    let mut devices: Vec<(String, Heard)> = Vec::new();
    for (address, heard) in entries
        .flatten()
        .filter_map(|entry| hid_device(&entry.path()))
    {
        match devices.iter_mut().find(|(each, _)| *each == address) {
            Some((_, was)) => {
                was.keys.extend(heard.keys);
                was.axes.extend(heard.axes);
                if was.release.is_empty() {
                    was.release = heard.release;
                }
            }
            None => devices.push((address, heard)),
        }
    }
    let mut found: Vec<(String, Facts)> = devices
        .into_iter()
        .map(|(address, heard)| (address, heard.facts()))
        .collect();
    found.sort();
    found
}

// --- Hearing changes ---

/// The kernel's uevents, as a socket.
fn uevents() -> io::Result<File> {
    // SAFETY: plain calls; the address is a valid sockaddr_nl of its size.
    unsafe {
        let fd = socket(
            AF_NETLINK,
            SOCK_DGRAM | SOCK_NONBLOCK | SOCK_CLOEXEC,
            NETLINK_KOBJECT_UEVENT,
        );
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let file = File::from_raw_fd(fd);
        let address = SockaddrNl {
            family: u16::try_from(AF_NETLINK).unwrap_or(16),
            pad: 0,
            pid: 0,
            groups: 1,
        };
        let length = u32::try_from(size_of::<SockaddrNl>()).unwrap_or(12);
        if bind(fd, (&raw const address).cast(), length) != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(file)
    }
}

/// The kinds of fact a uevent changes: displays, sound cards, cameras, or
/// none.
fn matters(uevent: &[u8]) -> u8 {
    uevent
        .split(|byte| *byte == 0)
        .filter_map(|field| HEARD.iter().find(|(said, _)| *said == field))
        .fold(0, |kinds, (_, kind)| kinds | kind)
}

/// inotify, watching each of these folders for what the mask names. A
/// folder that isn't there yet, as /dev/snd is on a machine with no sound
/// card, is heard of by its uevent instead.
fn watches(watched: &[(PathBuf, u32)]) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    for (path, mask) in watched {
        let mask = *mask;
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        unsafe { inotify_add_watch(fd, path.as_ptr(), mask) };
    }
    Ok(file)
}

/// When sound is read again after a sound device is heard, from then. A
/// stream's parameters are set, and it starts running, a moment after its
/// device opens, and nothing is heard when it does, so the first read
/// finds it not yet running.
const SETTLING: [Duration; 4] = [
    Duration::from_millis(250),
    Duration::from_millis(500),
    Duration::from_secs(1),
    Duration::from_secs(2),
];

/// How long poll waits: until the next read while sound settles, and
/// otherwise until something is heard.
fn waiting_for(settling: Option<(Instant, usize)>, now: Instant) -> c_int {
    settling
        .and_then(|(since, step)| SETTLING.get(step).map(|after| since + *after))
        .map_or(-1, |at| {
            let left = at.saturating_duration_since(now).as_millis();
            c_int::try_from(left).unwrap_or(c_int::MAX)
        })
}

/// The read after this one while sound settles, if there's one left.
fn settled_once(settling: Option<(Instant, usize)>) -> Option<(Instant, usize)> {
    let (since, step) = settling?;
    (step + 1 < SETTLING.len()).then_some((since, step + 1))
}

/// Whether inotify's events name a sound device or the lighting's file, or
/// lost some that might have.
fn names_one(events: &[u8], lighting: &str) -> bool {
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let (mask, length) = (word(4), word(12) as usize);
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or(name);
        if mask & IN_Q_OVERFLOW != 0
            || name.starts_with(b"pcm")
            || name.starts_with(b"control")
            || name == lighting.as_bytes()
        {
            return true;
        }
        rest = after;
    }
    false
}

/// A descriptor that becomes readable when SIGTERM or SIGINT arrives,
/// which are blocked so they wait for it.
fn stopping() -> io::Result<File> {
    let mut set = SigSet([0; 16]);
    // SAFETY: plain calls on a set of our own.
    let fd = unsafe {
        sigemptyset(&raw mut set);
        sigaddset(&raw mut set, SIGTERM);
        sigaddset(&raw mut set, SIGINT);
        if sigprocmask(SIG_BLOCK, &raw const set, std::ptr::null_mut()) != 0 {
            return Err(io::Error::last_os_error());
        }
        signalfd(-1, &raw const set, SFD_CLOEXEC)
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Reads everything waiting on a descriptor, saying which kinds of fact
/// `heard` found it to change.
fn heard(file: &mut File, heard: &dyn Fn(&[u8]) -> u8) -> u8 {
    let mut kinds = 0;
    let mut buffer = [0; 8192];
    while let Ok(read) = file.read(&mut buffer) {
        if read == 0 {
            break;
        }
        kinds |= heard(&buffer[..read]);
    }
    kinds
}

/// What `run` hears and reads from: the files it writes, the Pebble's
/// lighting file, the loopback cameras it's told of, and the control
/// device of each card not on USB, by the card's number.
struct Desk {
    folder: PathBuf,
    files: [PathBuf; 4],
    lighting: PathBuf,
    loopbacks: Vec<Loopback>,
    controls: Vec<(String, File)>,
}

impl Desk {
    fn new(folder: &Path, lighting: PathBuf, loopbacks: Vec<Loopback>) -> Desk {
        Desk {
            folder: folder.to_owned(),
            files: [
                folder.join("displays.json"),
                folder.join("sound.json"),
                folder.join("cameras.json"),
                folder.join("bluetooth.json"),
            ],
            lighting,
            loopbacks,
            controls: Vec::new(),
        }
    }

    /// What the control devices polled in `waiting`, in the order they're
    /// kept, say: sound, when a jack changed. One that has gone is polled
    /// no more, and let go of when sound is read again.
    fn heard_from_controls(&mut self, waiting: &mut [PollFd]) -> u8 {
        let mut due = 0;
        for (watched, (_, control)) in waiting.iter_mut().zip(&mut self.controls) {
            if watched.revents & (POLLERR | POLLHUP) != 0 {
                watched.fd = -1;
                due |= SOUND;
            } else if watched.revents & POLLIN != 0 {
                due |= heard(control, &|events| {
                    if a_jack_changed(events) { SOUND } else { 0 }
                });
            }
        }
        due
    }

    /// Opens the control device of each card not on USB that has come,
    /// and lets go of each whose card has gone, so a card that goes holds
    /// nothing of it open.
    fn follow_controls(&mut self) {
        let asound = Path::new("/proc/asound");
        let here = |number: &str| {
            let card = asound.join(format!("card{number}"));
            card.exists() && !card.join("usbid").exists()
        };
        self.controls.retain(|(number, _)| here(number));
        let Ok(entries) = fs::read_dir(asound) else {
            return;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(number) = name.strip_prefix("card") else {
                continue;
            };
            if here(number)
                && !self.controls.iter().any(|(each, _)| each == number)
                && let Some(control) = open_control(Path::new("/dev"), number)
            {
                self.controls.push((number.to_owned(), control));
            }
        }
    }
}

/// Writes the files of the kinds that are due: displays, sound, cameras
/// and Bluetooth.
fn rewrite(due: u8, desk: &mut Desk) {
    let (sys, dev) = (Path::new("/sys"), Path::new("/dev"));
    let mut written = Ok(());
    if due & DISPLAYS != 0 {
        written = written.and(write(&desk.files[0], &json(&displays(sys, &desk.folder))));
    }
    if due & SOUND != 0 {
        desk.follow_controls();
        let said = fs::read_to_string(&desk.lighting).ok();
        let lit = said.as_deref().map(str::trim);
        let controls = &desk.controls;
        let of = |number: &str| {
            controls
                .iter()
                .find(|(each, _)| each == number)
                .map(|(_, file)| control(file))
                .unwrap_or_default()
        };
        written = written.and(write(
            &desk.files[1],
            &json(&sound(Path::new("/proc/asound"), sys, lit, &of)),
        ));
    }
    if due & CAMERAS != 0 {
        written = written.and(write(
            &desk.files[2],
            &json(&cameras(sys, dev, Path::new(HOLDERS), &desk.loopbacks)),
        ));
    }
    if due & BLUETOOTH != 0 {
        written = written.and(write(&desk.files[3], &json(&bluetooth(sys))));
    }
    if let Err(error) = written {
        eprintln!("<4>can't write the desk's facts: {error}");
    }
}

fn run(folder: &Path, loopbacks: Vec<Loopback>) -> ExitCode {
    let lighting = folder
        .parent()
        .map_or_else(PathBuf::new, |runtime| runtime.join("pebble-lights"));
    let mut sounds = vec![(
        PathBuf::from("/dev/snd"),
        IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE | IN_CREATE | IN_DELETE,
    )];
    if let Some(runtime) = lighting.parent() {
        sounds.push((runtime.to_owned(), IN_CLOSE_WRITE | IN_MOVED_TO));
    }
    let holders = vec![(
        PathBuf::from(HOLDERS),
        IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE | IN_DELETE,
    )];
    // Made before it's watched for the monitors' facts.
    if let Err(error) = fs::create_dir_all(folder) {
        eprintln!("<3>can't make {}: {error}", folder.display());
        return ExitCode::FAILURE;
    }
    let monitors = vec![(folder.to_owned(), IN_MOVED_TO | IN_DELETE)];
    let (mut stop, mut kernel, mut changes, mut held, mut said) = match (
        stopping(),
        uevents(),
        watches(&sounds),
        watches(&holders),
        watches(&monitors),
    ) {
        (Ok(stop), Ok(kernel), Ok(changes), Ok(held), Ok(said)) => {
            (stop, kernel, changes, held, said)
        }
        (Err(error), ..)
        | (_, Err(error), ..)
        | (_, _, Err(error), ..)
        | (_, _, _, Err(error), _)
        | (.., Err(error)) => {
            eprintln!("<3>can't hear the desk's devices: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut desk = Desk::new(folder, lighting, loopbacks);
    let mut due = DISPLAYS | SOUND | CAMERAS | BLUETOOTH;
    let mut settling = None;
    // What's read again while it settles: sound, and cameras, whose sensor
    // powers up a moment after a program takes the loopback.
    let mut unsettled = 0;
    loop {
        rewrite(due, &mut desk);
        let mut waiting: Vec<PollFd> = [
            stop.as_raw_fd(),
            kernel.as_raw_fd(),
            changes.as_raw_fd(),
            held.as_raw_fd(),
            said.as_raw_fd(),
        ]
        .into_iter()
        .chain(desk.controls.iter().map(|(_, file)| file.as_raw_fd()))
        .map(PollFd::reading)
        .collect();
        due = 0;
        while due == 0 {
            let wait = waiting_for(settling, Instant::now());
            let count = waiting.len() as u64;
            // SAFETY: valid pollfds for the length given.
            let ready = unsafe { poll(waiting.as_mut_ptr(), count, wait) };
            if ready < 0 {
                continue;
            }
            if ready == 0 {
                settling = settled_once(settling);
                due |= unsettled;
                if settling.is_none() {
                    unsettled = 0;
                }
                continue;
            }
            if waiting[0].revents & POLLIN != 0 {
                let _ = stop.read(&mut [0; 128]);
                for file in &desk.files {
                    let _ = fs::remove_file(file);
                }
                return ExitCode::SUCCESS;
            }
            due |= desk.heard_from_controls(&mut waiting[5..]);
            // A card's devices, a display's EDID and a stream's parameters
            // settle a moment after the first word of them.
            std::thread::sleep(Duration::from_millis(150));
            due |= heard(&mut kernel, &matters);
            due |= heard(&mut changes, &|events| {
                if names_one(events, "pebble-lights") {
                    SOUND
                } else {
                    0
                }
            });
            due |= heard(&mut held, &|events| {
                if events.is_empty() { 0 } else { CAMERAS }
            });
            due |= heard(&mut said, &|events| {
                if names_a_monitor(events) { DISPLAYS } else { 0 }
            });
            if due & (SOUND | CAMERAS) != 0 {
                settling = Some((Instant::now(), 0));
                unsettled |= due & (SOUND | CAMERAS);
            }
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match &arguments[..] {
        [folder] => run(Path::new(folder), Vec::new()),
        [folder, listed] => match fs::read_to_string(listed) {
            Ok(text) => run(Path::new(folder), loopbacks(&text)),
            Err(error) => {
                eprintln!("<3>can't read {listed}: {error}");
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("usage: desk-facts FOLDER [LOOPBACKS]");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The HP Z27k G3's EDID, as it gave it on 2026-10-24.
    const Z27K_G3: &str = "\
        00ffffffffffff00220ed336000000001c1f0104b53c22783eee95a3544c99260f5054a10800d1c0a9c0a940d100b300\
        9500810081804dd000a0f0703e803020350055502100001a000000fd001d3c1c873c010a202020202020000000fc0048\
        50205a32376b2047330a2020000000ff00434e34313238324743310a202001c8020313b14b61100403021f1312115f01\
        e2006ba36600a0f0701f803020350055502100001a023a801871382d40582c450055502100001e283c80a070b0234030\
        20360055502100001a565e00a0a0a029503020350055502100001a000000000000000000000000000000000000000000\
        000000000000000000000000000000ed";

    fn bytes(hex: &str) -> Vec<u8> {
        let hex: Vec<u8> = hex.bytes().filter(u8::is_ascii_hexdigit).collect();
        hex.chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn named<'a>(facts: &'a Facts, name: &str) -> Option<&'a str> {
        facts
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, value)| value.as_str())
    }

    #[test]
    fn the_z27k_g3_says_who_made_it_and_what_it_is() {
        let edid = bytes(Z27K_G3);
        assert_eq!(edid.len(), 256);
        let facts = display(&edid).unwrap();
        assert_eq!(named(&facts, "maker"), Some("HP"));
        assert_eq!(named(&facts, "model"), Some("HP Z27k G3"));
        assert_eq!(named(&facts, "serial"), Some("CN41282GC1"));
        assert_eq!(named(&facts, "made"), Some("week 28, 2021"));
        assert_eq!(named(&facts, "size"), Some("60 \u{d7} 34 cm, 27.2 in"));
        assert_eq!(named(&facts, "native"), Some("3840 \u{d7} 2160 at 60 Hz"));
        assert_eq!(named(&facts, "refresh"), Some("29 to 60 Hz"));
        assert_eq!(named(&facts, "depth"), Some("10 bit"));
        assert_eq!(named(&facts, "input"), Some("DisplayPort"));
        assert_eq!(named(&facts, "colours"), Some("sRGB's primaries"));
        assert_eq!(named(&facts, "HDR"), Some("none"));
        assert_eq!(named(&facts, "EDID"), Some("1.4"));
    }

    #[test]
    fn an_input_says_its_depth_apart_from_its_link() {
        assert_eq!(input(0xB5), ("10 bit", "DisplayPort"));
        assert_eq!(input(0x80), ("", ""));
        assert_eq!(input(0x0F), ("", "analogue"));
    }

    #[test]
    fn wider_colours_are_said_against_srgb_s_area() {
        let mut edid = bytes(Z27K_G3);
        // DCI-P3's primaries in 1024ths, about 1.36 times sRGB's area:
        // red 0.680, 0.320; green 0.265, 0.690; blue 0.150, 0.060.
        for (at, value) in [
            (27, 696u16),
            (28, 328),
            (29, 271),
            (30, 707),
            (31, 154),
            (32, 61),
        ] {
            edid[at] = u8::try_from(value >> 2).unwrap();
        }
        edid[25] = 0;
        edid[26] &= 0x0f;
        let said = gamut(&edid);
        assert!(said.ends_with("sRGB's area"), "{said}");
        assert!(said.starts_with("1.3"), "{said}");
    }

    #[test]
    fn a_display_that_names_nothing_is_named_by_its_product_code() {
        let mut edid = bytes(Z27K_G3);
        // The name descriptor, the third, made a dummy one, the sum kept.
        let at = 54 + 2 * 18 + 3;
        assert_eq!(edid[at], 0xFC);
        edid[at] = 0x10;
        edid[127] = edid[127].wrapping_add(0xFC - 0x10);
        let facts = display(&edid).unwrap();
        assert_eq!(named(&facts, "model"), Some("product 36D3"));
    }

    #[test]
    fn what_isn_t_a_whole_edid_says_nothing() {
        let mut edid = bytes(Z27K_G3);
        edid[127] ^= 1;
        assert!(display(&edid).is_none());
        assert!(display(&[0; 64]).is_none());
    }

    #[test]
    fn an_edid_maker_code_is_three_letters() {
        assert_eq!(maker(0x22, 0x0E), "HPN");
        assert_eq!(maker(0x30, 0xE4), "LGD");
    }

    #[test]
    fn hdr_is_found_in_its_extended_block() {
        // A CTA block with one HDR static metadata block taking HDR10 and HLG.
        let mut extension = vec![0x02, 0x03, 0x08, 0x00, 0xE3, 0x06, 0x0C, 0x01];
        extension.resize(128, 0);
        assert_eq!(hdr(&extension).as_deref(), Some("HDR10 and HLG"));
        assert_eq!(hdr(&[0x70, 0x00]), None);
    }

    /// The MV7's stream file while it records, as the kernel wrote it.
    const MV7: &str = "Shure Inc Shure MV7 at usb-0000:00:14.0-1.1.4, full speed : USB Audio

Playback:
  Status: Stop
  Interface 2
    Altset 1
    Format: S24_3LE
    Channels: 2
    Endpoint: 0x01 (1 OUT) (ADAPTIVE)
    Rates: 44100, 48000

Capture:
  Status: Running
    Interface 1
    Altset 1
    Packet Size = 150
    Momentary freq = 48000 Hz (0x30.0000)
  Interface 1
    Altset 1
    Format: S24_3LE
    Channels: 1
    Endpoint: 0x82 (2 IN) (ASYNC)
    Rates: 44100, 48000
";

    #[test]
    fn a_usb_card_s_streams_say_what_they_offer_and_how_they_run() {
        let (speed, playback, capture) = streams(MV7);
        assert_eq!(speed, "full speed");
        assert!(!playback.running);
        assert_eq!(playback.clock, "adaptive");
        assert_eq!(
            offered(&playback.settings),
            "44.1 kHz or 48 kHz, 24 bit, stereo"
        );
        assert!(capture.running);
        assert_eq!(capture.momentary, "48000 Hz");
        assert_eq!(capture.clock, "asynchronous");
        assert_eq!(
            offered(&capture.settings),
            "44.1 kHz or 48 kHz, 24 bit, mono"
        );
    }

    #[test]
    fn a_running_stream_says_its_format_period_and_buffer() {
        let hw = "access: MMAP_INTERLEAVED\nformat: S24_3LE\nsubformat: STD\nchannels: 1\nrate: 48000 (48000/1)\nperiod_size: 256\nbuffer_size: 16384\n";
        assert_eq!(
            running(hw),
            "48 kHz, 24 bit, period 256 (5.3 ms), buffer 16384 (341.3 ms)"
        );
        assert_eq!(milliseconds(1024, 44_100), "23.2 ms");
        assert_eq!(milliseconds(0, 48_000), "0.0 ms");
        assert_eq!(running("closed\n"), "");
    }

    #[test]
    fn a_direction_says_what_plays_first_and_only_while_it_does() {
        let (speed, playback, capture) = streams(MV7);
        let recording = direction(&capture, &speed, "release 2.03", "48 kHz, 24 bit", false);
        assert_eq!(
            recording[0],
            ("recording".to_owned(), "48 kHz, 24 bit".to_owned())
        );
        assert_eq!(named(&recording, "measured rate"), Some("48000 Hz"));
        let idle = direction(&playback, &speed, "release 2.03", "", true);
        assert_eq!(named(&idle, "playing"), None);
        assert_eq!(named(&idle, "USB"), Some("full speed"));
        assert_eq!(named(&idle, "firmware"), Some("release 2.03"));
    }

    #[test]
    fn a_usb_release_reads_as_its_device_writes_it() {
        assert_eq!(release("1310\n"), "13.10");
        assert_eq!(release("0203"), "2.03");
        assert_eq!(release("0021"), "0.21");
        assert_eq!(release("21"), "21");
    }

    #[test]
    fn json_lists_each_device_s_facts_in_order_and_escapes_them() {
        let devices = vec![(
            "DP-4".to_owned(),
            vec![
                ("maker".to_owned(), "HP".to_owned()),
                ("model".to_owned(), "a \"quoted\" \\ name".to_owned()),
            ],
        )];
        assert_eq!(
            json(&devices),
            "{\n  \"DP-4\": [\n    {\"name\": \"maker\", \"value\": \"HP\"},\n    {\"name\": \"model\", \"value\": \"a \\\"quoted\\\" \\\\ name\"}\n  ]\n}\n"
        );
        assert_eq!(json(&[]), "{\n}\n");
    }

    /// A uevent as the kernel sends it: fields split by NULs.
    fn uevent(fields: &[&str]) -> Vec<u8> {
        fields.join("\0").into_bytes()
    }

    #[test]
    fn each_uevent_reads_again_only_the_kinds_it_changes() {
        let display = uevent(&[
            "change@/devices/pci0000:00/0000:00:02.0/drm/card0",
            "ACTION=change",
            "SUBSYSTEM=drm",
            "HOTPLUG=1",
        ]);
        assert_eq!(matters(&display), DISPLAYS);
        let card = uevent(&["add@/devices/.../sound/card1", "SUBSYSTEM=sound"]);
        assert_eq!(matters(&card), SOUND);
        let camera = uevent(&[
            "add@/devices/.../video4linux/video1",
            "SUBSYSTEM=video4linux",
        ]);
        assert_eq!(matters(&camera), CAMERAS);
        let plugged = uevent(&["add@/devices/.../usb3/3-4", "SUBSYSTEM=usb"]);
        assert_eq!(matters(&plugged), SOUND | CAMERAS);
        let battery = uevent(&["change@/devices/.../BAT0", "SUBSYSTEM=power_supply"]);
        assert_eq!(matters(&battery), 0);
        let mouse = uevent(&[
            "add@/devices/virtual/misc/uhid/0005:25A7:FAA0.0001",
            "SUBSYSTEM=hid",
        ]);
        assert_eq!(matters(&mouse), BLUETOOTH);
        let input = uevent(&["add@/devices/.../input/input42", "SUBSYSTEM=input"]);
        assert_eq!(matters(&input), BLUETOOTH);
    }

    #[test]
    fn power_is_said_in_volts_amps_and_watts_plainly() {
        assert_eq!(tenths(20_000), "20");
        assert_eq!(tenths(1500), "1.5");
        assert_eq!(chosen("[usb2] usb3"), Some("usb2"));
        assert_eq!(chosen("usb2 usb3"), None);
        assert_eq!(
            offered_power(&[(5000, 3000), (9000, 3000), (20_000, 3000)]),
            "5, 9 and 20 V at 3 A"
        );
        assert_eq!(offered_power(&[(5000, 3000)]), "5 V at 3 A");
        assert_eq!(
            offered_power(&[(5000, 3000), (20_000, 2250)]),
            "5 V at 3 A, 20 V at 2.3 A"
        );
        assert_eq!(offered_power(&[]), "");
    }

    /// Writes a stand-in sysfs file, with the folders it's in.
    fn put(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// A monitor's partner on a USB-C port, as UCSI shows one, charging the
    /// machine through the port's supply when it's port1.
    fn partner(root: &Path, port: &str) {
        let partner = format!("class/typec/{port}-partner");
        put(root, &format!("{partner}/{port}-partner.0/svid"), "ff01\n");
        put(root, &format!("{partner}/{port}-partner.0/active"), "yes\n");
        put(root, &format!("{partner}/usb_mode"), "[usb2] usb3\n");
        for (index, volts) in [(1, 5000), (2, 9000), (3, 12_000), (4, 15_000), (5, 20_000)] {
            let supply = format!("{partner}/pd3/source-capabilities/{index}:fixed_supply");
            put(root, &format!("{supply}/voltage"), &format!("{volts}mV\n"));
            put(root, &format!("{supply}/maximum_current"), "5000mA\n");
        }
        fs::create_dir_all(root.join("devices/platform/USBC000:00")).unwrap();
        fs::create_dir_all(root.join(format!("class/typec/{port}"))).unwrap();
        std::os::unix::fs::symlink(
            "../../../devices/platform/USBC000:00",
            root.join(format!("class/typec/{port}/device")),
        )
        .unwrap();
        let supply = "class/power_supply/ucsi-source-psy-USBC000:002";
        put(root, &format!("{supply}/online"), "1\n");
        put(root, &format!("{supply}/voltage_now"), "20000000\n");
        put(root, &format!("{supply}/current_max"), "5000000\n");
    }

    #[test]
    fn the_monitor_s_usb_c_cable_says_its_data_its_charge_and_its_offer() {
        let sys = Sys::new("usb-c");
        partner(&sys.0, "port1");
        let facts = usb_c(&sys.0);
        let said: Vec<(&str, &str)> = facts
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(
            said,
            vec![
                ("USB-C", "DisplayPort and USB 2"),
                ("charging", "100 W (20 V, 5 A)"),
                ("power offered", "5, 9, 12, 15 and 20 V at 5 A"),
            ]
        );
    }

    #[test]
    fn two_monitors_cables_say_nothing_since_neither_is_known_to_be_whose() {
        let sys = Sys::new("usb-c-two");
        partner(&sys.0, "port0");
        partner(&sys.0, "port1");
        assert!(usb_c(&sys.0).is_empty());
    }

    #[test]
    fn sound_is_read_again_as_it_settles_and_then_only_when_heard() {
        let since = Instant::now();
        assert_eq!(
            waiting_for(None, since),
            -1,
            "nothing settling waits for events"
        );
        assert_eq!(waiting_for(Some((since, 0)), since), 250);
        assert_eq!(waiting_for(Some((since, 3)), since), 2000);
        assert_eq!(
            waiting_for(Some((since, 1)), since + Duration::from_secs(9)),
            0,
            "one overdue reads at once"
        );
        let mut settling = Some((since, 0));
        let mut reads = 0;
        while settling.is_some() {
            settling = settled_once(settling);
            reads += 1;
        }
        assert_eq!(reads, SETTLING.len());
    }

    #[test]
    fn a_capabilities_file_s_words_give_their_bits_the_highest_word_first() {
        assert_eq!(set_bits("903"), vec![0, 1, 8, 11]);
        assert_eq!(set_bits("1f0000 0 0 0 0"), vec![272, 273, 274, 275, 276]);
        assert_eq!(set_bits("0"), Vec::<usize>::new());
        assert_eq!(set_bits("not hex"), Vec::<usize>::new());
    }

    /// A stand-in sysfs, gone when the test is.
    struct Sys(PathBuf);

    impl Sys {
        fn new(name: &str) -> Sys {
            let root = std::env::temp_dir().join(format!("desk-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("bus/hid/devices")).unwrap();
            Sys(root)
        }

        /// A HID device where the kernel keeps it, linked from the bus as
        /// the kernel links it, with one input.
        fn hid(&self, home: &str, name: &str, uevent: &str, input: [&str; 3]) {
            let real = self.0.join(home).join(name);
            let [version, keys, axes] = input;
            fs::create_dir_all(real.join("input/input42/id")).unwrap();
            fs::create_dir_all(real.join("input/input42/capabilities")).unwrap();
            fs::write(real.join("uevent"), uevent).unwrap();
            fs::write(real.join("input/input42/id/version"), version).unwrap();
            fs::write(real.join("input/input42/capabilities/key"), keys).unwrap();
            fs::write(real.join("input/input42/capabilities/rel"), axes).unwrap();
            std::os::unix::fs::symlink(&real, self.0.join("bus/hid/devices").join(name)).unwrap();
        }
    }

    impl Drop for Sys {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_bluetooth_mouse_says_how_it_s_carried_and_what_it_takes() {
        let sys = Sys::new("mouse");
        sys.hid(
            "devices/virtual/misc/uhid",
            "0005:25A7:FAA0.0001",
            "DRIVER=hid-generic\nHID_ID=0005:000025A7:0000FAA0\nHID_NAME=ProtoArc EM11 NL\n\
             HID_PHYS=b4:8c:9d:00:00:01\nHID_UNIQ=c3:9c:e3:2a:f3:6e\n",
            ["0508\n", "1f0000 0 0 0 0\n", "903\n"],
        );
        let said = |pairs: &[(&str, &str)]| -> Facts {
            pairs
                .iter()
                .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
                .collect()
        };
        assert_eq!(
            bluetooth(&sys.0),
            vec![(
                "C3:9C:E3:2A:F3:6E".to_owned(),
                said(&[
                    ("connection", "LE, HID over GATT"),
                    ("buttons", "5"),
                    ("wheel", "vertical"),
                    ("id", "25a7:faa0"),
                    ("release", "5.08"),
                ])
            )]
        );
    }

    #[test]
    fn a_classic_keyboard_says_its_keys_and_a_usb_device_is_left_out() {
        let sys = Sys::new("keyboard");
        sys.hid(
            "devices/pci0000:00/0000:00:14.7/bluetooth/hci0/hci0:3",
            "0005:05AC:0255.0002",
            "HID_ID=0005:000005AC:00000255\nHID_UNIQ=aa:bb:cc:dd:ee:ff\n",
            ["0050\n", "fffffffffffffffe ffffffffffffffff\n", "0\n"],
        );
        sys.hid(
            "devices/pci0000:00/0000:00:14.0/usb3/3-1",
            "0003:3297:4976.0003",
            "HID_ID=0003:00003297:00004976\nHID_UNIQ=\n",
            ["0100\n", "fffffffffffffffe ffffffffffffffff\n", "0\n"],
        );
        let found = bluetooth(&sys.0);
        assert_eq!(found.len(), 1, "{found:?}");
        let (address, facts) = &found[0];
        assert_eq!(address, "AA:BB:CC:DD:EE:FF");
        assert_eq!(
            facts[0],
            ("connection".to_owned(), "Classic, HIDP".to_owned())
        );
        // Bits 0 to 63 and 65 to 127, of which 126 are keys from KEY_ESC.
        assert!(
            facts.contains(&("keys".to_owned(), "126".to_owned())),
            "{facts:?}"
        );
        assert!(
            !facts
                .iter()
                .any(|(name, _)| name == "buttons" || name == "wheel")
        );
    }

    #[test]
    fn a_format_says_its_largest_mode_and_its_fastest() {
        // The MX Brio 705's MJPEG modes, some of them.
        let modes = [
            (640, 480, 30),
            (1920, 1080, 60),
            (2560, 1440, 30),
            (3840, 2160, 30),
        ];
        assert_eq!(
            summary(&modes),
            "3840 \u{d7} 2160 at 30; 1920 \u{d7} 1080 at 60"
        );
        assert_eq!(summary(&[(1920, 1080, 30)]), "1920 \u{d7} 1080 at 30");
        assert_eq!(summary(&[]), "");
        assert_eq!(rate(1, 30), 30);
        assert_eq!(rate(1001, 60000), 60);
        assert_eq!(rate(0, 30), 0);
    }

    #[test]
    fn a_pixel_format_is_named_as_people_say_it() {
        assert_eq!(format_name(u32::from_le_bytes(*b"MJPG")), "MJPEG");
        assert_eq!(format_name(u32::from_le_bytes(*b"YUYV")), "YUYV");
        assert_eq!(format_name(u32::from_le_bytes(*b"NV12")), "NV12");
        assert_eq!(format_name(u32::from_le_bytes(*b"H264")), "H.264");
    }

    #[test]
    fn the_brio_s_unit_is_found_among_its_descriptors() {
        // An interface descriptor, then an extension unit with id 10.
        let mut descriptors = vec![9, 0x04, 0, 0, 0, 0x0E, 0x01, 0, 0];
        let mut unit_descriptor = vec![27, 0x24, 0x06, 10];
        unit_descriptor.extend_from_slice(&BRIO_UNIT);
        unit_descriptor.resize(27, 0);
        descriptors.extend(unit_descriptor);
        assert_eq!(unit(&descriptors, &BRIO_UNIT), Some(10));
        assert_eq!(unit(&descriptors, &[0; 16]), None);
        assert_eq!(unit(&[0, 1, 2], &BRIO_UNIT), None, "a zero length ends it");
    }

    #[test]
    fn a_usb_speed_is_said_in_its_own_unit() {
        assert_eq!(speed("5000\n"), "5 Gbit/s");
        assert_eq!(speed("480"), "480 Mbit/s");
        assert_eq!(speed("12"), "12 Mbit/s");
        assert_eq!(speed(""), "");
    }

    #[test]
    fn a_camera_is_keyed_as_its_holders_file_is_named() {
        assert_eq!(
            holders_name("MX Brio 705 for Business\n"),
            "MX Brio 705 for Business"
        );
        assert_eq!(holders_name("A/B .cam"), "A-B .cam");
        assert_eq!(holders_name(".hidden"), "hidden");
    }

    /// The jacks the XPS 14's card has, as its control device names them,
    /// a headset plugged in where asked.
    fn xps_jacks(headset: bool) -> Vec<(String, bool)> {
        [
            ("HDMI/DP,pcm=5 Jack", false),
            ("HDMI/DP,pcm=6 Jack", true),
            ("cs42l45 IT 31 Microphone Jack", false),
            ("cs42l45 IT 33 Headset Jack", headset),
            ("cs42l45 OT 43 Headphone Jack", headset),
            ("cs42l45 OT 45 Headset Jack", headset),
        ]
        .into_iter()
        .map(|(name, plugged)| (name.to_owned(), plugged))
        .collect()
    }

    #[test]
    fn a_jack_says_what_s_plugged_into_its_own_device() {
        let jacks = xps_jacks(false);
        assert_eq!(jack(&jacks, 5, "HDMI1", true), "nothing");
        assert_eq!(jack(&jacks, 6, "HDMI2", true), "a display");
        assert_eq!(jack(&jacks, 0, "Jack Out", true), "nothing");
        assert_eq!(jack(&jacks, 1, "Jack In", false), "nothing");
        assert_eq!(jack(&jacks, 2, "Speaker", true), "");
        assert_eq!(jack(&jacks, 4, "Microphone", false), "");
        let headset = xps_jacks(true);
        assert_eq!(jack(&headset, 0, "Jack Out", true), "a headset");
        assert_eq!(jack(&headset, 1, "Jack In", false), "a headset");
        let headphones = [("Headphone Jack".to_owned(), true)];
        assert_eq!(jack(&headphones, 0, "Headphones", true), "headphones");
        assert_eq!(jack(&headphones, 1, "Headset Mic", false), "");
    }

    /// The XPS 14's `SoundWire` devices: its codec, then its four amplifiers.
    fn xps_parts() -> Vec<Part> {
        let part = |driver: &str, part: &str| Part {
            driver: driver.to_owned(),
            maker: "01fa".to_owned(),
            model: part.to_owned(),
        };
        let mut parts = vec![part("sdca_class", "4245")];
        parts.extend((0..4).map(|_| part("cs35l56", "3557")));
        parts
    }

    #[test]
    fn a_device_names_its_chip_as_the_parts_themselves_say() {
        let said = Control {
            jacks: Vec::new(),
            components: "HDA:80862822,80860101,00100000  cfg-amp:4 iec61937-pcm:7,6,5 \
                         hs:cs42l45 mic:cs42l45-dmic spk:cs35l56"
                .to_owned(),
            mixer: "Intel Panther Lake HDMI".to_owned(),
        };
        let parts = xps_parts();
        let chip_of = |device, name, playing| chip(&said, &parts, device, name, playing);
        let one = |name: &str, value: &str| vec![(name.to_owned(), value.to_owned())];
        assert_eq!(
            chip_of(2, "Speaker", true),
            one("amplifiers", "4 \u{d7} Cirrus Logic CS35L57")
        );
        assert_eq!(
            chip_of(0, "Jack Out", true),
            one("codec", "Cirrus Logic CS42L45")
        );
        assert_eq!(
            chip_of(4, "Microphone", false),
            one("codec", "Cirrus Logic CS42L45, digital microphones")
        );
        assert_eq!(
            chip_of(6, "HDMI2", true),
            one("codec", "Intel Panther Lake HDMI")
        );
        assert_eq!(chips("rt1320", &parts), "RT1320");
        let realtek = Part {
            driver: "rt722".to_owned(),
            maker: "025d".to_owned(),
            model: "0722".to_owned(),
        };
        assert_eq!(part_name(&realtek), "Realtek RT722");
    }

    #[test]
    fn a_built_in_card_says_each_device_by_its_path_and_what_it_runs_at() {
        let sys = Sys::new("built-in");
        let card = sys.0.join("asound/card0");
        put(&card, "id", "sofsoundwire\n");
        put(
            &card,
            "pcm2p/info",
            "card: 0\ndevice: 2\nid: Speaker (*)\nname: \n",
        );
        put(
            &card,
            "pcm2p/sub0/status",
            "state: RUNNING\nowner_pid   : 1\n",
        );
        put(
            &card,
            "pcm2p/sub0/hw_params",
            "access: MMAP_INTERLEAVED\nformat: S32_LE\nsubformat: STD\nchannels: 2\n\
             rate: 48000 (48000/1)\nperiod_size: 1024\nbuffer_size: 32768\n",
        );
        put(&card, "pcm1c/info", "id: Jack In (*)\n");
        put(&card, "pcm1c/sub0/status", "closed\n");
        put(&card, "pcm1c/sub0/hw_params", "closed\n");
        put(&card, "codec#2", "");
        let said = Control {
            jacks: xps_jacks(true),
            components: "hs:cs42l45 spk:cs35l56".to_owned(),
            mixer: String::new(),
        };
        let mut found = built_in(&card, "sofsoundwire", &said, &xps_parts());
        found.sort();
        let pairs = |facts: &Facts| {
            facts
                .iter()
                .map(|(name, value)| format!("{name}: {value}"))
                .collect::<Vec<_>>()
        };
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, "sofsoundwire,1:in");
        assert_eq!(
            pairs(&found[0].1),
            ["plugged in: a headset", "codec: Cirrus Logic CS42L45"]
        );
        assert_eq!(found[1].0, "sofsoundwire,2:out");
        assert_eq!(
            pairs(&found[1].1),
            [
                "playing: 48 kHz, 32 bit, period 1024 (21.3 ms), buffer 32768 (682.7 ms)",
                "amplifiers: 4 \u{d7} Cirrus Logic CS35L57"
            ]
        );
        // A card on USB is left to its own reading.
        put(&sys.0, "asound/card1/id", "MV7\n");
        put(&sys.0, "asound/card1/usbid", "14ed:1012\n");
        let all = sound(&sys.0.join("asound"), &sys.0, None, &|_| Control::default());
        assert!(all.iter().all(|(key, _)| key.starts_with("sofsoundwire,")));
    }

    #[test]
    fn only_a_jack_s_change_is_heard_from_a_control_device() {
        let event = |name: &str| {
            let mut event = [0u8; 72];
            event[4..8].copy_from_slice(&1u32.to_ne_bytes());
            event[24..24 + name.len()].copy_from_slice(name.as_bytes());
            event
        };
        let volume = event("Speaker Playback Volume");
        let jack = event("cs42l45 OT 43 Headphone Jack");
        assert!(!a_jack_changed(&volume));
        assert!(a_jack_changed(&[volume, jack].concat()));
    }

    #[test]
    fn a_loopback_says_its_mode_its_sensor_and_whether_it_s_powered() {
        assert_eq!(
            loopbacks("camera\tBuilt-in Camera\ti2c-OVTI08F4:00\nir-camera\tIR Camera\n\n"),
            vec![
                Loopback {
                    name: "camera".to_owned(),
                    card: "Built-in Camera".to_owned(),
                    sensor: "i2c-OVTI08F4:00".to_owned(),
                },
                Loopback {
                    name: "ir-camera".to_owned(),
                    card: "IR Camera".to_owned(),
                    sensor: String::new(),
                },
            ]
        );
        assert_eq!(
            looped_mode("NV12:1920x1080@300000/10000\n"),
            Some(("NV12".to_owned(), "1920 \u{d7} 1080 at 30".to_owned()))
        );
        assert_eq!(
            looped_mode("GREY:648x368@30"),
            Some(("GREY".to_owned(), "648 \u{d7} 368 at 30".to_owned()))
        );
        assert_eq!(looped_mode(""), None);
        let sys = Sys::new("loopback");
        let node = sys.0.join("class/video4linux/video33");
        put(&node, "name", "Built-in Camera\n");
        put(&node, "index", "0\n");
        put(&node, "format", "NV12:1920x1080@300000/10000\n");
        let sensor = sys.0.join("bus/i2c/devices/i2c-OVTI08F4:00");
        put(&sensor, "video4linux/v4l-subdev5/name", "ov08x40 17-0036\n");
        put(&sensor, "physical_location/panel", "front\n");
        put(&sensor, "power/runtime_status", "active\n");
        let holders = sys.0.join("holders");
        put(&holders, "camera", "firefox\n");
        let listed = loopbacks("camera\tBuilt-in Camera\ti2c-OVTI08F4:00\n");
        let found = cameras(&sys.0, &sys.0.join("dev"), &holders, &listed);
        let said: Vec<(&str, &str)> = found[0]
            .1
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        assert_eq!(found[0].0, "camera");
        assert_eq!(
            said,
            [
                ("streaming", "NV12 1920 \u{d7} 1080 at 30"),
                ("NV12", "1920 \u{d7} 1080 at 30"),
                ("sensor", "OmniVision OV08X40"),
                ("facing", "you"),
                ("power", "on")
            ]
        );
        // A loopback it isn't told of says nothing.
        assert!(cameras(&sys.0, &sys.0.join("dev"), &holders, &[]).is_empty());
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
    fn a_monitor_s_ddc_facts_are_found_by_its_edid() {
        let told = std::env::temp_dir().join(format!("desk-told-{}", std::process::id()));
        fs::create_dir_all(&told).unwrap();
        fs::write(
            told.join("ddc-220ed336000000001c1f"),
            "brightness\t40%\nnot a fact\nfirmware level\t1.0\n",
        )
        .unwrap();
        let said = monitor_said(&told, &bytes(Z27K_G3));
        assert_eq!(
            said,
            vec![
                ("brightness".to_owned(), "40%".to_owned()),
                ("firmware level".to_owned(), "1.0".to_owned()),
            ]
        );
        assert!(
            monitor_said(&told, &[0; 128]).is_empty(),
            "another monitor's"
        );
        fs::remove_dir_all(&told).unwrap();
    }

    #[test]
    fn a_monitor_s_facts_written_whole_are_heard_and_their_copy_is_not() {
        assert!(names_a_monitor(&event(
            IN_MOVED_TO,
            "ddc-220ed336000000001c1f"
        )));
        assert!(names_a_monitor(&event(
            IN_DELETE,
            "ddc-220ed336000000001c1f"
        )));
        assert!(!names_a_monitor(&event(
            IN_MOVED_TO,
            "ddc-220ed336000000001c1f.new"
        )));
        assert!(!names_a_monitor(&event(IN_MOVED_TO, "displays.json")));
    }

    #[test]
    fn a_sound_device_opening_or_the_lighting_changing_is_heard() {
        assert!(names_one(&event(IN_OPEN, "pcmC1D0p"), "pebble-lights"));
        assert!(names_one(
            &event(IN_MOVED_TO, "pebble-lights"),
            "pebble-lights"
        ));
        let mut other = event(IN_CLOSE_WRITE, "quickshell-locked");
        other.extend(event(IN_CLOSE_WRITE, "pebble-lights.new"));
        assert!(!names_one(&other, "pebble-lights"));
        assert!(names_one(&event(IN_Q_OVERFLOW, ""), "pebble-lights"));
    }
}

//! Writes what the launcher can't learn for itself about the desk's displays
//! and sound cards, for it to show behind each one's row.
//!
//! ```text
//! desk-facts FOLDER
//! ```
//!
//! Each kind goes to a file of its own in FOLDER, as JSON: an object of
//! devices, each an ordered list of `{"name": …, "value": …}`, the way the
//! launcher lists them. `displays.json` is keyed by the connector's name as
//! niri gives it ("DP-4", "eDP-1"), with what the display's EDID says of
//! itself: who made it and when, its size, native mode, refresh range, colour
//! depth and connection, its colours against sRGB's, and whether it takes
//! HDR. `sound.json` is keyed by a USB sound card's ALSA id and direction
//! ("MV7:in", "Plus:out"), PipeWire carrying the id on each node as
//! `alsa.id`, with what the card offers, how its clock keeps time, its USB
//! speed and firmware, and, while it plays or records, the format, period
//! and buffer it runs at and the rate the kernel measures it taking. The
//! Pebble X Plus's lighting comes from the file `pebble-lights` keeps beside
//! the folder.
//!
//! A file is written only when what it says changes, by writing a copy and
//! renaming it over the old one, and both are removed when this is stopped,
//! so a fact is never left behind its device. It hears the kernel's uevents
//! for displays and cards coming, going and changing, and inotify for a
//! playback or capture device opening and closing and the lighting's file
//! being written; nothing is looked at between.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::ffi::{CString, c_char, c_int, c_void};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Duration;

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
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

/// The kernel's uevents for these are the ones that change a fact.
const HEARD: [&[u8]; 3] = [b"SUBSYSTEM=drm", b"SUBSYSTEM=sound", b"SUBSYSTEM=usb"];
/// The Pebble X Plus, as its USB device names itself.
const PEBBLE: &str = "041e:329a";

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
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
/// byte.
fn input(byte: u8) -> String {
    if byte & 0x80 == 0 {
        return "analogue".to_owned();
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
    [depth, interface]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ")
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
    fact(&mut facts, "input", input(block[20]));
    fact(&mut facts, "colours", gamut(block));
    if let Some(extension) = edid.get(128..256) {
        fact(&mut facts, "HDR", hdr(extension).unwrap_or_default());
    }
    fact(&mut facts, "EDID", format!("{}.{}", block[18], block[19]));
    Some(facts)
}

/// Every connected display by niri's name for its connector, from the DRM
/// connectors `sys` lists.
fn displays(sys: &Path) -> Vec<(String, Facts)> {
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
            Some((connector.to_owned(), display(&edid)?))
        })
        .collect();
    found.sort();
    found
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

/// What a running stream's `hw_params` say it runs at.
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
    if let Some(period) = field("period_size") {
        let _ = write!(said, ", period {period}");
    }
    if let Some(buffer) = field("buffer_size") {
        let _ = write!(said, ", buffer {buffer}");
    }
    said
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

/// Every USB sound card's facts, by its ALSA id and direction, from the
/// cards `asound` lists and their devices in `sys`; the Pebble's lighting
/// from what its file says.
fn sound(asound: &Path, sys: &Path, lighting: Option<&str>) -> Vec<(String, Facts)> {
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
        let (Ok(id), Ok(stream0), Ok(usbid)) = (
            fs::read_to_string(card.join("id")),
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

/// Whether a uevent is about a display, a sound card or a USB device.
fn matters(uevent: &[u8]) -> bool {
    uevent
        .split(|byte| *byte == 0)
        .any(|field| HEARD.contains(&field))
}

/// inotify, watching /dev/snd for devices opening, closing, coming and
/// going, and the folder `pebble-lights` writes its file in.
fn watches(lighting: &Path) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let mut watched = vec![(
        PathBuf::from("/dev/snd"),
        IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE | IN_CREATE | IN_DELETE,
    )];
    if let Some(folder) = lighting.parent() {
        watched.push((folder.to_owned(), IN_CLOSE_WRITE | IN_MOVED_TO));
    }
    for (path, mask) in watched {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call. A machine with
        // no sound card yet has no /dev/snd, and the uevent for one coming
        // is heard instead.
        unsafe { inotify_add_watch(fd, path.as_ptr(), mask) };
    }
    Ok(file)
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

/// Reads everything waiting on a descriptor, saying whether `heard` found
/// any of it to matter.
fn heard(file: &mut File, heard: &dyn Fn(&[u8]) -> bool) -> bool {
    let mut any = false;
    let mut buffer = [0; 8192];
    while let Ok(read) = file.read(&mut buffer) {
        if read == 0 {
            break;
        }
        any |= heard(&buffer[..read]);
    }
    any
}

fn run(folder: &Path) -> ExitCode {
    let lighting = folder
        .parent()
        .map_or_else(PathBuf::new, |runtime| runtime.join("pebble-lights"));
    let (mut stop, mut kernel, mut changes) = match (stopping(), uevents(), watches(&lighting)) {
        (Ok(stop), Ok(kernel), Ok(changes)) => (stop, kernel, changes),
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => {
            eprintln!("<3>can't hear the desk's devices: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = fs::create_dir_all(folder) {
        eprintln!("<3>can't make {}: {error}", folder.display());
        return ExitCode::FAILURE;
    }
    let files = [folder.join("displays.json"), folder.join("sound.json")];
    loop {
        let said = fs::read_to_string(&lighting).ok();
        let written = write(&files[0], &json(&displays(Path::new("/sys")))).and_then(|()| {
            write(
                &files[1],
                &json(&sound(
                    Path::new("/proc/asound"),
                    Path::new("/sys"),
                    said.as_deref().map(str::trim),
                )),
            )
        });
        if let Err(error) = written {
            eprintln!("<4>can't write the desk's facts: {error}");
        }
        let mut waiting =
            [stop.as_raw_fd(), kernel.as_raw_fd(), changes.as_raw_fd()].map(|fd| PollFd {
                fd,
                events: POLLIN,
                revents: 0,
            });
        let mut news = false;
        while !news {
            // SAFETY: three valid pollfds for the length given.
            if unsafe { poll(waiting.as_mut_ptr(), 3, -1) } < 0 {
                continue;
            }
            if waiting[0].revents & POLLIN != 0 {
                let _ = stop.read(&mut [0; 128]);
                for file in &files {
                    let _ = fs::remove_file(file);
                }
                return ExitCode::SUCCESS;
            }
            // A card's devices, a display's EDID and a stream's parameters
            // settle a moment after the first word of them.
            std::thread::sleep(Duration::from_millis(150));
            news |= heard(&mut kernel, &matters);
            news |= heard(&mut changes, &|events| names_one(events, "pebble-lights"));
        }
    }
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let [folder] = &arguments[..] {
        run(Path::new(folder))
    } else {
        eprintln!("usage: desk-facts FOLDER");
        ExitCode::from(2)
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
        assert_eq!(named(&facts, "input"), Some("10 bit, DisplayPort"));
        assert_eq!(named(&facts, "colours"), Some("sRGB's primaries"));
        assert_eq!(named(&facts, "HDR"), Some("none"));
        assert_eq!(named(&facts, "EDID"), Some("1.4"));
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
        assert_eq!(running(hw), "48 kHz, 24 bit, period 256, buffer 16384");
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
    fn only_displays_sound_cards_and_usb_devices_matter() {
        assert!(matters(&uevent(&[
            "change@/devices/pci0000:00/0000:00:02.0/drm/card0",
            "ACTION=change",
            "SUBSYSTEM=drm",
            "HOTPLUG=1"
        ])));
        assert!(matters(&uevent(&[
            "add@/devices/.../sound/card1",
            "SUBSYSTEM=sound"
        ])));
        assert!(!matters(&uevent(&[
            "change@/devices/.../power_supply/BAT0",
            "SUBSYSTEM=power_supply"
        ])));
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

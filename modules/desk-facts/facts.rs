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
//! depth and connection, how its colours compare with sRGB's, its white point
//! and primaries as figures for the launcher to say, and whether it takes
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
//! from the sensor's; neither is ever opened. `bluetooth.json` is keyed by
//! a connected Bluetooth input device's address, as bluetoothd gives it to
//! the launcher, with how it's carried (LE's HID over GATT or Classic's
//! HIDP), its buttons and wheels or its keys, and its ids and release, from
//! the kernel's HID devices. `keyboards.json` is keyed by the name the
//! kernel gives each other keyboard, and each USB mouse, with how often
//! it's polled on USB, its keys or its buttons and wheel, and its firmware
//! release where its bus says one.
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

use std::ffi::c_int;
use std::fs::{self, File};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::process::ExitCode;

use crate::cameras::loopbacks;
use crate::hearing::run;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: u64, ...) -> c_int;
}

/// The native-endian word at `at` in a device's answer.
fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Asks a device a question by its ioctl, V4L2's, UVC's or ALSA's control
/// device's, saying whether it answered.
fn ask(node: &File, request: u64, buffer: &mut [u8]) -> bool {
    // SAFETY: each request is given a buffer of the size its number says.
    unsafe { ioctl(node.as_raw_fd(), request, buffer.as_mut_ptr()) == 0 }
}

/// A device's facts, in the order they're shown.
type Facts = Vec<(String, String)>;

fn fact(facts: &mut Facts, name: &str, value: impl Into<String>) {
    let value = value.into();
    if !value.is_empty() {
        facts.push((name.to_owned(), value));
    }
}

// Each kind of device is read in a file of its own, and how changes are heard
// in another. What several kinds read alike (USB descriptors, the input
// layer's capabilities, inotify's events) has a file of its own too, so a file
// imports the one that holds what it needs rather than a neighbour's.

mod bluetooth;
mod cable;
mod cameras;
mod displays;
mod hearing;
mod inotify;
mod input;
mod keyboards;
mod own_sound;
mod sound;
mod usb;
mod writing;

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

/// What more than one kind's tests read.
#[cfg(test)]
mod tests {
    use super::*;

    use std::path::PathBuf;

    pub(crate) fn bytes(hex: &str) -> Vec<u8> {
        let hex: Vec<u8> = hex.bytes().filter(u8::is_ascii_hexdigit).collect();
        hex.chunks(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    pub(crate) fn named<'a>(facts: &'a Facts, name: &str) -> Option<&'a str> {
        facts
            .iter()
            .find(|(each, _)| each == name)
            .map(|(_, value)| value.as_str())
    }

    /// The MV7's stream file while it records, as the kernel wrote it.
    pub(crate) const MV7: &str =
        "Shure Inc Shure MV7 at usb-0000:00:14.0-1.1.4, full speed : USB Audio

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

    /// Writes a stand-in sysfs file, with the folders it's in.
    pub(crate) fn put(root: &Path, path: &str, text: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    /// A stand-in sysfs, gone when the test is.
    pub(crate) struct Sys(pub(crate) PathBuf);

    impl Sys {
        pub(crate) fn new(name: &str) -> Sys {
            let root = std::env::temp_dir().join(format!("desk-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(root.join("bus/hid/devices")).unwrap();
            Sys(root)
        }

        /// A HID device where the kernel keeps it, linked from the bus as
        /// the kernel links it, with one input.
        pub(crate) fn hid(&self, home: &str, name: &str, uevent: &str, input: [&str; 3]) {
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

    /// An inotify event for a name, padded as the kernel pads it.
    pub(crate) fn event(mask: u32, name: &str) -> Vec<u8> {
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
}

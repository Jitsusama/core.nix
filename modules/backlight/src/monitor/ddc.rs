//! DDC/CI on a monitor's I2C bus: the messages asking for and setting a
//! VCP code and for the capabilities string, reading what the monitor
//! answers, its EDID's identity, and holding the Z27k G3's hot-plug
//! detection at Always on.

use std::ffi::{c_int, c_ulong};
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::facts;

// std links libc, so this comes from it.
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

/// The ioctl that says which address on the bus reads and writes go to.
const I2C_SLAVE: c_ulong = 0x0703;
/// Where a monitor answers DDC/CI.
const DDC_CI: u8 = 0x37;
/// Where a monitor gives its EDID.
const EDID: u8 = 0x50;
/// The VCP code that holds a monitor's brightness.
pub(super) const BRIGHTNESS: u8 = 0x10;
/// The VCP code, in the maker's own range, where HP's Z27k G3 keeps its
/// DisplayPort hot-plug detection: its low byte is 0 for Low power and 1 for
/// Always on. Read from one before and after the setting was changed on its
/// own menu; no other code changed.
pub(super) const HOT_PLUG: u8 = 0xE7;
pub(super) const ALWAYS_ON: u16 = 0x0001;
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
pub(super) const LONGEST_CAPABILITIES: u16 = 4096;
/// The VCP codes read for a monitor's facts, if it lists them: its contrast,
/// colour preset, panel, display controller and firmware level.
pub(super) const CONTRAST: u8 = 0x12;
pub(super) const PRESET: u8 = 0x14;
pub(super) const PANEL: u8 = 0xB6;
pub(super) const CONTROLLER: u8 = 0xC8;
pub(super) const FIRMWARE: u8 = 0xC9;

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
pub(super) fn get_vcp(vcp: u8) -> Vec<u8> {
    message(&[0x01, vcp])
}

/// Sets a VCP code.
pub(super) fn set_vcp(vcp: u8, value: u16) -> Vec<u8> {
    let [high, low] = value.to_be_bytes();
    message(&[0x03, vcp, high, low])
}

/// Asks for a monitor's capabilities string from an offset.
pub(super) fn capabilities_request(offset: u16) -> Vec<u8> {
    let [high, low] = offset.to_be_bytes();
    message(&[CAPABILITIES, high, low])
}

/// What a monitor says a VCP code holds.
#[derive(Debug, PartialEq)]
pub(super) struct Reading {
    pub(super) max: u16,
    pub(super) now: u16,
}

fn refused(why: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, why)
}

/// A monitor's answer to `get_vcp`, whose checksum counts the host's address.
pub(super) fn reading(reply: &[u8], vcp: u8) -> io::Result<Reading> {
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
pub(super) fn value_for(level: f64, max: u16) -> u16 {
    (level.clamp(0.0, 1.0) * f64::from(max)).round() as u16
}

/// Which monitor an EDID is: its maker, model, serial and when it was made.
pub(super) type Identity = [u8; 10];

/// The monitor an EDID's first block names, if it is one.
pub(super) fn identity(edid: &[u8]) -> Option<Identity> {
    let block = edid.get(..128)?;
    let whole = block.iter().fold(0u8, |sum, byte| sum.wrapping_add(*byte)) == 0;
    if block[..8] != [0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x00] || !whole {
        return None;
    }
    block[8..18].try_into().ok()
}

/// Whether a monitor is one whose hot-plug detection is held at Always on.
pub(super) fn holds_hot_plug(identity: &Identity) -> bool {
    identity[..4] == Z27K_G3
}

/// What to set a Z27k G3's hot-plug detection to, given what it holds: Always
/// on, keeping whatever else it says, but only if it's at Low power.
pub(super) fn hot_plug_to_set(now: u16) -> Option<u16> {
    let [_, low] = now.to_be_bytes();
    (low == 0).then_some(now | ALWAYS_ON)
}

/// Reads a Z27k G3's hot-plug detection, and sets it to Always on if it
/// isn't, giving what it holds after, if that's known.
pub(super) fn hold_hot_plug(name: &str, bus: &mut Bus) -> Option<u16> {
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
pub(super) fn hot_plug_named(value: u16) -> &'static str {
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
pub(super) fn reached(path: &Path) -> io::Result<Identity> {
    let mut file = open_at(path, EDID)?;
    file.write_all(&[0])?;
    let mut edid = [0; 128];
    file.read_exact(&mut edid)?;
    identity(&edid).ok_or_else(|| refused("its EDID isn't one".into()))
}

/// A monitor's bus, opened to talk to it.
pub(super) struct Bus {
    file: File,
}

impl Bus {
    pub(super) fn open(path: &Path) -> io::Result<Bus> {
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

    pub(super) fn read_vcp(&mut self, vcp: u8) -> io::Result<Reading> {
        self.send(&get_vcp(vcp))?;
        thread::sleep(ANSWERING);
        let mut reply = [0; 11];
        let read = self.file.read(&mut reply);
        thread::sleep(BETWEEN);
        reading(&reply[..read?], vcp)
    }

    pub(super) fn write_vcp(&mut self, vcp: u8, value: u16) -> io::Result<()> {
        let sent = self.send(&set_vcp(vcp, value));
        thread::sleep(BETWEEN);
        sent
    }

    /// One piece of the capabilities string, from an offset: none once it's
    /// ended.
    pub(super) fn read_capabilities(&mut self, offset: u16) -> io::Result<Vec<u8>> {
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

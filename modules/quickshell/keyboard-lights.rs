//! Says whether caps lock is on, in the file it's given as "1" or "0" and a
//! newline, for a shell that can't hear the keyboard's lights itself: niri's
//! IPC and Quickshell carry no lock state, and an LED's sysfs file sends no
//! change event.
//!
//! It takes every keyboard with a caps lock light, and new ones as they come,
//! and asks the kernel to hand it nothing but that light: keys are filtered
//! out before they reach it, so a keystroke never wakes it. Each keyboard has
//! a thread blocked reading it, and one more is blocked on the device
//! directory, so it sleeps until something changes and asks nothing. The
//! compositor sets the light on every keyboard it holds, so any one of them
//! says what the lock is.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::collections::HashSet;
use std::ffi::{CString, c_char, c_int, c_ulong, c_void};
use std::fs::{self, File, OpenOptions};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{FileExt, MetadataExt, OpenOptionsExt};
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::thread;

const DEVICES: &str = "/dev/input";

const EV_SYN: u32 = 0x00;
const EV_LED: u16 = 0x11;
const EV_MAX: usize = 0x1f;
const LED_CAPSL: u16 = 0x01;
const LED_MAX: usize = 0x0f;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CREATE: u32 = 0x100;

// An input_event on a 64-bit machine: a timeval, then type, code and value.
const EVENT: usize = 24;
// An inotify_event before its name: wd, mask, cookie and len.
const CHANGE: usize = 16;

#[repr(C)]
struct InputMask {
    kind: u32,
    codes_size: u32,
    codes_ptr: u64,
}

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

// The kernel's _IOC: direction, size, the evdev letter and the number.
const fn request(direction: c_ulong, number: c_ulong, size: usize) -> c_ulong {
    (direction << 30) | ((size as c_ulong) << 16) | ((b'E' as c_ulong) << 8) | number
}
const READ: c_ulong = 2;
const WRITE: c_ulong = 1;
const fn lights_supported(size: usize) -> c_ulong {
    request(READ, 0x20 + EV_LED as c_ulong, size)
}
const fn lights_lit(size: usize) -> c_ulong {
    request(READ, 0x19, size)
}
const SET_MASK: c_ulong = request(WRITE, 0x93, size_of::<InputMask>());

// The kernel's bitmasks are whole longs, and it refuses a mask whose size
// isn't, so each is kept as 64-bit words, which also keeps its bits in the
// kernel's order on any machine.
const fn words(bits: usize) -> usize {
    bits.div_ceil(64)
}

fn lit(bits: &[u64], bit: u16) -> bool {
    bits[bit as usize / 64] >> (bit % 64) & 1 == 1
}

fn only(bit: u16) -> [u64; 1] {
    [1 << bit]
}

/// What's been said, written in place so a reader never finds the file empty.
struct Said {
    file: File,
    last: Option<bool>,
}

impl Said {
    fn say(&mut self, on: bool) {
        if self.last == Some(on) {
            return;
        }
        let text: &[u8] = if on { b"1\n" } else { b"0\n" };
        if self.file.write_at(text, 0).is_ok() {
            self.last = Some(on);
        }
    }
}

/// The keyboards held, by device number, and what's been said.
struct Shared {
    held: Mutex<HashSet<u64>>,
    said: Mutex<Said>,
}

impl Shared {
    fn say(&self, on: bool) {
        self.said.lock().unwrap().say(on);
    }
}

/// Lets through the lights alone, and of them caps lock alone. The mask in
/// `EV_SYN`'s place is one of event types; the frames themselves always pass.
fn filter(fd: c_int) -> bool {
    const _: () = assert!(words(EV_MAX + 1) == 1 && words(LED_MAX + 1) == 1);
    let (kinds, caps) = (only(EV_LED), only(LED_CAPSL));
    [(EV_SYN, &kinds), (u32::from(EV_LED), &caps)]
        .iter()
        .all(|(kind, codes)| {
            let mask = InputMask {
                kind: *kind,
                codes_size: u32::try_from(size_of_val(*codes)).unwrap(),
                codes_ptr: codes.as_ptr() as u64,
            };
            // SAFETY: the mask and the codes it points to outlive the call.
            unsafe { ioctl(fd, SET_MASK, &raw const mask) >= 0 }
        })
}

/// Takes a device on if it has a caps lock light, says what it shows, and
/// listens to it until it goes.
fn take(name: &str, shared: &Arc<Shared>) {
    if !name.starts_with("event") {
        return;
    }
    let Ok(device) = File::open(format!("{DEVICES}/{name}")) else {
        return;
    };
    let fd = device.as_raw_fd();
    let mut lights = [0u64; words(LED_MAX + 1)];
    let size = size_of_val(&lights);
    // SAFETY: the kernel writes at most `size` bytes into `lights`.
    let supported = unsafe {
        ioctl(
            fd,
            lights_supported(size),
            lights.as_mut_ptr().cast::<c_void>(),
        )
    };
    if supported < 0 || !lit(&lights, LED_CAPSL) || !filter(fd) {
        return;
    }
    // A device whose permissions change is heard of twice.
    let Ok(number) = device.metadata().map(|data| data.rdev()) else {
        return;
    };
    if !shared.held.lock().unwrap().insert(number) {
        return;
    }
    // SAFETY: as above.
    if unsafe { ioctl(fd, lights_lit(size), lights.as_mut_ptr().cast::<c_void>()) } >= 0 {
        shared.say(lit(&lights, LED_CAPSL));
    }
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        listen(device, &shared);
        shared.held.lock().unwrap().remove(&number);
    });
}

/// Reads a keyboard's caps lock light as it changes, until the keyboard goes.
fn listen(mut device: File, shared: &Shared) {
    let mut events = [0u8; EVENT * 16];
    while let Ok(length) = device.read(&mut events) {
        if length == 0 {
            return;
        }
        for event in events[..length].as_chunks::<EVENT>().0 {
            let kind = u16::from_ne_bytes([event[16], event[17]]);
            let code = u16::from_ne_bytes([event[18], event[19]]);
            let value = i32::from_ne_bytes([event[20], event[21], event[22], event[23]]);
            if kind == EV_LED && code == LED_CAPSL {
                shared.say(value != 0);
            }
        }
    }
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: keyboard-lights FILE");
        return ExitCode::from(2);
    };
    // Kept rather than emptied, so a reader never finds it empty.
    let file = match OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o644)
        .open(&path)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let shared = Arc::new(Shared {
        held: Mutex::new(HashSet::new()),
        said: Mutex::new(Said { file, last: None }),
    });

    // A device appears before udev lets the input group read it, so a change
    // to its permissions is a second chance to take it.
    let directory = CString::new(DEVICES).unwrap();
    // SAFETY: plain calls; the path is a valid C string for the call.
    let watch = unsafe { inotify_init1(IN_CLOEXEC) };
    if watch < 0
        || unsafe { inotify_add_watch(watch, directory.as_ptr(), IN_CREATE | IN_ATTRIB) } < 0
    {
        eprintln!("{DEVICES}: {}", std::io::Error::last_os_error());
        return ExitCode::FAILURE;
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let mut changes = unsafe { File::from_raw_fd(watch) };

    match fs::read_dir(DEVICES) {
        Ok(entries) => entries
            .flatten()
            .for_each(|entry| take(&entry.file_name().to_string_lossy(), &shared)),
        Err(error) => {
            eprintln!("{DEVICES}: {error}");
            return ExitCode::FAILURE;
        }
    }
    {
        let mut said = shared.said.lock().unwrap();
        if said.last.is_none() {
            said.say(false);
        }
    }

    let mut buffer = [0u8; 4096];
    loop {
        let length = match changes.read(&mut buffer) {
            Ok(length) => length,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => {
                eprintln!("{DEVICES}: {error}");
                return ExitCode::FAILURE;
            }
        };
        let mut at = 0;
        while at + CHANGE <= length {
            let named = u32::from_ne_bytes(buffer[at + 12..at + 16].try_into().unwrap()) as usize;
            let name = &buffer[at + CHANGE..at + CHANGE + named];
            let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
            take(&String::from_utf8_lossy(name), &shared);
            at += CHANGE + named;
        }
    }
}

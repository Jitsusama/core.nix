//! Says when dictation's chord, Super and D, is let go, and how long it was
//! held, so holding it can mean "listen until I let go". niri runs a
//! chord's command as it's pressed and says nothing of its release, and the
//! keys themselves are the input group's, which an account mustn't join, or
//! anything it runs could read every key.
//!
//! So this runs as its own user in that group and takes only D and the two
//! Super keys from each keyboard: the kernel filters out every other key
//! before it reaches it. Whoever connects to the socket it's given hears a
//! line as the chord is let go, `up` and the milliseconds it was held, and
//! nothing else, so what it tells is only when the chord was used.
//!
//! Each keyboard has a thread blocked reading it and one more is blocked on
//! the device directory, as keyboard-lights does. It needs only the
//! standard library, which already links the C library these calls come
//! from.

use std::collections::HashSet;
use std::ffi::{CString, c_char, c_int, c_ulong, c_void};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::process::ExitCode;
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Instant;

const DEVICES: &str = "/dev/input";

const EV_SYN: u32 = 0x00;
const EV_KEY: u16 = 0x01;
const EV_MAX: usize = 0x1f;
const KEY_D: u16 = 32;
const KEY_LEFTMETA: u16 = 125;
const KEY_RIGHTMETA: u16 = 126;
const KEY_MAX: usize = 0x2ff;

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
const fn keys_supported(size: usize) -> c_ulong {
    request(READ, 0x20 + EV_KEY as c_ulong, size)
}
const SET_MASK: c_ulong = request(WRITE, 0x93, size_of::<InputMask>());

// The kernel's bitmasks are whole longs, kept as 64-bit words so their bits
// are in the kernel's order on any machine.
const fn words(bits: usize) -> usize {
    bits.div_ceil(64)
}
const KEY_WORDS: usize = words(KEY_MAX + 1);

fn has(bits: &[u64], bit: u16) -> bool {
    bits[bit as usize / 64] >> (bit % 64) & 1 == 1
}

fn mark(bits: &mut [u64], bit: u16) {
    bits[bit as usize / 64] |= 1 << (bit % 64);
}

/// What the chord's keys are doing: which devices hold each down, and when
/// the chord began, while it's held.
#[derive(Default)]
pub struct Chord {
    meta: HashSet<(u64, u16)>,
    d: HashSet<u64>,
    since: Option<Instant>,
}

impl Chord {
    /// Takes a key's change from `device`, at `now`, and gives how long the
    /// chord was held, in ms, when this change let it go.
    pub fn key(&mut self, device: u64, code: u16, down: bool, now: Instant) -> Option<u128> {
        match (code, down) {
            (KEY_D, true) => {
                self.d.insert(device);
            }
            (KEY_D, false) => {
                self.d.remove(&device);
            }
            (KEY_LEFTMETA | KEY_RIGHTMETA, true) => {
                self.meta.insert((device, code));
            }
            (KEY_LEFTMETA | KEY_RIGHTMETA, false) => {
                self.meta.remove(&(device, code));
            }
            _ => return None,
        }
        let held = !self.meta.is_empty() && !self.d.is_empty();
        match (self.since, held) {
            // D pressed under Super starts it; Super pressed under D doesn't,
            // as niri doesn't take that for the chord either.
            (None, true) if code == KEY_D => {
                self.since = Some(now);
                None
            }
            (Some(since), false) => {
                self.since = None;
                Some(now.duration_since(since).as_millis())
            }
            _ => None,
        }
    }

    /// Forgets what a device that went was holding.
    pub fn gone(&mut self, device: u64, now: Instant) -> Option<u128> {
        self.meta.retain(|(held, _)| *held != device);
        self.d.remove(&device);
        let held = !self.meta.is_empty() && !self.d.is_empty();
        match (self.since, held) {
            (Some(since), false) => {
                self.since = None;
                Some(now.duration_since(since).as_millis())
            }
            _ => None,
        }
    }
}

/// The keyboards held, the chord, and who's listening.
struct Shared {
    held: Mutex<HashSet<u64>>,
    chord: Mutex<Chord>,
    listeners: Mutex<Vec<UnixStream>>,
}

impl Shared {
    fn tell(&self, ms: u128) {
        let line = format!("up {ms}\n");
        self.listeners
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .retain_mut(|listener| listener.write_all(line.as_bytes()).is_ok());
    }
}

/// Lets through keys alone, and of them D and the two Super keys alone. The
/// mask in `EV_SYN`'s place is one of event types; the frames always pass.
fn filter(fd: c_int) -> bool {
    const _: () = assert!(words(EV_MAX + 1) == 1);
    let kinds = [1_u64 << EV_KEY];
    let mut keys = [0_u64; KEY_WORDS];
    for key in [KEY_D, KEY_LEFTMETA, KEY_RIGHTMETA] {
        mark(&mut keys, key);
    }
    [(EV_SYN, &kinds[..]), (u32::from(EV_KEY), &keys[..])]
        .iter()
        .all(|(kind, codes)| {
            let mask = InputMask {
                kind: *kind,
                codes_size: u32::try_from(size_of_val(*codes)).unwrap_or(0),
                codes_ptr: codes.as_ptr() as u64,
            };
            // SAFETY: the mask and the codes it points to outlive the call.
            unsafe { ioctl(fd, SET_MASK, &raw const mask) >= 0 }
        })
}

/// Takes a device on if it has D and a Super key, and listens to it until
/// it goes.
fn take(name: &str, shared: &Arc<Shared>) {
    if !name.starts_with("event") {
        return;
    }
    let Ok(device) = File::open(format!("{DEVICES}/{name}")) else {
        return;
    };
    let fd = device.as_raw_fd();
    let mut keys = [0_u64; KEY_WORDS];
    let size = size_of_val(&keys);
    // SAFETY: the kernel writes at most `size` bytes into `keys`.
    let supported = unsafe { ioctl(fd, keys_supported(size), keys.as_mut_ptr().cast::<c_void>()) };
    let keyboard = has(&keys, KEY_D) && (has(&keys, KEY_LEFTMETA) || has(&keys, KEY_RIGHTMETA));
    if supported < 0 || !keyboard || !filter(fd) {
        return;
    }
    // A device whose permissions change is heard of twice.
    let Ok(number) = device.metadata().map(|data| data.rdev()) else {
        return;
    };
    if !shared
        .held
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(number)
    {
        return;
    }
    let shared = Arc::clone(shared);
    thread::spawn(move || {
        listen(device, number, &shared);
        shared
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&number);
        let let_go = shared
            .chord
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .gone(number, Instant::now());
        if let Some(ms) = let_go {
            shared.tell(ms);
        }
    });
}

/// Reads a keyboard's chord keys as they change, until the keyboard goes.
/// A key held long enough to repeat says so again, which changes nothing.
fn listen(mut device: File, number: u64, shared: &Shared) {
    let mut events = [0_u8; EVENT * 16];
    while let Ok(length) = device.read(&mut events) {
        if length == 0 {
            return;
        }
        for event in events[..length].as_chunks::<EVENT>().0 {
            let kind = u16::from_ne_bytes([event[16], event[17]]);
            let code = u16::from_ne_bytes([event[18], event[19]]);
            let value = i32::from_ne_bytes([event[20], event[21], event[22], event[23]]);
            if kind != EV_KEY || value == 2 {
                continue;
            }
            let let_go = shared
                .chord
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .key(number, code, value == 1, Instant::now());
            if let Some(ms) = let_go {
                shared.tell(ms);
            }
        }
    }
}

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: dictation-chord SOCKET");
        return ExitCode::from(2);
    };
    let _ = fs::remove_file(&path);
    let listener = match UnixListener::bind(&path) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!("{path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    // Any account may hear when the chord is let go, as any may read caps
    // lock's state; it's the keys themselves that are kept back.
    if let Err(error) = fs::set_permissions(&path, fs::Permissions::from_mode(0o666)) {
        eprintln!("{path}: {error}");
        return ExitCode::FAILURE;
    }
    let shared = Arc::new(Shared {
        held: Mutex::new(HashSet::new()),
        chord: Mutex::new(Chord::default()),
        listeners: Mutex::new(Vec::new()),
    });

    // A device appears before udev lets the input group read it, so a change
    // to its permissions is a second chance to take it.
    let directory = CString::new(DEVICES).unwrap_or_default();
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

    thread::spawn({
        let shared = Arc::clone(&shared);
        move || {
            for stream in listener.incoming().flatten() {
                shared
                    .listeners
                    .lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .push(stream);
            }
        }
    });

    let mut buffer = [0_u8; 4096];
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
            let Ok(named) = buffer[at + 12..at + 16].try_into().map(u32::from_ne_bytes) else {
                break;
            };
            let named = named as usize;
            let name = &buffer[at + CHANGE..(at + CHANGE + named).min(length)];
            let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
            take(&String::from_utf8_lossy(name), &shared);
            at += CHANGE + named;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const KEYBOARD: u64 = 7;

    #[test]
    fn letting_go_of_d_says_how_long_the_chord_was_held() {
        let mut chord = Chord::default();
        let start = Instant::now();
        assert_eq!(chord.key(KEYBOARD, KEY_LEFTMETA, true, start), None);
        assert_eq!(chord.key(KEYBOARD, KEY_D, true, start), None);
        let later = start + Duration::from_millis(900);
        assert_eq!(chord.key(KEYBOARD, KEY_D, false, later), Some(900));
        assert_eq!(chord.key(KEYBOARD, KEY_LEFTMETA, false, later), None);
    }

    #[test]
    fn letting_go_of_super_first_ends_it_too() {
        let mut chord = Chord::default();
        let start = Instant::now();
        chord.key(KEYBOARD, KEY_RIGHTMETA, true, start);
        chord.key(KEYBOARD, KEY_D, true, start);
        let later = start + Duration::from_millis(450);
        assert_eq!(chord.key(KEYBOARD, KEY_RIGHTMETA, false, later), Some(450));
        assert_eq!(chord.key(KEYBOARD, KEY_D, false, later), None);
    }

    #[test]
    fn d_alone_or_super_after_d_is_no_chord() {
        let mut chord = Chord::default();
        let start = Instant::now();
        chord.key(KEYBOARD, KEY_D, true, start);
        assert_eq!(chord.key(KEYBOARD, KEY_D, false, start), None);
        chord.key(KEYBOARD, KEY_D, true, start);
        chord.key(KEYBOARD, KEY_LEFTMETA, true, start);
        assert_eq!(chord.key(KEYBOARD, KEY_LEFTMETA, false, start), None);
        assert_eq!(chord.key(KEYBOARD, KEY_D, false, start), None);
    }

    #[test]
    fn another_key_is_never_taken() {
        let mut chord = Chord::default();
        let start = Instant::now();
        chord.key(KEYBOARD, KEY_LEFTMETA, true, start);
        chord.key(KEYBOARD, KEY_D, true, start);
        // KEY_A, which the kernel's filter keeps out anyway.
        assert_eq!(chord.key(KEYBOARD, 30, false, start), None);
        assert_eq!(chord.key(KEYBOARD, KEY_D, false, start), Some(0));
    }

    #[test]
    fn a_keyboard_that_goes_lets_its_keys_go() {
        let mut chord = Chord::default();
        let start = Instant::now();
        chord.key(KEYBOARD, KEY_LEFTMETA, true, start);
        chord.key(KEYBOARD, KEY_D, true, start);
        let later = start + Duration::from_millis(30);
        assert_eq!(chord.gone(KEYBOARD, later), Some(30));
        assert_eq!(chord.gone(KEYBOARD, later), None);
    }

    #[test]
    fn the_filter_masks_name_the_chord_keys_alone() {
        let mut keys = [0_u64; KEY_WORDS];
        for key in [KEY_D, KEY_LEFTMETA, KEY_RIGHTMETA] {
            mark(&mut keys, key);
        }
        let marked: Vec<u16> = (0..=u16::try_from(KEY_MAX).unwrap_or(0))
            .filter(|key| has(&keys, *key))
            .collect();
        assert_eq!(marked, [KEY_D, KEY_LEFTMETA, KEY_RIGHTMETA]);
    }
}

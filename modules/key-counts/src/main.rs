//! Counts the keys pressed on the ErgoDox and the laptop's keyboard, into
//! the file it's given: for each keyboard, layer, modifiers held and key,
//! how many presses and how many held, and nothing else (tally.rs). It's
//! how the board's layout is weighed against what Joel really presses.
//!
//! It reads the ErgoDox by ZSA's vendor number and the laptop's keyboard as
//! keyd sends it on, since keyd holds the keyboard itself. Each has a thread
//! blocked reading it, and one more is blocked on the device directory for
//! a keyboard plugged in later, as dictation-chord does. The counts are
//! written a minute after one changes, the minute's keys together, so it
//! wakes for nothing at rest. A stop loses at most that minute.
//!
//! It needs only the standard library, which already links the C library
//! these calls come from, and the terminal kit, for its help, exits and
//! failures (docs/commands.md). It answers nothing, since what it counts
//! goes to its file, and a write that fails is said in the journal.

mod tally;

use std::collections::HashSet;
use std::ffi::{CString, c_char, c_int, c_ulong, c_void};
use std::fs::{self, File};
use std::io::Read;
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::{Arc, Condvar, Mutex, PoisonError};
use std::thread;
use std::time::Duration;

use tally::{Board, Tally};
use terminal_kit::{Exit, Failure, Help};

const DEVICES: &str = "/dev/input";
const ZSA: u16 = 0x3297;
const KEYD: &str = "keyd virtual keyboard";
const EV_KEY: u16 = 0x01;
const EVERY: Duration = Duration::from_secs(60);

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CREATE: u32 = 0x100;

// An input_event on a 64-bit machine: a timeval, then type, code and value.
const EVENT: usize = 24;
// An inotify_event before its name: wd, mask, cookie and len.
const CHANGE: usize = 16;

unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

// The kernel's _IOC: direction, size, the evdev letter and the number.
const fn request(number: c_ulong, size: usize) -> c_ulong {
    const READ: c_ulong = 2;
    (READ << 30) | ((size as c_ulong) << 16) | ((b'E' as c_ulong) << 8) | number
}
const NAME: usize = 256;
const GET_NAME: c_ulong = request(0x06, NAME);
const GET_ID: c_ulong = request(0x02, 8);

/// The keyboards being read, the counts, and what wakes the writer when
/// one changes.
struct Shared {
    held: Mutex<HashSet<u64>>,
    tally: Mutex<Tally>,
    changed: Condvar,
}

/// Which keyboard a device is, if it's one that's counted.
fn board(fd: c_int) -> Option<Board> {
    let mut id = [0_u16; 4];
    // SAFETY: the kernel writes the bus, vendor, product and version, 8 bytes.
    if unsafe { ioctl(fd, GET_ID, id.as_mut_ptr().cast::<c_void>()) } >= 0 && id[1] == ZSA {
        return Some(Board::Ergodox);
    }
    let mut name = [0_u8; NAME];
    // SAFETY: the kernel writes at most NAME bytes into `name`.
    let length = unsafe { ioctl(fd, GET_NAME, name.as_mut_ptr().cast::<c_void>()) };
    let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
    (length > 0 && name == KEYD.as_bytes()).then_some(Board::Laptop)
}

/// Takes a device on if it's a keyboard that's counted, and reads it until
/// it goes.
fn take(name: &str, shared: &Arc<Shared>) {
    if !name.starts_with("event") {
        return;
    }
    let Ok(device) = File::open(format!("{DEVICES}/{name}")) else {
        return;
    };
    let Some(board) = board(device.as_raw_fd()) else {
        return;
    };
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
        listen(device, number, board, &shared);
        shared
            .held
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .remove(&number);
        shared
            .tally
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .gone(number);
    });
}

/// Counts a keyboard's keys as they change, until the keyboard goes.
fn listen(mut device: File, number: u64, board: Board, shared: &Shared) {
    let mut events = [0_u8; EVENT * 16];
    while let Ok(length) = device.read(&mut events) {
        if length == 0 {
            return;
        }
        let mut tally = shared.tally.lock().unwrap_or_else(PoisonError::into_inner);
        for event in events[..length].as_chunks::<EVENT>().0 {
            let kind = u16::from_ne_bytes([event[16], event[17]]);
            let code = u16::from_ne_bytes([event[18], event[19]]);
            let value = i32::from_ne_bytes([event[20], event[21], event[22], event[23]]);
            if kind == EV_KEY {
                tally.key(number, board, code, value);
            }
        }
        if tally.changed() {
            shared.changed.notify_one();
        }
    }
}

/// Writes the counts beside the file, then over it, so a reader never sees
/// half of them.
fn write(path: &Path, text: &str) -> std::io::Result<()> {
    let mut beside = PathBuf::from(path);
    beside.set_extension("new");
    fs::write(&beside, text)?;
    fs::rename(&beside, path)
}

fn help() -> Help {
    Help::new(
        "key-counts",
        "counts the keys pressed on each keyboard, into FILE",
    )
    .command("FILE", "the counts, kept and added to")
    .example(
        "key-counts /var/lib/key-counts/counts.tsv",
        "what its service runs",
    )
    .exit(Exit::Failed, "FILE couldn't be read")
    .exit(Exit::Unavailable, "the keyboards couldn't be watched")
}

fn main() -> ExitCode {
    terminal_kit::main(&help(), env!("CARGO_PKG_VERSION"), |context| {
        let [file] = context.args.as_slice() else {
            return Err(Failure::new(Exit::Usage, "it takes FILE, the counts")
                .try_this("key-counts --help"));
        };
        count(PathBuf::from(file))
    })
}

/// The keyboards' devices couldn't be watched or read.
fn unwatched(error: &std::io::Error) -> Failure {
    Failure::new(Exit::Unavailable, format!("{DEVICES} couldn't be watched"))
        .because(error.to_string())
}

/// Counts every keyboard's keys into the file at `path`, for as long as
/// the keyboards can be watched.
fn count(path: PathBuf) -> Result<(), Failure> {
    let mut tally = Tally::default();
    match fs::read_to_string(&path) {
        Ok(text) => tally.load(&text),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(
                Failure::new(Exit::Failed, format!("{} couldn't be read", path.display()))
                    .because(error.to_string()),
            );
        }
    }
    let shared = Arc::new(Shared {
        held: Mutex::new(HashSet::new()),
        tally: Mutex::new(tally),
        changed: Condvar::new(),
    });

    // A device appears before udev lets the input group read it, so a change
    // to its permissions is a second chance to take it.
    let directory = CString::new(DEVICES).unwrap_or_default();
    // SAFETY: plain calls; the path is a valid C string for the call.
    let watch = unsafe { inotify_init1(IN_CLOEXEC) };
    if watch < 0
        || unsafe { inotify_add_watch(watch, directory.as_ptr(), IN_CREATE | IN_ATTRIB) } < 0
    {
        return Err(unwatched(&std::io::Error::last_os_error()));
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let mut changes = unsafe { File::from_raw_fd(watch) };

    match fs::read_dir(DEVICES) {
        Ok(entries) => entries
            .flatten()
            .for_each(|entry| take(&entry.file_name().to_string_lossy(), &shared)),
        Err(error) => return Err(unwatched(&error)),
    }

    thread::spawn({
        let shared = Arc::clone(&shared);
        move || {
            loop {
                // Asleep until a count changes, then a minute more, so a
                // minute's keys are written together.
                drop(
                    shared
                        .changed
                        .wait_while(
                            shared.tally.lock().unwrap_or_else(PoisonError::into_inner),
                            |tally| !tally.changed(),
                        )
                        .unwrap_or_else(PoisonError::into_inner),
                );
                thread::sleep(EVERY);
                let text = {
                    let mut tally = shared.tally.lock().unwrap_or_else(PoisonError::into_inner);
                    tally.take_changed();
                    tally.render()
                };
                if let Err(error) = write(&path, &text) {
                    eprintln!("{}: {error}", path.display());
                }
            }
        }
    });

    let mut buffer = [0_u8; 4096];
    loop {
        let length = match changes.read(&mut buffer) {
            Ok(length) => length,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(unwatched(&error)),
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

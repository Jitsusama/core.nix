//! Drives a Logitech Litra Glow, the light on a desk's monitor.
//!
//! ```text
//! litra on | off | status
//! litra brightness PERCENT
//! litra temperature KELVIN
//! litra follow FOLDER
//! ```
//!
//! The light takes 20-byte HID reports on its hidraw node, which systemd's
//! hwdb opens to the person at the seat: a header, a command, then its
//! values, and it answers a question in a report of the same shape.
//!
//! `follow` lights every light while any camera streams, by the files a
//! camera's watch keeps in FOLDER (one program a line, empty while nobody
//! streams), puts them out once none does, and puts them out when it's
//! stopped. It sleeps until a file in the folder changes, a light is
//! plugged in, or it's told to stop, so it costs nothing in between.
//!
//! It needs only the standard library, which already links the C library
//! these few calls come from.

use std::ffi::{CString, c_char, c_int};
use std::fs::{self, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::fd::{AsRawFd, FromRawFd};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

/// The lights this speaks to, by the `HID_ID` hidraw's uevent gives them:
/// the Glow, and the Beam, which takes the same reports.
const LIGHTS: [&str; 2] = [
    "HID_ID=0003:0000046D:0000C900",
    "HID_ID=0003:0000046D:0000C901",
];
const REPORT: usize = 20;

const POWER: u8 = 0x1c;
const POWER_IS: u8 = 0x01;
const BRIGHTNESS: u8 = 0x4c;
const BRIGHTNESS_IS: u8 = 0x31;
const TEMPERATURE: u8 = 0x9c;
const TEMPERATURE_IS: u8 = 0x81;
/// The Glow's range: its own units for brightness, kelvin for warmth.
const DIMMEST: u16 = 20;
const BRIGHTEST: u16 = 250;
const WARMEST: u16 = 2700;
const COOLEST: u16 = 6500;

const O_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CLOSE_WRITE: u32 = 0x008;
const IN_MOVED_TO: u32 = 0x080;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
const IN_DELETE_SELF: u32 = 0x400;
const POLLIN: i16 = 0x001;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

/// glibc's `sigset_t`, 1024 bits.
#[repr(C)]
struct SigSet([u64; 16]);

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
}

/// Every light plugged in, as its node in `dev`, by the hidraw nodes `sys`
/// lists.
fn lights(sys: &Path, dev: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(sys.join("class/hidraw")) else {
        return Vec::new();
    };
    let mut names: Vec<_> = entries.flatten().map(|entry| entry.file_name()).collect();
    names.sort();
    names
        .into_iter()
        .filter(|name| {
            let uevent = sys.join("class/hidraw").join(name).join("device/uevent");
            fs::read_to_string(uevent)
                .is_ok_and(|text| text.lines().any(|line| LIGHTS.contains(&line)))
        })
        .map(|name| dev.join(name))
        .collect()
}

/// A report the light reads: its header, the command, then the values.
fn report(command: u8, values: &[u8]) -> [u8; REPORT] {
    let mut report = [0; REPORT];
    report[..4].copy_from_slice(&[0x11, 0xff, 0x04, command]);
    report[4..4 + values.len()].copy_from_slice(values);
    report
}

/// The values in the light's answer to a question, once one for that
/// question comes.
fn answer(reply: &[u8], question: u8) -> Option<&[u8]> {
    match reply {
        [0x11, 0xff, 0x04, code, values @ ..] if *code == question => Some(values),
        _ => None,
    }
}

/// A share of the brightest, as the light's own units.
fn units(percent: u16) -> u16 {
    let percent = percent.min(100);
    DIMMEST + ((BRIGHTEST - DIMMEST) * percent + 50) / 100
}

/// The light's own units as a share of the brightest.
fn percent(units: u16) -> u16 {
    let units = units.clamp(DIMMEST, BRIGHTEST) - DIMMEST;
    (units * 100 + (BRIGHTEST - DIMMEST) / 2) / (BRIGHTEST - DIMMEST)
}

/// Warmth the light can make, to its nearest hundred kelvin.
fn kelvin(asked: u16) -> u16 {
    (asked.clamp(WARMEST, COOLEST) + 50) / 100 * 100
}

struct Light {
    file: File,
}

impl Light {
    fn open(node: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(node)?;
        Ok(Self { file })
    }

    fn tell(&mut self, command: u8, values: &[u8]) -> io::Result<()> {
        self.file.write_all(&report(command, values))
    }

    /// Asks a question and waits up to a second for its answer, passing
    /// over whatever else the light says meanwhile, such as its buttons.
    fn ask(&mut self, question: u8) -> io::Result<Vec<u8>> {
        self.tell(question, &[])?;
        let deadline = Instant::now() + Duration::from_secs(1);
        let mut reply = [0u8; 64];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(io::Error::new(ErrorKind::TimedOut, "no answer"));
            }
            let mut waiting = [PollFd {
                fd: self.file.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            }];
            let millis = c_int::try_from(left.as_millis()).unwrap_or(1000).max(1);
            // SAFETY: one valid pollfd for the length given.
            if unsafe { poll(waiting.as_mut_ptr(), 1, millis) } <= 0 {
                continue;
            }
            match self.file.read(&mut reply) {
                Ok(length) => {
                    if let Some(values) = answer(&reply[..length], question) {
                        return Ok(values.to_vec());
                    }
                }
                Err(error) if error.kind() == ErrorKind::WouldBlock => {}
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
    }
}

/// Tells every light plugged in; says so when there's none.
fn every(command: u8, values: &[u8]) -> ExitCode {
    let found = lights(Path::new("/sys"), Path::new("/dev"));
    if found.is_empty() {
        eprintln!("No Litra is plugged in.");
        return ExitCode::FAILURE;
    }
    let mut failed = false;
    for node in found {
        if let Err(error) = Light::open(&node).and_then(|mut light| light.tell(command, values)) {
            eprintln!("{}: {error}", node.display());
            failed = true;
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn status() -> ExitCode {
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        let said = Light::open(&node).and_then(|mut light| {
            Ok((
                light.ask(POWER_IS)?,
                light.ask(BRIGHTNESS_IS)?,
                light.ask(TEMPERATURE_IS)?,
            ))
        });
        match said {
            Ok((on, level, warmth)) if !on.is_empty() && level.len() >= 2 && warmth.len() >= 2 => {
                let state = if on[0] == 1 { "on" } else { "off" };
                let level = percent(u16::from_be_bytes([level[0], level[1]]));
                let warmth = u16::from_be_bytes([warmth[0], warmth[1]]);
                println!("{}: {state}, {level}%, {warmth} K", node.display());
            }
            Ok(_) => println!("{}: a short answer", node.display()),
            Err(error) => println!("{}: {error}", node.display()),
        }
    }
    ExitCode::SUCCESS
}

/// Whether any camera's file in the folder names a program. A file being
/// replaced is written beside it first, under its name and a number, and
/// a file starting with a dot is someone's half-written one.
fn streaming(folder: &Path) -> bool {
    let Ok(entries) = fs::read_dir(folder) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let temporary = name.starts_with('.')
            || name.rsplit_once('.').is_some_and(|(_, after)| {
                !after.is_empty() && after.bytes().all(|b| b.is_ascii_digit())
            });
        !temporary && fs::read_to_string(entry.path()).is_ok_and(|text| !text.trim().is_empty())
    })
}

/// Sets every light plugged in on or off, saying what failed.
fn light_all(on: bool) {
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        if let Err(error) =
            Light::open(&node).and_then(|mut light| light.tell(POWER, &[u8::from(on)]))
        {
            eprintln!("{}: {error}", node.display());
        }
    }
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

/// Watches the folder and /dev, the folder for a camera's file and /dev
/// for a light coming, or the permissions udev gives it after.
fn changes(folder: &Path) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let watches = [
        (
            folder.as_os_str(),
            IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE | IN_DELETE | IN_DELETE_SELF,
        ),
        (Path::new("/dev").as_os_str(), IN_CREATE | IN_ATTRIB),
    ];
    for (path, mask) in watches {
        let path = CString::new(path.as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        if unsafe { inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(file)
}

/// Whether a run of inotify events says the folder itself has gone.
fn folder_gone(events: &[u8]) -> bool {
    let mut offset = 0;
    while offset + 16 <= events.len() {
        let mask = u32::from_ne_bytes(events[offset + 4..offset + 8].try_into().unwrap());
        let length = u32::from_ne_bytes(events[offset + 12..offset + 16].try_into().unwrap());
        if mask & IN_DELETE_SELF != 0 {
            return true;
        }
        offset += 16 + length as usize;
    }
    false
}

fn follow(folder: &Path) -> ExitCode {
    let (mut stop, mut changed) = match (stopping(), changes(folder)) {
        (Ok(stop), Ok(changed)) => (stop, changed),
        (Err(error), _) | (_, Err(error)) => {
            eprintln!("{}: {error}", folder.display());
            return ExitCode::FAILURE;
        }
    };
    let mut lit = None;
    let mut lights_seen = Vec::new();
    let mut buffer = [0u8; 4096];
    loop {
        let wanted = streaming(folder);
        // A light that has come since is told too, whatever it was doing.
        let now = lights(Path::new("/sys"), Path::new("/dev"));
        if Some(wanted) != lit || now != lights_seen {
            light_all(wanted);
            lit = Some(wanted);
            lights_seen = now;
        }
        let mut waiting = [
            PollFd {
                fd: changed.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            PollFd {
                fd: stop.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
        ];
        // SAFETY: two valid pollfds for the length given.
        if unsafe { poll(waiting.as_mut_ptr(), 2, -1) } < 0 {
            continue;
        }
        if waiting[1].revents & POLLIN != 0 {
            let _ = stop.read(&mut buffer);
            light_all(false);
            return ExitCode::SUCCESS;
        }
        // A camera's watch replaces its file in two steps, and a light's
        // node comes before udev opens it; one look after they settle
        // answers for all of it.
        std::thread::sleep(Duration::from_millis(100));
        match changed.read(&mut buffer) {
            Ok(length) if folder_gone(&buffer[..length]) => {
                light_all(false);
                eprintln!("{} went away.", folder.display());
                return ExitCode::FAILURE;
            }
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                eprintln!("{}: {error}", folder.display());
                return ExitCode::FAILURE;
            }
        }
    }
}

fn number(text: &str, unit: &[char]) -> Option<u16> {
    text.trim_end_matches(unit).parse().ok()
}

const USAGE: &str =
    "usage: litra on | off | status | brightness PERCENT | temperature KELVIN | follow FOLDER";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments[..] {
        ["on"] => every(POWER, &[1]),
        ["off"] => every(POWER, &[0]),
        ["status"] => status(),
        ["brightness", share] if number(share, &['%']).is_some() => {
            let share = number(share, &['%']).unwrap_or_default();
            every(BRIGHTNESS, &units(share).to_be_bytes())
        }
        ["temperature", warmth] if number(warmth, &['K', 'k']).is_some() => {
            let warmth = number(warmth, &['K', 'k']).unwrap_or_default();
            every(TEMPERATURE, &kelvin(warmth).to_be_bytes())
        }
        ["follow", folder] => follow(Path::new(folder)),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A folder of the test's own, gone when it is.
    struct Scratch(PathBuf);

    impl std::ops::Deref for Scratch {
        type Target = Path;
        fn deref(&self) -> &Path {
            &self.0
        }
    }

    impl AsRef<Path> for Scratch {
        fn as_ref(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn root(name: &str) -> Scratch {
        let root = std::env::temp_dir().join(format!("litra-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Scratch(root)
    }

    fn node(root: &Path, name: &str, id: &str) {
        let device = root.join("sys/class/hidraw").join(name).join("device");
        fs::create_dir_all(&device).unwrap();
        fs::write(
            device.join("uevent"),
            format!("DRIVER=hid-generic\nHID_ID={id}\n"),
        )
        .unwrap();
    }

    #[test]
    fn the_lights_are_found_by_their_ids_and_nothing_else() {
        let root = root("found");
        node(&root, "hidraw0", "0003:0000046D:0000091D");
        node(&root, "hidraw1", "0003:0000046D:0000C900");
        node(&root, "hidraw2", "0003:00003297:00004976");
        node(&root, "hidraw3", "0003:0000046D:0000C901");
        assert_eq!(
            lights(&root.join("sys"), Path::new("/dev")),
            [PathBuf::from("/dev/hidraw1"), PathBuf::from("/dev/hidraw3")]
        );
        assert!(lights(&root.join("nothing"), Path::new("/dev")).is_empty());
    }

    #[test]
    fn a_report_is_twenty_bytes_behind_its_header() {
        let made = report(POWER, &[1]);
        assert_eq!(made[..5], [0x11, 0xff, 0x04, 0x1c, 1]);
        assert!(made[5..].iter().all(|byte| *byte == 0));
        assert_eq!(
            report(TEMPERATURE, &4000u16.to_be_bytes())[4..6],
            [0x0f, 0xa0]
        );
    }

    #[test]
    fn an_answer_is_taken_only_for_its_question() {
        let reply = [0x11, 0xff, 0x04, 0x31, 0x00, 0x3b, 0, 0];
        assert_eq!(answer(&reply, BRIGHTNESS_IS), Some(&reply[4..]));
        assert_eq!(answer(&reply, POWER_IS), None);
        assert_eq!(answer(&[0x11, 0xff], POWER_IS), None);
    }

    #[test]
    fn brightness_maps_both_ways_over_the_lights_range() {
        assert_eq!(units(0), DIMMEST);
        assert_eq!(units(100), BRIGHTEST);
        assert_eq!(units(250), BRIGHTEST);
        assert_eq!(units(50), 135);
        for share in 0..=100 {
            assert_eq!(percent(units(share)), share);
        }
    }

    #[test]
    fn warmth_is_held_to_what_the_light_makes() {
        assert_eq!(kelvin(1000), WARMEST);
        assert_eq!(kelvin(9000), COOLEST);
        assert_eq!(kelvin(4049), 4000);
        assert_eq!(kelvin(4050), 4100);
    }

    #[test]
    fn streaming_is_any_cameras_file_naming_a_program() {
        let root = root("streaming");
        assert!(!streaming(&root));
        fs::write(root.join("camera"), "").unwrap();
        fs::write(root.join("MX Brio 705 for Business"), "\n").unwrap();
        assert!(!streaming(&root));
        fs::write(root.join("camera.4123"), "zoom\n").unwrap();
        fs::write(root.join(".camera"), "zoom\n").unwrap();
        assert!(!streaming(&root), "a file being replaced is no camera");
        fs::write(root.join("MX Brio 705 for Business"), "zoom\n").unwrap();
        assert!(streaming(&root));
        assert!(!streaming(&root.join("nothing")));
    }

    #[test]
    fn the_folder_going_is_heard_among_other_events() {
        let mut events = Vec::new();
        for (mask, name) in [
            (IN_CLOSE_WRITE, &b"camera\0\0"[..]),
            (IN_DELETE_SELF, &b""[..]),
        ] {
            events.extend_from_slice(&1i32.to_ne_bytes());
            events.extend_from_slice(&mask.to_ne_bytes());
            events.extend_from_slice(&0u32.to_ne_bytes());
            events.extend_from_slice(&u32::try_from(name.len()).unwrap().to_ne_bytes());
            events.extend_from_slice(name);
        }
        assert!(folder_gone(&events));
        assert!(!folder_gone(&events[..24]));
    }
}

//! Drives a Logitech Litra Glow, the light on a desk's monitor.
//!
//! ```text
//! litra on | off | status [--format json]
//! litra brightness PERCENT
//! litra temperature KELVIN
//! litra follow FOLDER [LOCKED]
//! litra watch
//! ```
//!
//! The light takes 20-byte HID reports on its hidraw node, which systemd's
//! hwdb opens to the person at the seat: a header, a command, then its
//! values, and it answers a question in a report of the same shape.
//!
//! `follow` lights every light while any camera streams, by the files a
//! camera's watch keeps in FOLDER (one program a line, empty while nobody
//! streams), puts them out once none does, and puts them out when it's
//! stopped. Given LOCKED, a file that says "locked" while the screen is,
//! it keeps them out while it says so, which covers sleep too where the
//! screen locks before the machine sleeps; once unlocked, they follow the
//! cameras again, so one lit by its own button stays out. It sleeps until
//! a file in the folder changes, LOCKED is written, a light is plugged in,
//! or it's told to stop, so it costs nothing in between.
//!
//! `watch` prints every light's state as one line of JSON, the same as
//! `status --format json` gives, once at the start and again whenever it
//! changes, for a shell to show it. hidraw gives each reader every report
//! the light sends, so it hears the light's own buttons, which report
//! their new value, and any program's commands, which the light answers
//! with no value, so it asks the light again after one. A light's answer
//! to a question changes nothing but what it knows, so two watches never
//! set each other going. It sleeps until the light says something or a
//! light comes or goes.
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
/// What the light says unasked when its own buttons change it.
const POWER_CHANGED: u8 = 0x00;
const BRIGHTNESS_CHANGED: u8 = 0x10;
const TEMPERATURE_CHANGED: u8 = 0x20;
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
const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;
const LOCK_EX: c_int = 2;
const LOCK_UN: c_int = 8;

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
    fn flock(fd: c_int, operation: c_int) -> c_int;
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

/// Every light plugged in, each with the HID device behind its node. hidraw
/// gives a light plugged back in the name it had before, as nothing took it
/// meanwhile, but the HID device is made anew at every plug, so it tells a
/// light just plugged in, which comes up dark, from one that never left.
fn plugged(sys: &Path, dev: &Path) -> Vec<(PathBuf, Option<PathBuf>)> {
    lights(sys, dev)
        .into_iter()
        .map(|node| {
            let device = node.file_name().and_then(|name| {
                fs::canonicalize(sys.join("class/hidraw").join(name).join("device")).ok()
            });
            (node, device)
        })
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

    /// Sends a report and waits up to a second for the light's reply to it,
    /// an answer to a question or the bare acknowledgement of a command,
    /// handing whatever else it says meanwhile, such as its buttons, to
    /// `other`. The light keeps only the latest of reports sent closer than
    /// about 40 ms apart, so every program speaking to it takes its turn
    /// under a lock on its node, and lets go once the light has replied.
    fn exchange(
        &mut self,
        command: u8,
        values: &[u8],
        other: &mut dyn FnMut(&[u8]),
    ) -> io::Result<Vec<u8>> {
        let fd = self.file.as_raw_fd();
        // SAFETY: a plain call on a descriptor of our own.
        while unsafe { flock(fd, LOCK_EX) } != 0 {
            let error = io::Error::last_os_error();
            if error.kind() != ErrorKind::Interrupted {
                return Err(error);
            }
        }
        let replied = self
            .file
            .write_all(&report(command, values))
            .and_then(|()| self.reply(command, other));
        // SAFETY: as above; closing the file would let go of it too.
        unsafe { flock(fd, LOCK_UN) };
        replied
    }

    /// Tells the light something and waits for it to say it heard.
    fn tell(&mut self, command: u8, values: &[u8]) -> io::Result<()> {
        self.exchange(command, values, &mut |_| {}).map(drop)
    }

    fn reply(&mut self, question: u8, other: &mut dyn FnMut(&[u8])) -> io::Result<Vec<u8>> {
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
                Ok(length) => match answer(&reply[..length], question) {
                    Some(values) => return Ok(values.to_vec()),
                    None => other(&reply[..length]),
                },
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

/// What a light is doing, as far as it has said.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct State {
    on: Option<bool>,
    units: Option<u16>,
    kelvin: Option<u16>,
}

impl State {
    fn complete(&self) -> bool {
        self.on.is_some() && self.units.is_some() && self.kelvin.is_some()
    }
}

/// What a report from the light calls for.
#[derive(Debug, PartialEq)]
enum Heard {
    /// It said something new about itself.
    Changed,
    /// A command changed it without saying to what, so ask.
    Ask(u8),
    Nothing,
}

/// Takes in a report the light sent: a button's new value, an answer to
/// anybody's question, or the bare acknowledgement of anybody's command.
fn hear(state: &mut State, report: &[u8]) -> Heard {
    let [0x11, 0xff, 0x04, code, values @ ..] = report else {
        return Heard::Nothing;
    };
    let word = values
        .get(..2)
        .map(|word| u16::from_be_bytes([word[0], word[1]]));
    let before = *state;
    match *code {
        POWER_CHANGED | POWER_IS => {
            if let Some(on) = values.first() {
                state.on = Some(*on == 1);
            }
        }
        BRIGHTNESS_CHANGED | BRIGHTNESS_IS => state.units = word.or(state.units),
        TEMPERATURE_CHANGED | TEMPERATURE_IS => state.kelvin = word.or(state.kelvin),
        POWER => return Heard::Ask(POWER_IS),
        BRIGHTNESS => return Heard::Ask(BRIGHTNESS_IS),
        TEMPERATURE => return Heard::Ask(TEMPERATURE_IS),
        _ => return Heard::Nothing,
    }
    if *state == before {
        Heard::Nothing
    } else {
        Heard::Changed
    }
}

/// The lights that have said everything about themselves, as JSON.
fn json<'a>(lights: impl IntoIterator<Item = (&'a Path, &'a State)>) -> String {
    let said: Vec<String> = lights
        .into_iter()
        .filter_map(|(node, state)| {
            let (on, units, kelvin) = (state.on?, state.units?, state.kelvin?);
            let node = node
                .display()
                .to_string()
                .replace('\\', "\\\\")
                .replace('"', "\\\"");
            Some(format!(
                r#"{{"node":"{node}","on":{on},"percent":{},"kelvin":{kelvin}}}"#,
                percent(units)
            ))
        })
        .collect();
    format!("[{}]", said.join(","))
}

/// Asks a light everything about itself.
fn ask_state(node: &Path) -> io::Result<State> {
    let mut state = State::default();
    find_out(
        &mut Light::open(node)?,
        &mut state,
        vec![TEMPERATURE_IS, BRIGHTNESS_IS, POWER_IS],
    )?;
    if state.complete() {
        Ok(state)
    } else {
        Err(io::Error::new(ErrorKind::InvalidData, "a short answer"))
    }
}

fn status(as_json: bool) -> ExitCode {
    let mut said = Vec::new();
    for node in lights(Path::new("/sys"), Path::new("/dev")) {
        match ask_state(&node) {
            Ok(state) if as_json => said.push((node, state)),
            Ok(State {
                on: Some(on),
                units: Some(units),
                kelvin: Some(kelvin),
            }) => {
                let on = if on { "on" } else { "off" };
                println!("{}: {on}, {}%, {kelvin} K", node.display(), percent(units));
            }
            Ok(_) => {}
            Err(error) if as_json => eprintln!("{}: {error}", node.display()),
            Err(error) => println!("{}: {error}", node.display()),
        }
    }
    if as_json {
        println!(
            "{}",
            json(said.iter().map(|(node, state)| (node.as_path(), state)))
        );
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

/// Whether a lock's file says the screen is locked. No file says it isn't,
/// as before the shell first writes one.
fn locked(marker: &Path) -> bool {
    fs::read_to_string(marker).is_ok_and(|text| text.trim() == "locked")
}

/// Whether the lights should be lit: while a camera streams, unless the
/// screen is locked.
fn wanted(folder: &Path, lock: Option<&Path>) -> bool {
    streaming(folder) && !lock.is_some_and(locked)
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
/// for a light coming, or the permissions udev gives it after, and the
/// lock's folder, if there is one, for the lock's file being written.
fn changes(folder: &Path, lock: Option<&Path>) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let mut watches = vec![
        (
            folder.as_os_str(),
            IN_CLOSE_WRITE | IN_MOVED_TO | IN_CREATE | IN_DELETE | IN_DELETE_SELF,
        ),
        (Path::new("/dev").as_os_str(), IN_CREATE | IN_ATTRIB),
    ];
    // Its folder rather than the file, since the file can be replaced.
    if let Some(parent) = lock.and_then(Path::parent) {
        watches.push((parent.as_os_str(), IN_CLOSE_WRITE | IN_MOVED_TO));
    }
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

fn follow(folder: &Path, lock: Option<&Path>) -> ExitCode {
    let (mut stop, mut changed) = match (stopping(), changes(folder, lock)) {
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
        let wanted = wanted(folder, lock);
        // A light that has come since is told too, whatever it was doing,
        // and so is one plugged back in under its old name.
        let now = plugged(Path::new("/sys"), Path::new("/dev"));
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

/// Watches /dev for a light's node coming or going, or the permissions
/// udev gives it after it comes.
fn devices() -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    let dev = CString::new("/dev").map_err(io::Error::other)?;
    // SAFETY: the path is a valid C string for the call.
    if unsafe { inotify_add_watch(fd, dev.as_ptr(), IN_CREATE | IN_ATTRIB | IN_DELETE) } < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(file)
}

/// Asks a light each question in turn, last first, taking in whatever else
/// it says meanwhile, and asking about any command that's heard.
fn find_out(light: &mut Light, state: &mut State, mut asking: Vec<u8>) -> io::Result<()> {
    while let Some(question) = asking.pop() {
        let mut heard = Vec::new();
        let values = light.exchange(question, &[], &mut |report| {
            if let Heard::Ask(next) = hear(state, report) {
                heard.push(next);
            }
        })?;
        let mut answer = vec![0x11, 0xff, 0x04, question];
        answer.extend(values);
        hear(state, &answer);
        for next in heard {
            if !asking.contains(&next) {
                asking.insert(0, next);
            }
        }
    }
    Ok(())
}

struct Watched {
    node: PathBuf,
    light: Light,
    state: State,
}

fn watch() -> ExitCode {
    let mut devices = match devices() {
        Ok(devices) => devices,
        Err(error) => {
            eprintln!("/dev: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut watched: Vec<Watched> = Vec::new();
    let mut said: Option<String> = None;
    let mut look = true;
    let mut out = io::stdout().lock();
    let mut buffer = [0u8; 4096];
    loop {
        if look {
            let present = lights(Path::new("/sys"), Path::new("/dev"));
            watched.retain(|light| present.contains(&light.node));
            for node in present {
                if watched.iter().any(|light| light.node == node) {
                    continue;
                }
                // One udev hasn't opened yet is tried again when it does.
                let Ok(mut light) = Light::open(&node) else {
                    continue;
                };
                let mut state = State::default();
                let asking = vec![TEMPERATURE_IS, BRIGHTNESS_IS, POWER_IS];
                if find_out(&mut light, &mut state, asking).is_ok() {
                    watched.push(Watched { node, light, state });
                }
            }
            look = false;
        }

        let line = json(
            watched
                .iter()
                .map(|light| (light.node.as_path(), &light.state)),
        );
        if said.as_ref() != Some(&line) {
            if writeln!(out, "{line}").and_then(|()| out.flush()).is_err() {
                // Nobody is reading any more.
                return ExitCode::SUCCESS;
            }
            said = Some(line);
        }

        let mut waiting = vec![PollFd {
            fd: devices.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        waiting.extend(watched.iter().map(|light| PollFd {
            fd: light.light.file.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }));
        // SAFETY: valid pollfds for the length given.
        if unsafe { poll(waiting.as_mut_ptr(), waiting.len() as u64, -1) } < 0 {
            continue;
        }
        if waiting[0].revents & POLLIN != 0 {
            // A node comes before udev opens it to the seat.
            std::thread::sleep(Duration::from_millis(100));
            let _ = devices.read(&mut buffer);
            look = true;
        }
        let mut gone = Vec::new();
        for (index, light) in watched.iter_mut().enumerate() {
            let revents = waiting[index + 1].revents;
            if revents & (POLLERR | POLLHUP) != 0 {
                gone.push(index);
                continue;
            }
            if revents & POLLIN == 0 {
                continue;
            }
            let mut asking = Vec::new();
            let read = loop {
                match light.light.file.read(&mut buffer) {
                    Ok(0) => break Ok(()),
                    Ok(length) => {
                        if let Heard::Ask(question) = hear(&mut light.state, &buffer[..length])
                            && !asking.contains(&question)
                        {
                            asking.push(question);
                        }
                    }
                    Err(error) if error.kind() == ErrorKind::WouldBlock => break Ok(()),
                    Err(error) if error.kind() == ErrorKind::Interrupted => {}
                    Err(error) => break Err(error),
                }
            };
            if read
                .and_then(|()| find_out(&mut light.light, &mut light.state, asking))
                .is_err()
            {
                gone.push(index);
            }
        }
        for index in gone.into_iter().rev() {
            watched.remove(index);
            look = true;
        }
    }
}

fn number(text: &str, unit: &[char]) -> Option<u16> {
    text.trim_end_matches(unit).parse().ok()
}

const USAGE: &str = "usage: litra on | off | status [--format json] | brightness PERCENT | \
                     temperature KELVIN | follow FOLDER [LOCKED] | watch";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let arguments: Vec<&str> = arguments.iter().map(String::as_str).collect();
    match arguments[..] {
        ["on"] => every(POWER, &[1]),
        ["off"] => every(POWER, &[0]),
        ["status"] => status(false),
        ["status", "--format", "json"] => status(true),
        ["watch"] => watch(),
        ["brightness", share] if number(share, &['%']).is_some() => {
            let share = number(share, &['%']).unwrap_or_default();
            every(BRIGHTNESS, &units(share).to_be_bytes())
        }
        ["temperature", warmth] if number(warmth, &['K', 'k']).is_some() => {
            let warmth = number(warmth, &['K', 'k']).unwrap_or_default();
            every(TEMPERATURE, &kelvin(warmth).to_be_bytes())
        }
        ["follow", folder] => follow(Path::new(folder), None),
        ["follow", folder, lock] => follow(Path::new(folder), Some(Path::new(lock))),
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

    /// A light's node as the kernel makes it: hidraw's entry linked to the
    /// HID device behind it, which is numbered anew at each plug.
    fn plugged_in(root: &Path, name: &str, device: &str) {
        let hid = root.join("sys/devices/virtual/misc/uhid").join(device);
        fs::create_dir_all(&hid).unwrap();
        fs::write(hid.join("uevent"), "HID_ID=0003:0000046D:0000C900\n").unwrap();
        let entry = root.join("sys/class/hidraw").join(name);
        let _ = fs::remove_dir_all(&entry);
        fs::create_dir_all(&entry).unwrap();
        std::os::unix::fs::symlink(&hid, entry.join("device")).unwrap();
    }

    #[test]
    fn a_light_plugged_back_in_under_its_old_name_is_told_from_one_that_stayed() {
        let root = root("replugged");
        plugged_in(&root, "hidraw0", "0003:046D:C900.0001");
        let before = plugged(&root.join("sys"), Path::new("/dev"));
        assert_eq!(before, plugged(&root.join("sys"), Path::new("/dev")));
        plugged_in(&root, "hidraw0", "0003:046D:C900.0002");
        let after = plugged(&root.join("sys"), Path::new("/dev"));
        assert_eq!(
            lights(&root.join("sys"), Path::new("/dev")),
            [PathBuf::from("/dev/hidraw0")]
        );
        assert_ne!(before, after);
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

    /// A report as the light sends it: its header, a code, then values.
    fn from_light(code: u8, values: &[u8]) -> Vec<u8> {
        let mut sent = vec![0x11, 0xff, 0x04, code];
        sent.extend_from_slice(values);
        sent.resize(REPORT, 0);
        sent
    }

    #[test]
    fn the_lights_buttons_say_their_new_values() {
        let mut state = State::default();
        // As a Glow sent them: power on, brightness 84, 4500 K.
        assert_eq!(hear(&mut state, &from_light(0x00, &[1])), Heard::Changed);
        assert_eq!(
            hear(&mut state, &from_light(0x10, &[0x00, 0x54])),
            Heard::Changed
        );
        assert_eq!(
            hear(&mut state, &from_light(0x20, &[0x11, 0x94])),
            Heard::Changed
        );
        assert_eq!(
            state,
            State {
                on: Some(true),
                units: Some(84),
                kelvin: Some(4500),
            }
        );
        assert_eq!(
            hear(&mut state, &from_light(0x20, &[0x11, 0x94])),
            Heard::Nothing
        );
        assert_eq!(hear(&mut state, &from_light(0x00, &[0])), Heard::Changed);
        assert_eq!(state.on, Some(false));
    }

    #[test]
    fn a_command_heard_from_anybody_is_asked_about() {
        let mut state = State::default();
        assert_eq!(
            hear(&mut state, &from_light(POWER, &[])),
            Heard::Ask(POWER_IS)
        );
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS, &[])),
            Heard::Ask(BRIGHTNESS_IS)
        );
        assert_eq!(
            hear(&mut state, &from_light(TEMPERATURE, &[])),
            Heard::Ask(TEMPERATURE_IS)
        );
        assert_eq!(state, State::default());
    }

    #[test]
    fn an_answer_to_anybodys_question_is_taken_and_never_asked_about() {
        let mut state = State::default();
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS_IS, &[0x00, 0x3d])),
            Heard::Changed
        );
        assert_eq!(
            hear(&mut state, &from_light(BRIGHTNESS_IS, &[0x00, 0x3d])),
            Heard::Nothing
        );
        assert_eq!(
            hear(&mut state, &from_light(TEMPERATURE_IS, &[0x13, 0xec])),
            Heard::Changed
        );
        assert_eq!(state.units, Some(61));
        assert_eq!(state.kelvin, Some(5100));
        assert_eq!(hear(&mut state, &[0x11, 0xff]), Heard::Nothing);
        assert_eq!(hear(&mut state, &from_light(0x7f, &[1])), Heard::Nothing);
    }

    #[test]
    fn json_says_only_the_lights_that_have_said_everything() {
        let whole = State {
            on: Some(false),
            units: Some(DIMMEST + 39),
            kelvin: Some(5000),
        };
        let partial = State {
            on: Some(true),
            ..State::default()
        };
        assert_eq!(json([]), "[]");
        assert_eq!(
            json([
                (Path::new("/dev/hidraw1"), &whole),
                (Path::new("/dev/hidraw4"), &partial),
            ]),
            r#"[{"node":"/dev/hidraw1","on":false,"percent":17,"kelvin":5000}]"#
        );
        assert_eq!(
            json([
                (Path::new("/dev/a\"b"), &whole),
                (Path::new("/dev/c"), &whole)
            ]),
            r#"[{"node":"/dev/a\"b","on":false,"percent":17,"kelvin":5000},{"node":"/dev/c","on":false,"percent":17,"kelvin":5000}]"#
        );
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
    fn a_locked_screen_keeps_the_lights_out_while_a_camera_streams() {
        let root = root("locked");
        let cameras = root.join("cameras");
        fs::create_dir_all(&cameras).unwrap();
        fs::write(cameras.join("MX Brio 705 for Business"), "zoom\n").unwrap();
        let marker = root.join("quickshell-locked");
        assert!(wanted(&cameras, None));
        assert!(wanted(&cameras, Some(&marker)), "no file says unlocked");
        fs::write(&marker, "locked\n").unwrap();
        assert!(!wanted(&cameras, Some(&marker)));
        fs::write(&marker, "unlocked\n").unwrap();
        assert!(wanted(&cameras, Some(&marker)));
        fs::write(cameras.join("MX Brio 705 for Business"), "").unwrap();
        assert!(!wanted(&cameras, Some(&marker)));
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

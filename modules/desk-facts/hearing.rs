//! Hearing changes: the kernel's uevents, inotify over the folders facts come
//! from, a built-in card's jacks and the signals that stop it, reading again
//! only the kinds each is about.

use std::ffi::{c_int, c_void};
use std::fs::{self, File};
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use crate::bluetooth::bluetooth;
use crate::cameras::{Loopback, cameras};
use crate::displays::{displays, names_a_monitor};
use crate::inotify::{
    IN_CLOSE_NOWRITE, IN_CLOSE_WRITE, IN_CREATE, IN_DELETE, IN_MOVED_TO, IN_OPEN, Inotify, Kernel,
    names,
};
use crate::keyboards::keyboards;
use crate::own_sound::{a_jack_changed, control, open_control};
use crate::sound::sound;
use crate::watches::Watches;
use crate::writing::{json, write};

const AF_NETLINK: c_int = 16;
const SOCK_DGRAM: c_int = 2;
const SOCK_NONBLOCK: c_int = 0o4000;
const SOCK_CLOEXEC: c_int = 0o2_000_000;
const NETLINK_KOBJECT_UEVENT: c_int = 15;
const POLLIN: i16 = 0x001;
const POLLERR: i16 = 0x008;
const POLLHUP: i16 = 0x010;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

/// The kinds of fact, as bits, so a change reads again only its own.
const DISPLAYS: u8 = 1;
const SOUND: u8 = 2;
const CAMERAS: u8 = 4;
const BLUETOOTH: u8 = 8;
const KEYBOARDS: u8 = 16;
/// Every kind, read again when what changed can't be known.
const EVERY: u8 = DISPLAYS | SOUND | CAMERAS | BLUETOOTH | KEYBOARDS;
/// A socket's refusal once its buffer ran over, losing what it held.
const ENOBUFS: i32 = 105;
/// The kernel's uevents that change a fact, and the kinds each changes.
const HEARD: [(&[u8], u8); 7] = [
    (b"SUBSYSTEM=drm", DISPLAYS),
    (b"SUBSYSTEM=sound", SOUND),
    (b"SUBSYSTEM=usb", SOUND | CAMERAS),
    (b"SUBSYSTEM=video4linux", CAMERAS),
    (b"SUBSYSTEM=typec", DISPLAYS),
    (b"SUBSYSTEM=hid", BLUETOOTH),
    (b"SUBSYSTEM=input", BLUETOOTH | KEYBOARDS),
];
/// Where each camera's holders are kept (modules/usb-camera-holders).
const HOLDERS: &str = "/run/camera-holders";

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
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
}

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

/// How often a folder that can't be watched yet is tried again, at the
/// least, since nothing tells of room come free for a watch or inotify.
const RETRY: Duration = Duration::from_secs(10);

/// How long poll waits: until the next read while sound settles, no longer
/// than `RETRY` while a folder waits to be watched, and otherwise until
/// something is heard.
fn waiting_for(settling: Option<(Instant, usize)>, retrying: bool, now: Instant) -> c_int {
    let settles = settling
        .and_then(|(since, step)| SETTLING.get(step).map(|after| since + *after))
        .map(|at| at.saturating_duration_since(now));
    [settles, retrying.then_some(RETRY)]
        .into_iter()
        .flatten()
        .min()
        .map_or(-1, |left| {
            c_int::try_from(left.as_millis()).unwrap_or(c_int::MAX)
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
    names(events, |name| {
        name.starts_with(b"pcm") || name.starts_with(b"control") || name == lighting.as_bytes()
    })
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
/// `heard` found it to change: every kind, once the uevents' socket says
/// its buffer ran over and lost some. A read interrupted is read again,
/// and any other refusal is the error.
fn heard(file: &mut impl Read, heard: &dyn Fn(&[u8]) -> u8) -> io::Result<u8> {
    let mut kinds = 0;
    let mut buffer = [0; 8192];
    loop {
        match file.read(&mut buffer) {
            Ok(0) => return Ok(kinds),
            Ok(read) => kinds |= heard(&buffer[..read]),
            Err(error) if error.kind() == io::ErrorKind::WouldBlock => return Ok(kinds),
            Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
            Err(error) if error.raw_os_error() == Some(ENOBUFS) => {
                eprintln!("<4>uevents were lost, so every fact is read again");
                kinds = EVERY;
            }
            Err(error) => return Err(error),
        }
    }
}

/// What `run` hears and reads from: the files it writes, the Pebble's
/// lighting file, the loopback cameras it's told of, and the control
/// device of each card not on USB, by the card's number.
struct Desk {
    folder: PathBuf,
    files: [PathBuf; 5],
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
                folder.join("keyboards.json"),
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
        for (watched, (number, control)) in waiting.iter_mut().zip(&mut self.controls) {
            if watched.revents & (POLLERR | POLLHUP) != 0 {
                watched.fd = -1;
                due |= SOUND;
            } else if watched.revents & POLLIN != 0 {
                let jacks = heard(control, &|events| {
                    if a_jack_changed(events) { SOUND } else { 0 }
                });
                // One that can't be read is as one gone.
                due |= jacks.unwrap_or_else(|error| {
                    eprintln!("<4>can't read card {number}'s controls: {error}");
                    watched.fd = -1;
                    SOUND
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

/// Writes the files of the kinds that are due: displays, sound, cameras,
/// Bluetooth and keyboards.
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
    if due & KEYBOARDS != 0 {
        written = written.and(write(&desk.files[4], &json(&keyboards(sys))));
    }
    if let Err(error) = written {
        eprintln!("<4>can't write the desk's facts: {error}");
    }
}

/// The folders facts come from, watched: sound's, cameras' holders and the
/// monitors' facts.
struct Folders<K: Kernel = Inotify> {
    changes: Watches<K>,
    held: Watches<K>,
    said: Watches<K>,
}

impl<K: Kernel> Folders<K> {
    /// The descriptors poll waits on, which change as inotify starts.
    fn fds(&self) -> [c_int; 3] {
        [self.changes.fd(), self.held.fd(), self.said.fd()]
    }

    /// Whether a folder waits with nothing to wake it, so poll wakes to try
    /// it again.
    fn wait(&self) -> bool {
        self.unheard() != 0
    }

    /// The kinds of the folders that wait with nothing to wake them, which
    /// are read each time poll's wait runs out, so a folder refused for a
    /// while, or for good, leaves its kind's facts no staler than `RETRY`.
    fn unheard(&self) -> u8 {
        [
            (&self.changes, SOUND),
            (&self.held, CAMERAS),
            (&self.said, DISPLAYS),
        ]
        .into_iter()
        .filter(|(watches, _)| watches.waits())
        .fold(0, |kinds, (_, kind)| kinds | kind)
    }

    /// The kinds the folders say changed. A folder watched since, or
    /// removed, has its kind read again for what it holds now. A folder
    /// refused in a way waiting can't mend is an error.
    fn heard(&mut self) -> io::Result<u8> {
        let mut due = 0;
        let (events, moved) = self.changes.read()?;
        if moved || names_one(&events, "pebble-lights") {
            due |= SOUND;
        }
        let (events, moved) = self.held.read()?;
        if moved || !events.is_empty() {
            due |= CAMERAS;
        }
        let (events, moved) = self.said.read()?;
        if moved || names_a_monitor(&events) {
            due |= DISPLAYS;
        }
        Ok(due)
    }
}

/// Points poll's slots for the folders at their inotify, which changes as
/// it starts, and gives how long poll waits: no longer than `RETRY` while a
/// folder waits with nothing to wake it.
fn asked<K: Kernel>(
    folders: &Folders<K>,
    settling: &Settling,
    slots: &mut [PollFd],
    now: Instant,
) -> c_int {
    for (slot, fd) in slots.iter_mut().zip(folders.fds()) {
        slot.fd = fd;
    }
    waiting_for(settling.since, folders.wait(), now)
}

/// What's read again while it settles, and how far it has: sound, and
/// cameras, whose sensor powers up a moment after a program takes the
/// loopback.
#[derive(Default)]
struct Settling {
    /// When it began, and how many of `SETTLING`'s reads are done.
    since: Option<(Instant, usize)>,
    kinds: u8,
}

impl Settling {
    /// Settles anew the kinds among `due` that settle.
    fn begin(&mut self, due: u8, now: Instant) {
        if due & (SOUND | CAMERAS) != 0 {
            self.since = Some((now, 0));
            self.kinds |= due & (SOUND | CAMERAS);
        }
    }

    /// The kinds read again now, if their next read is due.
    fn step(&mut self, now: Instant) -> u8 {
        if waiting_for(self.since, false, now) != 0 {
            return 0;
        }
        self.since = settled_once(self.since);
        let due = self.kinds;
        if self.since.is_none() {
            self.kinds = 0;
        }
        due
    }
}

/// What's due once poll's wait runs out: settling's next read if it's due,
/// and, while a folder waits with nothing to wake it, what the folders say
/// once each is tried again, which settles as anything heard does, and the
/// kinds of those still waiting, as nothing says when theirs change.
fn timed_out<K: Kernel>(
    folders: &mut Folders<K>,
    settling: &mut Settling,
    now: Instant,
) -> io::Result<u8> {
    let mut due = settling.step(now);
    if folders.wait() {
        let heard = folders.heard()?;
        settling.begin(heard, now);
        due |= heard | folders.unheard();
    }
    Ok(due)
}

/// Ends the hearing with its facts removed, so nothing reads them stale:
/// a run's that failed, or one killed before it could remove them, which
/// the next run finds if it fails at its start.
fn end(desk: &Desk, code: ExitCode) -> ExitCode {
    for file in &desk.files {
        let _ = fs::remove_file(file);
    }
    code
}

/// What `run` starts with, or, when it can't start, the refusal said and
/// the hearing ended with its facts removed, which a run killed before it
/// could remove them may have left.
fn started<T>(desk: &Desk, opened: Result<T, String>) -> Result<T, ExitCode> {
    opened.map_err(|refusal| {
        eprintln!("<3>{refusal}");
        end(desk, ExitCode::FAILURE)
    })
}

/// The stop signals', the uevents' and the folders' descriptors.
type Opened = (File, File, Folders);

/// Opens what `run` hears, by the folders each `Watches` is given.
fn opened(
    sounds: &[(PathBuf, u32)],
    holders: &[(PathBuf, u32)],
    monitors: &[(PathBuf, u32)],
) -> Result<Opened, String> {
    match (
        stopping(),
        uevents(),
        Watches::new(sounds),
        Watches::new(holders),
        Watches::new(monitors),
    ) {
        (Ok(stop), Ok(kernel), Ok(changes), Ok(held), Ok(said)) => Ok((
            stop,
            kernel,
            Folders {
                changes,
                held,
                said,
            },
        )),
        (Err(error), ..)
        | (_, Err(error), ..)
        | (_, _, Err(error), ..)
        | (_, _, _, Err(error), _)
        | (.., Err(error)) => Err(format!("can't hear the desk's devices: {error}")),
    }
}

pub(crate) fn run(folder: &Path, loopbacks: Vec<Loopback>) -> ExitCode {
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
    let mut desk = Desk::new(folder, lighting, loopbacks);
    // Made before it's watched for the monitors' facts.
    let made = fs::create_dir_all(folder)
        .map_err(|error| format!("can't make {}: {error}", folder.display()));
    if let Err(code) = started(&desk, made) {
        return code;
    }
    let monitors = vec![(folder.to_owned(), IN_MOVED_TO | IN_DELETE)];
    let (mut stop, mut kernel, mut folders) =
        match started(&desk, opened(&sounds, &holders, &monitors)) {
            Ok(opened) => opened,
            Err(code) => return code,
        };
    let mut due = DISPLAYS | SOUND | CAMERAS | BLUETOOTH | KEYBOARDS;
    let mut settling = Settling::default();
    loop {
        rewrite(due, &mut desk);
        let mut waiting: Vec<PollFd> = [stop.as_raw_fd(), kernel.as_raw_fd(), -1, -1, -1]
            .into_iter()
            .chain(desk.controls.iter().map(|(_, file)| file.as_raw_fd()))
            .map(PollFd::reading)
            .collect();
        due = 0;
        while due == 0 {
            let wait = asked(&folders, &settling, &mut waiting[2..5], Instant::now());
            let count = waiting.len() as u64;
            // SAFETY: valid pollfds for the length given.
            let ready = unsafe { poll(waiting.as_mut_ptr(), count, wait) };
            if ready < 0 {
                continue;
            }
            // A fault of the program's, or inotify failing a read, fails
            // the unit, as at the start, so systemd starts it afresh, ever
            // more slowly, the fault in the journal each time.
            let heard = if ready == 0 {
                timed_out(&mut folders, &mut settling, Instant::now())
            } else if waiting[0].revents & POLLIN != 0 {
                let _ = stop.read(&mut [0; 128]);
                return end(&desk, ExitCode::SUCCESS);
            } else {
                due |= desk.heard_from_controls(&mut waiting[5..]);
                // A card's devices, a display's EDID and a stream's
                // parameters settle a moment after the first word of them.
                std::thread::sleep(Duration::from_millis(150));
                let said = heard(&mut kernel, &matters)
                    .map_err(|error| io::Error::new(error.kind(), format!("uevents: {error}")))
                    .and_then(|kinds| {
                        due |= kinds;
                        folders.heard()
                    });
                if let Ok(kinds) = said {
                    settling.begin(due | kinds, Instant::now());
                }
                said
            };
            match heard {
                Ok(heard) => due |= heard,
                Err(error) => {
                    eprintln!("<3>can't hear the desk's devices: {error}");
                    return end(&desk, ExitCode::FAILURE);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;

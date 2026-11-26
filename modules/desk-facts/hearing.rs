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
    IN_CLOSE_NOWRITE, IN_CLOSE_WRITE, IN_CREATE, IN_DELETE, IN_MOVED_TO, IN_OPEN, names,
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

/// The kinds the folders watched say changed: sound's, cameras' holders and
/// the monitors' facts. A folder watched since, or removed, has its kind
/// read again for what it holds now.
fn heard_from_folders(changes: &mut Watches, held: &mut Watches, said: &mut Watches) -> u8 {
    let mut due = 0;
    let (events, moved) = changes.read();
    if moved || names_one(&events, "pebble-lights") {
        due |= SOUND;
    }
    let (events, moved) = held.read();
    if moved || !events.is_empty() {
        due |= CAMERAS;
    }
    let (events, moved) = said.read();
    if moved || names_a_monitor(&events) {
        due |= DISPLAYS;
    }
    due
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
    // Made before it's watched for the monitors' facts.
    if let Err(error) = fs::create_dir_all(folder) {
        eprintln!("<3>can't make {}: {error}", folder.display());
        return ExitCode::FAILURE;
    }
    let monitors = vec![(folder.to_owned(), IN_MOVED_TO | IN_DELETE)];
    let (mut stop, mut kernel, mut changes, mut held, mut said) = match (
        stopping(),
        uevents(),
        Watches::new(&sounds),
        Watches::new(&holders),
        Watches::new(&monitors),
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
    let mut due = DISPLAYS | SOUND | CAMERAS | BLUETOOTH | KEYBOARDS;
    let mut settling = None;
    // What's read again while it settles: sound, and cameras, whose sensor
    // powers up a moment after a program takes the loopback.
    let mut unsettled = 0;
    loop {
        rewrite(due, &mut desk);
        let mut waiting: Vec<PollFd> = [
            stop.as_raw_fd(),
            kernel.as_raw_fd(),
            changes.fd(),
            held.fd(),
            said.fd(),
        ]
        .into_iter()
        .chain(desk.controls.iter().map(|(_, file)| file.as_raw_fd()))
        .map(PollFd::reading)
        .collect();
        due = 0;
        while due == 0 {
            let retrying = changes.waits() || held.waits() || said.waits();
            let wait = waiting_for(settling, retrying, Instant::now());
            let count = waiting.len() as u64;
            // SAFETY: valid pollfds for the length given.
            let ready = unsafe { poll(waiting.as_mut_ptr(), count, wait) };
            if ready < 0 {
                continue;
            }
            if ready == 0 {
                // Woken for settling, or to try again what can't be watched.
                if waiting_for(settling, false, Instant::now()) == 0 {
                    settling = settled_once(settling);
                    due |= unsettled;
                    if settling.is_none() {
                        unsettled = 0;
                    }
                }
                if retrying {
                    due |= heard_from_folders(&mut changes, &mut held, &mut said);
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
            due |= heard_from_folders(&mut changes, &mut held, &mut said);
            if due & (SOUND | CAMERAS) != 0 {
                settling = Some((Instant::now(), 0));
                unsettled |= due & (SOUND | CAMERAS);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inotify::IN_Q_OVERFLOW;
    use crate::tests::*;

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
        assert_eq!(matters(&input), BLUETOOTH | KEYBOARDS);
    }

    #[test]
    fn sound_is_read_again_as_it_settles_and_then_only_when_heard() {
        let since = Instant::now();
        assert_eq!(
            waiting_for(None, false, since),
            -1,
            "nothing settling waits for events"
        );
        assert_eq!(waiting_for(Some((since, 0)), false, since), 250);
        assert_eq!(waiting_for(Some((since, 3)), false, since), 2000);
        assert_eq!(
            waiting_for(Some((since, 1)), false, since + Duration::from_secs(9)),
            0,
            "one overdue reads at once"
        );
        assert_eq!(
            [
                waiting_for(None, true, since),
                waiting_for(Some((since, 3)), true, since),
                waiting_for(Some((since, 0)), true, since),
            ],
            [10_000, 2000, 250],
            "a folder waiting to be watched didn't bound the wait at RETRY, or overrode \
             settling's sooner one"
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
    fn a_sound_device_opening_or_the_lighting_changing_is_heard() {
        assert!(
            names_one(&event(IN_OPEN, "pcmC1D0p"), "pebble-lights"),
            "a sound device opening"
        );
        assert!(
            names_one(&event(IN_MOVED_TO, "pebble-lights"), "pebble-lights"),
            "the lighting written whole"
        );
        let mut other = event(IN_CLOSE_WRITE, "quickshell-locked");
        other.extend(event(IN_CLOSE_WRITE, "pebble-lights.new"));
        assert!(
            !names_one(&other, "pebble-lights"),
            "neither the lock nor the lighting's copy"
        );
        assert!(
            names_one(&event(IN_Q_OVERFLOW, ""), "pebble-lights"),
            "an overflow, which may have hidden either"
        );
    }
}

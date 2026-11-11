//! Puts the Creative Pebble X Plus's lighting out while nobody's using the
//! speakers, and back on the moment somebody is.
//!
//! ```text
//! pebble-lights LOCKED [IDLE_SECONDS]
//! ```
//!
//! The lighting is on while anything plays to the speakers, and for
//! `IDLE_SECONDS` (15 minutes unless given) after the last sound, or after the
//! screen was last unlocked, which means someone has come back. It goes out
//! once that's passed, at once when the screen locks with nothing playing,
//! which covers sleep, since the screen locks before every one, and when
//! this is stopped, as at shutdown, since the speakers stay powered from
//! the monitor. LOCKED is the file the shell's lock writes "locked" to while
//! it holds the screen.
//!
//! What plays is read from the speakers' own sound card: PipeWire opens its
//! playback device while anything plays to it and closes it about five
//! seconds after the last sound, and inotify says when either happens, so
//! nothing is looked at between. The lighting is switched over the
//! speakers' control channel, which firmware 1.27 and later offer: HID
//! reports `03 6a <command> <length LE16> <payload>`, command 0x3a for the
//! lighting, 0x25 to switch it and 0x26 for whether it's on, each set
//! acknowledged with command 0x02. Only its power is switched, once a
//! change, never its brightness: the speakers keep their lighting's settings
//! across power cycles, so a fade's dozens of writes a change could wear
//! whatever holds them.
//!
//! Someone switching the lighting at the speakers wins: switched off there,
//! it stays off until it's switched on again, and switched on there, it
//! counts as someone using them.
//!
//! What the lighting is doing, "on", "off" or "off by hand", goes to a
//! file named `pebble-lights` beside LOCKED, for the launcher's facts
//! (modules/desk-facts), and the file goes when the speakers do or this
//! stops.
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

/// The speakers, as hidraw's uevent and their USB device name them.
const SPEAKERS: &str = "HID_ID=0003:0000041E:0000329A";
const VENDOR: &str = "041e";
const PRODUCT: &str = "329a";
/// The control collection's usage page, 0xFF01, as a HID descriptor gives
/// it; the media keys' collection and the firmware update's have others.
const CONTROL_PAGE: [u8; 3] = [0x06, 0x01, 0xFF];

const REPORT_ID: u8 = 0x03;
const REPORT: usize = 64;
const TRANSPORT: u8 = 0x6A;
const ACKNOWLEDGE: u8 = 0x02;
const LIGHTING: u8 = 0x3A;
const SWITCH: u8 = 0x25;
const SWITCHED: u8 = 0x26;

const IDLE: Duration = Duration::from_mins(15);
/// How long the speakers have to acknowledge a command.
const ANSWERING: Duration = Duration::from_millis(300);

const O_NONBLOCK: c_int = 0o4000;
const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_ATTRIB: u32 = 0x004;
const IN_CLOSE_WRITE: u32 = 0x008;
const IN_CLOSE_NOWRITE: u32 = 0x010;
const IN_OPEN: u32 = 0x020;
const IN_MOVED_TO: u32 = 0x080;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
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

/// The speakers' control node in `dev`, by the hidraw nodes `sys` lists:
/// theirs, and the one whose descriptor holds the control collection.
fn control(sys: &Path, dev: &Path) -> Option<PathBuf> {
    let class = sys.join("class/hidraw");
    let mut names: Vec<_> = fs::read_dir(&class)
        .ok()?
        .flatten()
        .map(|entry| entry.file_name())
        .collect();
    names.sort();
    names
        .into_iter()
        .find(|name| {
            let device = class.join(name).join("device");
            let theirs = fs::read_to_string(device.join("uevent"))
                .is_ok_and(|text| text.lines().any(|line| line == SPEAKERS));
            theirs
                && fs::read(device.join("report_descriptor"))
                    .is_ok_and(|descriptor| descriptor.windows(3).any(|w| w == CONTROL_PAGE))
        })
        .map(|name| dev.join(name))
}

/// The number of the speakers' sound card, by the USB device it sits on: a
/// card's device is a USB interface, whose parent is the device.
fn card(sys: &Path) -> Option<u32> {
    let mut found: Vec<u32> = fs::read_dir(sys.join("class/sound"))
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let number = entry
                .file_name()
                .to_str()?
                .strip_prefix("card")?
                .parse()
                .ok()?;
            let interface = fs::canonicalize(entry.path().join("device")).ok()?;
            let usb = interface.parent()?;
            let read = |name: &str| fs::read_to_string(usb.join(name)).unwrap_or_default();
            (read("idVendor").trim() == VENDOR && read("idProduct").trim() == PRODUCT)
                .then_some(number)
        })
        .collect();
    found.sort_unstable();
    found.first().copied()
}

/// Whether a PCM's status in /proc says it's open; "closed" is all it says
/// when it isn't.
fn open(status: &str) -> bool {
    let status = status.trim();
    !status.is_empty() && status != "closed"
}

/// Whether the lock's file says the screen is locked. No file says it isn't.
fn locked(text: &str) -> bool {
    text.trim() == "locked"
}

/// Whether the lighting should be on: while sound plays, and for `idle`
/// after the last of it or the last unlock, unless the screen is locked.
fn wanted(playing: bool, locked: bool, quiet: Duration, idle: Duration) -> bool {
    playing || (!locked && quiet < idle)
}

/// A report for the speakers: its id, the transport, the command, the
/// payload's length and the payload, padded to a whole report.
fn frame(command: u8, payload: &[u8]) -> [u8; REPORT] {
    let mut report = [0; REPORT];
    let length = u16::try_from(payload.len()).unwrap_or(0).to_le_bytes();
    report[..5].copy_from_slice(&[REPORT_ID, TRANSPORT, command, length[0], length[1]]);
    report[5..5 + payload.len()].copy_from_slice(payload);
    report
}

/// What a report from the speakers says, as far as the lighting goes.
#[derive(Debug, PartialEq, Eq)]
enum Said {
    /// A lighting command was taken, or refused.
    Acknowledged {
        operation: u8,
        taken: bool,
    },
    /// The lighting is on or off, as an answer or of its own accord.
    Lit(bool),
    Other,
}

fn said(report: &[u8]) -> Said {
    let [REPORT_ID, TRANSPORT, command, low, high, ref rest @ ..] = *report else {
        return Said::Other;
    };
    let Some(payload) = rest.get(..usize::from(u16::from_le_bytes([low, high]))) else {
        return Said::Other;
    };
    match (command, payload) {
        (ACKNOWLEDGE, &[LIGHTING, status, operation, ..]) => Said::Acknowledged {
            operation,
            taken: status == 0,
        },
        (LIGHTING, &[SWITCHED, value, ..]) => Said::Lit(value != 0),
        _ => Said::Other,
    }
}

/// The speakers' control channel, opened.
struct Speakers {
    file: File,
}

impl Speakers {
    fn open(node: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(O_NONBLOCK)
            .open(node)?;
        Ok(Self { file })
    }

    /// Sends a lighting command and waits for the speakers to take it,
    /// handing whatever else they say meanwhile to `heard`.
    fn tell(&mut self, payload: &[u8], heard: &mut dyn FnMut(Said)) -> io::Result<()> {
        self.file.write_all(&frame(LIGHTING, payload))?;
        let deadline = Instant::now() + ANSWERING;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Err(io::Error::new(ErrorKind::TimedOut, "they didn't answer"));
            }
            let millis = c_int::try_from(left.as_millis()).unwrap_or(1).max(1);
            let mut waiting = [PollFd {
                fd: self.file.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            }];
            // SAFETY: one valid pollfd for the length given.
            if unsafe { poll(waiting.as_mut_ptr(), 1, millis) } <= 0 {
                continue;
            }
            let mut report = [0; REPORT];
            match self.file.read(&mut report) {
                Ok(length) => match said(&report[..length]) {
                    Said::Acknowledged { operation, taken } if operation == payload[0] => {
                        return if taken {
                            Ok(())
                        } else {
                            Err(io::Error::other("they refused it"))
                        };
                    }
                    other => heard(other),
                },
                Err(error)
                    if matches!(error.kind(), ErrorKind::WouldBlock | ErrorKind::Interrupted) => {}
                Err(error) => return Err(error),
            }
        }
    }

    /// Everything the speakers have said since last asked.
    fn heard(&mut self) -> io::Result<Vec<Said>> {
        let mut heard = Vec::new();
        let mut report = [0; REPORT];
        loop {
            match self.file.read(&mut report) {
                Ok(length) => heard.push(said(&report[..length])),
                Err(error) if error.kind() == ErrorKind::WouldBlock => return Ok(heard),
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
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

/// The watches: /dev for the control node coming or being opened to the
/// seat, /dev/snd for a sound card coming or going, the lock's folder for
/// the lock's file being written, and, once there is one, the speakers'
/// playback device for being opened and closed.
struct Watches {
    file: File,
    /// Whether /dev/snd is watched, which a machine with no sound card yet
    /// hasn't made; /dev's watch hears it come.
    sound: bool,
    playback: Option<u32>,
}

impl Watches {
    fn new(lock: &Path) -> io::Result<Self> {
        // SAFETY: a plain call.
        let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the descriptor is ours alone, and the file closes it.
        let watches = Self {
            file: unsafe { File::from_raw_fd(fd) },
            sound: false,
            playback: None,
        };
        watches.add(Path::new("/dev"), IN_CREATE | IN_DELETE | IN_ATTRIB)?;
        if let Some(folder) = lock.parent() {
            watches.add(folder, IN_CLOSE_WRITE | IN_MOVED_TO)?;
        }
        Ok(watches)
    }

    fn add(&self, path: &Path, mask: u32) -> io::Result<()> {
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        if unsafe { inotify_add_watch(self.file.as_raw_fd(), path.as_ptr(), mask) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }

    /// Watches a card's playback device, unless it already is. A device
    /// that goes takes its watch with it, so a card that comes back under
    /// the same number is watched anew.
    fn follow(&mut self, card: Option<u32>) {
        if !self.sound {
            self.sound = self
                .add(Path::new("/dev/snd"), IN_CREATE | IN_DELETE)
                .is_ok();
        }
        if card == self.playback {
            return;
        }
        self.playback = card.filter(|card| {
            let node = PathBuf::from(format!("/dev/snd/pcmC{card}D0p"));
            self.add(&node, IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE)
                .is_ok()
        });
    }

    /// Takes every event waiting, which says only that something changed.
    fn drain(&mut self) {
        let mut events = [0; 4096];
        while self.file.read(&mut events).is_ok_and(|read| read > 0) {}
    }
}

/// What the daemon knows.
struct Lights {
    lock: PathBuf,
    idle: Duration,
    speakers: Option<Speakers>,
    node: Option<PathBuf>,
    card: Option<u32>,
    playing: bool,
    locked: bool,
    /// When sound last stopped, or the screen was last unlocked.
    active: Instant,
    /// Whether the lighting is on, as the speakers last said or were told.
    lit: Option<bool>,
    /// What it was last told, until the speakers say it.
    told: Option<bool>,
    /// Whether someone switched it off at the speakers.
    by_hand: bool,
    /// Where what it's doing is said, and what was last said there.
    said: PathBuf,
    saying: Option<&'static str>,
}

impl Lights {
    fn new(lock: PathBuf, idle: Duration) -> Self {
        let said = lock.with_file_name("pebble-lights");
        Self {
            said,
            saying: None,
            lock,
            idle,
            speakers: None,
            node: None,
            card: None,
            playing: false,
            locked: false,
            active: Instant::now(),
            lit: None,
            told: None,
            by_hand: false,
        }
    }

    /// Looks at everything again: which speakers there are, what plays, and
    /// whether the screen is locked.
    fn look(&mut self) {
        let node = control(Path::new("/sys"), Path::new("/dev"));
        if node != self.node || (node.is_some() && self.speakers.is_none()) {
            self.speakers = node.as_deref().and_then(|node| Speakers::open(node).ok());
            if self.speakers.is_some() {
                eprintln!("<6>the Pebble X Plus's lighting is followed");
                self.lit = None;
                self.told = None;
                self.by_hand = false;
            }
            self.node = node;
        }
        self.card = card(Path::new("/sys"));
        let playing = self.card.is_some_and(|card| {
            fs::read_to_string(format!("/proc/asound/card{card}/pcm0p/sub0/status"))
                .is_ok_and(|status| open(&status))
        });
        if self.playing && !playing {
            self.active = Instant::now();
        }
        self.playing = playing;
        let locked = fs::read_to_string(&self.lock).is_ok_and(|text| locked(&text));
        if self.locked && !locked {
            self.active = Instant::now();
        }
        self.locked = locked;
    }

    /// Takes in what the speakers said of their own accord.
    fn hear(&mut self, said: &[Said]) {
        for said in said {
            let &Said::Lit(lit) = said else { continue };
            if self.told == Some(lit) {
                self.told = None;
            } else if self.lit != Some(lit) {
                eprintln!(
                    "<6>the Pebble X Plus's lighting was switched {} at the speakers",
                    if lit { "on" } else { "off" }
                );
                self.by_hand = !lit;
                if lit {
                    self.active = Instant::now();
                }
            }
            self.lit = Some(lit);
        }
    }

    fn wanted(&self) -> bool {
        let quiet = if self.playing {
            Duration::ZERO
        } else {
            self.active.elapsed()
        };
        !self.by_hand && wanted(self.playing, self.locked, quiet, self.idle)
    }

    /// Why the lighting goes as it does, for the journal.
    fn why(&self, on: bool) -> &'static str {
        match (on, self.playing, self.locked) {
            (true, true, _) => "sound is playing",
            (true, false, _) => "someone's back",
            (false, _, true) => "the screen is locked",
            (false, _, false) => "nothing has played for a while",
        }
    }

    /// Switches the lighting to what's wanted, unless it already is.
    fn settle(&mut self, on: bool, why: &str) {
        if self.lit == Some(on) {
            return;
        }
        let Some(speakers) = self.speakers.as_mut() else {
            return;
        };
        let mut heard = Vec::new();
        match speakers.tell(&[SWITCH, u8::from(on)], &mut |said| heard.push(said)) {
            Ok(()) => {
                eprintln!(
                    "<6>the Pebble X Plus's lighting goes {}: {why}",
                    if on { "on" } else { "out" }
                );
                // Told first, so their saying it as they switch is theirs.
                self.told = Some(on);
                self.lit = Some(on);
                self.hear(&heard);
            }
            Err(error) => {
                eprintln!("<4>can't switch the Pebble X Plus's lighting: {error}");
                self.speakers = None;
            }
        }
    }

    /// What the lighting is doing, in words, while there are speakers.
    fn doing(&self) -> Option<&'static str> {
        self.speakers.as_ref()?;
        match (self.lit?, self.by_hand) {
            (true, _) => Some("on"),
            (false, true) => Some("off by hand"),
            (false, false) => Some("off"),
        }
    }

    /// Says what the lighting is doing, when that's changed: written whole
    /// through a copy renamed over the file, or the file removed.
    fn say(&mut self) {
        let doing = self.doing();
        if doing == self.saying {
            return;
        }
        let copy = self.said.with_extension("new");
        let said = match doing {
            Some(words) => {
                fs::write(&copy, format!("{words}\n")).and_then(|()| fs::rename(&copy, &self.said))
            }
            None => fs::remove_file(&self.said).or_else(|error| {
                if error.kind() == ErrorKind::NotFound {
                    Ok(())
                } else {
                    Err(error)
                }
            }),
        };
        match said {
            Ok(()) => self.saying = doing,
            Err(error) => eprintln!("<4>can't say what the lighting does: {error}"),
        }
    }

    /// How long until the lighting should go out of itself, if it will.
    fn until_quiet(&self) -> Option<Duration> {
        (self.lit == Some(true) && !self.playing && !self.locked && !self.by_hand)
            .then(|| self.idle.saturating_sub(self.active.elapsed()))
    }
}

fn run(lock: PathBuf, idle: Duration) -> ExitCode {
    let (mut stop, mut watches) = match (stopping(), Watches::new(&lock)) {
        (Ok(stop), Ok(watches)) => (stop, watches),
        (Err(error), _) | (_, Err(error)) => {
            eprintln!("<3>can't watch the speakers: {error}");
            return ExitCode::FAILURE;
        }
    };
    let mut lights = Lights::new(lock, idle);
    loop {
        lights.look();
        watches.follow(lights.card);
        let heard = lights
            .speakers
            .as_mut()
            .map_or(Ok(Vec::new()), Speakers::heard);
        match heard {
            Ok(heard) => lights.hear(&heard),
            Err(_) => lights.speakers = None,
        }
        let on = lights.wanted();
        let why = lights.why(on);
        lights.settle(on, why);
        lights.say();

        let speakers = lights
            .speakers
            .as_ref()
            .map_or(-1, |speakers| speakers.file.as_raw_fd());
        let mut waiting = [
            PollFd {
                fd: watches.file.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            PollFd {
                fd: stop.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            // A negative descriptor is passed over.
            PollFd {
                fd: speakers,
                events: POLLIN,
                revents: 0,
            },
        ];
        let timeout = lights.until_quiet().map_or(-1, |left| {
            c_int::try_from(left.as_millis() + 1).unwrap_or(c_int::MAX)
        });
        // SAFETY: three valid pollfds for the length given.
        if unsafe { poll(waiting.as_mut_ptr(), 3, timeout) } < 0 {
            continue;
        }
        if waiting[1].revents & POLLIN != 0 {
            let mut signal = [0; 128];
            let _ = stop.read(&mut signal);
            lights.settle(false, "it's stopping");
            lights.speakers = None;
            lights.say();
            return ExitCode::SUCCESS;
        }
        if waiting[0].revents & POLLIN != 0 {
            // A node's permissions or a file's writing settle a moment after
            // the first event; one look after answers for all of them.
            std::thread::sleep(Duration::from_millis(100));
            watches.drain();
        }
    }
}

const USAGE: &str = "usage: pebble-lights LOCKED [IDLE_SECONDS]";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match &arguments[..] {
        [lock] => run(PathBuf::from(lock), IDLE),
        [lock, idle] if idle.parse::<u64>().is_ok() => run(
            PathBuf::from(lock),
            Duration::from_secs(idle.parse().unwrap_or(0)),
        ),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
#[path = "lights_tests.rs"]
mod tests;

//! Weighs a window low while every pi in it is one nobody is at.
//!
//! On 30 October pi windows running a turn, not idle ones, were what
//! competed with the apps for the processor: 11 busy sessions took 0.69
//! cores and 14 idle ones 0.19. Beside builds, with those windows at
//! weight 10 rather than 100, a stand-in app's worst frame came to 9.1 to
//! 11.5 ms against 12.7 to 24.5 ms over three interleaved rounds each, and
//! weight 25 gave nothing. pi itself still comes before the work it
//! starts, so a window goes low only while the harness marks every pi in
//! it as one nobody is at: a file named for its pid in `--background`,
//! there while it runs a turn nobody typed, has had no key for a minute
//! and holds no text. The first key removes it, and inotify wakes the
//! sorter at once, so the window has its weight back before the key is
//! drawn. A pi that writes no file, an older one or one with no terminal,
//! keeps its window where it was.
//!
//! A window whose pi started within `STARTING_FOR` weighs `STARTING`
//! instead, marked or not, so a pi Joel has just opened draws its screen
//! ahead of the sessions already running, and then goes back.

use std::collections::HashMap;
use std::ffi::{CString, c_char, c_int, c_void};
use std::io::{self, Read};
use std::os::fd::AsRawFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

/// A window's weight while someone may be at it, systemd's own default.
pub const AT: u64 = 100;

/// A window's weight while a pi in it is starting, ten times an app's.
/// On 5 November a pi starting in a window at 100, among 60 in the
/// agents' slice, took 6.1 to 13.5 s to a usable screen at 54 to 65% CPU
/// pressure, using 5.7 to 8.8 s of CPU: much of its start was spent
/// waiting its turn, while Joel waited on it.
pub const STARTING: u64 = 1000;

/// How long a pi counts as starting: past the slowest start under load,
/// after which its window goes back to an app's weight.
pub const STARTING_FOR: Duration = Duration::from_secs(30);

/// Whether a window whose pi run as `pis` goes low: it holds one at least,
/// and every one is `marked`.
pub fn low(pis: &[u32], marked: impl Fn(u32) -> bool) -> bool {
    !pis.is_empty() && pis.iter().all(|&pid| marked(pid))
}

/// The weight a window whose pi run as `pis` wants: `STARTING` while one
/// of them is `starting`, `weight` while it goes low, and an app's else.
pub fn wanted(pis: &[u32], starting: bool, marked: impl Fn(u32) -> bool, weight: u64) -> u64 {
    if starting {
        STARTING
    } else if low(pis, marked) {
        weight
    } else {
        AT
    }
}

/// Each window's weight as last set, so only a change is asked for.
#[derive(Default)]
pub struct Weighing {
    set: HashMap<String, u64>,
    /// Whether a failure to weigh has been said.
    pub failed: bool,
}

impl Weighing {
    /// The windows whose weight should change, each with the weight to set,
    /// from each window's unit and the weight it wants. One never weighed
    /// and wanting an app's is left at the weight it has.
    pub fn changes(&self, windows: &[(String, u64)]) -> Vec<(String, u64)> {
        windows
            .iter()
            .filter_map(|(unit, wanted)| {
                let had = self.set.get(unit).copied();
                (had.unwrap_or(AT) != *wanted).then(|| (unit.clone(), *wanted))
            })
            .collect()
    }

    /// Notes a window as weighed at `weight`.
    pub fn weighed(&mut self, unit: &str, weight: u64) {
        self.set.insert(unit.to_owned(), weight);
    }

    /// Forgets the windows no longer among `units`, which have closed.
    pub fn keep_only(&mut self, units: &[String]) {
        self.set.retain(|unit, _| units.contains(unit));
    }
}

/// What busctl is given to weigh a window's unit at `weight` for the
/// processor, as a runtime property gone at the next boot. The disk's
/// weight is left, since it acts only under a scheduler this disk lacks.
pub fn weigh_call(unit: &str, weight: u64) -> Vec<String> {
    let weight = weight.to_string();
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "1",
        "CPUWeight",
        "t",
        &weight,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
    fn read(fd: c_int, buf: *mut c_void, count: usize) -> isize;
    fn close(fd: c_int) -> c_int;
}

const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_CREATE: u32 = 0x100;
const IN_DELETE: u32 = 0x200;
const IN_MOVED_FROM: u32 = 0x40;
const IN_MOVED_TO: u32 = 0x80;
const POLLIN: i16 = 1;

/// A folder of marks, watched for one coming or going, and for a wake
/// another thread sends, as niri's focus moving does (`src/focus.rs`).
pub struct Watch {
    fd: c_int,
    woken: Option<UnixStream>,
}

impl Watch {
    /// Watches `dir`, which must exist.
    pub fn new(dir: &Path) -> io::Result<Self> {
        // SAFETY: no pointers are passed.
        let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        let watch = Self { fd, woken: None };
        let path = CString::new(dir.as_os_str().as_bytes())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        let mask = IN_CREATE | IN_DELETE | IN_MOVED_FROM | IN_MOVED_TO;
        // SAFETY: `path` is a C string that outlives the call.
        if unsafe { inotify_add_watch(fd, path.as_ptr(), mask) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(watch)
    }

    /// The end another thread writes a byte to, to end a wait at once.
    pub fn waker(&mut self) -> io::Result<UnixStream> {
        let (woken, waker) = UnixStream::pair()?;
        woken.set_nonblocking(true)?;
        waker.set_nonblocking(true)?;
        self.woken = Some(woken);
        Ok(waker)
    }

    /// Waits up to `longest` for a mark to come or go, or a wake, and says
    /// whether one did, reading every event waiting so the next wait
    /// starts clean.
    pub fn wait(&self, longest: Duration) -> bool {
        let mut fds = [
            PollFd {
                fd: self.fd,
                events: POLLIN,
                revents: 0,
            },
            PollFd {
                fd: self.woken.as_ref().map_or(-1, AsRawFd::as_raw_fd),
                events: POLLIN,
                revents: 0,
            },
        ];
        let ms = c_int::try_from(longest.as_millis()).unwrap_or(c_int::MAX);
        // SAFETY: `fds` is two valid entries that outlive the call; poll
        // passes over the one with a negative descriptor.
        if unsafe { poll(fds.as_mut_ptr(), 2, ms) } <= 0 {
            return false;
        }
        let mut buffer = [0u8; 4096];
        // SAFETY: `buffer` is writable for its whole length, and the
        // descriptor doesn't block, so this ends once nothing is left.
        while unsafe { read(self.fd, buffer.as_mut_ptr().cast(), buffer.len()) } > 0 {}
        if let Some(mut woken) = self.woken.as_ref() {
            while woken.read(&mut buffer).is_ok_and(|read| read > 0) {}
        }
        true
    }
}

impl Drop for Watch {
    fn drop(&mut self) {
        // SAFETY: the descriptor is this watch's own, closed once.
        unsafe { close(self.fd) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Instant;

    #[test]
    fn a_window_goes_low_only_while_every_pi_in_it_is_marked() {
        let marked = |pid| pid == 20 || pid == 31;
        assert!(low(&[20], marked));
        assert!(low(&[20, 31], marked));
        // Two in one window: one someone may be at keeps it up.
        assert!(!low(&[20, 42], marked));
        // A window with no pi is no pi's to weigh.
        assert!(!low(&[], marked));
    }

    #[test]
    fn a_window_whose_pi_is_starting_weighs_more_than_an_app_marked_or_not() {
        let marked = |pid| pid == 20;
        assert_eq!(wanted(&[20], true, marked, 10), STARTING);
        assert_eq!(wanted(&[42], true, marked, 10), STARTING);
        assert_eq!(wanted(&[20], false, marked, 10), 10);
        assert_eq!(wanted(&[42], false, marked, 10), AT);
        const { assert!(STARTING > AT) };
    }

    #[test]
    fn only_a_change_is_asked_for_and_a_window_comes_back_up_once_unmarked() {
        let mut weighing = Weighing::default();
        let windows = |wanted| vec![("kitty-1-1.scope".to_owned(), wanted)];
        assert_eq!(
            weighing.changes(&windows(AT)),
            [] as [(std::string::String, u64); 0]
        );
        assert_eq!(
            weighing.changes(&windows(10)),
            [("kitty-1-1.scope".to_owned(), 10)]
        );
        weighing.weighed("kitty-1-1.scope", 10);
        assert_eq!(
            weighing.changes(&windows(10)),
            [] as [(std::string::String, u64); 0]
        );
        assert_eq!(
            weighing.changes(&windows(AT)),
            [("kitty-1-1.scope".to_owned(), AT)]
        );
        weighing.keep_only(&[]);
        assert_eq!(
            weighing.changes(&windows(AT)),
            [] as [(std::string::String, u64); 0]
        );
        // A new window's pi starting is asked for at once.
        assert_eq!(
            weighing.changes(&windows(STARTING)),
            [("kitty-1-1.scope".to_owned(), STARTING)]
        );
    }

    #[test]
    fn a_mark_removed_wakes_the_wait_at_once_and_none_waits_it_out() {
        let dir = std::env::temp_dir().join(format!("low-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("20"), "").unwrap();
        let watch = Watch::new(&dir).unwrap();

        let began = Instant::now();
        assert!(!watch.wait(Duration::from_millis(100)));
        assert!(began.elapsed() >= Duration::from_millis(100));

        // The event waits for the next wait, as a key's would between two.
        fs::remove_file(dir.join("20")).unwrap();
        let began = Instant::now();
        assert!(watch.wait(Duration::from_secs(10)));
        assert!(began.elapsed() < Duration::from_secs(5));
        // Read whole, so the next wait starts clean.
        assert!(!watch.wait(Duration::ZERO));
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_wake_ends_the_wait_at_once() {
        let dir = std::env::temp_dir().join(format!("woken-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let mut watch = Watch::new(&dir).unwrap();
        let mut waker = watch.waker().unwrap();
        assert!(
            !watch.wait(Duration::from_millis(50)),
            "no wake, no mark: the wait runs out"
        );
        std::io::Write::write_all(&mut waker, &[1]).unwrap();
        let began = Instant::now();
        assert!(watch.wait(Duration::from_secs(10)), "woken");
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "woken at once, not at the wait's end"
        );
        assert!(!watch.wait(Duration::ZERO), "read whole");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_window_is_weighed_for_the_processor_alone_until_the_next_boot() {
        let call = weigh_call("kitty-1-1.scope", 10);
        assert_eq!(
            call[8..],
            ["kitty-1-1.scope", "true", "1", "CPUWeight", "t", "10"]
        );
    }
}

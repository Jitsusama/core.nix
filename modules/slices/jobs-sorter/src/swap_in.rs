//! Reads a session window's swapped-out memory back as Joel comes to it,
//! rather than one fault at a time as he types.
//!
//! A pi window he comes back to after a while finds much of its heap in
//! swap, and on 10 November its first keys took 300 to 600 ms, once 1.9 s,
//! while thousands of pages came back from disk one fault at a time. Asked
//! to read all of a heap back at once with `MADV_WILLNEED`, the kernel
//! queues every read together: in the lab 505 MiB pushed to disk came back
//! in 0.56 s rather than 8.3 s, and from zswap in 0.38 s rather than 0.68.
//!
//! Asking for another process's memory takes `CAP_SYS_NICE`, which no user
//! service can hold, so the sorter starts `memory-swap-in@PID.service`,
//! which only an administrator may (modules/slices/swap-in.rules). It runs
//! this as root, which keeps when each window was last read back where its
//! account can't reach ([`STAMPS`]) and runs the rest as the pid's own
//! account with `CAP_SYS_NICE` alone ([`as_owner`]). As that account it
//! asks the account's own niri ([`is_niri`]) which window has the focus,
//! and reads nothing back unless the pid belongs to it ([`belongs`]), or
//! the kitty drawing that window, asked the same way, says the pid's scope
//! is its focused window's ([`named_by_kitty`], `src/kitty.rs`); a window
//! is read back at most every [`EVERY`]. It only hints: nothing is
//! changed but what memory is resident.

use std::ffi::{c_long, c_void};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;

use crate::focus::{self, Window};
use crate::kitty;

/// How long the focus must stay on a window before its memory is read
/// back, so passing over windows on the way to another reads none of
/// theirs. Joel moved over ten windows in two seconds on 10 November.
pub const SETTLE: Duration = Duration::from_millis(250);

/// How often one window's memory may be read back, however often the focus
/// comes back to it.
pub const EVERY: Duration = Duration::from_secs(30);

/// Root's own folder for when each window was last read back: a stamp in
/// the account's runtime folder could be cleared by any of its processes,
/// so an agent could have a window read back as often as it liked.
const STAMPS: &str = "/run/memory-swap-in";

/// The `madvise` hint that asks for a range to be read in ahead of use.
const MADV_WILLNEED: c_long = 3;

/// The most ranges one `process_madvise` takes, the kernel's `UIO_MAXIOV`.
const RANGES_AT_ONCE: usize = 1024;

const SYS_PIDFD_OPEN: c_long = 434;
const SYS_PROCESS_MADVISE: c_long = 440;
const SOL_SOCKET: i32 = 1;
const SO_PEERCRED: i32 = 17;
const LOCK_EX: i32 = 2;

unsafe extern "C" {
    fn syscall(number: c_long, ...) -> c_long;
    fn close(fd: i32) -> i32;
    fn getsockopt(fd: i32, level: i32, name: i32, value: *mut c_void, len: *mut u32) -> i32;
    fn flock(fd: i32, operation: i32) -> i32;
}

/// Who is on the far end of a Unix socket, as `SO_PEERCRED` gives it.
#[repr(C)]
#[derive(Default)]
struct Peer {
    pid: i32,
    uid: u32,
    gid: u32,
}

/// One range of memory, as `process_madvise` takes them.
#[repr(C)]
struct Range {
    base: usize,
    len: usize,
}

/// `jobs-sorter swap-in PID` as root, or `jobs-sorter swap-in --owned PID`
/// as the pid's account; what it did, or why it did nothing.
///
/// Only being asked wrongly fails, or asked for a process of root's, which
/// nobody may have read back. Anything it can't find or ask, the
/// process gone, niri's socket or its manager not answering, only means
/// nothing is read back, since it only hints: a failed unit fails the
/// switch it lands in, and on 13 November one asked while a switch
/// reloaded the account's manager found no socket of niri's, and the
/// switch went back.
pub fn main(args: &[String]) -> Result<String, String> {
    let done = match args {
        [pid] => {
            let pid = parse(pid)?;
            if roots(pid) {
                return Err(format!(
                    "process {pid} is root's, whose memory isn't read back"
                ));
            }
            become_owner(pid)
        }
        [owned, pid] if owned == "--owned" => read_back(parse(pid)?),
        _ => return Err("usage: jobs-sorter swap-in [--owned] PID".to_owned()),
    };
    Ok(done.unwrap_or_else(|error| format!("Read nothing back: {error}.")))
}

/// Whether `pid` runs as root, as far as `/proc` says.
fn roots(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/status"))
        .is_ok_and(|status| owner(&status, "Uid:") == Some(0))
}

fn parse(pid: &str) -> Result<u32, String> {
    pid.parse()
        .ok()
        .filter(|pid| *pid > 1)
        .ok_or_else(|| format!("{pid:?} isn't a process"))
}

/// As root: reads `pid`'s window back as the account that owns it, keeping
/// `CAP_SYS_NICE` and nothing else, unless the window was read back in the
/// last [`EVERY`].
///
/// A window is known here by its process's scope, since each session window
/// has one of its own. Its stamp is written only once memory was read back,
/// so a call for a window without the focus can't use up its turn, and one
/// lock over [`STAMPS`] keeps two calls from both finding a window due.
fn become_owner(pid: u32) -> Result<String, String> {
    let status = fs::read_to_string(format!("/proc/{pid}/status"))
        .map_err(|error| format!("process {pid} can't be read: {error}"))?;
    let (Some(uid), Some(gid)) = (owner(&status, "Uid:"), owner(&status, "Gid:")) else {
        return Err(format!("process {pid} says no owner"));
    };
    // Asked by `main` already, and asked again here, since the pid may
    // have been taken by another process since.
    if uid == 0 {
        return Err(format!(
            "process {pid} is root's, whose memory isn't read back"
        ));
    }
    let scope =
        fs::metadata(scope_of(pid)?).map_err(|error| format!("process {pid}'s scope: {error}"))?;
    let stamps = File::open(STAMPS).map_err(|error| format!("{STAMPS}: {error}"))?;
    // SAFETY: flock takes an open descriptor, held until `stamps` is dropped.
    if unsafe { flock(stamps.as_raw_fd(), LOCK_EX) } != 0 {
        return Err(format!("{STAMPS} can't be locked"));
    }
    let stamp = Path::new(STAMPS).join(format!("{uid}-{}", scope.ino()));
    let last = fs::metadata(&stamp).and_then(|meta| meta.modified()).ok();
    if !due(last, SystemTime::now()) {
        return Ok(format!(
            "Process {pid}'s window was read back within {}s.",
            EVERY.as_secs()
        ));
    }
    let me =
        std::env::current_exe().map_err(|error| format!("this program can't be found: {error}"))?;
    let ran = Command::new("setpriv")
        .args(as_owner(uid, gid))
        .arg(me)
        .args(["swap-in", "--owned", &pid.to_string()])
        .output()
        .map_err(|error| format!("setpriv wouldn't run: {error}"))?;
    let said = String::from_utf8_lossy(&ran.stderr).trim().to_owned();
    if !ran.status.success() {
        return Err(said
            .strip_prefix("jobs-sorter swap-in: ")
            .unwrap_or(&said)
            .to_owned());
    }
    if said.starts_with("Read back") {
        fs::write(&stamp, b"").map_err(|error| format!("{}: {error}", stamp.display()))?;
    }
    Ok(said)
}

/// The cgroup folder `pid` runs in.
fn scope_of(pid: u32) -> Result<PathBuf, String> {
    let cgroup = fs::read_to_string(format!("/proc/{pid}/cgroup"))
        .map_err(|error| format!("process {pid}: {error}"))?;
    Ok(Path::new("/sys/fs/cgroup").join(
        cgroup
            .trim()
            .split("::")
            .nth(1)
            .unwrap_or_default()
            .trim_start_matches('/'),
    ))
}

/// What `setpriv` is given to run a program as `uid` and `gid`, holding
/// only `CAP_SYS_NICE`, the one `process_madvise` asks of a caller acting
/// on another process.
pub fn as_owner(uid: u32, gid: u32) -> Vec<String> {
    vec![
        format!("--reuid={uid}"),
        format!("--regid={gid}"),
        "--clear-groups".to_owned(),
        "--inh-caps=-all,+sys_nice".to_owned(),
        "--ambient-caps=+sys_nice".to_owned(),
        "--bounding-set=-all,+sys_nice".to_owned(),
        "--".to_owned(),
    ]
}

/// A process's real owner from its status's `Uid:` or `Gid:` line.
pub fn owner(status: &str, field: &str) -> Option<u32> {
    status
        .lines()
        .find_map(|line| line.strip_prefix(field))?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

/// What a process says of itself, as far as finding its window goes.
#[derive(Debug, Default)]
pub struct Process {
    /// The app id it was started with, from its environment.
    pub app_id: Option<String>,
    /// Its cgroup's own name, such as `kitty-2684-26.scope`.
    pub scope: String,
    /// Its parent, its parent's parent, and so on.
    pub ancestors: Vec<u32>,
}

/// Whether `process` runs in `window`, a session window, as the sorter
/// finds a window's scope (`focus::scope_of`): started with its app id,
/// or else in the one scope of the kitty that draws it, `kittys` scopes
/// in all, which the process runs under.
pub fn belongs(process: &Process, window: &Window, kittys: usize) -> bool {
    if !window.session() {
        return false;
    }
    if let Some(app_id) = &process.app_id {
        return *app_id == window.app_id;
    }
    kittys == 1
        && process.scope.starts_with(&format!("kitty-{}-", window.pid))
        && process.ancestors.contains(&window.pid)
}

/// Whether `process` runs in `window`, a session window, by what the kitty
/// drawing it says: `named`, the scope of the program in its focused OS
/// window, is the process's, under that kitty. On 11 November 48 of 58 pi
/// windows shared one kitty with no app id, so [`belongs`] found none.
pub fn named_by_kitty(process: &Process, window: &Window, named: Option<&str>) -> bool {
    window.session()
        && process.ancestors.contains(&window.pid)
        && named.is_some_and(|named| named == process.scope)
}

/// Whether a window last read back at `last` may be read back `now`.
pub fn due(last: Option<SystemTime>, now: SystemTime) -> bool {
    last.is_none_or(|last| now.duration_since(last).is_ok_and(|since| since >= EVERY))
}

/// The private anonymous ranges in a process's `maps`, its heap and stacks
/// among them: the memory that goes to swap.
pub fn anonymous(maps: &str) -> Vec<(usize, usize)> {
    maps.lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let (span, perms) = (fields.next()?, fields.next()?);
            let inode = fields.nth(2)?;
            let path = fields.next().unwrap_or_default();
            let named_anonymous = path.is_empty()
                || path == "[heap]"
                || path.starts_with("[stack")
                || path.starts_with("[anon:");
            if inode != "0" || perms.as_bytes().get(3) != Some(&b'p') || !named_anonymous {
                return None;
            }
            let (start, end) = span.split_once('-')?;
            let start = usize::from_str_radix(start, 16).ok()?;
            let end = usize::from_str_radix(end, 16).ok()?;
            (end > start).then_some((start, end - start))
        })
        .collect()
}

/// Whether a socket's `peer` is the account's own niri: `me`, and the main
/// process of its `niri.service`, `niri`. Any of the account's processes
/// could put a socket named like niri's in its runtime folder and answer
/// with whatever window it liked.
pub fn is_niri(peer: (u32, u32), niri: Option<u32>, me: u32) -> bool {
    let (pid, uid) = peer;
    uid == me && pid > 1 && niri == Some(pid)
}

/// The main process of the account's `niri.service`, as its manager says.
fn niri_service(runtime: &Path) -> Option<u32> {
    let said = Command::new("systemctl")
        .args([
            "--user",
            "show",
            "niri.service",
            "--property=MainPID",
            "--value",
        ])
        .env("XDG_RUNTIME_DIR", runtime)
        .output()
        .ok()?;
    String::from_utf8_lossy(&said.stdout).trim().parse().ok()
}

/// The pid and uid on the far end of `stream`.
pub fn peer(stream: &UnixStream) -> Option<(u32, u32)> {
    let mut peer = Peer::default();
    let mut len = u32::try_from(size_of::<Peer>()).ok()?;
    // SAFETY: `peer` is a live ucred, `len` its size, as SO_PEERCRED fills.
    let got = unsafe {
        getsockopt(
            stream.as_raw_fd(),
            SOL_SOCKET,
            SO_PEERCRED,
            (&raw mut peer).cast::<c_void>(),
            &raw mut len,
        )
    };
    (got == 0).then_some(())?;
    Some((u32::try_from(peer.pid).ok()?, peer.uid))
}

/// The window the account's niri says has the focus, asked over its socket
/// in `runtime`.
fn focused(runtime: &Path, me: u32) -> Result<Option<Window>, String> {
    let niri = niri_service(runtime);
    let sockets =
        fs::read_dir(runtime).map_err(|error| format!("{}: {error}", runtime.display()))?;
    let mut sockets: Vec<PathBuf> = sockets
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let niris = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("niri."));
            niris
                && path
                    .extension()
                    .is_some_and(|extension| extension == "sock")
        })
        .collect();
    sockets.sort();
    for socket in sockets {
        let Ok(mut stream) = UnixStream::connect(&socket) else {
            continue;
        };
        if !peer(&stream).is_some_and(|peer| is_niri(peer, niri, me)) {
            continue;
        }
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .map_err(|error| error.to_string())?;
        stream
            .write_all(b"\"FocusedWindow\"\n")
            .map_err(|error| error.to_string())?;
        let mut reply = String::new();
        BufReader::new(stream)
            .read_line(&mut reply)
            .map_err(|error| format!("niri didn't answer: {error}"))?;
        return answered(&reply);
    }
    Err(format!(
        "no socket in {} is niri.service's own",
        runtime.display()
    ))
}

/// The focused window in niri's answer to `FocusedWindow`.
pub fn answered(reply: &str) -> Result<Option<Window>, String> {
    let reply: Value =
        serde_json::from_str(reply).map_err(|_| "niri's answer wasn't JSON".to_owned())?;
    let window = reply
        .get("Ok")
        .and_then(|ok| ok.get("FocusedWindow"))
        .ok_or_else(|| format!("niri answered {reply}"))?;
    Ok(focus::window(window).map(|(window, _)| window))
}

/// What `/proc` says of `pid` for [`belongs`].
fn process(pid: u32) -> Result<Process, String> {
    let environ = fs::read(format!("/proc/{pid}/environ"))
        .map_err(|error| format!("process {pid}: {error}"))?;
    let cgroup =
        fs::read_to_string(format!("/proc/{pid}/cgroup")).map_err(|error| error.to_string())?;
    let mut ancestors = Vec::new();
    let mut at = pid;
    while at > 1 && ancestors.len() < 64 {
        let Some(parent) = fs::read_to_string(format!("/proc/{at}/status"))
            .ok()
            .and_then(|status| owner(&status, "PPid:"))
        else {
            break;
        };
        ancestors.push(parent);
        at = parent;
    }
    Ok(Process {
        app_id: focus::app_id(&environ),
        scope: cgroup
            .trim()
            .rsplit('/')
            .next()
            .unwrap_or_default()
            .to_owned(),
        ancestors,
    })
}

/// How many scopes beside `pid`'s are drawn by the kitty `kitty`.
fn kittys(pid: u32, kitty: u32) -> usize {
    let Ok(cgroup) = fs::read_to_string(format!("/proc/{pid}/cgroup")) else {
        return 0;
    };
    let path = cgroup.trim().split("::").nth(1).unwrap_or_default();
    let parent = Path::new("/sys/fs/cgroup").join(path.trim_start_matches('/'));
    let own = format!("kitty-{kitty}-");
    parent
        .parent()
        .and_then(|slice| fs::read_dir(slice).ok())
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&own))
        .count()
}

/// As the account `pid` runs as: waits [`SETTLE`], then reads back the
/// swapped memory of every process in `pid`'s scope, if `pid` belongs to
/// the session window niri says has the focus.
fn read_back(pid: u32) -> Result<String, String> {
    let me = fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| owner(&status, "Uid:"))
        .ok_or("this process says no owner")?;
    thread::sleep(SETTLE);
    let runtime = PathBuf::from(format!("/run/user/{me}"));
    let Some(window) = focused(&runtime, me)? else {
        return Ok("No window has the focus, so nothing is read back.".to_owned());
    };
    let found = process(pid)?;
    if !belongs(&found, &window, kittys(pid, window.pid))
        && !named_by_kitty(
            &found,
            &window,
            kitty::ask(&window, me)
                .ok()
                .and_then(|shown| kitty::named(&window, true, &shown))
                .as_deref(),
        )
    {
        // The focus moving on within SETTLE is the most common reason, as
        // Joel passes over windows, so it's an answer rather than a
        // failure: on 11 November two such left their units failed.
        return Ok(format!(
            "Process {pid} isn't in the focused window {} ({}), so nothing is read back.",
            window.id, window.app_id
        ));
    }
    let scope = scope_of(pid)?;
    let procs =
        fs::read_to_string(scope.join("cgroup.procs")).map_err(|error| error.to_string())?;
    let started = Instant::now();
    let (mut processes, mut swapped) = (0, 0);
    for each in procs.lines().filter_map(|line| line.parse::<u32>().ok()) {
        let Ok(status) = fs::read_to_string(format!("/proc/{each}/status")) else {
            continue;
        };
        if owner(&status, "Uid:") != Some(me) {
            continue;
        }
        swapped += owner(&status, "VmSwap:").unwrap_or(0);
        if let Ok(maps) = fs::read_to_string(format!("/proc/{each}/maps")) {
            hint(each, &anonymous(&maps))?;
            processes += 1;
        }
    }
    Ok(format!(
        "Read back {} MiB in swap for window {} ({}), {processes} processes, in {} ms.",
        swapped >> 10,
        window.id,
        window.app_id,
        started.elapsed().as_millis()
    ))
}

/// Asks the kernel to read `ranges` of `pid`'s memory in.
fn hint(pid: u32, ranges: &[(usize, usize)]) -> Result<(), String> {
    // SAFETY: pidfd_open takes a pid and flags and returns a descriptor or -1.
    let pidfd = unsafe { syscall(SYS_PIDFD_OPEN, c_long::from(pid), 0 as c_long) };
    if pidfd < 0 {
        // It ended since the scope was read.
        return Ok(());
    }
    let ranges: Vec<Range> = ranges
        .iter()
        .map(|&(base, len)| Range { base, len })
        .collect();
    let mut refused = None;
    for chunk in ranges.chunks(RANGES_AT_ONCE) {
        // SAFETY: the ranges are a live slice of `chunk.len()` iovecs, as
        // process_madvise reads them; the memory named is the target's.
        let done = unsafe {
            syscall(
                SYS_PROCESS_MADVISE,
                pidfd,
                chunk.as_ptr(),
                chunk.len(),
                MADV_WILLNEED,
                0 as c_long,
            )
        };
        if done < 0 {
            refused.get_or_insert_with(std::io::Error::last_os_error);
        }
    }
    // SAFETY: pidfd is the descriptor pidfd_open gave, closed once.
    unsafe { close(i32::try_from(pidfd).unwrap_or(-1)) };
    match refused {
        Some(error) if error.raw_os_error() == Some(1) => Err(format!(
            "the kernel won't let process {pid}'s memory be read in: {error}"
        )),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(app_id: &str, pid: u32) -> Window {
        Window {
            id: 7,
            app_id: app_id.to_owned(),
            pid,
            title: String::new(),
        }
    }

    #[test]
    fn one_in_a_shared_kitty_belongs_to_the_window_its_kitty_names() {
        let process = Process {
            app_id: None,
            scope: "kitty-2684-128.scope".into(),
            ancestors: vec![3_451_126, 2684, 1],
        };
        let shared = window("pi-spawned", 2684);
        assert!(
            named_by_kitty(&process, &shared, Some("kitty-2684-128.scope")),
            "its kitty names its scope"
        );
        assert!(
            !named_by_kitty(&process, &shared, Some("kitty-2684-121.scope")),
            "its kitty names another"
        );
        assert!(
            !named_by_kitty(&process, &shared, None),
            "its kitty names none"
        );
        assert!(
            !named_by_kitty(
                &process,
                &window("pi-spawned", 4242),
                Some("kitty-2684-128.scope")
            ),
            "not under the kitty drawing the window"
        );
        assert!(
            !named_by_kitty(
                &process,
                &window("kitty", 2684),
                Some("kitty-2684-128.scope")
            ),
            "a window holding no session"
        );
    }

    #[test]
    fn a_process_started_with_the_windows_app_id_belongs_to_it() {
        let process = Process {
            app_id: Some("pi-session.7".into()),
            scope: "kitty-2684-26.scope".into(),
            ancestors: vec![2684, 1],
        };
        assert!(
            belongs(&process, &window("pi-session.7", 2684), 40),
            "its own app id"
        );
        assert!(
            !belongs(&process, &window("pi-session.8", 2684), 40),
            "another window's"
        );
    }

    #[test]
    fn one_with_none_belongs_to_the_one_scope_of_the_kitty_drawing_the_window() {
        let process = Process {
            app_id: None,
            scope: "kitty-839371-0.scope".into(),
            ancestors: vec![839_371, 1729, 1],
        };
        let spawned = window("pi-spawned.133320004", 839_371);
        assert!(belongs(&process, &spawned, 1), "the kitty's one scope");
        assert!(
            !belongs(&process, &spawned, 2),
            "a kitty drawing two can't say which"
        );
        assert!(
            !belongs(&process, &window("pi-spawned.1", 2684), 1),
            "another kitty's window"
        );
        let elsewhere = Process {
            ancestors: vec![1729, 1],
            ..process
        };
        assert!(
            !belongs(&elsewhere, &spawned, 1),
            "not started under that kitty"
        );
    }

    #[test]
    fn nothing_belongs_to_a_window_holding_no_session() {
        let process = Process {
            app_id: Some("com.google.Chrome".into()),
            ..Process::default()
        };
        assert!(
            !belongs(&process, &window("com.google.Chrome", 63670), 0),
            "only a session window's"
        );
    }

    #[test]
    fn a_window_is_read_back_at_most_once_every_thirty_seconds() {
        let now = SystemTime::now();
        assert!(due(None, now), "never read back");
        assert!(!due(Some(now - Duration::from_secs(29)), now), "29 s ago");
        assert!(due(Some(now - EVERY), now), "30 s ago");
        assert!(
            !due(Some(now + Duration::from_secs(5)), now),
            "a stamp from a clock set back"
        );
    }

    #[test]
    fn only_the_accounts_own_niri_is_asked_for_the_focus() {
        assert!(
            is_niri((2183, 1000), Some(2183), 1000),
            "niri.service's own"
        );
        assert!(
            !is_niri((4242, 1000), Some(2183), 1000),
            "another of the account's processes"
        );
        assert!(
            !is_niri((2183, 1001), Some(2183), 1000),
            "another account's"
        );
        assert!(!is_niri((2183, 1000), None, 1000), "no niri.service runs");
        assert!(
            !is_niri((0, 1000), Some(0), 1000),
            "a service that isn't running says 0"
        );
    }

    #[test]
    fn only_private_anonymous_memory_is_read_back() {
        let maps = "\
55d0c2a00000-55d0c2a21000 rw-p 00000000 00:00 0                          [heap]
7f0000000000-7f0000100000 rw-p 00000000 00:00 0 \n\
7f0000100000-7f0000200000 ---p 00000000 00:00 0 \n\
7f0000200000-7f0000300000 rw-s 00000000 00:01 4242                       /memfd:shared (deleted)
7f0000300000-7f0000301000 r-xp 00001000 fd:01 123                        /nix/store/x/lib/libc.so
7f0000400000-7f0000401000 rw-p 00000000 00:00 0                          [anon:v8]
7ffd00000000-7ffd00021000 rw-p 00000000 00:00 0                          [stack]
7ffd00100000-7ffd00102000 r--p 00000000 00:00 0                          [vvar]
";
        assert_eq!(
            anonymous(maps),
            vec![
                (0x55d0_c2a0_0000, 0x21000),
                (0x7f00_0000_0000, 0x10_0000),
                (0x7f00_0010_0000, 0x10_0000),
                (0x7f00_0040_0000, 0x1000),
                (0x7ffd_0000_0000, 0x21000),
            ],
            "the heap, the anonymous ranges and the stack, not files, shared memory or vvar"
        );
    }

    #[test]
    fn niris_focused_window_is_read_from_its_answer() {
        let reply = concat!(
            r#"{"Ok":{"FocusedWindow":"#,
            r#"{"id":410,"app_id":"pi-spawned.1","pid":839371,"is_focused":true}}}"#
        );
        assert_eq!(
            answered(reply),
            Ok(Some(window("pi-spawned.1", 839_371)).map(|w| Window { id: 410, ..w })),
            "the window"
        );
        assert_eq!(
            answered(r#"{"Ok":{"FocusedWindow":null}}"#),
            Ok(None),
            "none focused"
        );
        assert!(answered(r#"{"Err":"no"}"#).is_err(), "an error is said");
    }

    #[test]
    fn a_process_is_run_as_its_owner_with_the_nice_capability_alone() {
        assert_eq!(
            as_owner(1000, 100),
            [
                "--reuid=1000",
                "--regid=100",
                "--clear-groups",
                "--inh-caps=-all,+sys_nice",
                "--ambient-caps=+sys_nice",
                "--bounding-set=-all,+sys_nice",
                "--"
            ],
            "setpriv's arguments"
        );
        assert_eq!(
            owner("Name:\tpi\nUid:\t1000\t1000\t1000\t1000\n", "Uid:"),
            Some(1000),
            "the real uid"
        );
    }

    #[test]
    fn what_it_cant_find_reads_nothing_back_and_fails_nothing() {
        // Past any pid_max, so no process is ever there to be read.
        let gone = main(&["4194305".to_owned()]);
        assert!(
            gone.as_ref()
                .is_ok_and(|said| said.starts_with("Read nothing back: ")),
            "a window it couldn't find failed its unit, and with it a switch: {gone:?}"
        );
        assert!(
            main(&["1".to_owned()]).is_err(),
            "asked for no process, it said nothing was wrong"
        );
        assert!(
            roots(1),
            "init wasn't found to be root's, so its memory could be asked for"
        );
        assert!(
            main(&["--other".to_owned(), "7".to_owned()]).is_err(),
            "asked wrongly, it said nothing was wrong"
        );
    }
}

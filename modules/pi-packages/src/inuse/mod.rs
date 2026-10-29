//! Whether any process on the machine still uses a version.
//!
//! A running pi keeps the version it loaded: its native modules mapped,
//! files open, and the code it imports later read from the same folder.
//! So a version `current` has left is removed only once nothing holds a
//! file in it open or stands in it, and nothing runs it. What runs it is
//! what a process said it loaded, when it said (loaded.rs), and else
//! what it maps: a pi that reloaded keeps mapping each version's native
//! addon it ever loaded, since one can't be unloaded, so its maps name
//! versions it no longer runs. Linux says what a process uses through
//! `/proc` and macOS through libproc (macos.rs); elsewhere nothing is
//! known to use it.

use std::fs;
use std::path::Path;
use std::time::SystemTime;

use crate::loaded::Said;

#[cfg(target_os = "macos")]
mod macos;

/// What one process is seen doing with a version's folder.
#[derive(Debug, Clone, Copy, Default)]
pub struct Seen {
    /// It stands in the folder.
    pub stands: bool,
    /// It holds a file in the folder open.
    pub holds: bool,
    /// It maps a file in the folder.
    pub maps: bool,
}

/// Whether a process uses a version, from what it is seen doing and
/// what it said it loaded: `None` when it said nothing, else whether it
/// named this version. Standing in a folder or holding a file open is
/// use, whatever it said; a map is only when it said nothing.
#[must_use]
pub fn uses(seen: Seen, said: Option<bool>) -> bool {
    seen.stands || seen.holds || said.unwrap_or(seen.maps)
}

/// Whether a process uses the version in `dir`, or one said it runs it.
#[must_use]
pub fn in_use(dir: &Path, said: &Said) -> bool {
    // Both kernels say every path with its links resolved, so the folder
    // is compared that way too; one that can't be resolved is gone.
    let Ok(dir) = fs::canonicalize(dir) else {
        return false;
    };
    if said.names(&dir) {
        return true;
    }
    let Some(prefix) = dir
        .to_str()
        .map(|dir| format!("{}/", dir.trim_end_matches('/')))
    else {
        return false;
    };
    let inside = |path: &str| path.starts_with(&prefix) || path == prefix.trim_end_matches('/');
    any_process(&inside, &|pid| {
        said.of(pid).map(|named| named == dir.as_path())
    })
}

/// When process `pid` started, when it runs and can be asked.
#[must_use]
pub fn started(pid: u32) -> Option<SystemTime> {
    started_at(pid)
}

#[cfg(target_os = "macos")]
fn any_process(inside: &dyn Fn(&str) -> bool, said: &dyn Fn(u32) -> Option<bool>) -> bool {
    macos::any_process(inside, said)
}

#[cfg(target_os = "macos")]
fn started_at(pid: u32) -> Option<SystemTime> {
    macos::started(pid)
}

/// How many clock ticks `/proc` counts a second in: `USER_HZ`, which the
/// kernel's interface fixes at 100 whatever its own tick.
#[cfg(not(target_os = "macos"))]
const TICKS_A_SECOND: u64 = 100;

#[cfg(not(target_os = "macos"))]
fn started_at(pid: u32) -> Option<SystemTime> {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    // Its name, in brackets, may hold spaces; the fields follow the last.
    let (_, fields) = stat.rsplit_once(')')?;
    // The 22nd field, start time, is the 20th after the name.
    let ticks: u64 = fields.split_whitespace().nth(19)?.parse().ok()?;
    let boot: u64 = fs::read_to_string("/proc/stat")
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("btime "))?
        .trim()
        .parse()
        .ok()?;
    let since_epoch = std::time::Duration::from_secs(boot)
        + std::time::Duration::from_millis(ticks.saturating_mul(1000) / TICKS_A_SECOND);
    SystemTime::UNIX_EPOCH.checked_add(since_epoch)
}

#[cfg(not(target_os = "macos"))]
fn any_process(inside: &dyn Fn(&str) -> bool, said: &dyn Fn(u32) -> Option<bool>) -> bool {
    let Ok(processes) = fs::read_dir("/proc") else {
        return false;
    };
    processes.flatten().any(|process| {
        let Some(pid) = process
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            return false;
        };
        let proc = process.path();
        let seen = Seen {
            stands: fs::read_link(proc.join("cwd"))
                .ok()
                .and_then(|cwd| cwd.to_str().map(inside))
                .unwrap_or(false),
            holds: fs::read_dir(proc.join("fd")).is_ok_and(|fds| {
                fds.flatten().any(|fd| {
                    fs::read_link(fd.path())
                        .ok()
                        .and_then(|target| target.to_str().map(inside))
                        .unwrap_or(false)
                })
            }),
            maps: fs::read_to_string(proc.join("maps")).is_ok_and(|maps| {
                maps.lines()
                    .filter_map(|line| line.split_whitespace().nth(5))
                    .any(inside)
            }),
        };
        uses(seen, said(pid))
    })
}

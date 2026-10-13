//! Leftovers in a temp directory: what a program made and forgot.

use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::fs::{self, Metadata};
use std::os::unix::fs::{FileTypeExt, MetadataExt};
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// One top-level entry of a temp directory and what lies under it.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub path: PathBuf,
    pub uid: u32,
    /// The latest change anywhere under it, in wall seconds; infinite when
    /// some of it couldn't be read.
    pub newest: f64,
    /// Its files' logical sizes, added up.
    pub size: u64,
    pub socket: bool,
    pub checkout: bool,
}

/// More entries than this under one and it counts as changed now.
const BUDGET: usize = 50_000;

/// Names other tools own, which they clean up themselves or reuse.
const PROTECTED: [&str; 4] = [
    "pi-quest-",
    "systemd-private-",
    "snap-private-tmp",
    "TemporaryItems",
];

/// Temp names made by mkdtemp and its kin end in a random run of letters
/// and digits, or a pid, perhaps with an extension after it.
static RUST_TEMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\.tmp[A-Za-z0-9]{6}$").unwrap());
static PYTHON_TEMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^tmp[a-z0-9_]{8}$").unwrap());
static SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".[-_.]([A-Za-z0-9]{6,})(\.[a-z]{1,4})?$").unwrap());
static PID_SUFFIX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r".-[0-9]+(\.[a-z]{1,4})?$").unwrap());

/// True when a name carries a random or pid suffix, as a one-off does. A
/// name without is a cache meant to be reused, idle-looking while every
/// start leans on it. Six characters drawn as mkdtemp draws them hold a
/// digit or both cases all but one time in two hundred, while a word holds
/// neither, so `-closure` is a name's own and `-Xy12Ab` isn't.
fn made_up(name: &str) -> bool {
    let random = |run: &str| {
        let has = |test: fn(&char) -> bool| run.chars().any(|c| test(&c));
        has(char::is_ascii_digit)
            || (has(char::is_ascii_uppercase) && has(char::is_ascii_lowercase))
    };
    RUST_TEMP.is_match(name)
        || PYTHON_TEMP.is_match(name)
        || PID_SUFFIX.is_match(name)
        || SUFFIX.captures(name).is_some_and(|found| random(&found[1]))
}

/// When something last changed: an unpacked archive keeps its files' old
/// modification times, but the change time is when each landed here, and
/// nothing can set that back.
fn changed(meta: &Metadata) -> f64 {
    let modified = meta.mtime() as f64 + meta.mtime_nsec() as f64 / 1e9;
    let changed = meta.ctime() as f64 + meta.ctime_nsec() as f64 / 1e9;
    modified.max(changed)
}

/// Adds what lies under `dir` to `entry`, never through a symlink or into
/// another filesystem. False when it can't all be judged.
fn walk(dir: &Path, device: u64, entry: &mut Entry, seen: &mut usize) -> bool {
    let Ok(children) = fs::read_dir(dir) else {
        return false;
    };
    for child in children {
        *seen += 1;
        let Ok(child) = child else { return false };
        let Ok(meta) = child.metadata() else {
            return false;
        };
        if *seen > BUDGET || meta.dev() != device {
            return false;
        }
        entry.newest = entry.newest.max(changed(&meta));
        let kind = meta.file_type();
        entry.socket |= kind.is_socket();
        entry.checkout |= child.file_name() == ".git";
        if kind.is_dir() {
            if !walk(&child.path(), device, entry, seen) {
                return false;
            }
        } else {
            entry.size += meta.len();
        }
    }
    true
}

/// Every top-level entry of a temp directory this user owns, each with the
/// latest change under it and whether it holds a socket or a checkout.
pub fn survey(temp: &Path) -> Vec<Entry> {
    let temp = fs::canonicalize(temp).unwrap_or_else(|_| temp.to_owned());
    let Ok(top) = fs::read_dir(&temp) else {
        return Vec::new();
    };
    // SAFETY: geteuid has no preconditions and can't fail.
    let me = unsafe { libc::geteuid() };
    let mut found = Vec::new();
    for item in top.filter_map(Result::ok) {
        let Ok(meta) = item.metadata() else { continue };
        if meta.uid() != me {
            continue;
        }
        let mut entry = Entry {
            path: item.path(),
            uid: meta.uid(),
            newest: changed(&meta),
            size: 0,
            socket: meta.file_type().is_socket(),
            checkout: item.file_name() == ".git",
        };
        if meta.is_dir() {
            if !walk(&item.path(), meta.dev(), &mut entry, &mut 0) {
                entry.newest = f64::INFINITY;
            }
        } else {
            entry.size = meta.len();
        }
        found.push(entry);
    }
    found
}

/// The entries a pass may take: this user's, with a made-up name not named
/// for the user, holding
/// no socket or checkout, and not any live process's TMPDIR or above one.
/// A dev server can sit in a shell for days writing nothing, and its
/// scratch is still its own. Whether a process works in one is for the
/// caller to judge.
pub fn leftovers(entries: Vec<Entry>, uid: u32, tmpdirs: &[PathBuf]) -> Vec<Entry> {
    let tmpdirs: Vec<PathBuf> = tmpdirs
        .iter()
        .map(|dir| fs::canonicalize(dir).unwrap_or_else(|_| dir.clone()))
        .collect();
    let offered = |entry: &Entry| {
        let Some(name) = entry.path.file_name().and_then(|name| name.to_str()) else {
            return false;
        };
        let hidden = name.starts_with('.') && !RUST_TEMP.is_match(name);
        // Named for its user rather than a process: a program's runtime
        // directory, which lives as long as the program does.
        let users = name.strip_suffix(&format!("-{uid}")).is_some();
        entry.uid == uid
            && !users
            && !hidden
            && !PROTECTED.iter().any(|prefix| name.starts_with(prefix))
            && made_up(name)
            && !entry.socket
            && !entry.checkout
            && !tmpdirs.iter().any(|dir| dir.starts_with(&entry.path))
    };
    entries.into_iter().filter(offered).collect()
}

pub fn candidate(entry: &Entry, awake: &Awake, now: f64) -> Candidate {
    Candidate {
        path: entry.path.clone(),
        class: Class::Temp,
        idle: awake.since(entry.newest.min(now), now).max(0.0),
        size: entry.size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

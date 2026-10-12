//! Turning the targets and installs discovery found into candidates.

use crate::activity::Use;
use crate::age::{last_build, last_commit, last_write, mtime, profiles, subdirs};
use crate::clock::Awake;
use crate::discover::{Checkout, Found, Kind};
use crate::plan::{Candidate, Class};
use std::fs;
use std::path::{Path, PathBuf};

/// What was learned about a found tree this time round.
#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub used: Use,
    pub finished: bool,
    pub size: u64,
    /// When it was last built or installed into, in wall seconds.
    pub last_used: f64,
}

/// One crate's incremental directory in a target, one per set of flags it
/// was compiled with.
#[derive(Clone, Debug, PartialEq)]
pub struct Variant {
    pub path: PathBuf,
    pub last_used: f64,
}

/// Every crate variant in a target's profiles, dated by its newest session:
/// rustc starts one on every incremental compile and keeps the last that
/// finished. One with a session still `-working` is mid-compile and isn't
/// offered.
pub fn variants(target: &Path) -> Vec<Variant> {
    let crates = profiles(target)
        .into_iter()
        .flat_map(|p| subdirs(&p.join("incremental")));
    let mut found = Vec::new();
    for path in crates {
        let sessions = subdirs(&path);
        let named = |dir: &PathBuf| dir.file_name().map(|n| n.to_string_lossy().into_owned());
        let names: Vec<String> = sessions.iter().filter_map(named).collect();
        if names.iter().any(|name| name.ends_with("-working")) {
            continue;
        }
        let stamps = sessions
            .iter()
            .zip(&names)
            .filter(|(_, name)| name.starts_with("s-"));
        let newest = stamps.filter_map(|(dir, _)| mtime(dir)).reduce(f64::max);
        let Some(last_used) = newest.or_else(|| mtime(&path)) else {
            continue;
        };
        found.push(Variant { path, last_used });
    }
    found
}

/// More entries than this at an install's top and it counts as used now.
const INSTALL_BUDGET: usize = 50_000;

fn checkout(found: &Found) -> Option<&Path> {
    match &found.checkout {
        Checkout::At(path) => Some(path),
        Checkout::Gone(_) | Checkout::Unknown => None,
    }
}

/// A venv's package directories, one per interpreter it was made with.
fn site_packages(venv: &Path) -> Vec<PathBuf> {
    let children = |dir: &Path| -> Vec<PathBuf> {
        let Ok(entries) = fs::read_dir(dir) else {
            return Vec::new();
        };
        entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect()
    };
    let libs = children(venv).into_iter().filter(|path| {
        path.file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with("lib"))
    });
    let pythons = libs.flat_map(|lib| children(&lib));
    pythons.map(|python| python.join("site-packages")).collect()
}

/// When a tree was last built, installed into or committed in. A target's
/// build is read from what only cargo writes. An install's is read from its
/// top level, where npm, pnpm and yarn each rewrite a state file on every
/// install and a new package makes a new directory; a package's own files
/// keep whatever times it was published with, so nothing deeper is read. A
/// venv's packages land in `site-packages`, three levels down.
pub fn last_used(found: &Found) -> f64 {
    let checkout = checkout(found);
    if found.kind == Kind::Target {
        return last_build(&found.path, checkout);
    }
    let top = last_write(&found.path, 1, INSTALL_BUDGET);
    let packages = site_packages(&found.path)
        .into_iter()
        .map(|dir| last_write(&dir, 0, 0));
    let committed = checkout.and_then(last_commit);
    packages.chain(committed).fold(top, f64::max)
}

/// The candidate a found tree makes, or none while something works in it or
/// can't be read. A tree whose checkout is gone is spent like a finished
/// one, and has no checkout left to judge its use by.
pub fn candidate(found: &Found, seen: &Seen, awake: &Awake, now: f64) -> Option<Candidate> {
    let occupied = match seen.used {
        Use::Busy(_) | Use::Unknown(_) => return None,
        Use::Occupied(_) => true,
        Use::Free => false,
    };
    let class = match found.kind {
        Kind::Target => Class::Target,
        Kind::Install => Class::Install,
    };
    let gone = matches!(found.checkout, Checkout::Gone(_));
    Some(Candidate {
        path: found.path.clone(),
        class,
        // Used later than now, or too big to date, is used now.
        idle: awake.since(seen.last_used.min(now), now).max(0.0),
        size: seen.size,
        occupied,
        spent: seen.finished || gone,
        checkout: checkout(found).map(Path::to_owned),
    })
}

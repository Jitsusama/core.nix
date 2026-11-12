//! What each running pi says it loaded of a package.
//!
//! A pi writes `<package>/loaded/<pid>` each time it loads the package,
//! through a rename, holding `{"root": "<the version folder>"}`, and
//! removes it when it ends. A pi still writes the file after it reloads,
//! so the file names the version it runs, where its maps name every
//! version it ever loaded (inuse/).
//!
//! A file counts only while its pid runs a process that started before
//! the file was last written. A pid the system gave again belongs to a
//! process that started after the one that wrote the file had ended, so
//! it can't keep a version that process said it ran. A file that doesn't
//! count is removed.
//!
//! Each file that counts also says when its pi started, which the sweep
//! needs for every package at once: node keeps where it found a module
//! for as long as the process lives, so a pi resolves through `current`
//! to the version it led to when it first looked, in any package it
//! loads, whichever version it says it runs now (version.rs).
//!
//! A file may also say which pi wrote it, `"pi": {"node", "bundle",
//! "tmp"}`: the node it runs on, the folder of pi's `cli.js`, and its
//! temporary folder, where jiti keeps what it transpiled (jiti.rs). A pi
//! names its process `pi` and keeps no command line, so only the pi can
//! say. An update warms each such pi's cache before a swap (warm.rs).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::inuse;
use crate::version::Place;

/// How far a process's start may seem to follow the file it wrote: the
/// boot time Linux reckons a start from is whole seconds.
const CLOCK_SLACK: Duration = Duration::from_secs(2);

/// The version each live process said it runs, by pid, links resolved,
/// and when the first of them started.
#[derive(Debug, Clone, Default)]
pub struct Said {
    runs: HashMap<u32, PathBuf>,
    earliest: Option<SystemTime>,
    pis: Vec<Pi>,
}

/// Which pi a live process is: the node it runs on, the folder of pi's
/// `cli.js` it started from, and its temporary folder.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Pi {
    pub node: PathBuf,
    pub bundle: PathBuf,
    pub tmp: PathBuf,
}

impl Pi {
    /// The pi a file's `pi` names, while its node, its `cli.js` and its
    /// temporary folder are all there.
    fn from(value: &Value) -> Option<Self> {
        let path = |key: &str| value[key].as_str().map(PathBuf::from);
        let pi = Self {
            node: path("node")?,
            bundle: path("bundle")?,
            tmp: path("tmp")?,
        };
        let there = pi.node.is_file() && pi.bundle.join("cli.js").is_file() && pi.tmp.is_dir();
        there.then_some(pi)
    }
}

impl Said {
    /// What `pid` said it runs, when it said.
    #[must_use]
    pub fn of(&self, pid: u32) -> Option<&Path> {
        self.runs.get(&pid).map(PathBuf::as_path)
    }

    /// Whether a live process said it runs the version in `dir`, given
    /// with its links resolved.
    #[must_use]
    pub fn names(&self, dir: &Path) -> bool {
        self.runs.values().any(|runs| runs == dir)
    }

    /// When the longest-running of the processes that said started.
    #[must_use]
    pub fn earliest(&self) -> Option<SystemTime> {
        self.earliest
    }

    /// Each pi the live processes said they are, once each, in order.
    #[must_use]
    pub fn pis(&self) -> &[Pi] {
        &self.pis
    }
}

/// What the live processes said they loaded of the package at `place`,
/// removing each file that no longer counts. A folder that isn't there
/// says nothing.
#[must_use]
pub fn read(place: &Place) -> Said {
    let Ok(entries) = fs::read_dir(place.loaded()) else {
        return Said::default();
    };
    let mut runs = HashMap::new();
    let mut earliest: Option<SystemTime> = None;
    let mut pis = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            // A file being written, `.<pid>.new`, or not one of these.
            continue;
        };
        let Some(started) = started_by(&path, pid) else {
            // Gone already is as good as removed.
            let _ = fs::remove_file(&path);
            continue;
        };
        earliest = Some(earliest.map_or(started, |earliest| earliest.min(started)));
        let Some(value) = said_in(&path) else {
            continue;
        };
        if let Some(root) = runs_of(&value) {
            runs.insert(pid, root);
        }
        pis.extend(Pi::from(&value["pi"]));
    }
    pis.sort();
    pis.dedup();
    Said {
        runs,
        earliest,
        pis,
    }
}

/// When `pid` started, when it is the process that wrote the file at
/// `path`.
fn started_by(path: &Path, pid: u32) -> Option<SystemTime> {
    let written = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    let started = inuse::started(pid)?;
    (started <= written + CLOCK_SLACK).then_some(started)
}

/// What the file at `path` says, when it reads.
fn said_in(path: &Path) -> Option<Value> {
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// The version a file names, while it is there.
fn runs_of(value: &Value) -> Option<PathBuf> {
    // A version since removed can't be run from.
    fs::canonicalize(value["root"].as_str()?).ok()
}

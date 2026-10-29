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

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::Value;

use crate::inuse;
use crate::version::Place;

/// How far a process's start may seem to follow the file it wrote: the
/// boot time Linux reckons a start from is whole seconds.
const CLOCK_SLACK: Duration = Duration::from_secs(2);

/// The version each live process said it runs, by pid, links resolved.
#[derive(Debug, Clone, Default)]
pub struct Said {
    runs: HashMap<u32, PathBuf>,
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
        match runs_of(&path, pid) {
            Some(root) => {
                runs.insert(pid, root);
            }
            None => {
                // Gone already is as good as removed.
                let _ = fs::remove_file(&path);
            }
        }
    }
    Said { runs }
}

/// The version the file at `path` names for `pid`, when it counts.
fn runs_of(path: &Path, pid: u32) -> Option<PathBuf> {
    let written = fs::metadata(path).and_then(|meta| meta.modified()).ok()?;
    let started = inuse::started(pid)?;
    if started > written + CLOCK_SLACK {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    // A version since removed can't be run from.
    fs::canonicalize(value["root"].as_str()?).ok()
}

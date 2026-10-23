//! The desktop's trash, as the freedesktop.org specification lays it out:
//! what was deleted under `files/`, and beside each under `info/` a
//! `.trashinfo` saying when, in local time with no zone.

use crate::clock::{Awake, local};
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// One thing in the trash.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    /// Where it lies, under `files/`.
    pub path: PathBuf,
    /// When it was deleted, in wall seconds.
    pub deleted: f64,
    pub size: u64,
}

const SUFFIX: &str = ".trashinfo";

static DELETED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^DeletionDate=(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2}):(\d{2})").unwrap()
});

fn deletion_date(info: &Path) -> Option<f64> {
    let text = fs::read_to_string(info).ok()?;
    let found = DELETED.captures(&text)?;
    let field = |n: usize| found[n].parse::<i32>().ok();
    local([
        field(1)?,
        field(2)?,
        field(3)?,
        field(4)?,
        field(5)?,
        field(6)?,
    ])
}

/// When a path last changed state, which a move into the trash does.
fn arrived(path: &Path) -> Option<f64> {
    let meta = fs::symlink_metadata(path).ok()?;
    Some(crate::units::wall(meta.ctime(), meta.ctime_nsec()))
}

/// The logical size of what lies under a path, never through a symlink or
/// into another filesystem.
fn size(path: &Path, device: Option<u64>) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if device.is_some_and(|device| device != meta.dev()) {
        return 0;
    }
    if !meta.is_dir() {
        return meta.len();
    }
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };
    let entries = entries.filter_map(Result::ok);
    entries
        .map(|entry| size(&entry.path(), Some(meta.dev())))
        .sum()
}

fn names(dir: &Path) -> Vec<OsString> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.file_name())
        .collect()
}

/// Everything in a trash, with when it was deleted. One with no readable
/// date is dated by when it arrived. An info file whose file is gone is an
/// item too, so it's tidied away.
pub fn items(trash: &Path) -> Vec<Item> {
    let files = trash.join("files");
    let info = trash.join("info");
    let mut all: BTreeSet<OsString> = names(&files).into_iter().collect();
    for name in names(&info) {
        let bytes = name.as_encoded_bytes();
        if let Some(stem) = bytes.strip_suffix(SUFFIX.as_bytes()) {
            // SAFETY: cut from an OsStr at an ASCII suffix.
            all.insert(unsafe { OsStr::from_encoded_bytes_unchecked(stem) }.to_owned());
        }
    }
    let mut found = Vec::new();
    for name in all {
        let path = files.join(&name);
        let mut info_name = name.clone();
        info_name.push(SUFFIX);
        let info_path = info.join(info_name);
        let deleted = deletion_date(&info_path)
            .or_else(|| arrived(&path))
            .or_else(|| arrived(&info_path));
        let Some(deleted) = deleted else { continue };
        found.push(Item {
            size: size(&path, None),
            path,
            deleted,
        });
    }
    found
}

pub fn candidate(item: &Item, awake: &Awake, now: f64) -> Candidate {
    Candidate {
        path: item.path.clone(),
        class: Class::Trash,
        idle: awake.since(item.deleted.min(now), now).max(0.0),
        size: item.size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

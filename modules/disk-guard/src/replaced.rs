//! Replaced builds: the older builds of a crate a target holds beside a
//! newer one, which cargo keeps until the target goes. A change of
//! features, flags or a dependency's version gives a crate a new digest,
//! and a busy checkout gathers them for as long as it's built in.

use crate::age::mtime;
use regex_lite::Regex;
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// One build of a crate in a profile's `deps`: its library artifacts and
/// dep-info, and the fingerprint cargo judges it by.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub deps: PathBuf,
    pub name: String,
    pub digest: String,
    pub files: Vec<PathBuf>,
    pub fingerprint: Option<PathBuf>,
    /// When its newest file was written, in wall seconds.
    pub written: f64,
    pub size: u64,
}

/// A library cargo built, which later builds read: `lib<crate>-<digest>`
/// with the library's own suffix. Binaries and tests are nobody's input.
static LIBRARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^lib(?P<name>\w+)-(?P<digest>[0-9a-f]{16})\.(rlib|rmeta|so|a|dylib)$").unwrap()
});

/// The dep-info rustc writes beside it, named without the prefix.
static DEP_INFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>\w+)-(?P<digest>[0-9a-f]{16})\.d$").unwrap());

/// Each profile's `deps` in a target: `<profile>/deps`, and a cross
/// target's `<triple>/<profile>/deps`. Never through a symlink.
pub fn deps_dirs(target: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for profile in subdirs(target) {
        if is_dir(&profile.join("deps")) {
            found.push(profile.join("deps"));
        }
        for nested in subdirs(&profile) {
            if is_dir(&nested.join("deps")) {
                found.push(nested.join("deps"));
            }
        }
    }
    found.sort();
    found
}

/// Every build in `target` a newer build of its crate replaced.
pub fn replaced(target: &Path) -> Vec<Build> {
    deps_dirs(target)
        .iter()
        .flat_map(|deps| older(builds(deps)))
        .collect()
}

/// The builds in one `deps`, by crate and digest, holding a library.
fn builds(deps: &Path) -> Vec<Build> {
    let Ok(entries) = fs::read_dir(deps) else {
        return Vec::new();
    };
    let mut groups: BTreeMap<(String, String), (Vec<PathBuf>, bool)> = BTreeMap::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let (parsed, library) = match LIBRARY.captures(name) {
            Some(parsed) => (parsed, true),
            None => match DEP_INFO.captures(name) {
                Some(parsed) => (parsed, false),
                None => continue,
            },
        };
        let key = (parsed["name"].to_owned(), parsed["digest"].to_owned());
        let group = groups.entry(key).or_default();
        group.0.push(entry.path());
        group.1 |= library;
    }
    let fingerprints = deps.parent().map(|profile| profile.join(".fingerprint"));
    groups
        .into_iter()
        .filter(|(_, (_, library))| *library)
        .map(|((name, digest), (mut files, _))| {
            files.sort();
            let metadata: Vec<_> = files
                .iter()
                .filter_map(|f| f.symlink_metadata().ok())
                .collect();
            Build {
                deps: deps.to_owned(),
                fingerprint: fingerprints
                    .as_deref()
                    .and_then(|dir| fingerprint(dir, &digest)),
                written: files
                    .iter()
                    .filter_map(|file| mtime(file))
                    .fold(0.0, f64::max),
                size: metadata.iter().map(|meta| meta.size()).sum(),
                name,
                digest,
                files,
            }
        })
        .collect()
}

/// All but the newest build of each crate.
fn older(builds: Vec<Build>) -> Vec<Build> {
    let mut by_crate: BTreeMap<String, Vec<Build>> = BTreeMap::new();
    for build in builds {
        by_crate.entry(build.name.clone()).or_default().push(build);
    }
    by_crate
        .into_values()
        .flat_map(|mut builds| {
            builds.sort_by(|a, b| {
                (a.written, &a.digest)
                    .partial_cmp(&(b.written, &b.digest))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            builds.pop();
            builds
        })
        .collect()
}

/// The fingerprint directory of a build: its package's name, which may
/// differ from the crate's, and the same digest.
fn fingerprint(dir: &Path, digest: &str) -> Option<PathBuf> {
    let suffix = format!("-{digest}");
    subdirs(dir).into_iter().find(|path| {
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with(&suffix))
    })
}

fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect()
}

fn is_dir(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|meta| meta.is_dir())
}

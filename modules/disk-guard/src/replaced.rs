//! Replaced builds: the older builds of a crate, or of a test or binary,
//! a target holds beside a newer one, which cargo keeps until the target
//! goes. A change of features, flags, a dependency's version or the code
//! itself gives a unit a new digest, and a busy checkout gathers them for
//! as long as it's built in.

use crate::age::mtime;
use regex_lite::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

/// One build in a profile's `deps`: a crate's library artifacts, or an
/// executable, with its dep-info, and the fingerprint cargo judges it by.
#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub deps: PathBuf,
    pub name: String,
    pub digest: String,
    pub files: Vec<PathBuf>,
    pub fingerprint: Option<PathBuf>,
    /// The unit it was built for, as its fingerprint says. Nothing for a
    /// library whose fingerprint doesn't say, and an executable never
    /// lacks one.
    pub unit: Option<Unit>,
    /// When its newest file was written, in wall seconds.
    pub written: f64,
    pub size: u64,
}

/// What a build was built as, by its fingerprint: only a build of the same
/// unit and variant replaces another.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unit {
    /// Its package and the name its fingerprint gives it, such as
    /// `server/test-integration-test-integration`. A binary and its unit
    /// tests share a crate name, as two packages' tests can, and a test
    /// checked rather than built is named as a library is.
    pub name: String,
    /// Its features, target, profile and mode, kind, flags and config, as
    /// cargo hashes them: one crate built with two sets of features, or
    /// both checked and built, holds a build of each, both in use.
    pub variant: String,
}

/// What of a fingerprint's JSON says which variant of a unit a build is,
/// leaving what changes when a build replaces another: the code, the
/// dependencies, the features its manifest declares and the compiler.
const VARIANT: [&str; 6] = [
    "features",
    "target",
    "profile",
    "compile_kind",
    "rustflags",
    "config",
];

/// When each file the watch marks was last opened, as it saw it, and since
/// when each folder it marks has been watched. Kept across restarts,
/// since a build replaced a week ago is judged by a week of it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Uses {
    pub opened: BTreeMap<PathBuf, f64>,
    pub watched: BTreeMap<PathBuf, f64>,
    /// Each folder this guard holds a mark on: when the mark landed, and
    /// the folder it landed on. Never kept, since a mark dies with the
    /// guard that placed it, and one on a folder since made anew hears
    /// nothing of the new one.
    #[serde(skip)]
    pub heard: BTreeMap<PathBuf, Heard>,
    /// The last moment the watch lost events, which could have been opens
    /// of anything it marks. Never kept, as the marks aren't.
    #[serde(skip)]
    pub unheard: f64,
}

/// A mark this guard placed on a folder: when, and on which folder, by
/// the handle the watch read before marking it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Heard {
    pub since: f64,
    pub handle: Vec<u8>,
}

impl Uses {
    pub fn opened(&mut self, path: &Path, at: f64) {
        let last = self.opened.entry(path.to_owned()).or_insert(at);
        *last = last.max(at);
    }

    /// Notes that `dir` is watched from `at`, unless it already was.
    pub fn watching(&mut self, dir: &Path, at: f64) {
        self.watched.entry(dir.to_owned()).or_insert(at);
    }

    /// This guard's mark landed at `at` on the folder `handle` names, at
    /// `dir`. A mark again on the folder it already marked keeps its time;
    /// one on a folder made anew at the same path starts afresh.
    pub fn marked(&mut self, dir: &Path, at: f64, handle: Vec<u8>) {
        match self.heard.get_mut(dir) {
            Some(heard) if heard.handle == handle => {}
            Some(heard) => *heard = Heard { since: at, handle },
            None => {
                self.heard
                    .insert(dir.to_owned(), Heard { since: at, handle });
            }
        }
    }

    /// The watch lost events at `at`, which could have been opens of
    /// anything it watches, so everything counts as used then.
    pub fn lost(&mut self, at: f64) {
        for since in self.watched.values_mut() {
            *since = since.max(at);
        }
        self.unheard = self.unheard.max(at);
    }

    /// Forgets every mark, and since when each folder was watched, for a
    /// guard that hears nothing: an open it missed could be of anything,
    /// so no build is dated by what it heard before, and a replaced build
    /// stays as it does where nothing watches.
    pub fn deaf(&mut self) {
        self.heard.clear();
        self.watched.clear();
    }

    /// Since when this guard has heard every one of `dirs` without a gap:
    /// its mark on each, on the folder there now, and its last lost
    /// events. Nothing when one isn't marked, or was made anew since, or
    /// there are none.
    pub fn heard_since(&self, dirs: &[PathBuf]) -> Option<f64> {
        if dirs.is_empty() {
            return None;
        }
        dirs.iter().try_fold(self.unheard, |since, dir| {
            let heard = self.heard.get(dir)?;
            let now = crate::opens::handle(dir).ok()?;
            (now == heard.handle).then_some(since.max(heard.since))
        })
    }

    /// Drops what's no longer there, so the store holds only what's built.
    pub fn forget_gone(&mut self) {
        let there = |path: &PathBuf| path.symlink_metadata().is_ok();
        self.opened.retain(|path, _| there(path));
        self.watched.retain(|path, _| there(path));
        self.heard.retain(|path, _| there(path));
    }

    /// When anything right in one of `dirs` was last opened, which for a
    /// target's [`watched_dirs`] is when a cargo last ran in it or a build
    /// last read one of its libraries. Nothing deeper counts, so a
    /// target kept inside, or rustc's incremental sessions, never date it.
    pub fn last_opened_in(&self, dirs: &[PathBuf]) -> Option<f64> {
        dirs.iter()
            .flat_map(|dir| {
                // Paths order by component, so whatever lies in `dir`
                // follows it at once.
                self.opened
                    .range(dir.clone()..)
                    .take_while(move |(path, _)| path.starts_with(dir))
                    .filter(move |(path, _)| path.parent() == Some(dir.as_path()))
                    .map(|(_, &at)| at)
            })
            .reduce(f64::max)
    }

    /// When a build was last used: its newest open, when it was written if
    /// later, and never before its `deps` was watched, since an open before
    /// then went unseen. Nothing when nobody watches it.
    pub fn last_used(&self, build: &Build) -> Option<f64> {
        let since = *self.watched.get(&build.deps)?;
        let opened = build.files.iter().filter_map(|file| self.opened.get(file));
        Some(opened.fold(since.max(build.written), |last, &at| last.max(at)))
    }
}

/// A library cargo built, which later builds read: `lib<crate>-<digest>`
/// with the library's own suffix.
static LIBRARY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^lib(?P<name>\w+)-(?P<digest>[0-9a-f]{16})\.(rlib|rmeta|so|a|dylib)$").unwrap()
});

/// The dep-info rustc writes beside it, named without the prefix.
static DEP_INFO: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>\w+)-(?P<digest>[0-9a-f]{16})\.d$").unwrap());

/// An executable cargo built, a test, benchmark or binary: named as its
/// dep-info is, with no suffix, and executable.
static EXECUTABLE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?P<name>\w+)-(?P<digest>[0-9a-f]{16})$").unwrap());

/// Each profile's `deps` in a target: `<profile>/deps`, and a cross
/// target's `<triple>/<profile>/deps`. Never through a symlink, and never
/// in a target kept inside this one, which is its own.
pub fn deps_dirs(target: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for profile in subdirs(target) {
        if crate::discover::kept_inside(&profile) {
            continue;
        }
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

/// What the watch marks in a target: each profile's `deps`, whose
/// libraries later builds read, and the profile itself, whose
/// `.cargo-build-lock` every cargo opens, even one that finds every unit
/// fresh and writes nothing. On 18 November a fresh `cargo test` and
/// `cargo clippy` each opened that lock and changed no time in the build.
pub fn watched_dirs(target: &Path) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = deps_dirs(target)
        .into_iter()
        .flat_map(|deps| [deps.parent().map(Path::to_owned), Some(deps)])
        .flatten()
        .collect();
    dirs.sort();
    dirs.dedup();
    dirs
}

/// Every build in `target` a newer build of its crate replaced.
pub fn replaced(target: &Path) -> Vec<Build> {
    deps_dirs(target)
        .iter()
        .flat_map(|deps| older(builds(deps)))
        .collect()
}

/// What a build in `deps` holds besides its dep-info.
#[derive(Clone, Copy, Default, PartialEq)]
enum Made {
    #[default]
    Nothing,
    Library,
    Executable,
}

/// Whether `entry` is a file that runs.
fn runs(entry: &fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    entry
        .metadata()
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

/// The builds in one `deps`, by name and digest, holding a library or an
/// executable whose fingerprint names its unit.
fn builds(deps: &Path) -> Vec<Build> {
    let Ok(entries) = fs::read_dir(deps) else {
        return Vec::new();
    };
    let mut groups: BTreeMap<(String, String), (Vec<PathBuf>, Made)> = BTreeMap::new();
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        let (parsed, made) = if let Some(parsed) = LIBRARY.captures(name) {
            (parsed, Made::Library)
        } else if let Some(parsed) = DEP_INFO.captures(name) {
            (parsed, Made::Nothing)
        } else if let Some(parsed) = EXECUTABLE.captures(name)
            && runs(&entry)
        {
            (parsed, Made::Executable)
        } else {
            continue;
        };
        let key = (parsed["name"].to_owned(), parsed["digest"].to_owned());
        let group = groups.entry(key).or_default();
        group.0.push(entry.path());
        if made != Made::Nothing {
            group.1 = made;
        }
    }
    let fingerprints = deps
        .parent()
        .map(|profile| fingerprints(&profile.join(".fingerprint")))
        .unwrap_or_default();
    groups
        .into_iter()
        .filter_map(|((name, digest), (mut files, made))| {
            if made == Made::Nothing {
                return None;
            }
            let fingerprint = fingerprints.get(&digest).cloned();
            let unit = fingerprint.as_deref().and_then(|dir| unit(dir, &digest));
            // Nothing else says which unit an executable is.
            if made == Made::Executable && unit.is_none() {
                return None;
            }
            files.sort();
            let metadata: Vec<_> = files
                .iter()
                .filter_map(|f| f.symlink_metadata().ok())
                .collect();
            Some(Build {
                deps: deps.to_owned(),
                fingerprint,
                unit,
                written: files
                    .iter()
                    .filter_map(|file| mtime(file))
                    .fold(0.0, f64::max),
                size: metadata.iter().map(MetadataExt::size).sum(),
                name,
                digest,
                files,
            })
        })
        .collect()
}

/// The unit a build's fingerprint is for: its package and the name it
/// gives the unit, the one file in it that isn't its dep-info, its JSON,
/// when it was invoked, or a build script's output; and the variant its
/// JSON beside that file names. Nothing when any of it can't be read.
fn unit(fingerprint: &Path, digest: &str) -> Option<Unit> {
    let package = fingerprint
        .file_name()?
        .to_str()?
        .strip_suffix(&format!("-{digest}"))?;
    let mut names: Vec<String> = fs::read_dir(fingerprint)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            !(name.starts_with("dep-")
                || name.starts_with("output-")
                || Path::new(name)
                    .extension()
                    .is_some_and(|suffix| suffix.eq_ignore_ascii_case("json"))
                || name == "invoked.timestamp")
        })
        .collect();
    names.sort();
    let [unit] = &names[..] else { return None };
    let json = fs::read(fingerprint.join(format!("{unit}.json"))).ok()?;
    let json: serde_json::Value = serde_json::from_slice(&json).ok()?;
    let variant = VARIANT
        .iter()
        .map(|field| json.get(field).map(ToString::to_string))
        .collect::<Option<Vec<_>>>()?
        .join(" ");
    Some(Unit {
        name: format!("{package}/{unit}"),
        variant,
    })
}

/// All but the newest build of each unit and variant, and of each crate
/// whose fingerprint doesn't say, leaving any a profile shares a file
/// with: an executable cargo copied up as the profile's own binary is the
/// one a person runs.
fn older(builds: Vec<Build>) -> Vec<Build> {
    /// What builds replace each other.
    #[derive(PartialEq, Eq, PartialOrd, Ord)]
    enum Kin {
        Unit(Unit),
        Crate(String),
    }
    let mut by_unit: BTreeMap<Kin, Vec<Build>> = BTreeMap::new();
    for build in builds {
        let kin = match &build.unit {
            Some(unit) => Kin::Unit(unit.clone()),
            None => Kin::Crate(build.name.clone()),
        };
        by_unit.entry(kin).or_default().push(build);
    }
    by_unit
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
        .filter(|build| !shared(build))
        .collect()
}

/// Whether another name holds one of a build's files, as cargo's copy of
/// a binary up into its profile does.
fn shared(build: &Build) -> bool {
    build.files.iter().any(|file| {
        file.symlink_metadata()
            .is_ok_and(|meta| meta.is_file() && meta.nlink() > 1)
    })
}

/// Each fingerprint directory in `dir`, by its digest: it is named for
/// its package, which may differ from the crate, and the build's digest.
/// Read once a profile, since a busy one holds thousands.
fn fingerprints(dir: &Path) -> BTreeMap<String, PathBuf> {
    subdirs(dir)
        .into_iter()
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?;
            let (_, digest) = name.rsplit_once('-')?;
            Some((digest.to_owned(), path))
        })
        .collect()
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

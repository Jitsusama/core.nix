//! Finding what the guard may reclaim: cargo targets and dependency
//! installs, each with the checkout it belongs to.

use std::fs;
use std::io::{self, BufRead};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Target,
    Install,
}

/// The checkout something was built for.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Checkout {
    At(PathBuf),
    /// Deleted: the directory its sources were in is gone.
    Gone(PathBuf),
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Found {
    pub path: PathBuf,
    pub kind: Kind,
    pub checkout: Checkout,
}

/// Cargo tags every target it builds into, and writes its rustc info at the
/// top even of one another tool made first without the tag.
const CACHE_TAG: &str = "CACHEDIR.TAG";
const RUSTC_INFO: &str = ".rustc_info.json";

/// The lockfiles that say a `node_modules` holds an install made from one.
const LOCKFILES: [&str; 6] = [
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lock",
    "bun.lockb",
];

/// How many dep-info files to read before giving up on placing a target.
/// Most belong to its dependencies.
const DEP_INFO_BUDGET: usize = 200;

/// Sources under these belong to a toolchain or a dependency.
const FOREIGN_SOURCES: [&str; 3] = ["/.cargo/", "/.rustup/", "/nix/store/"];

/// Whether cargo marked this directory as a target.
fn marked(dir: &Path) -> bool {
    if dir
        .join(RUSTC_INFO)
        .symlink_metadata()
        .is_ok_and(|m| m.is_file())
    {
        return true;
    }
    let mut head = Vec::with_capacity(512);
    let tag = fs::File::open(dir.join(CACHE_TAG));
    tag.is_ok_and(|tag| {
        io::Read::read_to_end(&mut io::Read::take(tag, 512), &mut head).is_ok()
            && String::from_utf8_lossy(&head).contains("created by cargo")
    })
}

/// The subdirectories of a directory, never through a symlink.
fn subdirs(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry.file_type().ok()?.is_dir().then(|| entry.path())
        })
        .collect()
}

/// The top of the git checkout holding `dir`, if any.
fn git_top(dir: &Path) -> Option<PathBuf> {
    dir.ancestors()
        .find(|d| d.join(".git").symlink_metadata().is_ok())
        .map(Path::to_owned)
}

/// Every target and install under `root`, going at most `depth` levels
/// down when given. The walk never follows a symlink or crosses a mount,
/// though it goes on into a Btrfs subvolume, as the work broker clones each
/// tree. It skips git directories and bare repositories, and stops at
/// each target and install, where the files are many and nothing more is
/// to be found. A target is one cargo marked, or one named `target` beside
/// a manifest before cargo has marked it; a `target` with no manifest
/// beside it holds targets, one per project of a checkout holding many.
pub fn walk(root: &Path, depth: Option<usize>) -> io::Result<Vec<Found>> {
    let root = fs::canonicalize(root)?;
    let device = fs::metadata(&root)?.dev();
    let mut found = Vec::new();
    // Each directory goes with its own device, so a subvolume is judged
    // once, where the device changes, rather than at every directory in it.
    let mut pending = vec![(root, 0, device)];
    while let Some((dir, level, device)) = pending.pop() {
        if depth.is_some_and(|depth| level >= depth) {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        let entries: Vec<_> = entries.filter_map(Result::ok).collect();
        let has = |name: &str| entries.iter().any(|entry| entry.file_name() == name);
        // A cargo home tags its downloads as it tags a target.
        let cargo_home = has(".package-cache") || has(".global-cache");
        for entry in &entries {
            let (name, path) = (entry.file_name(), entry.path());
            let name = name.to_string_lossy();
            let is_dir = entry.file_type().is_ok_and(|kind| kind.is_dir());
            let downloads = cargo_home && (name == "registry" || name == "git");
            if !is_dir || downloads || name == ".git" || name.ends_with(".git") {
                continue;
            }
            let Ok(here) = fs::symlink_metadata(&path).map(|m| m.dev()) else {
                continue;
            };
            if here != device && !crate::pool::within(&dir, &path) {
                continue;
            }
            let install = |path: PathBuf| Found {
                path,
                kind: Kind::Install,
                checkout: Checkout::At(git_top(&dir).unwrap_or_else(|| dir.clone())),
            };
            if name == "node_modules" {
                if has("package.json") && LOCKFILES.iter().any(|lock| has(lock)) {
                    found.push(install(path));
                }
            } else if name == "target" && has("Cargo.toml") {
                let checkout = Checkout::At(git_top(&dir).unwrap_or_else(|| dir.clone()));
                found.push(Found {
                    path,
                    kind: Kind::Target,
                    checkout,
                });
            } else if marked(&path) {
                let checkout = checkout_of(&path);
                found.push(Found {
                    path,
                    kind: Kind::Target,
                    checkout,
                });
            } else if name == "target" {
                for project in subdirs(&path).into_iter().filter(|p| marked(p)) {
                    let checkout = checkout_of(&project);
                    found.push(Found {
                        path: project,
                        kind: Kind::Target,
                        checkout,
                    });
                }
            } else if path.join("pyvenv.cfg").is_file() {
                if name == ".venv" {
                    found.push(install(path));
                }
            } else {
                pending.push((path, level + 1, here));
            }
        }
    }
    Ok(found)
}

/// The absolute sources on a dep-info file's first rule, whose spaces are
/// escaped with a backslash.
fn dep_info_sources(file: &Path) -> Vec<PathBuf> {
    let Ok(file) = fs::File::open(file) else {
        return Vec::new();
    };
    let mut line = String::new();
    if io::BufReader::new(io::Read::take(file, 1 << 16))
        .read_line(&mut line)
        .is_err()
    {
        return Vec::new();
    }
    let Some((_, sources)) = line.split_once(": ") else {
        return Vec::new();
    };
    let mut paths = Vec::new();
    let mut current = String::new();
    let mut chars = sources.trim_end().chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&' ') => current.push(chars.next().unwrap()),
            ' ' => paths.push(std::mem::take(&mut current)),
            c => current.push(c),
        }
    }
    paths.push(current);
    paths
        .into_iter()
        .filter(|p| p.starts_with('/'))
        .map(PathBuf::from)
        .collect()
}

/// The dep-info files of a target: those cargo writes beside what it
/// uplifts into a profile first, then rustc's own in `deps`, since a tree
/// that has only run its tests has none of the former.
fn dep_infos(target: &Path) -> impl Iterator<Item = PathBuf> {
    let mut profiles = Vec::new();
    for child in subdirs(target) {
        profiles.extend(subdirs(&child));
        profiles.push(child);
    }
    let inner = ["deps", "build", "incremental", ".fingerprint"];
    let (deps, uplifted): (Vec<_>, Vec<_>) = profiles
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n == "deps" || !inner.iter().any(|i| n == *i))
        })
        .partition(|p| p.file_name().is_some_and(|n| n == "deps"));
    uplifted.into_iter().chain(deps).flat_map(|dir| {
        let entries = fs::read_dir(dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok);
        entries
            .filter(|e| e.file_type().is_ok_and(|t| t.is_file()))
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "d"))
            .collect::<Vec<_>>()
    })
}

/// The checkout a target was built from, by the sources its dep-info
/// names. A source that still exists places it. When none does, the
/// checkout is gone, unless the nearest directory that survives is on
/// another filesystem than the target, as a drive's mount point is.
pub fn checkout_of(target: &Path) -> Checkout {
    let mut missing = None;
    for file in dep_infos(target).take(DEP_INFO_BUDGET) {
        for source in dep_info_sources(&file) {
            let text = source.to_string_lossy();
            if source.starts_with(target) || FOREIGN_SOURCES.iter().any(|f| text.contains(f)) {
                continue;
            }
            let Some(dir) = source.parent() else { continue };
            if source.exists() {
                return Checkout::At(git_top(dir).unwrap_or_else(|| dir.to_owned()));
            }
            missing.get_or_insert(source);
        }
    }
    let Some(source) = missing else {
        return Checkout::Unknown;
    };
    let mut gone = source.as_path();
    let survivor = loop {
        match gone.parent() {
            Some(parent) if !parent.exists() => gone = parent,
            Some(parent) => break parent,
            None => return Checkout::Unknown,
        }
    };
    let device = |path: &Path| fs::metadata(path).map(|m| m.dev()).ok();
    if device(survivor).is_some() && device(survivor) == device(target) {
        Checkout::Gone(gone.to_owned())
    } else {
        Checkout::Unknown
    }
}

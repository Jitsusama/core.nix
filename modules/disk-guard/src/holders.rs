//! What holds a guard's disks, for whoever has to clear what the guard may
//! not: each folder straight under a source's roots, and each cache, store
//! and scratch folder whole, with who it belongs to and whether the guard
//! may clear any of it. A disk the guard has emptied of all it may still
//! fills, and Joel asked why it wasn't helping; the answer is what holds
//! the rest, and whose it is.

use crate::activity::Use;
use crate::anchor::Dir;
use crate::config::{Config, Source};
use crate::units::shown;
use crate::words::tilde;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// How many holders a status file lists, biggest first.
pub const SHOWN: usize = 10;

/// Whose a holder is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A quest's workspace, named by the quest.
    Quest,
    /// A git worktree, named by its folder.
    Worktree,
    /// A cache, named by its source.
    Cache,
    /// Container storage.
    Containers,
    /// Anything else: a checkout, a store, scratch, the trash.
    Other,
}

impl Kind {
    /// Which of two kinds a place both name is: the one that says more.
    fn rank(self) -> u8 {
        match self {
            Kind::Other => 0,
            Kind::Worktree => 1,
            Kind::Quest | Kind::Cache | Kind::Containers => 2,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Owner {
    pub kind: Kind,
    pub name: String,
}

/// A folder sized as one.
#[derive(Clone, Debug, PartialEq)]
pub struct Place {
    pub path: PathBuf,
    pub owner: Owner,
}

/// One holder, as the status file says it.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Holder {
    pub path: PathBuf,
    pub bytes: u64,
    pub owner: Owner,
    /// Whether any of it is something the guard clears, once it has sat
    /// idle long enough for the disk's band.
    pub clearable: bool,
    /// Why the guard may clear none of it, when it may not.
    pub why: Option<String>,
    pub says: String,
}

/// Every place the sources name. A folder two sources name is the kind
/// that says more; a folder holding another place is left out, since the
/// places beneath it say whose its space is and it would count them twice.
pub fn places(config: &Config) -> Vec<Place> {
    let mut found = Vec::new();
    for (name, source) in &config.sources {
        let whole = |kind| Place {
            path: resolved(source_path(source).unwrap_or_else(|| Path::new("/"))),
            owner: Owner {
                kind,
                name: name.clone(),
            },
        };
        match source {
            Source::Cargo { roots, .. } | Source::Installs { roots, .. } => {
                for root in roots {
                    found.extend(children(root, |child| {
                        if child.join(".git").is_file() {
                            Kind::Worktree
                        } else {
                            Kind::Other
                        }
                    }));
                }
            }
            Source::Workspaces { path, .. } => found.extend(children(path, |_| Kind::Quest)),
            Source::Cache { .. } => found.push(whole(Kind::Cache)),
            Source::Podman { .. } => found.push(whole(Kind::Containers)),
            Source::Finished { .. } => {}
            _ => found.push(whole(Kind::Other)),
        }
    }
    let mut places: Vec<Place> = Vec::new();
    for place in found {
        match places.iter_mut().find(|kept| kept.path == place.path) {
            Some(kept) if place.owner.kind.rank() > kept.owner.kind.rank() => *kept = place,
            Some(_) => {}
            None => places.push(place),
        }
    }
    let holding = |place: &Place| {
        places
            .iter()
            .any(|other| other.path != place.path && other.path.starts_with(&place.path))
    };
    places
        .iter()
        .filter(|place| !holding(place))
        .cloned()
        .collect()
}

/// The folder a source that isn't a set of roots names.
fn source_path(source: &Source) -> Option<&Path> {
    Some(match source {
        Source::Cache { path, .. }
        | Source::Scratch { path, .. }
        | Source::Snapshots { path, .. }
        | Source::Workspaces { path, .. }
        | Source::Temp { path }
        | Source::Trash { path }
        | Source::Logs { path }
        | Source::Podman { path, .. }
        | Source::Producer { path, .. }
        | Source::Coredumps { path }
        | Source::NixBuilds { path } => path,
        Source::Cargo { .. } | Source::Installs { .. } | Source::Finished { .. } => return None,
    })
}

/// The folders straight under `root`, each named by itself, never through
/// a link.
fn children(root: &Path, kind: impl Fn(&Path) -> Kind) -> Vec<Place> {
    let root = resolved(root);
    let Ok(entries) = fs::read_dir(&root) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| {
            let path = entry.path();
            Place {
                owner: Owner {
                    kind: kind(&path),
                    name: entry.file_name().to_string_lossy().into_owned(),
                },
                path,
            }
        })
        .collect()
}

/// A path with its links resolved, as the survey resolves its roots, so a
/// place and an offer beneath it agree on where they are.
fn resolved(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_owned())
}

/// Each place's size, as it takes the disk: its blocks, with hard links
/// counted once, read beneath it and never through a link.
pub fn measure(places: Vec<Place>) -> Vec<(Place, u64)> {
    places
        .into_iter()
        .map(|place| {
            let bytes = Dir::open(&place.path)
                .and_then(|dir| dir.size())
                .unwrap_or(0);
            (place, bytes)
        })
        .collect()
}

/// The biggest holders, from what was measured, what the last survey
/// offered (each candidate's path and size) and the trees it kept for
/// their use.
pub fn holders(
    measured: &[(Place, u64)],
    offered: &[(PathBuf, u64)],
    kept: &[(PathBuf, Use)],
    home: Option<&Path>,
) -> Vec<Holder> {
    let mut holders: Vec<Holder> = measured
        .iter()
        .filter(|(_, bytes)| *bytes > 0)
        .map(|(place, bytes)| {
            let clears: u64 = offered
                .iter()
                .filter(|(path, _)| path.starts_with(&place.path))
                .map(|(_, size)| size)
                .sum();
            let why = (clears == 0).then(|| why(place, kept));
            let name = called(&place.owner, &place.path, home);
            let says = match &why {
                Some(why) => format!("{name} holds {}; {why}.", shown(*bytes)),
                None => format!(
                    "{name} holds {}, of which {} is builds or caches the guard clears once \
                     they sit idle long enough.",
                    shown(*bytes),
                    shown(clears.min(*bytes))
                ),
            };
            Holder {
                path: place.path.clone(),
                bytes: *bytes,
                owner: place.owner.clone(),
                clearable: why.is_none(),
                why,
                says,
            }
        })
        .collect();
    holders.sort_by_key(|holder| std::cmp::Reverse(holder.bytes));
    holders.truncate(SHOWN);
    holders
}

/// Why the guard may clear none of a place.
fn why(place: &Place, kept: &[(PathBuf, Use)]) -> String {
    let used = kept
        .iter()
        .filter(|(path, _)| path.starts_with(&place.path))
        .map(|(_, used)| *used)
        .find(|used| matches!(used, Use::Busy(_) | Use::Unknown(_)));
    match (used, place.owner.kind) {
        (Some(Use::Busy(pid)), _) => format!("a build is using it (process {pid})"),
        (Some(Use::Unknown(pid)), _) => {
            format!("process {pid} may be using it, and the guard can't read what it does")
        }
        (_, Kind::Quest) => "its quest is still open, or only just finished".to_owned(),
        (_, Kind::Containers) => "podman prunes only images no container uses".to_owned(),
        (_, Kind::Cache) => "it's empty, or being refilled".to_owned(),
        _ => "nothing in it is a build or cache the guard clears".to_owned(),
    }
}

/// A holder as a sentence opens with it.
fn called(owner: &Owner, path: &Path, home: Option<&Path>) -> String {
    match owner.kind {
        Kind::Quest => format!("{}'s workspace", owner.name),
        Kind::Worktree => format!("The worktree {}", owner.name),
        Kind::Cache => format!("The {} cache", owner.name),
        Kind::Containers => "Container storage".to_owned(),
        Kind::Other => tilde(path, home),
    }
}

//! What holds a guard's disks, for whoever has to clear what the guard may
//! not: each folder straight under a source's roots, and each cache, store
//! and scratch folder whole, with who it belongs to and whether the guard
//! may clear any of it. A disk the guard has emptied of all it may still
//! fills, and Joel asked why it wasn't helping; the answer is what holds
//! the rest, and whose it is.

use crate::activity::Use;
use crate::anchor::Dir;
use crate::config::{Config, Source};
use crate::keeps::{Keep, Keeps, covering};
use crate::units::shown;
use crate::words::tilde;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// How many holders a status file lists, biggest first.
pub const SHOWN: usize = 10;

/// Whose a holder is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Holder {
    pub path: PathBuf,
    pub bytes: u64,
    pub owner: Owner,
    /// What a sentence about it calls it: the quest, worktree or cache's
    /// name, or its folder.
    pub name: String,
    /// Whether any of it is something the guard clears, once it has sat
    /// idle long enough for the disk's band. A keep leaves this as the
    /// guard judged it, so letting go of the keep needs no new judging.
    pub clearable: bool,
    /// How much of it a clear asked for now would take.
    pub clears: u64,
    /// Whether a clear of it, or of part of it, is waiting or under way.
    #[serde(default)]
    pub clearing: bool,
    /// Why the guard may clear none of it, when it may not.
    pub why: Option<String>,
    /// Joel's keep on it, when he keeps it or a folder holding it.
    pub kept: Option<Keep>,
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
            Source::Git { repos } => found.extend(repos.iter().map(|repo| Place {
                path: resolved(repo),
                owner: Owner {
                    kind: Kind::Other,
                    name: name.clone(),
                },
            })),
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
        Source::Cargo { .. }
        | Source::Installs { .. }
        | Source::Git { .. }
        | Source::Finished { .. } => return None,
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
/// One thing the last survey offered: where, how big, and whether a clear
/// asked for now would take it.
#[derive(Clone, Debug, PartialEq)]
pub struct Offered {
    pub path: PathBuf,
    pub size: u64,
    pub now: bool,
}

pub fn holders(
    measured: &[(Place, u64)],
    offered: &[Offered],
    kept: &[(PathBuf, Use)],
    home: Option<&Path>,
) -> Vec<Holder> {
    let mut holders: Vec<Holder> = measured
        .iter()
        .filter(|(_, bytes)| *bytes > 0)
        .map(|(place, bytes)| {
            let inside: Vec<&Offered> = offered
                .iter()
                .filter(|offer| offer.path.starts_with(&place.path))
                .collect();
            let clears: u64 = inside
                .iter()
                .filter(|offer| offer.now)
                .map(|offer| offer.size)
                .sum();
            let offered: u64 = inside.iter().map(|offer| offer.size).sum();
            let why = (offered == 0).then(|| why(place, kept));
            let mut holder = Holder {
                path: place.path.clone(),
                bytes: *bytes,
                owner: place.owner.clone(),
                name: match place.owner.kind {
                    Kind::Other => tilde(&place.path, home),
                    _ => place.owner.name.clone(),
                },
                clearable: why.is_none(),
                clears: clears.min(*bytes),
                clearing: false,
                why,
                kept: None,
                says: String::new(),
            };
            holder.says = says(&holder, home);
            holder
        })
        .collect();
    holders.sort_by_key(|holder| std::cmp::Reverse(holder.bytes));
    holders.truncate(SHOWN);
    holders
}

/// Marks each holder with the keep that covers it, or none, and with
/// whether a clear of it, or of part of it, is asked, and says it again.
/// Marking is the same however often it's done, so each writer of the
/// status file marks what it writes with the keeps and asks as they are.
pub fn mark(holders: &mut [Holder], keeps: &Keeps, clearing: &[PathBuf], home: Option<&Path>) {
    for holder in holders {
        holder.kept = covering(keeps, &holder.path).cloned();
        holder.clearing = clearing
            .iter()
            .any(|asked| asked.starts_with(&holder.path) || holder.path.starts_with(asked));
        holder.says = says(holder, home);
    }
}

/// One sentence for a holder: whose it is, how much it holds, and what
/// keeps it or how much of it the guard clears.
fn says(holder: &Holder, home: Option<&Path>) -> String {
    let name = called(&holder.owner, &holder.path, home);
    let bytes = shown(holder.bytes);
    match (&holder.kept, &holder.why) {
        (Some(keep), _) => format!("{name} holds {bytes}; you keep it: {}.", keep.reason),
        (None, Some(why)) => format!("{name} holds {bytes}; {why}."),
        (None, None) if holder.clears == 0 => format!(
            "{name} holds {bytes}; the guard clears some of it once it has sat idle long \
             enough."
        ),
        (None, None) => format!(
            "{name} holds {bytes}, of which {} can be cleared now.",
            shown(holder.clears)
        ),
    }
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

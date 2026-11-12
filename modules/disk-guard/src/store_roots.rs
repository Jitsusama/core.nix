//! Store roots: links a tool keeps under a folder to hold a build of its
//! own from collection, as a development shell keeps its environment,
//! which the tool makes again when next asked. They're found among Nix's
//! indirect roots, which name every such link, rather than by walking
//! folders that can hold hundreds of trees.

use crate::age::{last_write, mtime};
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use regex_lite::Regex;
use std::fs;
use std::path::{Path, PathBuf};

/// One link found, with what dates its last use.
#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub path: PathBuf,
    /// The folder straight under its root that holds it, which a process
    /// working anywhere in keeps it.
    pub tree: PathBuf,
    /// The newest write in the place its pattern names, in wall seconds;
    /// infinite when there was too much to read.
    pub written: f64,
}

/// How far down a place's last write is looked for, and how many entries
/// may be read on the way before it counts as written now: a tool's own
/// folder of links and marks is small.
const DEPTH: usize = 2;
const BUDGET: usize = 1_000;

/// The links Nix's indirect roots in `auto` name that lie under `root`,
/// whose path `pattern` matches, and that still link into `store`. Each
/// is dated by the newest write in the place the pattern's `place` group
/// names, or in the folder holding it without one, so a tool that writes
/// beside its link each time it's used keeps it.
pub fn found(auto: &Path, root: &Path, pattern: &Regex, store: &Path) -> Vec<Link> {
    let Ok(listing) = fs::read_dir(auto) else {
        return Vec::new();
    };
    listing
        .filter_map(Result::ok)
        .filter_map(|entry| fs::read_link(entry.path()).ok())
        .filter_map(|path| link(path, root, pattern, store))
        .collect()
}

fn link(path: PathBuf, root: &Path, pattern: &Regex, store: &Path) -> Option<Link> {
    let tree = root.join(path.strip_prefix(root).ok()?.components().next()?);
    if tree == path || !fs::read_link(&path).ok()?.starts_with(store) {
        return None;
    }
    let named = path.to_str()?;
    let matched = pattern.captures(named)?;
    let place = match matched.name("place") {
        Some(place) => PathBuf::from(&named[..place.end()]),
        None => path.parent()?.to_owned(),
    };
    Some(Link {
        written: last_write(&place, DEPTH, BUDGET).max(mtime(&path).unwrap_or(0.0)),
        path,
        tree,
    })
}

/// The candidate a link makes, idle for the awake time since its place
/// was last written, going once it's been `unused` awake seconds.
pub fn candidate(link: &Link, unused: f64, awake: &Awake, now: f64) -> Candidate {
    let size = fs::symlink_metadata(&link.path).map_or(0, |meta| {
        use std::os::unix::fs::MetadataExt;
        meta.blocks() * 512
    });
    Candidate {
        path: link.path.clone(),
        class: Class::StoreRoot { unused },
        idle: awake.since(link.written.min(now), now).max(0.0),
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

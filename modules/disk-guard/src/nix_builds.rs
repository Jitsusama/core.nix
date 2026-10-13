//! Build directories Nix leaves behind: `nix-<pid>-<counter>` under its
//! build directory, made by the process running the build, which removes
//! it when the build ends unless the process died first.

use crate::activity::Process;
use crate::clock::Awake;
use crate::plan::{Candidate, Class};
use crate::snapshots::{self, Boot, Snapshot};
use regex_lite::Regex;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;
use std::sync::LazyLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Build {
    pub snapshot: Snapshot,
    /// When a failed build was kept on purpose, with `--keep-failed`.
    pub kept: Option<f64>,
}

static NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^nix-(?P<pid>\d+)-\d+$").expect("the build pattern compiles"));

/// The programs that run a build: the daemon's worker, or a client
/// building without one.
const MAKERS: [&str; 5] = ["nix-daemon", "nix", "nix-build", "nix-store", "nix-shell"];

/// How long a kept failure stays while the disk is healthy, to be looked
/// into.
const KEPT: f64 = 7.0 * 86_400.0;

/// Every build directory in `dir`. Nix makes one private and opens it to
/// everyone only when keeping a failure, so one others can read was kept,
/// dated by that change.
pub fn find(dir: &Path) -> Vec<Build> {
    snapshots::entries(dir, &NAME)
        .into_iter()
        .map(|snapshot| {
            let kept = fs::symlink_metadata(&snapshot.path)
                .ok()
                .filter(|meta| meta.mode() & 0o044 != 0)
                .map(|meta| meta.ctime() as f64);
            Build { snapshot, kept }
        })
        .collect()
}

/// Whether no process running a build could still be using it.
pub fn orphaned(build: &Build, processes: &[Process], boot: Boot) -> bool {
    snapshots::orphaned(&build.snapshot, processes, &MAKERS, boot)
}

/// An abandoned build is waste. A failure kept on purpose ages as scratch
/// from when it was kept: a week while healthy, then by the scratch
/// windows.
pub fn candidate(build: &Build, size: u64, awake: &Awake, now: f64) -> Candidate {
    let (class, from) = match build.kept {
        Some(kept) => (
            Class::Scratch {
                healthy: Some(KEPT),
            },
            kept.max(build.snapshot.born),
        ),
        None => (Class::NixBuild, build.snapshot.born),
    };
    Candidate {
        path: build.snapshot.path.clone(),
        class,
        idle: awake.since(from.min(now), now).max(0.0),
        size,
        occupied: false,
        spent: false,
        checkout: None,
    }
}

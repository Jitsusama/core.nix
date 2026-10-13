//! What the sources offer: every candidate, with how it's reclaimed and
//! the filesystem it lies on, so a pass runs on one filesystem with only
//! what lies there.

use crate::activity::{self, Process, Use};
use crate::anchor::Dir;
use crate::clock::Awake;
use crate::config::{CacheProtocol, Config, Source};
use crate::discover::{self, Checkout, Kind};
use crate::plan::{Candidate, Class};
use crate::snapshots::Boot;
use crate::{
    age, caches, coredumps, logs, nix_builds, scratch, snapshots, temp, trash, trees, workspaces,
};
use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// How a candidate is reclaimed, under the root it was found beneath,
/// which every delete is anchored to.
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    /// A cargo target, with its locks held.
    Target { root: PathBuf },
    /// An incremental variant inside a target, with rustc's locks held.
    Variant { root: PathBuf },
    /// One entry, judged, moved aside, judged again and removed.
    Entry { root: PathBuf },
    /// An item of a desktop trash and its info file.
    Trash { trash: PathBuf },
    /// A quest workspace, whose repositories prune its worktrees after.
    Workspace { root: PathBuf },
    /// A log, trimmed in place to its last part.
    Trim { root: PathBuf },
    /// A source's command, run where it frees space.
    Run {
        root: PathBuf,
        source: String,
        argv: Vec<String>,
        timeout: Duration,
    },
    /// A batch of Nix's garbage, deleted by `nix store delete`.
    Collect {
        root: PathBuf,
        nix: PathBuf,
        paths: Vec<String>,
    },
    /// The system daemon's garbage on the filesystem mounted at `root`,
    /// asked of it at `socket` in the pass's band.
    Ask { root: PathBuf, socket: PathBuf },
    /// A profile's generations, expired by `nix-env` beside `nix`; `root`
    /// is the directory holding the profile.
    Expire {
        root: PathBuf,
        nix: PathBuf,
        profile: PathBuf,
        numbers: Vec<u32>,
    },
}

impl Action {
    /// The directory its delete is anchored to.
    pub fn root(&self) -> &Path {
        match self {
            Action::Target { root }
            | Action::Variant { root }
            | Action::Entry { root }
            | Action::Workspace { root }
            | Action::Trim { root }
            | Action::Run { root, .. }
            | Action::Collect { root, .. }
            | Action::Ask { root, .. }
            | Action::Expire { root, .. } => root,
            Action::Trash { trash } => trash,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Offer {
    pub candidate: Candidate,
    pub action: Action,
    /// The filesystem it lies on.
    pub device: u64,
}

/// What a survey judges by.
pub struct World<'a> {
    pub processes: &'a [Process],
    /// The guard's own pid, whose opens judge nothing.
    pub me: u32,
    /// The account the guard reclaims for.
    pub uid: u32,
    pub boot: Boot,
    pub awake: &'a Awake,
    /// Wall seconds.
    pub now: f64,
    /// Whether a checkout's work is finished, as its predicate says.
    pub finished: &'a mut dyn FnMut(&Path) -> bool,
    /// When each source's command last ran, in wall seconds, by name.
    pub ran: &'a BTreeMap<String, f64>,
}

/// How deep and how widely a workspace's writes are read.
const WRITE_DEPTH: usize = 4;
const WRITE_BUDGET: usize = 50_000;

/// Every candidate the sources offer now.
pub fn survey(config: &Config, world: &mut World) -> Vec<Offer> {
    let mut offers = Vec::new();
    for (name, source) in &config.sources {
        match source {
            Source::Cargo { roots, depth } => {
                for root in roots {
                    trees_under(root, *depth, Kind::Target, world, &mut offers);
                }
            }
            Source::Installs { roots, depth } => {
                for root in roots {
                    trees_under(root, *depth, Kind::Install, world, &mut offers);
                }
            }
            Source::Cache {
                path,
                from,
                protocol,
                rank,
            } => {
                let Some(device) = device(path) else { continue };
                match protocol {
                    Some(CacheProtocol::GoBuild) => {
                        for entry in caches::go_build(path) {
                            let candidate =
                                caches::go_entry(&entry, *rank, *from, world.awake, world.now);
                            offers.push(Offer {
                                candidate,
                                action: Action::Entry { root: path.clone() },
                                device,
                            });
                        }
                    }
                    None => {
                        let Some(parent) = path.parent() else {
                            continue;
                        };
                        let candidate = caches::whole(path, *rank, *from, size(path));
                        offers.push(Offer {
                            candidate,
                            action: Action::Entry {
                                root: parent.to_owned(),
                            },
                            device,
                        });
                    }
                }
            }
            Source::Scratch { path, healthy } => {
                let Some(device) = device(path) else { continue };
                let healthy = healthy.map(|span| span.0.as_secs_f64());
                for entry in scratch::entries(path) {
                    let size = size(&entry.path);
                    let candidate =
                        scratch::candidate(&entry, healthy, size, world.awake, world.now);
                    offers.push(Offer {
                        candidate,
                        action: Action::Entry { root: path.clone() },
                        device,
                    });
                }
            }
            Source::Snapshots {
                path,
                pattern,
                command,
            } => {
                let (Some(device), Ok(pattern)) = (device(path), regex_lite::Regex::new(pattern))
                else {
                    continue;
                };
                for snapshot in snapshots::entries(path, &pattern) {
                    if snapshots::orphaned(&snapshot, world.processes, &[command], world.boot) {
                        let candidate = snapshots::candidate(
                            &snapshot,
                            size(&snapshot.path),
                            world.awake,
                            world.now,
                        );
                        offers.push(Offer {
                            candidate,
                            action: Action::Entry { root: path.clone() },
                            device,
                        });
                    }
                }
            }
            Source::Workspaces { path, quests } => {
                let Some(device) = device(path) else { continue };
                for workspace in workspaces::find(path, quests) {
                    let written = age::last_write(&workspace.path, WRITE_DEPTH, WRITE_BUDGET);
                    let size = size(&workspace.path);
                    if let Some(candidate) =
                        workspaces::candidate(&workspace, written, size, world.awake, world.now)
                    {
                        offers.push(Offer {
                            candidate,
                            action: Action::Workspace { root: path.clone() },
                            device,
                        });
                    }
                }
            }
            Source::Temp { path } => {
                let Some(device) = device(path) else { continue };
                let tmpdirs: Vec<PathBuf> = world
                    .processes
                    .iter()
                    .flat_map(|process| &process.env)
                    .filter(|(name, _)| name == "TMPDIR")
                    .map(|(_, value)| PathBuf::from(value))
                    .collect();
                for entry in temp::leftovers(temp::survey(path), world.uid, &tmpdirs) {
                    let candidate = temp::candidate(&entry, world.awake, world.now);
                    offers.push(Offer {
                        candidate,
                        action: Action::Entry { root: path.clone() },
                        device,
                    });
                }
            }
            Source::Trash { path } => {
                let Some(device) = device(path) else { continue };
                for item in trash::items(path) {
                    let candidate = trash::candidate(&item, world.awake, world.now);
                    offers.push(Offer {
                        candidate,
                        action: Action::Trash {
                            trash: path.clone(),
                        },
                        device,
                    });
                }
            }
            Source::Coredumps { path } => {
                let Some(device) = device(path) else { continue };
                for dump in coredumps::find(path) {
                    let candidate = coredumps::candidate(&dump, world.awake, world.now);
                    offers.push(Offer {
                        candidate,
                        action: Action::Entry { root: path.clone() },
                        device,
                    });
                }
            }
            Source::NixBuilds { path } => {
                let Some(device) = device(path) else { continue };
                for build in nix_builds::find(path) {
                    if nix_builds::orphaned(&build, world.processes, world.boot) {
                        let size = size(&build.snapshot.path);
                        let candidate = nix_builds::candidate(&build, size, world.awake, world.now);
                        offers.push(Offer {
                            candidate,
                            action: Action::Entry { root: path.clone() },
                            device,
                        });
                    }
                }
            }
            Source::Logs { path } => {
                let Some(device) = device(path) else { continue };
                for log in logs::oversized(path, logs::LIMIT) {
                    let Ok(meta) = fs::symlink_metadata(&log) else {
                        continue;
                    };
                    let candidate = Candidate {
                        path: log,
                        class: Class::Log,
                        idle: 0.0,
                        size: meta.len().saturating_sub(logs::KEEP),
                        occupied: false,
                        spent: false,
                        checkout: None,
                    };
                    offers.push(Offer {
                        candidate,
                        action: Action::Trim { root: path.clone() },
                        device,
                    });
                }
            }
            Source::Producer {
                command,
                path,
                every,
                from,
                timeout,
            } => {
                let Some(device) = device(path) else { continue };
                let idle = since_ran(world, name);
                offers.push(Offer {
                    candidate: Candidate {
                        path: named(path, name),
                        class: Class::Producer {
                            every: every.map(|span| span.0.as_secs_f64()),
                            from: *from,
                        },
                        idle,
                        // What it frees is measured once it has run.
                        size: 0,
                        occupied: false,
                        spent: false,
                        checkout: None,
                    },
                    action: Action::Run {
                        root: path.clone(),
                        source: name.clone(),
                        argv: command.clone(),
                        timeout: timeout.0,
                    },
                    device,
                });
            }
            Source::Podman { command, path } => {
                let Some(device) = device(path) else { continue };
                let program = command.to_string_lossy().into_owned();
                for (unused, source, how) in [
                    (false, name.clone(), "--build-cache"),
                    (true, format!("{name}:unused"), "--all"),
                ] {
                    let argv = [&program, "image", "prune", "--force", how];
                    offers.push(Offer {
                        candidate: Candidate {
                            path: named(path, &source),
                            class: Class::Podman { unused },
                            idle: since_ran(world, &source),
                            size: 0,
                            occupied: false,
                            spent: false,
                            checkout: None,
                        },
                        action: Action::Run {
                            root: path.clone(),
                            source,
                            argv: argv.map(str::to_owned).to_vec(),
                            timeout: PODMAN_TIMEOUT,
                        },
                        device,
                    });
                }
            }
            Source::Finished { .. } => {}
        }
    }
    offers
}

/// The targets or installs beneath a root, and a target's variants. A
/// tree a build is running in, or whose use can't be judged, offers
/// nothing.
fn trees_under(
    root: &Path,
    depth: Option<u32>,
    kind: Kind,
    world: &mut World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(root) else { return };
    let Ok(found) = discover::walk(root, depth.map(|depth| depth as usize)) else {
        return;
    };
    for tree in found.into_iter().filter(|tree| tree.kind == kind) {
        let checkout = match &tree.checkout {
            Checkout::At(dir) => Some(dir.as_path()),
            _ => None,
        };
        let used = activity::judge(world.processes, world.me, &tree.path, checkout);
        // The candidate refuses these too; deciding first spares sizing a
        // tree a build is writing to.
        if matches!(used, Use::Busy(_) | Use::Unknown(_)) {
            continue;
        }
        let finished = checkout.is_some_and(|dir| (world.finished)(dir));
        let seen = trees::Seen {
            used,
            finished,
            size: size(&tree.path),
            last_used: trees::last_used(&tree),
        };
        let Some(candidate) = trees::candidate(&tree, &seen, world.awake, world.now) else {
            continue;
        };
        let action = match kind {
            Kind::Target => Action::Target {
                root: root.to_owned(),
            },
            Kind::Install => Action::Entry {
                root: root.to_owned(),
            },
        };
        offers.push(Offer {
            candidate,
            action,
            device,
        });
        if kind == Kind::Target {
            for variant in trees::variants(&tree.path) {
                let candidate = Candidate {
                    path: variant.path.clone(),
                    class: Class::Variant,
                    idle: world
                        .awake
                        .since(variant.last_used.min(world.now), world.now)
                        .max(0.0),
                    size: size(&variant.path),
                    occupied: false,
                    spent: false,
                    checkout: checkout.map(Path::to_owned),
                };
                offers.push(Offer {
                    candidate,
                    action: Action::Variant {
                        root: root.to_owned(),
                    },
                    device,
                });
            }
        }
    }
}

/// How long podman's prunes may take.
const PODMAN_TIMEOUT: Duration = Duration::from_secs(300);

/// A command's candidate, named for where it runs and what it is, since
/// it is no file of its own.
pub(crate) fn named(path: &Path, source: &str) -> PathBuf {
    let mut named = path.as_os_str().to_owned();
    named.push("#");
    named.push(source);
    PathBuf::from(named)
}

/// The awake time since a source's command last ran, or forever.
fn since_ran(world: &World, source: &str) -> f64 {
    world.ran.get(source).map_or(f64::INFINITY, |&at| {
        world.awake.since(at.min(world.now), world.now)
    })
}

/// The filesystem a source's directory is on, as its space is shared.
fn device(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().filter(|meta| meta.is_dir())?;
    crate::pool::pool(path).ok()
}

/// What an entry holds: a directory's allocated size beneath it, a file's
/// own, nothing when it can't be read.
fn size(path: &Path) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Dir::open(path).and_then(|dir| dir.size()).unwrap_or(0),
        Ok(meta) => meta.blocks() * 512,
        Err(_) => 0,
    }
}

//! What the sources offer: every candidate, with how it's reclaimed and
//! the filesystem it lies on, so a pass runs on one filesystem with only
//! what lies there. Logs, podman and producers offer nothing here; they
//! run as upkeep.

use crate::activity::{self, Process, Use};
use crate::anchor::Dir;
use crate::clock::Awake;
use crate::config::{CacheProtocol, Config, Source};
use crate::discover::{self, Checkout, Kind};
use crate::plan::{Candidate, Class};
use crate::snapshots::Boot;
use crate::{
    age, caches, coredumps, nix_builds, scratch, snapshots, temp, trash, trees, workspaces,
};
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

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
}

impl Action {
    /// The directory its delete is anchored to.
    pub fn root(&self) -> &Path {
        match self {
            Action::Target { root }
            | Action::Variant { root }
            | Action::Entry { root }
            | Action::Workspace { root } => root,
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
}

/// How deep and how widely a workspace's writes are read.
const WRITE_DEPTH: usize = 4;
const WRITE_BUDGET: usize = 50_000;

/// Every candidate the sources offer now.
pub fn survey(config: &Config, world: &mut World) -> Vec<Offer> {
    let mut offers = Vec::new();
    for source in config.sources.values() {
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
            Source::Logs { .. }
            | Source::Podman { .. }
            | Source::Producer { .. }
            | Source::Finished { .. } => {}
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

fn device(path: &Path) -> Option<u64> {
    fs::metadata(path)
        .ok()
        .filter(|meta| meta.is_dir())
        .map(|meta| meta.dev())
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

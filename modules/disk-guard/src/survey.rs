//! What the sources offer: every candidate, with how it's reclaimed and
//! the filesystem it lies on, so a pass runs on one filesystem with only
//! what lies there.

use crate::activity::{self, Process, Use};
use crate::anchor::{Counted, Dir};
use crate::band::Band;
use crate::clock::Awake;
use crate::config::{CacheProtocol, Config, Source};
use crate::discover::{self, Checkout, Kind};
use crate::git;
use crate::plan::{Candidate, Class};
use crate::replaced::Uses;
use crate::snapshots::Boot;
use crate::units::Span;
use crate::{
    age, caches, coredumps, logs, nix_builds, scratch, snapshots, temp, trash, trees, workspaces,
};
use std::cell::RefCell;
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
    /// An older build of a crate inside `target`, with cargo's locks held:
    /// its libraries, dep-info and fingerprint.
    Build { root: PathBuf, target: PathBuf },
    /// One entry, judged, moved aside, judged again and removed.
    Entry { root: PathBuf },
    /// One entry of a cache that refills itself and trims itself by age,
    /// as Go's build cache does: removed without judging who uses it.
    CacheEntry { root: PathBuf },
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
        /// Where Nix's clients keep their temporary roots, read again just
        /// before each delete.
        roots: PathBuf,
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
            | Action::Build { root, .. }
            | Action::Entry { root }
            | Action::CacheEntry { root }
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
    /// The account's home, for the account's guard: a checkout missing
    /// from its filesystem was deleted, since that filesystem is here.
    pub home: Option<&'a Path>,
    pub boot: Boot,
    pub awake: &'a Awake,
    /// Wall seconds.
    pub now: f64,
    /// Whether a checkout's work is finished, as its predicate says.
    pub finished: &'a mut dyn FnMut(&Path) -> bool,
    /// When each source's command last ran, in wall seconds, by name.
    pub ran: &'a BTreeMap<String, f64>,
    /// What each producer's watch said when it last ran, by name.
    pub said: &'a BTreeMap<String, u64>,
    /// When each build's artifacts were last opened, as far as the watch
    /// on their `deps` has seen.
    pub uses: &'a Uses,
    /// The trees found and kept for their use without being offered, as
    /// the survey goes, so a plan or explanation can say why.
    pub kept: Vec<(PathBuf, Use)>,
    /// What the survey's sizes have counted, so blocks several candidates
    /// share, as trees cloned with their builds inside do, count once:
    /// deleting them all frees those once.
    pub counted: RefCell<Counted>,
    /// How many times the survey walked a root for targets and installs.
    pub walked: usize,
    /// Whether the daemon has been told to stop, asked as the survey walks,
    /// so a stop never waits on a walk. A stopped survey offers nothing
    /// more, and what it offered is acted on by nobody.
    pub stop: &'a dyn Fn() -> bool,
}

/// How deep and how widely a workspace's writes are read.
const WRITE_DEPTH: usize = 4;
const WRITE_BUDGET: usize = 50_000;

/// The config with only the sources that could offer something inside
/// one of `folders`: those lying inside one, or holding one. A clear
/// surveys only these, so it starts in seconds rather than after a walk
/// of everything. A source that offers from no folder of its own stays.
pub fn within(config: &Config, folders: &[PathBuf]) -> Config {
    let touches = |source: &Source| {
        let paths = source.under();
        paths.is_empty()
            || paths.iter().any(|path| {
                let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
                folders
                    .iter()
                    .any(|folder| folder.starts_with(&path) || path.starts_with(folder))
            })
    };
    let mut scoped = config.clone();
    scoped.sources.retain(|_, source| touches(source));
    scoped
}

/// Every candidate the sources offer now.
pub fn survey(config: &Config, world: &mut World) -> Vec<Offer> {
    let mut offers = Vec::new();
    // One walk finds targets and installs both, so a root the two sources
    // share is walked once a survey, not once a source.
    let mut walks = Walks::new();
    for (name, source) in &config.sources {
        if (world.stop)() {
            return offers;
        }
        match source {
            Source::Cargo { roots, depth } => {
                for root in roots {
                    if let Some((root, device, found)) = walks.under(root, *depth, world) {
                        trees_under(&root, device, &found, Kind::Target, world, &mut offers);
                    }
                }
            }
            Source::Installs { roots, depth } => {
                for root in roots {
                    if let Some((root, device, found)) = walks.under(root, *depth, world) {
                        trees_under(&root, device, &found, Kind::Install, world, &mut offers);
                    }
                }
            }
            Source::Cache {
                path,
                from,
                protocol,
                rank,
            } => offer_cache(path, *from, *protocol, *rank, world, &mut offers),
            Source::Scratch { path, healthy } => {
                offer_scratch(path, *healthy, world, &mut offers);
            }
            Source::Snapshots {
                path,
                pattern,
                command,
            } => offer_snapshots(path, pattern, command, world, &mut offers),
            Source::Git { repos } => {
                for repo in repos {
                    offer_git(repo, world, &mut offers);
                }
            }
            Source::Workspaces { path, quests, .. } => {
                offer_workspaces(path, quests, world, &mut offers);
            }
            Source::Temp { path } => offer_temp(path, world, &mut offers),
            Source::Trash { path } => offer_trash(path, world, &mut offers),
            Source::Coredumps { path } => offer_coredumps(path, world, &mut offers),
            Source::NixBuilds { path } => offer_nix_builds(path, world, &mut offers),
            Source::Logs { path } => offer_logs(path, &mut offers),
            Source::Producer {
                command,
                path,
                every,
                from,
                watch,
                timeout,
            } => {
                let Some(device) = device(path) else { continue };
                let idle = if changed(
                    watch,
                    world.ran.get(name).copied(),
                    world.said.get(name).copied(),
                ) {
                    f64::INFINITY
                } else {
                    since_ran(world, name)
                };
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
                offer_podman(name, command, path, world, &mut offers);
            }
            Source::Finished { .. } => {}
        }
    }
    offers
}

/// A cache's offer: each entry of a Go build cache, or else the whole cache.
fn offer_cache(
    path: &Path,
    from: Band,
    protocol: Option<CacheProtocol>,
    rank: u8,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let Some(device) = device(path) else { return };
    match protocol {
        Some(CacheProtocol::GoBuild) => {
            for entry in caches::go_build(path) {
                let candidate = caches::go_entry(&entry, rank, from, world.awake, world.now);
                offers.push(Offer {
                    candidate,
                    action: Action::CacheEntry {
                        root: path.to_owned(),
                    },
                    device,
                });
            }
        }
        None => {
            let Some(parent) = path.parent() else {
                return;
            };
            // Idle since anything in it was last written, or not at all
            // where that can't be told, so only a band that takes it at
            // once does.
            let recent = world.now - crate::plan::CACHE_UNWRITTEN;
            let idle = caches::last_written(path, caches::WALKED, recent).map_or(0.0, |written| {
                world
                    .awake
                    .since(written.min(world.now), world.now)
                    .max(0.0)
            });
            let candidate = caches::whole(path, rank, from, size(path, world), idle);
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

/// A scratch directory's entries.
fn offer_scratch(path: &Path, healthy: Option<Span>, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    let healthy = healthy.map(|span| span.0.as_secs_f64());
    for entry in scratch::entries(path) {
        let size = size(&entry.path, world);
        let candidate = scratch::candidate(&entry, healthy, size, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// The snapshots matching `pattern` that no process running `command` keeps.
fn offer_snapshots(
    path: &Path,
    pattern: &str,
    command: &str,
    world: &World,
    offers: &mut Vec<Offer>,
) {
    let (Some(device), Ok(pattern)) = (device(path), regex_lite::Regex::new(pattern)) else {
        return;
    };
    for snapshot in snapshots::entries(path, &pattern) {
        if snapshots::orphaned(&snapshot, world.processes, &[command], world.boot) {
            let candidate = snapshots::candidate(
                &snapshot,
                size(&snapshot.path, world),
                world.awake,
                world.now,
            );
            offers.push(Offer {
                candidate,
                action: Action::Entry {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// What git staged in the repository whose git directory is `repo`, once
/// no git works on it. Each goes whole, on the next pass, however young:
/// git removes them as it exits, so one still there with no git at work
/// is one a killed git left, and waiting on its age only keeps it.
fn offer_git(repo: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(repo) else { return };
    if git::busy(&git::places(repo), world.processes, world.uid) {
        return;
    }
    let root = repo.join("objects");
    for path in git::temporaries(repo) {
        let size = size(&path, world);
        offers.push(Offer {
            candidate: git::candidate(path, size),
            action: Action::Entry { root: root.clone() },
            device,
        });
    }
}

/// The quest workspaces in `path`, judged by their quests' READMEs in
/// `quests`, and in each one still at work the browser profiles its `tmp/`
/// holds at rest and what else it left there unwritten, but for what a
/// process works in. Use is judged only of an entry old enough to offer,
/// and before it's sized, which walks all of it; where processes work is
/// read once, as there can be tens of thousands of entries to ask of.
fn offer_workspaces(path: &Path, quests: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    let places = activity::Workplaces::new(world.processes, world.me);
    let worked_in = |path: &Path| places.work_in(path);
    let (over, working): (Vec<_>, Vec<_>) = workspaces::find(path, quests)
        .into_iter()
        .partition(workspaces::over);
    for workspace in working {
        let tmp = workspace.path.join("tmp");
        let profiles = temp::profiles(&tmp, world.uid, world.now);
        for entry in &profiles {
            offers.push(Offer {
                candidate: temp::profile_candidate(entry, world.awake, world.now),
                action: Action::Entry {
                    root: resolved(&tmp),
                },
                device,
            });
        }
        let a_profile = |path: &Path| profiles.iter().any(|profile| profile.path == path);
        for entry in scratch::entries(&tmp) {
            let young = scratch::idle(&entry, world.awake, world.now) < scratch::QUEST_UNWRITTEN;
            if young || a_profile(&entry.path) || worked_in(&entry.path) {
                continue;
            }
            let size = || size(&entry.path, world);
            if let Some(candidate) = scratch::quest_candidate(&entry, size, world.awake, world.now)
            {
                offers.push(Offer {
                    candidate,
                    action: Action::Entry {
                        root: resolved(&tmp),
                    },
                    device,
                });
            }
        }
        for entry in scratch::beside_tmp(&workspace.path) {
            let idle = scratch::idle(&entry, world.awake, world.now);
            if idle < scratch::QUEST_SCRATCH_UNWRITTEN || worked_in(&entry.path) {
                continue;
            }
            let size = || size(&entry.path, world);
            if let Some(candidate) =
                scratch::quest_scratch_candidate(&entry, size, world.awake, world.now)
            {
                offers.push(Offer {
                    candidate,
                    action: Action::Entry {
                        root: resolved(&workspace.path),
                    },
                    device,
                });
            }
        }
    }
    for workspace in over {
        let written = age::last_write(&workspace.path, WRITE_DEPTH, WRITE_BUDGET);
        let size = size(&workspace.path, world);
        if let Some(candidate) =
            workspaces::candidate(&workspace, written, size, world.awake, world.now)
        {
            offers.push(Offer {
                candidate,
                action: Action::Workspace {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// What's left in a temporary directory, but for any a process names as its
/// `TMPDIR`.
fn offer_temp(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let path = &resolved(path);
    let Some(device) = device(path) else { return };
    let tmpdirs: Vec<PathBuf> = world
        .processes
        .iter()
        .flat_map(|process| &process.env)
        .filter(|(name, _)| name == "TMPDIR")
        .map(|(_, value)| PathBuf::from(value))
        .collect();
    let boot = world.boot.wall;
    for entry in temp::leftovers(temp::survey(path), world.uid, &tmpdirs, boot) {
        let candidate = temp::candidate(&entry, world.awake, world.now, boot);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// A desktop trash's items.
fn offer_trash(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for item in trash::items(path) {
        let candidate = trash::candidate(&item, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Trash {
                trash: path.to_owned(),
            },
            device,
        });
    }
}

/// The core dumps kept in `path`.
fn offer_coredumps(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for dump in coredumps::find(path) {
        let candidate = coredumps::candidate(&dump, world.awake, world.now);
        offers.push(Offer {
            candidate,
            action: Action::Entry {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// The build directories Nix kept in `path` whose builder is gone.
fn offer_nix_builds(path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    for build in nix_builds::find(path) {
        if nix_builds::orphaned(&build, world.processes, world.boot) {
            let size = size(&build.snapshot.path, world);
            let candidate = nix_builds::candidate(&build, size, world.awake, world.now);
            offers.push(Offer {
                candidate,
                action: Action::Entry {
                    root: path.to_owned(),
                },
                device,
            });
        }
    }
}

/// The logs beneath `path` past their limit, each offering what trimming it
/// drops.
fn offer_logs(path: &Path, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
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
            action: Action::Trim {
                root: path.to_owned(),
            },
            device,
        });
    }
}

/// Podman's two prunes: its build cache, and every image no container uses.
fn offer_podman(name: &str, command: &Path, path: &Path, world: &World, offers: &mut Vec<Offer>) {
    let Some(device) = device(path) else { return };
    let program = command.to_string_lossy().into_owned();
    for (unused, source, how) in [
        (false, name.to_owned(), "--build-cache"),
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
                root: path.to_owned(),
                source,
                argv: argv.map(str::to_owned).to_vec(),
                timeout: PODMAN_TIMEOUT,
            },
            device,
        });
    }
}

/// The targets or installs beneath a root, and a target's variants. A
/// tree a build is running in, or whose use can't be judged, offers
/// nothing.
/// The roots a survey has walked for targets and installs, by root and
/// depth, with the device each lives on and what the walk found.
type Walked = BTreeMap<(PathBuf, Option<u32>), Option<(u64, Vec<discover::Found>)>>;

struct Walks(Walked);

impl Walks {
    fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// What a walk of `root` found, walking it only the first time asked.
    /// None when the root is on no device or won't read.
    fn under(
        &mut self,
        root: &Path,
        depth: Option<u32>,
        world: &mut World,
    ) -> Option<(PathBuf, u64, Vec<discover::Found>)> {
        let root = resolved(root);
        let walked = self.0.entry((root.clone(), depth)).or_insert_with(|| {
            let device = device(&root)?;
            world.walked += 1;
            let found = discover::walk_until(
                &root,
                depth.map(|depth| depth as usize),
                world.home,
                world.stop,
            )
            .ok()?;
            Some((device, found))
        });
        walked
            .as_ref()
            .map(|(device, found)| (root, *device, found.clone()))
    }
}

/// What reclaiming a target would free. It's emptied in place, keeping its
/// benchmarks and the targets kept inside it, so what those hold doesn't
/// count. The ones inside are offered first, so what each holds is counted
/// already, but for one being worked in, which was never sized.
fn target_frees(target: &Path, world: &World) -> u64 {
    let working: u64 = world
        .kept
        .iter()
        .filter(|(path, _)| path.parent() == Some(target))
        .map(|(path, _)| whole(path))
        .sum();
    crate::reclaim::KEEP
        .iter()
        .fold(size(target, world), |left, kept| {
            left.saturating_sub(whole(&target.join(kept)))
        })
        .saturating_sub(working)
}

fn trees_under(
    root: &Path,
    device: u64,
    found: &[discover::Found],
    kind: Kind,
    world: &mut World,
    offers: &mut Vec<Offer>,
) {
    for tree in found.iter().filter(|tree| tree.kind == kind) {
        let checkout = match &tree.checkout {
            Checkout::At(dir) => Some(dir.as_path()),
            _ => None,
        };
        let within = discover::enclosing(&tree.path);
        let used = activity::judge_within(
            world.processes,
            world.me,
            &tree.path,
            checkout,
            within.as_deref(),
        );
        // The candidate refuses these too; deciding first spares sizing a
        // tree a build is writing to.
        if matches!(used, Use::Busy(_) | Use::Unknown(_)) {
            world.kept.push((tree.path.clone(), used));
            continue;
        }
        let frees = match kind {
            Kind::Target => target_frees(&tree.path, world),
            Kind::Install => size(&tree.path, world),
        };
        if kind == Kind::Target && frees == 0 {
            continue;
        }
        let finished = checkout.is_some_and(|dir| (world.finished)(dir));
        let seen = trees::Seen {
            used,
            finished,
            size: frees,
            last_used: trees::last_used(tree),
        };
        let Some(candidate) = trees::candidate(tree, &seen, world.awake, world.now) else {
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
                    size: size(&variant.path, world),
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
            // Each older build of a crate, dated by its last open; where
            // nothing watches, nothing can say it's unused.
            for build in crate::replaced::replaced(&tree.path) {
                let Some(last_used) = world.uses.last_used(&build) else {
                    continue;
                };
                let candidate = Candidate {
                    path: build.deps.join(format!("{}-{}", build.name, build.digest)),
                    class: Class::Replaced,
                    idle: world
                        .awake
                        .since(last_used.min(world.now), world.now)
                        .max(0.0),
                    size: build.size,
                    occupied: false,
                    spent: false,
                    checkout: checkout.map(Path::to_owned),
                };
                offers.push(Offer {
                    candidate,
                    action: Action::Build {
                        root: root.to_owned(),
                        target: tree.path.clone(),
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

/// Whether any of the files a producer watches changed after `ran`, the
/// wall second it last ran; never, when it hasn't run, since it's due
/// anyway. A file that can't be read hasn't changed.
/// Whether what a producer watches says something new since it ran:
/// against what it said then, where that was noted, or else whether any
/// of it was written since. Writing alone isn't news: a janitor that
/// rewrites its marker and its links each time it looks, finding nothing
/// new, set off a walk of every source every ten minutes on 29 October.
pub fn changed(watch: &[PathBuf], ran: Option<f64>, said_then: Option<u64>) -> bool {
    if watch.is_empty() {
        return false;
    }
    match said_then {
        Some(then) => said(watch) != then,
        None => written_since(watch, ran),
    }
}

/// How many levels below a watched folder are read.
const WATCH_DEPTH: usize = 3;

/// How much of a watched file is read.
const WATCH_BYTES: u64 = 64 << 10;

/// What `watch` says now, as one number: each link's target, the start
/// of each file and its length, and each folder's names and links
/// `WATCH_DEPTH` levels down, so the same written again says the same.
pub fn said(watch: &[PathBuf]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::hash::DefaultHasher::new();
    for path in watch {
        path.hash(&mut hasher);
        say(path, WATCH_DEPTH, &mut hasher);
    }
    hasher.finish()
}

fn say(path: &Path, depth: usize, hasher: &mut impl std::hash::Hasher) {
    use std::hash::Hash;
    use std::io::Read;
    let Ok(meta) = fs::symlink_metadata(path) else {
        "gone".hash(hasher);
        return;
    };
    if meta.is_symlink() {
        fs::read_link(path).ok().hash(hasher);
    } else if meta.is_dir() {
        let mut names: Vec<_> = fs::read_dir(path)
            .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
            .unwrap_or_default();
        names.sort();
        for name in names {
            name.hash(hasher);
            if depth > 0 {
                say(&path.join(&name), depth - 1, hasher);
            }
        }
    } else {
        meta.len().hash(hasher);
        let mut start = Vec::new();
        let _ =
            fs::File::open(path).and_then(|file| file.take(WATCH_BYTES).read_to_end(&mut start));
        start.hash(hasher);
    }
}

/// Whether any of `watch` was written since `ran`.
fn written_since(watch: &[PathBuf], ran: Option<f64>) -> bool {
    let Some(ran) = ran else { return false };
    watch.iter().any(|path| {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
            .is_some_and(|at| at.as_secs_f64() > ran)
    })
}

/// The awake time since a source's command last ran, or forever.
fn since_ran(world: &World, source: &str) -> f64 {
    world.ran.get(source).map_or(f64::INFINITY, |&at| {
        world.awake.since(at.min(world.now), world.now)
    })
}

/// The filesystem a source's directory is on, as its space is shared.
fn device(path: &Path) -> Option<u64> {
    fs::metadata(path).ok().filter(fs::Metadata::is_dir)?;
    crate::pool::pool(path).ok()
}

/// A root as the entries found beneath it are named: where it lives, so a
/// root named through a link, as a Mac names /var/folders through
/// /var -> /private/var, still holds what it offers. A root that can't be
/// resolved is kept as named and offers nothing anyway.
fn resolved(root: &Path) -> PathBuf {
    fs::canonicalize(root).unwrap_or_else(|_| root.to_owned())
}

/// What an entry holds, less what the survey has counted already.
fn size(path: &Path, world: &World) -> u64 {
    measured(path, &mut world.counted.borrow_mut(), world.stop)
}

/// What an entry holds: a directory's allocated size beneath it, a file's
/// own, nothing when it can't be read, less what's in `counted`.
fn measured(path: &Path, counted: &mut Counted, stop: &dyn Fn() -> bool) -> u64 {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => Dir::open(path)
            .and_then(|dir| dir.size_until(counted, stop))
            .unwrap_or(0),
        Ok(meta) => meta.blocks() * 512,
        Err(_) => 0,
    }
}

/// What an entry takes beneath the directory holding it: itself too, as a
/// directory's own blocks count among its parent's, where a filesystem
/// gives a directory any. It's measured alone, as what a reclaim keeps
/// holds all it holds, whatever else shares it.
fn whole(path: &Path) -> u64 {
    let mut counted = Counted::default();
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.is_dir() => meta.blocks() * 512 + measured(path, &mut counted, &|| false),
        _ => measured(path, &mut counted, &|| false),
    }
}

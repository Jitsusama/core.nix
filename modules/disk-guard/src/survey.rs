//! What the sources offer: every candidate, with how it's reclaimed and
//! the filesystem it lies on, so a pass runs on one filesystem with only
//! what lies there.
//!
//! The trees a cargo or installs source walks to, and the budget a cargo
//! source's builds are held to, are in `builds`; what every other kind of
//! source offers is in `sources`. What they all share, the offer itself,
//! what a survey judges by, and how an entry is sized, is here.

mod builds;
mod sources;

pub use builds::{
    BUDGET_LOOKS_EVERY, BUDGET_RECENT, BUDGET_UNHEARD, REPLACED_UNUSED, budget_bytes, past_budget,
};
use builds::{Walks, hold_to_budget, trees_under};
use sources::{
    Worked, offer_cache, offer_coredumps, offer_git, offer_logs, offer_nix_builds, offer_podman,
    offer_producer, offer_scratch, offer_snapshots, offer_store_roots, offer_temp, offer_trash,
    offer_workspaces,
};

use crate::activity::{Process, Use};
use crate::anchor::{Counted, Dir};
use crate::clock::Awake;
use crate::config::{Config, Source};
use crate::discover::Kind;
use crate::plan::{Candidate, Class};
use crate::replaced::Uses;
use crate::snapshots::Boot;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
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
    /// An entry of a live quest's `tmp/`, read whole again just before it
    /// goes, as an `Entry` then: kept if anything in it was written after
    /// `written`, the newest write its survey found, or it holds work now.
    LiveEntry { root: PathBuf, written: f64 },
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
        /// Where Nix keeps its database, whose write lock another process
        /// holding puts a delete off.
        database: PathBuf,
        paths: Vec<String>,
        /// The files running programs hold known only by Nix's hard link
        /// to them, so a path holding one is left out of the delete.
        links: std::collections::BTreeSet<crate::held::File>,
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
            | Action::LiveEntry { root, .. }
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

    /// The newest write its survey found, for an entry read again before
    /// it goes; none for any other.
    pub fn surveyed_write(&self) -> Option<f64> {
        match self {
            Action::LiveEntry { written, .. } => Some(*written),
            _ => None,
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
    /// Whether a checkout is a snapshot a broker pinned at a commit.
    pub pinned: &'a dyn Fn(&Path) -> bool,
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
    /// What earlier surveys sized of quest workspaces, kept between them.
    pub sizes: RefCell<Sizes>,
    /// How many times the survey walked a root for targets and installs.
    pub walked: usize,
    /// Whether the daemon has been told to stop, asked as the survey walks,
    /// so a stop never waits on a walk. A stopped survey offers nothing
    /// more, and what it offered is acted on by nobody.
    pub stop: &'a dyn Fn() -> bool,
}

/// The sizes of quest workspaces and the entries they hold, each by the
/// last write it was sized at, kept by a daemon from one survey to the
/// next. On 16 November 215 workspaces held 70,882 entries, 68,202 of them
/// unwritten for six hours, and every pass of a filling disk walked each
/// old one whole to size it again, reading 178 GB in four hours.
///
/// A size only says what taking an entry would free: whether it may be
/// taken is dated afresh each survey, so a size kept from an earlier one
/// never makes anything look idle. A write deeper than the dating reads
/// leaves a size behind until a shallower one, and a size known this way
/// adds nothing to the survey's shared blocks, so a block two entries
/// share may count in both; either only misstates what a pass frees.
#[derive(Debug, Default)]
pub struct Sizes {
    /// Each entry's size, by the bits of the write it was sized at.
    known: BTreeMap<PathBuf, (u64, u64)>,
    /// The entries asked for since the last survey began.
    asked: BTreeSet<PathBuf>,
    /// How many were walked to be sized, rather than known.
    pub walked: usize,
}

impl Sizes {
    /// What `path`, last written at `written`, holds: known when an earlier
    /// survey sized it at that write, `measure`d otherwise. A write too
    /// deep to date is never known, as it may be under way.
    fn of(&mut self, path: &Path, written: f64, measure: impl FnOnce() -> u64) -> u64 {
        self.asked.insert(path.to_owned());
        let at = written.to_bits();
        if let Some(&(was, size)) = self.known.get(path)
            && was == at
        {
            return size;
        }
        self.walked += 1;
        let size = measure();
        if written.is_finite() {
            self.known.insert(path.to_owned(), (at, size));
        } else {
            self.known.remove(path);
        }
        size
    }

    /// Forgets every entry the survey just ended never asked for, gone or
    /// no longer old, so what's kept is only what the next may ask.
    pub fn ended(&mut self) {
        let asked = std::mem::take(&mut self.asked);
        self.known.retain(|path, _| asked.contains(path));
    }
}

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
            Source::Cargo {
                roots,
                depth,
                budget,
            } => {
                let start = offers.len();
                let bytes = budget_bytes(*budget, roots);
                for root in roots {
                    if let Some((root, device, found)) = walks.under(root, *depth, world) {
                        let (at, budgeted) = ((root.as_path(), device), bytes.is_some());
                        trees_under(at, &found, Kind::Target, budgeted, world, &mut offers);
                    }
                }
                if let Some(bytes) = bytes {
                    hold_to_budget(
                        &mut offers[start..],
                        bytes,
                        world.uses,
                        world.awake,
                        world.now,
                    );
                }
            }
            Source::Installs { roots, depth } => {
                for root in roots {
                    if let Some((root, device, found)) = walks.under(root, *depth, world) {
                        let at = (root.as_path(), device);
                        trees_under(at, &found, Kind::Install, false, world, &mut offers);
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
            Source::Workspaces {
                path,
                quests,
                sessions,
                rosters,
                ..
            } => {
                let worked = Worked::read(sessions.as_deref(), rosters.as_deref(), world);
                offer_workspaces(path, quests, &worked, world, &mut offers);
            }
            Source::Temp { path } => offer_temp(path, world, &mut offers),
            Source::Trash { path } => offer_trash(path, world, &mut offers),
            Source::Coredumps { path } => offer_coredumps(path, world, &mut offers),
            Source::NixBuilds { path } => offer_nix_builds(path, world, &mut offers),
            Source::StoreRoots {
                roots,
                pattern,
                unused,
            } => offer_store_roots(roots, pattern, *unused, &config.nix, world, &mut offers),
            Source::Logs { path } => offer_logs(path, &mut offers),
            Source::Producer { .. } => offer_producer(name, source, world, &mut offers),
            Source::Podman { command, path } => {
                offer_podman(name, command, path, world, &mut offers);
            }
            Source::Finished { .. } => {}
        }
    }
    offers
}

/// A command's candidate, named for where it runs and what it is, since
/// it is no file of its own.
pub(crate) fn named(path: &Path, source: &str) -> PathBuf {
    let mut named = path.as_os_str().to_owned();
    named.push("#");
    named.push(source);
    PathBuf::from(named)
}

/// Whether a candidate is gone already, as the pass on the disk beneath an
/// image may be offered what the image's own pass took: one of `tried`,
/// what this round's passes tried, or else no longer there. A command is
/// no file, so only `tried` says. A build's offer names a file only an
/// executable has, so its own files say.
pub fn gone(candidate: &Candidate, tried: &BTreeSet<PathBuf>) -> bool {
    if tried.contains(&candidate.path) {
        true
    } else if candidate.class.command() {
        false
    } else if candidate.class == Class::Replaced {
        !crate::replaced::stands(&candidate.path)
    } else {
        fs::symlink_metadata(&candidate.path).is_err()
    }
}

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
            .map_or_default(|entries| entries.flatten().map(|entry| entry.file_name()).collect());
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

/// What an entry last written at `written` holds, as `size` says, unless
/// an earlier survey sized it at that write.
fn sized(path: &Path, written: f64, world: &World) -> u64 {
    world
        .sizes
        .borrow_mut()
        .of(path, written, || size(path, world))
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

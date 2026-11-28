//! The trees a walk finds beneath a cargo or installs source: targets,
//! installs, a target's variants and the older builds a newer one
//! replaced, and a cargo source's builds held to its budget.

use super::{Action, Offer, World, device, resolved, size, whole};
use crate::activity::{self, Use};
use crate::clock::Awake;
use crate::config::{Config, Source};
use crate::discover::{self, Checkout, Kind};
use crate::plan::{Candidate, Class};
use crate::replaced::Uses;
use crate::trees;
use crate::units::Size;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The roots a survey has walked for targets and installs, by root and
/// depth, with the device each lives on and what the walk found.
type Walked = BTreeMap<(PathBuf, Option<u32>), Option<(u64, Vec<discover::Found>)>>;

pub(super) struct Walks(Walked);

impl Walks {
    pub(super) fn new() -> Self {
        Self(BTreeMap::new())
    }

    /// What a walk of `root` found, walking it only the first time asked.
    /// None when the root is on no device or won't read.
    pub(super) fn under(
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

/// Offers what `kind` of tree the walk `found` under a root holds, on
/// `device`, and a target's variants and older builds with it. A tree a
/// build is running in, or whose use can't be judged, offers nothing.
/// Only a `budgeted` source's replaced builds go an hour unused.
pub(super) fn trees_under(
    (root, device): (&Path, u64),
    found: &[discover::Found],
    kind: Kind,
    budgeted: bool,
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
            pinned: checkout.is_some_and(|dir| (world.pinned)(dir)),
            size: frees,
            last_used: trees::last_used(tree),
        };
        let Some(candidate) = trees::candidate(tree, &seen, world.awake, world.now) else {
            continue;
        };
        // In a snapshot nobody edits, a crate's incremental sessions serve
        // only a build running now, and rustc's locks keep the one it
        // uses; so they go at once while nobody sits in the tree.
        let snapshot_idle = seen.pinned && !candidate.occupied;
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
                    spent: snapshot_idle,
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
            let held = Held {
                pinned: seen.pinned,
                budgeted,
                checkout,
            };
            replaced_in(&tree.path, &held, (root, device), world, offers);
        }
    }
}

/// What holds a target's builds: whether it's a pinned snapshot's, whether
/// its source holds builds to a budget, and the checkout it builds.
struct Held<'a> {
    pinned: bool,
    budgeted: bool,
    checkout: Option<&'a Path>,
}

/// Offers each older build in `target`, dated by its last open; where
/// nothing watches, nothing can say it's unused.
fn replaced_in(
    target: &Path,
    held: &Held,
    (root, device): (&Path, u64),
    world: &World,
    offers: &mut Vec<Offer>,
) {
    for build in crate::replaced::replaced(target) {
        let Some(last_used) = world.uses.last_used(&build) else {
            continue;
        };
        let idle = world
            .awake
            .since(last_used.min(world.now), world.now)
            .max(0.0);
        // Only an executable a newer build of its own unit and variant
        // replaced is taken as done with: a library another build differs
        // from only by how a dependency was built may be one a live build
        // uses, and every crate above it with it, while a live executable
        // so taken costs only the link its next run makes again.
        let heard_unused = held.budgeted
            && build.in_variant
            && idle >= REPLACED_UNUSED
            && world
                .uses
                .heard_since(std::slice::from_ref(&build.deps))
                .is_some_and(|since| {
                    world.awake.since(since.min(world.now), world.now) >= REPLACED_UNUSED
                });
        let candidate = Candidate {
            path: build.deps.join(format!("{}-{}", build.name, build.digest)),
            class: Class::Replaced,
            idle,
            size: build.size,
            occupied: false,
            // A gate snapshot builds one commit after another, and on 17
            // November one kept 89 builds of a single test crate, 55 GiB in
            // all: an older build an hour unused there is an earlier
            // gate's. One running in the target keeps them all till it
            // ends, since a target a process works in offers nothing.
            spent: (held.pinned && idle >= trees::SNAPSHOT_UNUSED) || heard_unused,
            checkout: held.checkout.map(Path::to_owned),
        };
        offers.push(Offer {
            candidate,
            action: Action::Build {
                root: root.to_owned(),
                target: target.to_owned(),
            },
            device,
        });
    }
}

/// How long a replaced build in a budgeted builds folder stays after its
/// last use, where this guard heard its `deps` all that time. On 18
/// November a gate's build held 28 GiB of test binaries a newer build of
/// the same test replaced, nearly all one to six hours old, which the
/// day a replaced build otherwise waits never reached. Where the guard
/// didn't hear it the whole hour, an open it missed could be a build's
/// still in use, so it waits its day.
pub const REPLACED_UNUSED: f64 = 3600.0;

/// How long a build stays after its last use, whatever the budget, since
/// a run of checks reuses one build minutes apart and a cap that took it
/// the moment its cargo ended would have every check rebuild it. A
/// quarter of an hour rather than an hour, since on 18 November a
/// builds disk held 56 GiB against its 20, 55 of it in builds used
/// within the hour; cargo's own locks and a shell sitting in one keep a
/// build someone is at, however long ago it was last written.
pub const BUDGET_RECENT: f64 = 900.0;

/// How long a build stays after its last use where the watch may have
/// missed a cargo opening it in the last [`BUDGET_RECENT`], as the floor
/// was before opens were heard.
pub const BUDGET_UNHEARD: f64 = 3600.0;

/// How often a builds disk still past its budget makes upkeep due, since
/// one whose builds are all recent stays past it after the pass, and a
/// survey reads every build.
pub const BUDGET_LOOKS_EVERY: f64 = 3600.0;

/// The bytes a cargo source's budget allows, in its first root's disk.
/// A share of a disk that can't be measured, and a budget that comes to
/// nothing, hold nothing to a budget, rather than everything to none.
pub fn budget_bytes(budget: Option<Size>, roots: &[PathBuf]) -> Option<u64> {
    let bytes = match (budget?, roots.first()) {
        (Size::Bytes(bytes), _) => bytes,
        (share, Some(root)) => share.of(crate::sys::space(root).ok()?.total),
        (_, None) => return None,
    };
    (bytes > 0).then_some(bytes)
}

/// Spends the builds past `budget`, counting the most recently used
/// first, so the least recently used go in any band. A build a cargo is
/// running in was never offered, so it is never taken; one a shell sits
/// in, or used within its floor, counts against the budget but stays.
/// One already spent goes anyway, so it takes no room. On 17 November a
/// shared builds folder held 63 GiB in 3,298 builds, 59 GiB of it
/// written within a day, which no idle window reaches.
pub(super) fn hold_to_budget(
    offers: &mut [Offer],
    budget: u64,
    uses: &Uses,
    awake: &Awake,
    now: f64,
) {
    let mut builds: Vec<(Used, &mut Candidate)> = offers
        .iter_mut()
        .map(|offer| &mut offer.candidate)
        .filter(|candidate| candidate.class == Class::Target && !candidate.spent)
        .map(|candidate| (Used::of(candidate, uses, awake, now), candidate))
        .collect();
    builds.sort_by(|(a, _), (b, _)| a.idle.total_cmp(&b.idle));
    let mut held: u64 = 0;
    for (used, build) in builds {
        held = held.saturating_add(build.size);
        if held > budget && !build.occupied && used.idle >= used.floor {
            build.spent = true;
        }
    }
}

/// How long ago a build was last used, as the budget reads it, and how
/// long it stays after that.
struct Used {
    idle: f64,
    floor: f64,
}

impl Used {
    /// A cargo that finds a build fresh writes nothing in it but opens its
    /// profile's lock, so a build is used when last written or when the
    /// watch last heard anything right in its profile or `deps` opened.
    /// Only the budget reads opens: a reader such as a backup would
    /// otherwise hold back every other window. The quarter-hour floor
    /// stands only where this guard's marks have heard those folders, as
    /// they are now, for a whole one, since an open it missed would make
    /// a build in use look old: one not yet marked since the guard
    /// started, one made anew since its mark, one past lost events, and
    /// anywhere nothing hears, as on macOS or in `plan`, keeps its hour.
    /// Both are counted in awake time, as every other idle is, so a
    /// suspend ages neither an open nor a mark.
    fn of(build: &Candidate, uses: &Uses, awake: &Awake, now: f64) -> Self {
        let dirs = crate::replaced::watched_dirs(&build.path);
        let idle = uses.last_opened_in(&dirs).map_or(build.idle, |opened| {
            build.idle.min(awake.since(opened, now))
        });
        let floor = match uses.heard_since(&dirs) {
            Some(since) if awake.since(since, now) >= BUDGET_RECENT => BUDGET_RECENT,
            _ => BUDGET_UNHEARD,
        };
        Self { idle, floor }
    }
}

/// Whether a cargo source's disk holds more than its budget, read from
/// the disk's own count, so a builds disk filling on a healthy root is
/// seen within a minute, without a survey.
pub fn past_budget(config: &Config) -> bool {
    config.sources.values().any(|source| match source {
        Source::Cargo { roots, budget, .. } => budget_bytes(*budget, roots).is_some_and(|bytes| {
            roots.first().is_some_and(|root| {
                crate::sys::space(root)
                    .is_ok_and(|space| space.total.saturating_sub(space.free) > bytes)
            })
        }),
        _ => false,
    })
}

//! Which candidates a pass takes in a band, and in what order.

use crate::band::Band;
use crate::units::{float, whole};
use serde::{Deserialize, Deserializer, Serialize};
use std::path::PathBuf;

/// The groups a pass takes from, cheapest loss first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Group {
    Waste,
    Garbage,
    Variants,
    Producers,
    Caches,
    Idle,
    Replaced,
    Occupied,
}

/// What a candidate is.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Class {
    Target,
    Install,
    /// An incremental session a newer one of its crate replaced.
    Variant,
    /// Scratch whose source keeps it while healthy for this long.
    Scratch {
        healthy: Option<f64>,
    },
    /// An entry of a quest's `tmp/` while the quest is at work, which its
    /// session may still mean to read.
    QuestTemp,
    /// What a quest at work left in its workspace beside `tmp/`: a lab, a
    /// clone, a run, likelier than `tmp/`'s to be read again.
    QuestScratch,
    /// A link in a quest at work's `tmp/` into the store, which keeps a
    /// build from collection and holds nothing a build can't make again.
    QuestRoot,
    /// A link a tool keeps to hold a build of its own, which it makes
    /// again when next asked, once its place has gone `unused` awake
    /// seconds with nothing written.
    StoreRoot {
        unused: f64,
    },
    /// A snapshot whose process is gone.
    Snapshot,
    /// What git staged in a repository no git works on now.
    GitTemporary,
    /// A quest workspace whose quest is done with it.
    Workspace,
    Temp,
    Trash,
    /// A cache, ranked by how cheaply it refills, taken from a band on.
    Cache {
        rank: u8,
        from: Band,
    },
    Coredump,
    /// A Nix build directory no build holds.
    NixBuild,
    /// An older build of a crate a newer one replaced.
    Replaced,
    /// A batch of Nix's garbage, in the place the garbage policy gave it,
    /// and whether a healthy disk keeps it, being young and in budget.
    Garbage {
        kept: bool,
        order: u16,
    },
    /// A profile's generations that may expire, and whether a healthy
    /// disk keeps them, being young. Expiring frees nothing itself: it
    /// makes garbage of what only they kept.
    Generations {
        kept: bool,
    },
    /// A producer's own collection, run every so long, or from a band on.
    Producer {
        every: Option<f64>,
        from: Option<Band>,
    },
    /// A log past its limit, trimmed to its last part.
    Log,
    /// podman's prune of dangling images and build cache, or of every
    /// image no container uses.
    Podman {
        unused: bool,
    },
}

impl Class {
    /// Whether it's a command, whose yield is known only by measuring.
    pub fn command(self) -> bool {
        matches!(
            self,
            Class::Producer { .. }
                | Class::Podman { .. }
                | Class::Garbage { .. }
                | Class::Generations { .. }
        )
    }

    /// The kind's name, which what it frees is learned under.
    pub fn kind(self) -> &'static str {
        match self {
            Class::Target => "target",
            Class::Install => "install",
            Class::Variant => "variant",
            Class::Scratch { .. } => "scratch",
            Class::QuestTemp => "quest-temp",
            Class::QuestScratch => "quest-scratch",
            Class::QuestRoot => "quest-root",
            Class::StoreRoot { .. } => "store-root",
            Class::Snapshot => "snapshot",
            Class::GitTemporary => "git-temporary",
            Class::Workspace => "workspace",
            Class::Temp => "temp",
            Class::Trash => "trash",
            Class::Cache { .. } => "cache",
            Class::Coredump => "coredump",
            Class::NixBuild => "nix-build",
            Class::Replaced => "replaced",
            Class::Garbage { .. } => "garbage",
            Class::Generations { .. } => "generations",
            Class::Producer { .. } => "producer",
            Class::Log => "log",
            Class::Podman { .. } => "podman",
        }
    }
}

/// Something a pass may reclaim. `idle` is the awake time since it was last
/// used, or since it was kept for what it was kept for; `size` is logical.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Candidate {
    pub path: PathBuf,
    pub class: Class,
    #[serde(deserialize_with = "forever_when_null")]
    pub idle: f64,
    pub size: u64,
    /// A shell or agent sits in its tree.
    pub occupied: bool,
    /// Its tree is finished or gone.
    pub spent: bool,
    /// The checkout it was built for, when known.
    pub checkout: Option<PathBuf>,
}

/// An idle time read back: JSON writes one that's forever, as a producer
/// that never ran is, as null, and a survey holding one must still read.
fn forever_when_null<'de, D: Deserializer<'de>>(read: D) -> Result<f64, D::Error> {
    Ok(Option::<f64>::deserialize(read)?.unwrap_or(f64::INFINITY))
}

/// One candidate a plan takes, by its index among those offered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Step {
    pub index: usize,
    pub group: Group,
    pub predicted: u64,
}

const HOUR: f64 = 3600.0;
const DAY: f64 = 86400.0;

/// How long nothing may have written to a cache before a filling disk takes
/// it, ahead of its own band.
pub const CACHE_UNWRITTEN: f64 = 7.0 * DAY;

/// A window per band, healthy first: how long a candidate must have been
/// idle, in awake seconds, before that band takes it. None never does.
pub type Windows = [Option<f64>; 4];

const NEVER: Option<f64> = None;
const AT_ONCE: Option<f64> = Some(0.0);

#[expect(
    clippy::unnecessary_wraps,
    reason = "a window in the table is an Option, where None is never"
)]
fn hours(n: f64) -> Option<f64> {
    Some(n * HOUR)
}

#[expect(clippy::unnecessary_wraps, reason = "a window, as hours and days are")]
fn minutes(n: f64) -> Option<f64> {
    Some(n * 60.0)
}

#[expect(
    clippy::unnecessary_wraps,
    reason = "a window in the table is an Option, where None is never"
)]
fn days(n: f64) -> Option<f64> {
    Some(n * DAY)
}

/// Whether a Clear Joel asks for takes it: what a critically full disk
/// takes now, but nothing a build or a shell is using and none of the
/// store's garbage, which its own policy collects.
pub fn clears(candidate: &Candidate) -> bool {
    group(candidate, Band::CriticallyFull)
        .is_some_and(|group| !matches!(group, Group::Occupied | Group::Garbage))
}

/// The group a band takes a candidate in, if it takes it at all.
pub fn group(candidate: &Candidate, band: Band) -> Option<Group> {
    let (group, windows) = windows(candidate);
    let window = windows[band as usize]?;
    (candidate.idle >= window).then_some(group)
}

/// The group a candidate goes in, and how long it must have been idle
/// before each band takes it. This is the plan's table; a tree someone
/// sits in goes only as occupied, however finished it is.
pub fn windows(candidate: &Candidate) -> (Group, Windows) {
    let tree = matches!(candidate.class, Class::Target | Class::Install);
    let (group, windows): (Group, Windows) = match candidate.class {
        Class::Target | Class::Install if candidate.occupied => match candidate.class {
            Class::Target => (Group::Occupied, [NEVER, NEVER, days(1.0), AT_ONCE]),
            _ => (Group::Occupied, [NEVER, NEVER, NEVER, AT_ONCE]),
        },
        _ if tree && candidate.spent => (Group::Waste, [AT_ONCE; 4]),
        // A leftover from before this boot has no maker left to come back.
        Class::Temp if candidate.spent => (Group::Waste, [AT_ONCE; 4]),
        // Even a healthy disk sheds a build or an install nobody has used
        // for a week or two of awake time: one wanted again costs a build,
        // and keeping every one ever made is how a disk fills.
        Class::Target => (Group::Idle, [days(7.0), hours(6.0), hours(2.0), AT_ONCE]),
        Class::Install => (Group::Idle, [days(14.0), days(3.0), hours(2.0), AT_ONCE]),
        Class::Variant => (
            Group::Variants,
            [days(7.0), days(1.0), hours(2.0), hours(2.0)],
        ),
        Class::Scratch { healthy } => (
            Group::Variants,
            [healthy, days(1.0), hours(6.0), hours(2.0)],
        ),
        // A quest at work may come back for what it left, so a week with
        // nothing written is the sign it won't, on a disk merely filling
        // too: on one laptop that sat filling for days, its quests' tmp/
        // folders held 124 GiB, a quarter unwritten for a week.
        Class::QuestTemp => (Group::Idle, [days(7.0), days(7.0), days(1.0), hours(6.0)]),
        // Beside tmp/, a lab or a run is likelier to be read again, so a
        // disk nearly full waits two days for one rather than one. On one
        // laptop on 11 November, 53 GiB lay there, 11 GiB of it unwritten
        // for a week.
        Class::QuestScratch => (Group::Idle, [days(7.0), days(7.0), days(2.0), hours(12.0)]),
        // A build a quest linked into its tmp/ is made again in minutes, or
        // fetched, so a day unwritten is enough whatever the band. On one
        // laptop on 12 November, 51 such links held 4.3 GiB of the store,
        // some unwritten for eighteen days.
        Class::QuestRoot => (Group::Idle, [days(1.0), days(1.0), hours(6.0), hours(6.0)]),
        // Its source says how long its tool's builds are worth keeping;
        // a disk nearly full waits no more than six hours, as for a
        // quest's links. On one laptop on 12 November, development
        // shells' links in trees a day unused held 9.7 GiB of the store.
        Class::StoreRoot { unused } => {
            let pressed = Some(unused.min(6.0 * HOUR));
            (Group::Idle, [Some(unused), Some(unused), pressed, pressed])
        }
        // Of a log, what's trimmed is history nobody reads; the rest stays.
        Class::Snapshot | Class::GitTemporary | Class::NixBuild | Class::Log => {
            (Group::Waste, [AT_ONCE; 4])
        }
        // A quest that's over owns nothing: its scratch goes once nothing
        // has written to it for a few minutes, whatever the disk's band.
        Class::Workspace => (Group::Waste, [minutes(10.0); 4]),
        Class::Trash => (Group::Waste, [days(30.0), days(7.0), days(1.0), AT_ONCE]),
        Class::Temp => (Group::Waste, [days(1.0), days(1.0), hours(6.0), hours(2.0)]),
        // A cache goes from its own band on; before it, on a filling disk,
        // once nothing has written to it for a week, since a cache in use
        // is written to all the time and one that isn't refills when asked.
        Class::Cache { from, .. } => {
            let from = |band: Band| {
                if band >= from {
                    AT_ONCE
                } else if band >= Band::Filling {
                    Some(CACHE_UNWRITTEN)
                } else {
                    NEVER
                }
            };
            (
                Group::Caches,
                [
                    from(Band::Healthy),
                    from(Band::Filling),
                    from(Band::NearlyFull),
                    from(Band::CriticallyFull),
                ],
            )
        }
        Class::Coredump if candidate.idle < DAY => {
            (Group::Occupied, [NEVER, NEVER, NEVER, AT_ONCE])
        }
        // A crash is in the journal once its dump is a day old, and on 9
        // November two of Nix's from two days before held 3.7 GB of a disk
        // filling for days, so a filling disk keeps them only that day.
        Class::Coredump => (Group::Waste, [days(7.0), days(1.0), days(1.0), AT_ONCE]),
        Class::Replaced => (
            Group::Replaced,
            [
                Some(3.0 * DAY),
                Some(DAY + HOUR),
                Some(DAY + HOUR),
                Some(DAY + HOUR),
            ],
        ),
        // Their own policies decide how much each takes.
        // What's young and within budget waits for a filling disk.
        Class::Garbage { kept: true, .. } | Class::Generations { kept: true } => {
            (Group::Garbage, [NEVER, AT_ONCE, AT_ONCE, AT_ONCE])
        }
        Class::Garbage { kept: false, .. } | Class::Generations { kept: false } => {
            (Group::Garbage, [AT_ONCE; 4])
        }
        Class::Producer { every, from } => {
            // From its band on as often as an hour allows, so a band that
            // keeps returning doesn't run a long collection every pass.
            let window = |band: Band| match from {
                Some(from) if band >= from => Some(every.map_or(HOUR, |every| every.min(HOUR))),
                _ => every,
            };
            (
                Group::Producers,
                [
                    window(Band::Healthy),
                    window(Band::Filling),
                    window(Band::NearlyFull),
                    window(Band::CriticallyFull),
                ],
            )
        }
        // At most hourly, like a producer's, once the disk fills. Images no
        // container uses go once a day on a filling disk: on 9 November they
        // were 16.6 of podman's 16.9 GB and grew 3 GB a day, on a disk that
        // had been filling for days and so never pruned them.
        Class::Podman { unused: false } => {
            (Group::Caches, [NEVER, hours(1.0), hours(1.0), hours(1.0)])
        }
        Class::Podman { unused: true } => {
            (Group::Occupied, [NEVER, days(1.0), hours(1.0), hours(1.0)])
        }
    };
    (group, windows)
}

/// What a band takes, cheapest loss first: by group, caches by how cheaply
/// they refill, then the longest idle. Waste costs nothing to lose, so it
/// goes largest first, and a few big builds aren't left behind thousands of
/// small temp entries that are idle longer. With a shortfall the plan stops once
/// what it predicts freeing covers it, each candidate predicted at its size
/// times the share of its size its class has been measured to free; without
/// one, as on a healthy disk's upkeep, it takes everything due.
pub fn plan(
    candidates: &[Candidate],
    band: Band,
    shortfall: Option<u64>,
    ratio: impl Fn(Class) -> f64,
) -> Vec<Step> {
    let mut steps: Vec<Step> = candidates
        .iter()
        .enumerate()
        .filter_map(|(index, candidate)| {
            let group = group(candidate, band)?;
            let predicted = whole(float(candidate.size) * ratio(candidate.class).max(0.0));
            Some(Step {
                index,
                group,
                predicted,
            })
        })
        .collect();
    let rank = |step: &Step| match candidates[step.index].class {
        Class::Cache { rank, .. } => u16::from(rank),
        // After every cache: images are the slowest to fetch again.
        Class::Podman { .. } => u16::MAX,
        // Generations, ranked with everything else at 0, come first, since
        // what they free is garbage, then the garbage policy's own order: by
        // age, then by size.
        Class::Garbage { order, .. } => order.saturating_add(1),
        _ => 0,
    };
    steps.sort_by(|a, b| {
        let idle = |step: &Step| candidates[step.index].idle;
        let size = |step: &Step| candidates[step.index].size;
        let waste = a.group == Group::Waste && b.group == Group::Waste;
        (a.group, rank(a))
            .cmp(&(b.group, rank(b)))
            .then_with(|| {
                if waste {
                    size(b).cmp(&size(a))
                } else {
                    std::cmp::Ordering::Equal
                }
            })
            .then(idle(b).total_cmp(&idle(a)))
    });
    if let Some(shortfall) = shortfall {
        let mut covered = 0;
        let enough = steps.iter().position(|step| {
            covered += step.predicted;
            covered >= shortfall
        });
        if let Some(last) = enough {
            steps.truncate(last + 1);
        }
    }
    steps
}

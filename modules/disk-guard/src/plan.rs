//! Which candidates a pass takes in a band, and in what order.

use crate::band::Band;
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Class {
    Target,
    Install,
    /// An incremental session a newer one of its crate replaced.
    Variant,
    /// Scratch whose source keeps it while healthy for this long.
    Scratch {
        healthy: Option<f64>,
    },
    /// A snapshot whose process is gone.
    Snapshot,
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
            Class::Producer { .. } | Class::Podman { .. } | Class::Garbage { .. }
        )
    }

    /// The kind's name, which what it frees is learned under.
    pub fn kind(self) -> &'static str {
        match self {
            Class::Target => "target",
            Class::Install => "install",
            Class::Variant => "variant",
            Class::Scratch { .. } => "scratch",
            Class::Snapshot => "snapshot",
            Class::Workspace => "workspace",
            Class::Temp => "temp",
            Class::Trash => "trash",
            Class::Cache { .. } => "cache",
            Class::Coredump => "coredump",
            Class::NixBuild => "nix-build",
            Class::Replaced => "replaced",
            Class::Garbage { .. } => "garbage",
            Class::Producer { .. } => "producer",
            Class::Log => "log",
            Class::Podman { .. } => "podman",
        }
    }
}

/// Something a pass may reclaim. `idle` is the awake time since it was last
/// used, or since it was kept for what it was kept for; `size` is logical.
#[derive(Clone, Debug, PartialEq)]
pub struct Candidate {
    pub path: PathBuf,
    pub class: Class,
    pub idle: f64,
    pub size: u64,
    /// A shell or agent sits in its tree.
    pub occupied: bool,
    /// Its tree is finished or gone.
    pub spent: bool,
    /// The checkout it was built for, when known.
    pub checkout: Option<PathBuf>,
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

/// A window per band, healthy first: how long a candidate must have been
/// idle, in awake seconds, before that band takes it. None never does.
type Windows = [Option<f64>; 4];

const NEVER: Option<f64> = None;
const AT_ONCE: Option<f64> = Some(0.0);

fn hours(n: f64) -> Option<f64> {
    Some(n * HOUR)
}

fn days(n: f64) -> Option<f64> {
    Some(n * DAY)
}

/// The group a band takes a candidate in, if it takes it at all. The
/// windows are the plan's table; a tree someone sits in goes only as
/// occupied, however finished it is.
pub fn group(candidate: &Candidate, band: Band) -> Option<Group> {
    let tree = matches!(candidate.class, Class::Target | Class::Install);
    let (group, windows): (Group, Windows) = match candidate.class {
        Class::Target | Class::Install if candidate.occupied => match candidate.class {
            Class::Target => (Group::Occupied, [NEVER, NEVER, days(1.0), AT_ONCE]),
            _ => (Group::Occupied, [NEVER, NEVER, NEVER, AT_ONCE]),
        },
        _ if tree && candidate.spent => (Group::Waste, [AT_ONCE; 4]),
        Class::Target => (Group::Idle, [NEVER, hours(6.0), hours(2.0), AT_ONCE]),
        Class::Install => (Group::Idle, [NEVER, NEVER, hours(2.0), AT_ONCE]),
        Class::Variant => (
            Group::Variants,
            [days(7.0), days(1.0), hours(2.0), hours(2.0)],
        ),
        Class::Scratch { healthy } => (
            Group::Variants,
            [healthy, days(1.0), hours(6.0), hours(2.0)],
        ),
        Class::Snapshot | Class::NixBuild => (Group::Waste, [AT_ONCE; 4]),
        Class::Workspace => (Group::Waste, [days(30.0), days(7.0), days(1.0), AT_ONCE]),
        Class::Temp => (Group::Waste, [days(1.0), days(1.0), hours(6.0), hours(2.0)]),
        Class::Trash => (Group::Waste, [days(30.0), days(7.0), days(1.0), AT_ONCE]),
        Class::Cache { from, .. } => {
            let from = |band: Band| if band >= from { AT_ONCE } else { NEVER };
            (
                Group::Caches,
                [
                    from(Band::Healthy),
                    from(Band::Pressure),
                    from(Band::Urgent),
                    from(Band::Critical),
                ],
            )
        }
        Class::Coredump if candidate.idle < DAY => {
            (Group::Occupied, [NEVER, NEVER, NEVER, AT_ONCE])
        }
        Class::Coredump => (Group::Waste, [NEVER, NEVER, days(1.0), AT_ONCE]),
        Class::Replaced => (
            Group::Replaced,
            [NEVER, Some(DAY + HOUR), Some(DAY + HOUR), Some(DAY + HOUR)],
        ),
        // Their own policies decide how much each takes.
        // What's young and within budget waits for pressure.
        Class::Garbage { kept: true, .. } => (Group::Garbage, [NEVER, AT_ONCE, AT_ONCE, AT_ONCE]),
        Class::Garbage { kept: false, .. } => (Group::Garbage, [AT_ONCE; 4]),
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
                    window(Band::Pressure),
                    window(Band::Urgent),
                    window(Band::Critical),
                ],
            )
        }
        // What's trimmed is history nobody reads; the rest stays.
        Class::Log => (Group::Waste, [AT_ONCE; 4]),
        // At most hourly, like a producer's.
        Class::Podman { unused: false } => (Group::Caches, [NEVER, NEVER, hours(1.0), hours(1.0)]),
        Class::Podman { unused: true } => (Group::Occupied, [NEVER, NEVER, NEVER, hours(1.0)]),
    };
    let window = windows[band as usize]?;
    (candidate.idle >= window).then_some(group)
}

/// What a band takes, cheapest loss first: by group, caches by how cheaply
/// they refill, then the longest idle. With a shortfall the plan stops once
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
            let predicted = (candidate.size as f64 * ratio(candidate.class).max(0.0)) as u64;
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
        // The garbage policy's own order: by age, then by size.
        Class::Garbage { order, .. } => order,
        _ => 0,
    };
    steps.sort_by(|a, b| {
        let idle = |step: &Step| candidates[step.index].idle;
        (a.group, rank(a))
            .cmp(&(b.group, rank(b)))
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

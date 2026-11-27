//! Ranks each pi host's unit as the sessions it holds would rank in
//! windows of their own.
//!
//! A pi host (modules/pi/host.nix) is one node process holding many pi
//! conversations, run as `pi-host-<realm>-<kind>.service` in agents.slice.
//! Its conversations' screens live in the host, and a terminal that shows
//! one runs only `mycelium attach --host <key> --session <id>`, which
//! takes no CPU of note. So ranking windows misses them: on 11 November
//! K7's hosted liveness audit (gap 17) found the host's unit never weighed,
//! never held while Joel was at a window showing one of its conversations,
//! and an orchestrator it held kept no floor of its own.
//!
//! So, for each host unit under a `--hosts` prefix:
//!
//! - Its weight is a window's: [`focus::WEIGHT`] while a window Joel is at
//!   shows one of its conversations, the low weight while the harness marks
//!   the host's pid as one nobody is at, which it does only while every
//!   conversation it holds may yield (the harness's pid-mark.ts), and an
//!   app's otherwise. A host's start isn't weighed up as a pi's is: that
//!   is a pi Joel just opened, and a host starts with no window waiting.
//! - Its floor is the unit's own, plus [`focus::FLOOR`] for each window
//!   Joel is at that shows one of its conversations, and [`keep::FLOOR`]
//!   for each orchestrator among them, as each would have in a window.
//!   With neither, a window that only shows one adds [`keep::PI_FLOOR`],
//!   once, as a window running a pi of its own keeps.
//! - While any hold it, it keeps a hard floor of [`holds::HARD`] for each
//!   viewer [`holds::shares`] gives it, beside the unit's own.
//! - While any but the last hold it, nothing of it is written to the
//!   disk's swap and systemd-oomd ends it last, as a held window's.
//!
//! A host may run each conversation in a unit of its own instead,
//! `<conversations><key>@<slot>.service` under `--conversations`, the slot
//! named by a file in `--slots` that holds the session (`src/joins.rs`
//! finds which unit a viewer joins). Each such unit is
//! ranked the same way, by the windows showing its one conversation and
//! whether that orchestrates, save that oomd isn't told to end it last,
//! as its busy commands stay in it; and a window showing it gives the host
//! nothing: on 21 November 75 conversations ran in units of their own and
//! the two hosts' units held 21 and 34 MB, so every floor given a host
//! kept only that, and the windows Joel was at kept none of theirs.
//!
//! Giving back puts the unit's own settings back, not systemd's defaults
//! as a window's give back does: a host's unit sets a floor and oomd's
//! avoid of its own. Those are read from the unit's files, leaving out
//! the runtime ones this sets, since what systemd reports includes them
//! (`src/own.rs`).
//!
//! Its adjustment for the kernel's OOM killer is left at the account's.
//! Ending a host ends every conversation it holds, which
//! [`keep::GIVES_WAY`] would make the first a shortage takes.
//!
//! Long because its cases, more than half of it, share the helpers that
//! build a unit's own settings and signs, and split apart would repeat them.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::own::{Own, template};
use crate::{focus, holds, keep, low};

/// How long a unit's own settings are trusted before they're read again,
/// so a switch that changes them reaches a host that keeps running.
pub const OWN_FOR: Duration = Duration::from_secs(60);

/// How long a unit that has gone is asked after until systemd says it
/// stopped ([`Hosts::leave`]).
pub const PENDING_FOR: Duration = Duration::from_secs(60);

/// A host unit's rank: the properties set on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rank {
    pub weight: u64,
    pub floor: u64,
    pub hard: u64,
    pub writeback: bool,
    pub preference: String,
}

/// What ranks a host now, beside its unit's own settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Signs {
    /// Windows Joel is at that show one of its conversations.
    pub viewed: usize,
    /// Of those, how many it keeps a hard floor for ([`holds::shares`]).
    pub held: usize,
    /// Windows that show one of its conversations, whether he's at them
    /// or not.
    pub shown: usize,
    /// Orchestrators among its conversations.
    pub orchestrators: usize,
    /// Whether the harness marks the host as one nobody is at.
    pub marked: bool,
    /// The memory its unit holds now, as its memory.current says; none
    /// when that can't be read.
    pub current: Option<u64>,
    /// Whether its unit keeps the busy commands its conversations start,
    /// as a conversation's own unit does: only a host's are moved to jobs.
    pub keeps_its_work: bool,
}

/// The rank a host wants from its unit's `own` settings and its `signs`,
/// going down to `low` while it is marked and nothing holds it.
pub fn wanted(own: &Own, signs: Signs, low: u64) -> Rank {
    let viewed = u64::try_from(signs.viewed).unwrap_or(u64::MAX);
    let orchestrators = u64::try_from(signs.orchestrators).unwrap_or(u64::MAX);
    let floor = own
        .floor
        .saturating_add(viewed.saturating_mul(focus::FLOOR))
        .saturating_add(orchestrators.saturating_mul(keep::FLOOR));
    let kept = signs.viewed > 0 || signs.orchestrators > 0;
    // A window that only shows a conversation gives it a pi's window's floor, once, as a window
    // running its own pi keeps one; a held window's or an orchestrator's stands in for it.
    let floor = if signs.shown > 0 && !kept {
        floor.saturating_add(keep::PI_FLOOR)
    } else {
        floor
    };
    // oomd passes over a held unit only when its busy commands are moved to jobs: one keeping
    // them, as a conversation's own unit does, may hold a build's gigabytes, and passing over
    // such units on 15 November ended 38 small ones beside them, freeing nothing.
    let preference = if kept && !signs.keeps_its_work && own.preference != "omit" {
        "avoid".to_owned()
    } else {
        own.preference.clone()
    };
    let weight = if signs.viewed > 0 {
        focus::WEIGHT
    } else if signs.marked {
        low
    } else {
        low::AT
    };
    // A window held keeps a hard floor too, whole whatever the unit holds now, so a unit just
    // started or reclaimed grows into it, for as many as `holds::shares` leaves it.
    let held = u64::try_from(signs.held)
        .unwrap_or(u64::MAX)
        .saturating_mul(holds::HARD);
    Rank {
        weight,
        floor,
        hard: own.hard.saturating_add(held),
        writeback: writeback(own.writeback, &signs, floor),
        preference,
    }
}

/// Whether a unit keeps writeback to the disk's swap at `floor`, as `own`
/// says, but while Joel is at it and it's read to fit: one process holds
/// every conversation a host runs, and #3070 kept windows from holding all.
pub fn writeback(own: bool, signs: &Signs, floor: u64) -> bool {
    own && (signs.viewed == 0 || signs.current.is_none_or(|current| current > floor))
}

/// How many hard floors for windows a unit keeps beside its own `hard`,
/// as its memory.min, `min`, says: each [`holds::HARD`] or part of one,
/// and [`focus::MOST`] when it can't be read, so a unit not known to keep
/// none counts as keeping as many as it could.
pub fn kept_hard(min: Option<u64>, hard: u64) -> usize {
    min.map_or(focus::MOST, |min| {
        let kept = min.saturating_sub(hard).div_ceil(holds::HARD);
        usize::try_from(kept).map_or(focus::MOST, |kept| kept.min(focus::MOST))
    })
}

/// The orchestrators a unit's `roots` run, as `orchestrating` counts them
/// by pid, and whether `marked` says the harness marks any of them. Every
/// root counts, since a command a conversation left behind is a root
/// beside the conversation's own process.
pub fn counted(
    roots: &[u32],
    orchestrating: &HashMap<u32, usize>,
    marked: impl Fn(u32) -> bool,
) -> (usize, bool) {
    let orchestrators = roots
        .iter()
        .filter_map(|root| orchestrating.get(root))
        .sum();
    (orchestrators, roots.iter().any(|root| marked(*root)))
}

/// The host units under `slice` whose names start with `prefix`.
pub fn units(slice: &Path, prefix: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(slice) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.starts_with(prefix)
                        && Path::new(name)
                            .extension()
                            .is_some_and(|ext| ext == "service")
                })
        })
        .collect()
}

/// The pids among `procs`, each `(pid, parent)`, whose parent is outside
/// them: the process the unit started, and any command one left behind.
pub fn roots(procs: &[(u32, u32)]) -> Vec<u32> {
    let inside: HashSet<u32> = procs.iter().map(|(pid, _)| *pid).collect();
    procs
        .iter()
        .filter(|(_, parent)| !inside.contains(parent))
        .map(|(pid, _)| *pid)
        .collect()
}

/// The host among a unit's processes, each as its pid and its parent's:
/// the one process whose parent is outside the unit, as a subagent a
/// conversation starts is a child of the host.
pub fn host_of(procs: &[(u32, u32)]) -> Option<u32> {
    match roots(procs)[..] {
        [root] => Some(root),
        _ => None,
    }
}

/// The number of orchestrator sessions each pid holds, from presences
/// read as `keep::orchestrators` reads them: one per window's pi, and as
/// many as a host holds for its pid.
pub fn orchestrating(
    presences: &[keep::Presence],
    boot: &str,
    runs: impl Fn(u32) -> bool,
) -> HashMap<u32, usize> {
    let pids = keep::orchestrators(presences, boot, &runs);
    let named: HashSet<&str> = presences
        .iter()
        .filter(|session| session.boot.as_deref() == Some(boot) && session.pid.is_some_and(&runs))
        .filter_map(|session| session.orchestrated_by.as_deref())
        .collect();
    let mut counts = HashMap::new();
    for session in presences {
        let Some(pid) = session.pid else { continue };
        if session.boot.as_deref() == Some(boot)
            && pids.contains(&pid)
            && session
                .addresses
                .iter()
                .any(|address| named.contains(address.as_str()))
        {
            *counts.entry(pid).or_insert(0) += 1;
        }
    }
    counts
}

/// What busctl is given to set a host unit's rank, as runtime properties
/// gone at the next boot. Its weight is the disk's as well as the
/// processor's, as `low::weigh_call` says why.
pub fn rank_call(unit: &str, rank: &Rank) -> Vec<String> {
    let weight = rank.weight.to_string();
    let floor = rank.floor.to_string();
    let writeback = rank.writeback.to_string();
    let hard = rank.hard.to_string();
    [
        "--user",
        "--quiet",
        "call",
        "org.freedesktop.systemd1",
        "/org/freedesktop/systemd1",
        "org.freedesktop.systemd1.Manager",
        "SetUnitProperties",
        "sba(sv)",
        unit,
        "true",
        "6",
        "CPUWeight",
        "t",
        &weight,
        "MemoryLow",
        "t",
        &floor,
        "MemoryZSwapWriteback",
        "b",
        &writeback,
        "ManagedOOMPreference",
        "s",
        &rank.preference,
        "MemoryMin",
        "t",
        &hard,
        "IOWeight",
        "t",
        &weight,
    ]
    .iter()
    .map(|arg| (*arg).to_string())
    .collect()
}

/// Each host unit's own settings as last read, with the host's pid and
/// when, and its rank as last set, so only a change is asked for.
#[derive(Default)]
pub struct Hosts {
    own: HashMap<String, (u32, Instant, Own)>,
    set: HashMap<String, Rank>,
    /// The hard floors each unit's rank as last set keeps for windows
    /// Joel is at ([`Signs::held`]).
    held: HashMap<String, usize>,
    /// The units whose last rank call failed, so the kernel still keeps
    /// what they were last set with.
    failing: HashSet<String>,
    /// Whether a failure to rank a host has been said.
    pub failed: bool,
    /// How far the settings units that stopped left behind have been
    /// taken back (`crate::sweep`).
    pub sweep: Sweep,
    /// The units said to hold no one host, ranked by all their roots, so
    /// each is said once.
    pub unclear: HashSet<String>,
}

/// How far the settings units that no longer run left behind have been
/// taken back (`crate::own::forget`), and what of it has been said.
#[derive(Default)]
pub struct Sweep {
    /// Whether every unit's have been since the sorter started.
    pub done: bool,
    /// Whether a failure to ask which units stopped has been said.
    pub failed: bool,
    /// When the sweep may ask again after an ask that failed
    /// (`crate::own::ASK_AGAIN_AFTER`).
    pub held_off: Option<Instant>,
    /// Whether a failure to mark a unit's settings as the sorter's
    /// (`crate::own::mark`) has been said.
    pub unmarked: bool,
    /// The units ranked before that have gone, each with when it went,
    /// whose settings are taken back once systemd says they stopped: one
    /// just gone may still be stopping ([`Hosts::leave`]).
    pub pending: HashMap<String, Instant>,
    /// The units systemctl last said were stopping, running or failed
    /// though none of their processes were seen, each with when to ask
    /// after it again (`crate::own::ASK_AGAIN_AFTER`): one failed is kept
    /// loaded, its settings listed, until it's reset, and one not ranked
    /// since the sorter started is never pending.
    pub later: HashMap<String, Instant>,
    /// The units whose settings couldn't all be taken back, so each is
    /// said once.
    pub unswept: HashSet<String>,
}

impl Hosts {
    /// A unit's own settings while its host runs as `pid`, read with
    /// `read` when none were read for that pid within [`OWN_FOR`] of `now`.
    /// A read that fails keeps what was read before, or systemd's defaults.
    pub fn own(
        &mut self,
        unit: &str,
        pid: u32,
        now: Instant,
        read: impl FnOnce() -> Option<Own>,
    ) -> Own {
        match self.own.get(unit) {
            Some((had, at, own)) if *had == pid && now.saturating_duration_since(*at) < OWN_FOR => {
                own.clone()
            }
            before => {
                let own = read()
                    .or_else(|| before.map(|(_, _, own)| own.clone()))
                    .unwrap_or_default();
                self.own.insert(unit.to_owned(), (pid, now, own.clone()));
                own
            }
        }
    }

    /// The floor last set on `unit`, if any was.
    pub fn floor_set(&self, unit: &str) -> Option<u64> {
        self.set.get(unit).map(|rank| rank.floor)
    }

    /// Whether `unit` should be set to `rank`: it changed, or this sorter
    /// never set it, as one before it may have left it held.
    pub fn changed(&self, unit: &str, rank: &Rank) -> bool {
        self.set.get(unit).is_none_or(|set| set != rank)
    }

    /// Notes a unit as set to `rank`, keeping `held` hard floors for
    /// windows Joel is at.
    pub fn done(&mut self, unit: &str, rank: Rank, held: usize) {
        self.set.insert(unit.to_owned(), rank);
        self.held.insert(unit.to_owned(), held);
        self.failing.remove(unit);
    }

    /// Notes a unit's rank call as failed, so the hard floors it was
    /// last set with count as kept until one lands. One this sorter never
    /// set, as it restarted or the unit was briefly empty and forgotten,
    /// may keep what a call before left it, so it counts `kept`'s, what
    /// the kernel says it keeps ([`kept_hard`]).
    pub fn fail(&mut self, unit: &str, kept: impl FnOnce() -> usize) {
        self.failing.insert(unit.to_owned());
        if !self.held.contains_key(unit) {
            self.held.insert(unit.to_owned(), kept());
        }
    }

    /// The hard floors `unit` keeps only as its last rank call failed,
    /// if it failed: a give-back that fails leaves them with the kernel,
    /// as a pi's does (`holds::Holding::hard_floor`).
    pub fn kept(&self, unit: &str) -> Option<usize> {
        if self.failing.contains(unit) {
            Some(self.held.get(unit).copied().unwrap_or(0))
        } else {
            None
        }
    }

    /// The hard floors `unit` keeps as far as this sorter knows: those
    /// its last rank that landed keeps, those the kernel said it kept when
    /// its first call failed ([`Hosts::fail`]), or none.
    pub fn last_held(&self, unit: &str) -> usize {
        self.held.get(unit).copied().unwrap_or(0)
    }

    /// The hard floors every unit keeps only as its last rank call
    /// failed ([`Hosts::kept`]), viewed or not: a viewed one's shares are
    /// given again, but until one lands the kernel keeps what it had.
    pub fn hard_left(&self) -> usize {
        self.failing.iter().filter_map(|unit| self.kept(unit)).sum()
    }

    /// Notes each unit ranked before and not among `units` now as gone at
    /// `now`, so its settings are taken back once it has stopped, and
    /// one among them again as no longer gone. One still not stopped
    /// [`PENDING_FOR`] after it went is asked after no more, as each look
    /// would ask again while it lasts; the next sorter's first look takes
    /// back what it left.
    pub fn leave(&mut self, units: &[String], now: Instant) {
        for unit in self.set.keys().filter(|unit| !units.contains(unit)) {
            self.sweep.pending.entry(unit.clone()).or_insert(now);
        }
        self.sweep.pending.retain(|unit, since| {
            !units.contains(unit) && now.saturating_duration_since(*since) < PENDING_FOR
        });
    }

    /// Forgets all this sorter set on `unit`, whose settings were taken
    /// back, so it's ranked afresh should it start again under its name.
    pub fn forget(&mut self, unit: &str) {
        self.set.remove(unit);
        self.held.remove(unit);
        self.failing.remove(unit);
        self.sweep.pending.remove(unit);
    }

    /// Forgets the units no longer among `units`, whose hosts stopped.
    pub fn keep_only(&mut self, units: &[String]) {
        // A template's settings serve every instance of it, so they stay while one runs.
        let templates: HashSet<String> = units.iter().filter_map(|unit| template(unit)).collect();
        self.own
            .retain(|unit, _| units.contains(unit) || templates.contains(unit));
        self.set.retain(|unit, _| units.contains(unit));
        self.held.retain(|unit, _| units.contains(unit));
        self.failing.retain(|unit| units.contains(unit));
        self.unclear.retain(|unit| units.contains(unit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn attended() -> Own {
        Own {
            floor: 512 << 20,
            hard: 0,
            writeback: true,
            preference: "avoid".to_owned(),
        }
    }

    #[test]
    fn a_host_nothing_holds_keeps_its_units_own_settings() {
        let rank = wanted(&attended(), Signs::default(), 10);
        assert_eq!(
            rank,
            Rank {
                weight: low::AT,
                floor: 512 << 20,
                hard: 0,
                writeback: true,
                preference: "avoid".to_owned(),
            },
            "a host nothing holds is at an app's weight with its unit's own floor"
        );
    }

    #[test]
    fn a_marked_host_goes_low_until_joel_is_at_one_of_its_windows() {
        let marked = Signs {
            marked: true,
            ..Signs::default()
        };
        assert_eq!(
            wanted(&attended(), marked, 10).weight,
            10,
            "a host every conversation of which may yield goes low"
        );
        let viewed = Signs {
            viewed: 1,
            ..marked
        };
        assert_eq!(
            wanted(&attended(), viewed, 10).weight,
            focus::WEIGHT,
            "a window Joel is at comes before a mark, as a window's focus does"
        );
    }

    #[test]
    fn each_window_joel_is_at_adds_a_held_windows_floor_and_keeps_it_in_memory() {
        let own = Own::default();
        let rank = wanted(
            &own,
            Signs {
                viewed: 2,
                current: Some(GIB),
                ..Signs::default()
            },
            10,
        );
        assert_eq!(
            rank.floor,
            2 * focus::FLOOR,
            "two windows shown from one host each keep a held window's floor"
        );
        assert!(
            !rank.writeback,
            "a held host's pages stay out of the disk's swap while it fits its floor"
        );
        assert_eq!(
            rank.preference, "avoid",
            "systemd-oomd ends a held host last"
        );
    }

    #[test]
    fn each_window_joel_is_at_holds_its_unit_hard_whatever_it_holds_now() {
        let own = Own {
            hard: 64 << 20,
            ..Own::default()
        };
        let held = |viewed, current| {
            wanted(
                &own,
                Signs {
                    viewed: 3,
                    held: viewed,
                    current,
                    ..Signs::default()
                },
                10,
            )
            .hard
        };
        assert_eq!(
            held(1, Some(30 << 20)),
            (64 << 20) + holds::HARD,
            "a unit just started or reclaimed keeps a whole held window's hard floor, to grow into"
        );
        assert_eq!(
            held(1, None),
            (64 << 20) + holds::HARD,
            "so does one whose memory can't be read"
        );
        assert_eq!(
            held(2, Some(3 * GIB)),
            (64 << 20) + 2 * holds::HARD,
            "each window it's given keeps one, whatever the unit holds"
        );
        assert_eq!(
            held(0, Some(GIB)),
            64 << 20,
            "none given keeps the unit's own, however many windows Joel is at show it"
        );
    }

    #[test]
    fn a_held_unit_is_set_again_only_as_its_rank_changes() {
        let mut hosts = Hosts::default();
        let viewed = |hard, current| {
            wanted(
                &Own {
                    hard,
                    ..Own::default()
                },
                Signs {
                    viewed: 1,
                    held: 1,
                    current: Some(current),
                    ..Signs::default()
                },
                10,
            )
        };
        hosts.done("u", viewed(0, 300 << 20), 0);
        assert!(
            !hosts.changed("u", &viewed(0, 310 << 20)),
            "what a held unit holds moving isn't asked for again"
        );
        assert!(
            hosts.changed("u", &viewed(64 << 20, 300 << 20)),
            "a switch changing the unit's own hard floor reaches it"
        );
        assert!(
            hosts.changed("u", &wanted(&Own::default(), Signs::default(), 10)),
            "giving the hold back is"
        );
    }

    #[test]
    fn each_orchestrator_a_host_holds_adds_an_orchestrators_floor() {
        let rank = wanted(
            &attended(),
            Signs {
                orchestrators: 2,
                ..Signs::default()
            },
            10,
        );
        assert_eq!(
            rank.floor,
            (512 << 20) + 2 * keep::FLOOR,
            "each orchestrator keeps its own floor on top of the host's"
        );
        assert!(
            rank.writeback,
            "an orchestrator alone leaves writeback be, as a kept window's"
        );
        assert_eq!(
            wanted(
                &Own::default(),
                Signs {
                    orchestrators: 1,
                    ..Signs::default()
                },
                10
            )
            .preference,
            "avoid",
            "a host holding an orchestrator is the last systemd-oomd ends"
        );
    }

    #[test]
    fn a_viewed_host_past_its_floor_keeps_writeback() {
        let viewed = |current| Signs {
            viewed: 1,
            current,
            ..Signs::default()
        };
        let floor = (512 << 20) + focus::FLOOR;
        assert!(
            !wanted(&attended(), viewed(Some(floor)), 10).writeback,
            "a viewed host at its floor keeps its pages out of the disk's swap"
        );
        assert!(
            wanted(&attended(), viewed(Some(floor + 1)), 10).writeback,
            "one past its floor may write back, so none of its conversations must stay in memory"
        );
        assert!(
            wanted(&attended(), viewed(None), 10).writeback,
            "one whose memory can't be read may not fit, so writeback stays"
        );
    }

    #[test]
    fn a_unit_that_omits_itself_from_oomd_stays_omitted() {
        let own = Own {
            preference: "omit".to_owned(),
            ..Own::default()
        };
        assert_eq!(
            wanted(
                &own,
                Signs {
                    viewed: 1,
                    ..Signs::default()
                },
                10
            )
            .preference,
            "omit",
            "holding a host never lets oomd end one it was told never to"
        );
    }

    #[test]
    fn a_conversation_any_window_shows_keeps_a_pis_floor_softly() {
        let shown = Signs {
            shown: 2,
            ..Signs::default()
        };
        let rank = wanted(&Own::default(), shown, 10);
        assert_eq!(
            rank.floor,
            keep::PI_FLOOR,
            "one floor for any number of windows"
        );
        assert_eq!(rank.preference, "none", "oomd may end it before a held one");
        assert_eq!(rank.weight, low::AT, "showing it isn't being at it");
        let held = Signs { viewed: 1, ..shown };
        assert_eq!(
            wanted(&Own::default(), held, 10).floor,
            focus::FLOOR,
            "a held window's floor stands in for a shown one's"
        );
        let orchestrating = Signs {
            orchestrators: 1,
            ..shown
        };
        assert_eq!(
            wanted(&Own::default(), orchestrating, 10).floor,
            keep::FLOOR,
            "so does an orchestrator's"
        );
        let keeping = Signs {
            keeps_its_work: true,
            ..held
        };
        assert_eq!(
            (
                wanted(&Own::default(), keeping, 10).floor,
                wanted(&Own::default(), keeping, 10).preference.as_str()
            ),
            (focus::FLOOR, "none"),
            "a unit keeping its conversation's builds keeps its floor, but oomd may still end it"
        );
    }

    #[test]
    fn every_root_counts_its_orchestrators_and_marks() {
        let orchestrating = HashMap::from([(20, 1), (30, 2)]);
        assert_eq!(
            counted(&[10, 20], &orchestrating, |pid| pid == 10),
            (1, true),
            "a conversation's orchestrator counts beside a command it left, and so does a mark"
        );
        assert_eq!(
            counted(&[10, 30], &orchestrating, |_| false),
            (2, false),
            "each root's orchestrators add up"
        );
        assert_eq!(
            counted(&[10], &orchestrating, |_| false),
            (0, false),
            "a root running none, unmarked, adds nothing"
        );
    }

    #[test]
    fn the_host_is_the_one_process_whose_parent_is_outside() {
        assert_eq!(
            host_of(&[(10, 1)]),
            Some(10),
            "a host alone is its unit's host"
        );
        assert_eq!(
            host_of(&[(10, 1), (11, 10), (12, 11)]),
            Some(10),
            "a subagent a conversation starts is the host's child"
        );
        assert_eq!(host_of(&[]), None, "a unit with nothing in it has no host");
        assert_eq!(
            host_of(&[(10, 1), (20, 1)]),
            None,
            "two roots can't say which is the host"
        );
        assert_eq!(
            roots(&[(10, 1), (20, 1), (21, 20)]),
            [10, 20],
            "a command its host left behind is a root beside it, so both are counted"
        );
    }

    #[test]
    fn a_host_counts_each_orchestrator_it_holds() {
        let boot = "b00t";
        let session = |address: &str, by: Option<&str>, pid: u32| keep::Presence {
            addresses: vec![address.to_owned()],
            orchestrated_by: by.map(str::to_owned),
            quest: None,
            pid: Some(pid),
            boot: Some(boot.to_owned()),
        };
        let presences = [
            session("ORCH1", None, 7),
            session("ORCH2", None, 7),
            session("IDLE", None, 7),
            session("W1", Some("ORCH1"), 8),
            session("W2", Some("ORCH2"), 9),
            session("SOLO", None, 5),
            session("W3", Some("SOLO"), 9),
        ];
        let counts = orchestrating(&presences, boot, |_| true);
        assert_eq!(counts.get(&7), Some(&2), "the host holds two orchestrators");
        assert_eq!(counts.get(&5), Some(&1), "a window's pi orchestrates once");
        assert_eq!(counts.get(&8), None, "a worker orchestrates nobody");
    }

    #[test]
    fn own_settings_are_read_again_for_a_new_host_or_after_a_while() {
        let mut hosts = Hosts::default();
        let start = Instant::now();
        let mut reads = 0;
        let mut read = |own: Own| {
            reads += 1;
            Some(own)
        };
        assert_eq!(hosts.own("u", 1, start, || read(attended())), attended());
        assert_eq!(
            hosts.own("u", 1, start + Duration::from_secs(1), || read(
                Own::default()
            )),
            attended(),
            "within a minute the same host's settings aren't read again"
        );
        assert_eq!(
            hosts.own("u", 2, start + Duration::from_secs(2), || read(
                Own::default()
            )),
            Own::default(),
            "a host started again reads its unit again"
        );
        assert_eq!(
            hosts.own("u", 2, start + OWN_FOR + Duration::from_secs(3), || None),
            Own::default(),
            "a failed read keeps what was read before"
        );
        assert_eq!(reads, 2, "only a new host was read again");
        hosts.own("c@.service", 0, start, || Some(attended()));
        hosts.unclear.insert("c@1.service".to_owned());
        hosts.keep_only(&["c@1.service".to_owned()]);
        assert_eq!(
            hosts.own("c@.service", 0, start, || None),
            attended(),
            "a template's settings stay while an instance of it runs"
        );
        assert!(
            hosts.own.contains_key("c@.service") && !hosts.own.contains_key("u"),
            "the template stays and the gone host's settings go"
        );
        assert!(
            hosts.unclear.contains("c@1.service"),
            "a running unit's doubt stays said, so it isn't said again"
        );
        hosts.keep_only(&[]);
        assert!(
            hosts.own.is_empty() && hosts.unclear.is_empty(),
            "a gone unit's settings and said doubt are forgotten"
        );
    }

    #[test]
    fn a_rank_is_set_once_and_again_once_it_changes() {
        let mut hosts = Hosts::default();
        let rank = wanted(&attended(), Signs::default(), 10);
        assert!(
            hosts.changed("u", &rank),
            "a unit this sorter never set is set, as one before it may have held it"
        );
        hosts.done("u", rank.clone(), 0);
        assert!(
            !hosts.changed("u", &rank),
            "a rank already set isn't asked for again"
        );
        let held = wanted(
            &attended(),
            Signs {
                viewed: 1,
                ..Signs::default()
            },
            10,
        );
        assert!(hosts.changed("u", &held), "a change is asked for");
        hosts.keep_only(&[]);
        assert!(
            hosts.changed("u", &rank),
            "a stopped host's unit is forgotten"
        );
    }

    #[test]
    fn a_unit_ranked_before_and_gone_now_is_asked_after_for_a_minute() {
        let mut hosts = Hosts::default();
        let rank = wanted(&attended(), Signs::default(), 10);
        hosts.done("a", rank.clone(), 0);
        hosts.done("b", rank.clone(), 0);
        let start = Instant::now();
        let pending = |hosts: &Hosts| {
            let mut units: Vec<String> = hosts.sweep.pending.keys().cloned().collect();
            units.sort();
            units
        };
        hosts.leave(&["a".to_owned(), "b".to_owned(), "c".to_owned()], start);
        assert!(
            pending(&hosts).is_empty(),
            "no unit has gone while both run, and one starting hasn't"
        );
        hosts.leave(&["a".to_owned()], start);
        assert_eq!(
            pending(&hosts),
            ["b"],
            "a unit ranked before and gone now has gone"
        );
        hosts.keep_only(&["a".to_owned()]);
        hosts.leave(&["a".to_owned()], start + PENDING_FOR / 2);
        assert_eq!(
            pending(&hosts),
            ["b"],
            "once forgotten, it's still asked after, from when it went"
        );
        hosts.leave(&["a".to_owned()], start + PENDING_FOR);
        assert!(pending(&hosts).is_empty(), "but no longer than a minute");
        hosts.done("b", rank.clone(), 0);
        hosts.leave(&["a".to_owned()], start);
        hosts.leave(&["a".to_owned(), "b".to_owned()], start);
        assert!(pending(&hosts).is_empty(), "one back again hasn't gone");
        hosts.fail("a", || 2);
        hosts.forget("a");
        assert!(
            hosts.changed("a", &rank) && hosts.hard_left() == 0,
            "a unit whose settings were taken back is ranked afresh, keeping nothing"
        );
    }

    #[test]
    fn a_host_whose_rank_fails_counts_the_hard_floors_it_still_keeps() {
        let mut hosts = Hosts::default();
        let rank = |held| {
            wanted(
                &attended(),
                Signs {
                    viewed: 1,
                    held,
                    ..Signs::default()
                },
                10,
            )
        };
        hosts.done("a", rank(2), 2);
        hosts.done("b", rank(1), 1);
        assert_eq!(
            hosts.hard_left(),
            0,
            "hosts ranked as asked leave no hard floor behind"
        );
        hosts.fail("a", || panic!("a unit set before isn't read"));
        assert_eq!(
            hosts.hard_left(),
            2,
            "a host whose give-back failed still keeps the two it was set with"
        );
        assert_eq!(
            (hosts.kept("a"), hosts.kept("b")),
            (Some(2), None),
            "viewed or not, as only the kernel knows whether its next call lands"
        );
        hosts.done("a", rank(2), 2);
        assert_eq!(
            hosts.hard_left(),
            0,
            "once a rank keeping floors lands, they're counted in its shares, not left"
        );
        hosts.fail("b", || panic!("a unit set before isn't read"));
        assert_eq!(hosts.hard_left(), 1, "b's one is left as its call fails");
        hosts.keep_only(&["a".to_owned()]);
        assert_eq!(
            hosts.kept("b"),
            None,
            "a stopped host's floors went with its unit, its failure with them"
        );
        assert_eq!(hosts.hard_left(), 0, "so none is left");
    }

    #[test]
    fn a_host_whose_first_call_fails_counts_what_its_memory_min_keeps() {
        let mut hosts = Hosts::default();
        let own = 256 << 20;
        hosts.fail("a", || kept_hard(Some(own + 2 * holds::HARD), own));
        assert_eq!(
            hosts.hard_left(),
            2,
            "a host a sorter before this one set, whose first call fails, keeps its two"
        );
        hosts.fail("a", || panic!("what it keeps is read once"));
        hosts.fail("b", || kept_hard(None, 0));
        assert_eq!(
            hosts.hard_left(),
            2 + focus::MOST,
            "one whose memory.min can't be read counts as keeping all it could"
        );
        assert_eq!(
            [Some(own), Some(own + 1), Some(u64::MAX)].map(|min| kept_hard(min, own)),
            [0, 1, focus::MOST],
            "none past its own floor is none, a part of one counts, and at most three do"
        );
    }

    #[test]
    fn the_disk_weighs_a_host_as_the_processor_does() {
        let rank = Rank {
            weight: 400,
            floor: 0,
            hard: 0,
            writeback: true,
            preference: "none".to_string(),
        };
        let call = rank_call("pi-host-work-attended.service", &rank);
        let weight = |name: &str| {
            let at = call.iter().position(|arg| arg == name).expect(name);
            call[at + 2].clone()
        };
        assert_eq!(weight("CPUWeight"), "400");
        assert_eq!(weight("IOWeight"), "400", "the disk follows the rank, not systemd's 100");
    }

    #[test]
    fn the_call_sets_all_six_properties() {
        let call = rank_call(
            "pi-host-work-attended.service",
            &wanted(&attended(), Signs::default(), 10),
        );
        let tail: Vec<&str> = call[9..].iter().map(String::as_str).collect();
        assert_eq!(
            tail,
            [
                "true",
                "6",
                "CPUWeight",
                "t",
                "100",
                "MemoryLow",
                "t",
                "536870912",
                "MemoryZSwapWriteback",
                "b",
                "true",
                "ManagedOOMPreference",
                "s",
                "avoid",
                "MemoryMin",
                "t",
                "0",
                "IOWeight",
                "t",
                "100"
            ],
            "the call gives back the unit's own floors and preference"
        );
        let properties: Vec<&str> = tail[2..].iter().step_by(3).copied().collect();
        assert_eq!(
            properties,
            crate::own::SET,
            "what the call sets is what a sweep takes back, or one's file would stay"
        );
    }
}

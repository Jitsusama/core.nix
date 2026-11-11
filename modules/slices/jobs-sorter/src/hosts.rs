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
//! - While any of those hold it, nothing of it is written to the disk's
//!   swap and systemd-oomd ends it last, as a held window's.
//!
//! Giving back puts the unit's own settings back, not systemd's defaults
//! as a window's give back does: a host's unit sets a floor and oomd's
//! avoid of its own. Those are read from the unit's files, leaving out
//! the runtime ones this sets, since what systemd reports includes them.
//!
//! Its adjustment for the kernel's OOM killer is left at the account's.
//! Ending a host ends every conversation it holds, which
//! [`keep::GIVES_WAY`] would make the first a shortage takes.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crate::{focus, keep, low};

/// How long a unit's own settings are trusted before they're read again,
/// so a switch that changes them reaches a host that keeps running.
pub const OWN_FOR: Duration = Duration::from_secs(60);

/// The folder runtime properties set through systemd's manager land in,
/// which a unit's own settings leave out.
const RUNTIME_CONTROL: &str = "/systemd/user.control/";

/// A unit's own settings, as far as ranking it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Own {
    pub floor: u64,
    pub writeback: bool,
    pub preference: String,
}

impl Default for Own {
    /// systemd's defaults, which a unit that says nothing has.
    fn default() -> Self {
        Own {
            floor: 0,
            writeback: true,
            preference: "none".to_owned(),
        }
    }
}

/// A host unit's rank: the properties set on it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rank {
    pub weight: u64,
    pub floor: u64,
    pub writeback: bool,
    pub preference: String,
}

/// What ranks a host now, beside its unit's own settings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Signs {
    /// Windows Joel is at that show one of its conversations.
    pub viewed: usize,
    /// Orchestrators among its conversations.
    pub orchestrators: usize,
    /// Whether the harness marks the host as one nobody is at.
    pub marked: bool,
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
    let preference = if kept && own.preference != "omit" {
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
    Rank {
        weight,
        floor,
        writeback: own.writeback && signs.viewed == 0,
        preference,
    }
}

/// The host unit a viewer's words join, `<prefix><key>.service`, when
/// they are mycelium's `attach --host <key>`.
pub fn viewed(words: &[String], prefix: &str) -> Option<String> {
    let attach = words.iter().position(|word| word == "attach")?;
    let rest = &words[attach + 1..];
    let key = rest.iter().enumerate().find_map(|(at, word)| {
        if word == "--host" {
            rest.get(at + 1).cloned()
        } else {
            word.strip_prefix("--host=").map(str::to_owned)
        }
    })?;
    (!key.is_empty() && !key.contains('/')).then(|| format!("{prefix}{key}.service"))
}

/// A unit's own settings from `systemctl cat`'s text, its files each
/// under a `# <path>` line, leaving out the runtime ones set through the
/// manager. A later file's setting wins, and an empty one is systemd's
/// default again. A percentage is of `total`, the machine's memory.
pub fn own(text: &str, total: u64) -> Own {
    let mut own = Own::default();
    let mut counted = true;
    for line in text.lines() {
        let line = line.trim();
        if let Some(path) = line.strip_prefix("# /") {
            counted = !format!("/{path}").contains(RUNTIME_CONTROL);
            continue;
        }
        if !counted || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim() {
            "MemoryLow" => {
                own.floor = if value.is_empty() {
                    0
                } else {
                    size(value, total).unwrap_or(own.floor)
                };
            }
            "MemoryZSwapWriteback" => {
                own.writeback = value.is_empty() || !matches!(value, "no" | "false" | "0" | "off");
            }
            "ManagedOOMPreference" => {
                own.preference = if value.is_empty() {
                    "none".to_owned()
                } else {
                    value.to_owned()
                };
            }
            _ => {}
        }
    }
    own
}

/// A size as systemd reads one: bytes, with K, M, G or T for powers of
/// 1024, `infinity`, or a percentage of `total`.
pub fn size(value: &str, total: u64) -> Option<u64> {
    if value == "infinity" {
        return Some(u64::MAX);
    }
    if let Some(percent) = value.strip_suffix('%') {
        let percent: u64 = percent.trim().parse().ok()?;
        return Some(total / 100 * percent);
    }
    let (digits, shift) = match value.chars().last()? {
        'K' => (&value[..value.len() - 1], 10),
        'M' => (&value[..value.len() - 1], 20),
        'G' => (&value[..value.len() - 1], 30),
        'T' => (&value[..value.len() - 1], 40),
        _ => (value, 0),
    };
    let number: u64 = digits.trim().parse().ok()?;
    number.checked_mul(1 << shift)
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
                        && Path::new(name).extension().is_some_and(|ext| ext == "service")
                })
        })
        .collect()
}

/// The host among a unit's processes, each as its pid and its parent's:
/// the one process whose parent is outside the unit, as a subagent a
/// conversation starts is a child of the host.
pub fn host_of(procs: &[(u32, u32)]) -> Option<u32> {
    let inside: HashSet<u32> = procs.iter().map(|(pid, _)| *pid).collect();
    let mut roots = procs
        .iter()
        .filter(|(_, parent)| !inside.contains(parent))
        .map(|(pid, _)| *pid);
    let root = roots.next()?;
    roots.next().is_none().then_some(root)
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
/// gone at the next boot.
pub fn rank_call(unit: &str, rank: &Rank) -> Vec<String> {
    let weight = rank.weight.to_string();
    let floor = rank.floor.to_string();
    let writeback = rank.writeback.to_string();
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
        "4",
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
    /// Whether a failure to rank a host has been said.
    pub failed: bool,
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

    /// Whether `unit` should be set to `rank`: it changed, or this sorter
    /// never set it, as one before it may have left it held.
    pub fn changed(&self, unit: &str, rank: &Rank) -> bool {
        self.set.get(unit) != Some(rank)
    }

    /// Notes a unit as set to `rank`.
    pub fn done(&mut self, unit: &str, rank: Rank) {
        self.set.insert(unit.to_owned(), rank);
    }

    /// Forgets the units no longer among `units`, whose hosts stopped.
    pub fn keep_only(&mut self, units: &[String]) {
        self.own.retain(|unit, _| units.contains(unit));
        self.set.retain(|unit, _| units.contains(unit));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const GIB: u64 = 1 << 30;

    fn attended() -> Own {
        Own {
            floor: 512 << 20,
            writeback: true,
            preference: "avoid".to_owned(),
        }
    }

    fn words(line: &str) -> Vec<String> {
        line.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_host_nothing_holds_keeps_its_units_own_settings() {
        let rank = wanted(&attended(), Signs::default(), 10);
        assert_eq!(
            rank,
            Rank {
                weight: low::AT,
                floor: 512 << 20,
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
                ..Signs::default()
            },
            10,
        );
        assert_eq!(
            rank.floor,
            2 * focus::FLOOR,
            "two windows shown from one host each keep a held window's floor"
        );
        assert!(!rank.writeback, "a held host's pages stay out of the disk's swap");
        assert_eq!(rank.preference, "avoid", "systemd-oomd ends a held host last");
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
        assert!(rank.writeback, "an orchestrator alone leaves writeback be, as a kept window's");
        assert_eq!(
            wanted(&Own::default(), Signs { orchestrators: 1, ..Signs::default() }, 10).preference,
            "avoid",
            "a host holding an orchestrator is the last systemd-oomd ends"
        );
    }

    #[test]
    fn a_unit_that_omits_itself_from_oomd_stays_omitted() {
        let own = Own {
            preference: "omit".to_owned(),
            ..Own::default()
        };
        assert_eq!(
            wanted(&own, Signs { viewed: 1, ..Signs::default() }, 10).preference,
            "omit",
            "holding a host never lets oomd end one it was told never to"
        );
    }

    #[test]
    fn a_viewer_names_the_host_unit_it_joins() {
        let line = "mycelium --realm work attach --host work-attended --session 0193-ab";
        assert_eq!(
            viewed(&words(line), "pi-host-"),
            Some("pi-host-work-attended.service".to_owned()),
            "attach --host names the host's unit"
        );
        assert_eq!(
            viewed(&words("mycelium attach --host=work-background --session 1"), "pi-host-"),
            Some("pi-host-work-background.service".to_owned()),
            "the joined form names it too"
        );
        assert_eq!(
            viewed(&words("mycelium --realm work attach h0"), "pi-host-"),
            None,
            "attaching a held program joins no host"
        );
        assert_eq!(
            viewed(&words("ssh --host x attach"), "pi-host-"),
            None,
            "a --host before attach is another program's"
        );
        assert_eq!(
            viewed(&words("mycelium attach --host ../x"), "pi-host-"),
            None,
            "a key that isn't a unit's name names none"
        );
    }

    #[test]
    fn a_units_own_settings_leave_out_the_runtime_ones() {
        let text = "\
# /home/j/.config/systemd/user/pi-host-work-attended.service -> /nix/store/x
[Service]
ManagedOOMPreference=avoid
MemoryLow=512M

# /run/user/1000/systemd/user.control/pi-host-work-attended.service.d/50-MemoryLow.conf
[Service]
MemoryLow=3221225472
ManagedOOMPreference=none
";
        assert_eq!(
            own(text, 64 * GIB),
            attended(),
            "a floor this sorter set at runtime isn't the unit's own"
        );
    }

    #[test]
    fn a_later_file_wins_and_an_empty_setting_is_the_default_again() {
        let text = "\
# /etc/systemd/user/pi-host-a.service
[Service]
MemoryLow=1G
MemoryZSwapWriteback=no
# /etc/systemd/user/pi-host-a.service.d/10-more.conf
[Service]
MemoryLow=
MemoryLow=10%
";
        let own = own(text, 100 * GIB);
        assert_eq!(own.floor, 10 * GIB, "the last floor said is the one that holds");
        assert!(!own.writeback, "writeback turned off stays off");
        assert_eq!(
            super::own("[Service]\nMemoryLow=\n", GIB).floor,
            0,
            "an empty floor is systemd's default"
        );
    }

    #[test]
    fn sizes_read_as_systemd_reads_them() {
        assert_eq!(size("512M", 0), Some(512 << 20), "M is mebibytes");
        assert_eq!(size("1G", 0), Some(GIB), "G is gibibytes");
        assert_eq!(size("4096", 0), Some(4096), "no suffix is bytes");
        assert_eq!(size("infinity", 0), Some(u64::MAX), "infinity is no limit");
        assert_eq!(size("25%", 8 * GIB), Some(2 * GIB), "a percentage is of memory");
        assert_eq!(size("lots", 0), None, "words aren't a size");
    }

    #[test]
    fn the_host_is_the_one_process_whose_parent_is_outside() {
        assert_eq!(host_of(&[(10, 1)]), Some(10), "a host alone is its unit's host");
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
    }

    #[test]
    fn a_host_counts_each_orchestrator_it_holds() {
        let boot = "b00t";
        let session = |address: &str, by: Option<&str>, pid: u32| keep::Presence {
            addresses: vec![address.to_owned()],
            orchestrated_by: by.map(str::to_owned),
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
            hosts.own("u", 1, start + Duration::from_secs(1), || read(Own::default())),
            attended(),
            "within a minute the same host's settings aren't read again"
        );
        assert_eq!(
            hosts.own("u", 2, start + Duration::from_secs(2), || read(Own::default())),
            Own::default(),
            "a host started again reads its unit again"
        );
        assert_eq!(
            hosts.own("u", 2, start + OWN_FOR + Duration::from_secs(3), || None),
            Own::default(),
            "a failed read keeps what was read before"
        );
        assert_eq!(reads, 2, "only a new host was read again");
    }

    #[test]
    fn a_rank_is_set_once_and_again_once_it_changes() {
        let mut hosts = Hosts::default();
        let rank = wanted(&attended(), Signs::default(), 10);
        assert!(
            hosts.changed("u", &rank),
            "a unit this sorter never set is set, as one before it may have held it"
        );
        hosts.done("u", rank.clone());
        assert!(!hosts.changed("u", &rank), "a rank already set isn't asked for again");
        let held = wanted(&attended(), Signs { viewed: 1, ..Signs::default() }, 10);
        assert!(hosts.changed("u", &held), "a change is asked for");
        hosts.keep_only(&[]);
        assert!(hosts.changed("u", &rank), "a stopped host's unit is forgotten");
    }

    #[test]
    fn the_call_sets_all_four_properties() {
        let call = rank_call("pi-host-work-attended.service", &wanted(&attended(), Signs::default(), 10));
        let tail: Vec<&str> = call[9..].iter().map(String::as_str).collect();
        assert_eq!(
            tail,
            [
                "true",
                "4",
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
                "avoid"
            ],
            "the call gives back the unit's own floor and preference"
        );
    }
}

//! The day's read for `cadence day`: every figure the loop judges, each
//! parsed from the text its source writes, so the parsing is tested
//! without the machine. judge.rs holds a day against the ones before it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{builders, gate, health, machine, passed, phases};

mod switches;

pub use switches::switches;

/// One day's figures, kept as `day/<date>.json` in cadence's folder so the
/// next days are held against it.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Day {
    /// The local date the day ends on, as `2026-11-12`.
    pub date: String,
    /// The day's span, in seconds since the epoch.
    pub from: i64,
    pub to: i64,
    /// Each measure's figure, by its key in `measures()`.
    pub figures: BTreeMap<String, f64>,
    /// Each kind of thing that went wrong, by its key in `KINDS`, and how
    /// many times each one did: a program's crashes, a unit's failures, a
    /// check's flakes.
    pub named: BTreeMap<String, BTreeMap<String, u32>>,
}

/// A figure the loop reads each day, where more is always worse.
pub struct Measure {
    /// Its figure's key, or, with `*` in place of whose it is, each one's:
    /// `code.*.long-files` holds `code.core.long-files`, a repository's,
    /// and `launcher.keys.late.on.*` holds a screen's, as `DP-4@60Hz`.
    pub key: &'static str,
    /// Which of the loop's axes it reads: resources, time to done,
    /// quality, polish and the others the loop's research names.
    pub axis: &'static str,
    /// Where it's read from, so anyone can read it again.
    pub evidence: &'static str,
    /// What it is, as the note says it: "{says} went from 3 to 5".
    pub says: &'static str,
    /// What it's counted in, after the number: "%", " min", "".
    pub unit: &'static str,
    /// Past this it's a finding whatever the days before were.
    pub bar: Option<f64>,
    /// A move smaller than this is no move, however large a fifth of it.
    pub floor: f64,
}

const LAUNCHER: &str = "journalctl --user -u quickshell over the day, its launcher: lines";
const KILLS: &str = "journalctl -k --grep=oom-kill: over the day";
const STALLS: &str = "lag-watch's day files, each slice's mean cpu_some and memory_full";
const UPDATES: &str = "journalctl -u update-to-main.service over the day";

/// The launcher's share of keys past the bar on each screen, or pair of
/// screens, by `speed::launcher`'s `DP-4@60Hz+eDP-1@120Hz`.
const SCREENS: &str = "launcher.keys.late.on.*";

/// Screens as a note says them: `DP-4@60Hz+eDP-1` as "DP-4 at 60 Hz and
/// eDP-1".
fn on(screens: &str) -> String {
    screens
        .split('+')
        .map(|screen| match screen.split_once('@') {
            Some((output, hertz)) => {
                format!("{output} at {} Hz", hertz.trim_end_matches("Hz"))
            }
            None => screen.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" and ")
}

/// How much worse than its median a figure must be to be a regression.
pub const WORSE: f64 = 1.2;

impl Measure {
    /// Each of a day's figures this measure holds, by its key.
    pub fn read<'a>(&self, figures: &'a BTreeMap<String, f64>) -> Vec<(&'a str, f64)> {
        figures
            .iter()
            .filter(|(key, _)| self.holds(key))
            .map(|(key, now)| (key.as_str(), *now))
            .collect()
    }

    /// Whether this measure holds a figure's key.
    pub fn holds(&self, key: &str) -> bool {
        if self.key.contains('*') {
            self.whose(key).is_some()
        } else {
            key == self.key
        }
    }

    /// What a figure of this measure is, as a note says it.
    #[must_use]
    pub fn said(&self, key: &str) -> String {
        match self.whose(key) {
            None => self.says.to_owned(),
            // A disk is whose by its mount, which reads as where, not whose.
            Some(mount) if mount.starts_with('/') => format!("{} at {mount}", self.says),
            Some(screens) if self.key == SCREENS => format!("{} on {}", self.says, on(screens)),
            Some(whose) => format!("{whose}'s {}", self.says),
        }
    }

    /// Whose a figure is, when this measure is each repository's.
    fn whose<'k>(&self, key: &'k str) -> Option<&'k str> {
        let (before, after) = self.key.split_once('*')?;
        let whose = key.strip_prefix(before)?.strip_suffix(after)?;
        (!whose.is_empty()).then_some(whose)
    }

    /// How many times its median a figure must be to be a regression: a
    /// fifth over it, except for a repository's counts, of what it let in,
    /// which move only when someone adds or takes one away, so any rise
    /// past their floor is one. A repository's times, which drift day to
    /// day, take the fifth like any other time.
    pub fn worse(&self) -> f64 {
        if self.key.contains('*') && self.unit.is_empty() {
            1.0
        } else {
            WORSE
        }
    }
}

/// Every figure the day reads: the delivery cycle's here, the machine's
/// own from health in machine.rs, the builders' in builders.rs, the merge
/// gate's in gate.rs, how long gates took in passed.rs and where that
/// time went in phases.rs, and each repository's from code-health in
/// health.rs.
pub fn measures() -> impl Iterator<Item = &'static Measure> {
    MEASURES
        .iter()
        .chain(machine::MEASURES)
        .chain(builders::MEASURES)
        .chain(gate::MEASURES)
        .chain(passed::MEASURES)
        .chain(phases::MEASURES)
        .chain(health::MEASURES)
}

/// The delivery cycle's figures. The bars are docs/delivery.md's targets.
const MEASURES: &[Measure] = &[
    Measure {
        key: "oom.machine",
        axis: "resources",
        evidence: KILLS,
        says: "machine-wide out-of-memory kills",
        unit: "",
        bar: Some(0.0),
        floor: 1.0,
    },
    Measure {
        key: "oom.cgroup",
        axis: "resources",
        evidence: KILLS,
        says: "kills inside a slice's own memory limit",
        unit: "",
        bar: None,
        floor: 3.0,
    },
    Measure {
        key: "pressure.cpu.jobs",
        axis: "resources",
        evidence: STALLS,
        says: "the agents' jobs' CPU pressure",
        unit: "%",
        bar: Some(15.0),
        floor: 2.0,
    },
    Measure {
        key: "pressure.cpu.app",
        axis: "responsiveness",
        evidence: STALLS,
        says: "CPU pressure on Joel's apps",
        unit: "%",
        bar: Some(1.0),
        floor: 1.0,
    },
    Measure {
        key: "pressure.cpu.session",
        axis: "responsiveness",
        evidence: STALLS,
        says: "CPU pressure on Joel's session",
        unit: "%",
        bar: Some(1.0),
        floor: 1.0,
    },
    Measure {
        key: "pressure.memory.app",
        axis: "responsiveness",
        evidence: STALLS,
        says: "memory stalls of Joel's apps",
        unit: "%",
        bar: Some(1.0),
        floor: 0.2,
    },
    Measure {
        key: "pressure.memory.session",
        axis: "responsiveness",
        evidence: STALLS,
        says: "memory stalls of Joel's session",
        unit: "%",
        bar: Some(1.0),
        floor: 0.2,
    },
    Measure {
        key: "switch.minutes.median",
        axis: "time to done",
        evidence: UPDATES,
        says: "a switch's median length",
        unit: " min",
        bar: None,
        floor: 3.0,
    },
    Measure {
        key: "switch.minutes.longest",
        axis: "time to done",
        evidence: UPDATES,
        says: "the longest switch",
        unit: " min",
        bar: Some(60.0),
        floor: 10.0,
    },
    // The launcher's, against docs/design.md's Latency Budgets: 50 ms
    // from a key or a chord to its frame, and 30 ms from a chord to the
    // surface having the keys. They're read on responsiveness, where
    // `cadence axes` reads the launcher, so a finding lists on the axis
    // its reading does.
    Measure {
        key: "launcher.opening.presented",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "the launcher's median opening, chord to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 10.0,
    },
    Measure {
        key: "launcher.opening.usable",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "the launcher's median opening, chord to answering keys",
        unit: " ms",
        bar: Some(30.0),
        floor: 30.0,
    },
    Measure {
        key: "launcher.keys.presented",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "a key's median time in the launcher, key to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 5.0,
    },
    Measure {
        key: "launcher.keys.late",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "the launcher's keys past 50 ms",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    // Each screen's share apart, as `DP-4@60Hz`, so a day at the monitor is
    // judged against days at the monitor, its frames twice the panel's.
    Measure {
        key: SCREENS,
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "the launcher's keys past 50 ms",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    Measure {
        key: "launcher.changes.presented",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "a daemon's change's median time in the launcher, line to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 5.0,
    },
    Measure {
        key: "launcher.drawers.presented",
        axis: "responsiveness",
        evidence: LAUNCHER,
        says: "a row's details' median time in the launcher, key to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 5.0,
    },
    // Only the pieces that aren't light: update-to-main leaves the light
    // ones, a written file or a folder of links, to the laptop by design,
    // and builds the rest here only when the builders can't.
    Measure {
        key: "switch.pieces.heavy",
        axis: "resources",
        evidence: UPDATES,
        says: "pieces that aren't light a switch built on the laptop",
        unit: "",
        bar: Some(0.0),
        floor: 1.0,
    },
];

/// A kind of thing that goes wrong by name, judged by whether a name is
/// new: one not seen in the days before.
pub struct Kind {
    pub key: &'static str,
    pub axis: &'static str,
    pub evidence: &'static str,
    /// "{says} {name}, N times".
    pub says: &'static str,
    /// For a kind whose names come back every day, the least move in a
    /// name's count that's a move: past it and a fifth over its median,
    /// a name seen before is worse too. None judges only new names.
    pub floor: Option<f64>,
}

pub const KINDS: &[Kind] = &[
    Kind {
        key: "crashes",
        axis: "quality",
        evidence: "coredumpctl list over the day, by program",
        says: "a crash of",
        floor: None,
    },
    Kind {
        key: "failed",
        axis: "quality",
        evidence: "journalctl MESSAGE_ID=d9b373ed55a64feb8242e02dbe79a49c over the day, by unit",
        says: "a failure of",
        floor: None,
    },
    // A service that says no ready line has its restarts timed only to
    // its start, so its owner is told once, as it's new, to add one.
    Kind {
        key: "unstamped",
        axis: "polish",
        evidence: "health's ~/.local/state/health/<date>.json, its unstamped",
        says: "restarts with no ready line from",
        floor: None,
    },
    Kind {
        key: "flaky",
        axis: "quality",
        evidence: "nix-offload's there.jsonl, a check failed and passed at one commit",
        says: "a flake in",
        floor: None,
    },
    Kind {
        key: "code.queued-on",
        axis: "time to done",
        evidence: "merge-net's answers.jsonl, the files each hold and run again turned on",
        // Its names are a cause and a zone, as `held:modules/repos`: held
        // back behind another's gate, or sent to run again as main moved.
        says: "changes queued on",
        // The launcher's pytest file held 11 changes back by 09:50 on 12
        // November, as it had most days, so only a new name was ever said.
        floor: Some(3.0),
    },
];

/// The axis a finding's key reads and where it's read from: its
/// measure's, or for `crashes:biome` its kind's.
#[must_use]
pub fn about(key: &str) -> Option<(&'static str, &'static str)> {
    let kind = key.split_once(':').map_or(key, |(kind, _)| kind);
    measures()
        .find(|m| m.holds(key))
        .map(|m| (m.axis, m.evidence))
        .or_else(|| {
            KINDS
                .iter()
                .find(|k| k.key == kind)
                .map(|k| (k.axis, k.evidence))
        })
}

/// Each program that dumped core, by the name of its file, from
/// `coredumpctl --json=short list`.
#[must_use]
pub fn crashes(json: &str) -> BTreeMap<String, u32> {
    #[derive(Deserialize)]
    struct Dump {
        exe: Option<String>,
    }
    let dumps: Vec<Dump> = serde_json::from_str(json).unwrap_or_default();
    let mut named = BTreeMap::new();
    for dump in dumps {
        let Some(exe) = dump.exe else { continue };
        let name = exe.rsplit('/').next().unwrap_or(&exe).to_owned();
        *named.entry(name).or_insert(0) += 1;
    }
    named
}

/// Each unit that failed, from `journalctl -o json` of systemd's message
/// for a unit that failed, a system's and the account's alike. A part of
/// its name that's only digits, a scope's pid, is put as `*`, so the
/// agents' many work scopes are one name.
#[must_use]
pub fn failed_units(json_lines: &str) -> BTreeMap<String, u32> {
    #[derive(Deserialize)]
    struct Entry {
        #[serde(rename = "UNIT")]
        unit: Option<String>,
        #[serde(rename = "USER_UNIT")]
        user_unit: Option<String>,
    }
    let mut named = BTreeMap::new();
    for line in json_lines.lines() {
        let Ok(entry) = serde_json::from_str::<Entry>(line) else {
            continue;
        };
        let Some(unit) = entry.user_unit.or(entry.unit) else {
            continue;
        };
        *named.entry(without_pids(&unit)).or_insert(0) += 1;
    }
    named
}

fn without_pids(unit: &str) -> String {
    let mut name = String::new();
    let mut part = String::new();
    for c in unit.chars().chain(std::iter::once('\0')) {
        if c == '-' || c == '.' || c == '@' || c == '\0' {
            let digits = !part.is_empty() && part.chars().all(|d| d.is_ascii_digit());
            name.push_str(if digits { "*" } else { &part });
            if c != '\0' {
                name.push(c);
            }
            part.clear();
        } else {
            part.push(c);
        }
    }
    name
}

/// The kernel's out-of-memory kills, from `journalctl -k -o short-unix
/// --grep=oom-kill:`: machine-wide ones, and ones inside a cgroup's own
/// limit.
#[must_use]
pub fn oom_kills(kernel: &str) -> (u32, u32) {
    let kills = kernel.lines().filter(|line| line.contains("oom-kill:"));
    kills.fold((0, 0), |(machine, cgroup), line| {
        if line.contains("CONSTRAINT_MEMCG") {
            (machine, cgroup + 1)
        } else {
            (machine + 1, cgroup)
        }
    })
}

/// The mean of each slice's stalls over lag-watch's records that ended
/// between `from` and `to`, from its day files' lines, keyed as
/// `pressure.cpu.app`: its `cpu_some` and `memory_full`, in percent.
#[must_use]
pub fn pressure(lines: &str, from: i64, to: i64) -> BTreeMap<String, f64> {
    #[derive(Deserialize)]
    struct Stall {
        cpu_some: f64,
        memory_full: f64,
    }
    #[derive(Deserialize)]
    struct Record {
        ended: i64,
        stalls: BTreeMap<String, Stall>,
    }
    let mut sums: BTreeMap<String, (f64, u32)> = BTreeMap::new();
    for line in lines.lines() {
        let Ok(record) = serde_json::from_str::<Record>(line) else {
            continue;
        };
        if record.ended <= from || record.ended > to {
            continue;
        }
        for (slice, stall) in record.stalls {
            for (kind, value) in [("cpu", stall.cpu_some), ("memory", stall.memory_full)] {
                let sum = sums.entry(format!("pressure.{kind}.{slice}")).or_default();
                sum.0 += value;
                sum.1 += 1;
            }
        }
    }
    sums.into_iter()
        .map(|(key, (sum, n))| (key, sum / f64::from(n)))
        .collect()
}

/// The middle of `values`, or nothing when there are none.
#[must_use]
pub fn median(values: &[f64]) -> Option<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let n = sorted.len();
    match n {
        0 => None,
        _ if n % 2 == 1 => Some(sorted[n / 2]),
        _ => Some(f64::midpoint(sorted[n / 2 - 1], sorted[n / 2])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_repositorys_figure_is_about_its_measures_axis() {
        assert_eq!(
            about("code.core.nix.long-files"),
            Some(("organization", health::HEALTH)),
            "a repository's figure didn't find the measure that holds it"
        );
        assert_eq!(
            about("code..long-files"),
            None,
            "a figure naming no repository was taken for one"
        );
    }

    #[test]
    fn a_figure_is_said_by_what_it_is_of() {
        // On 17 November a screen's finding read "DP-4@60Hz's the launcher's
        // keys" in the week's review, and a pair of screens had no words.
        let said = |key: &str| measures().find(|m| m.holds(key)).map(|m| m.said(key));
        let keys = "the launcher's keys past 50 ms";
        assert_eq!(
            [
                said("launcher.keys.late.on.DP-4@60Hz"),
                said("launcher.keys.late.on.DP-4@60Hz+eDP-1@120Hz"),
                said("launcher.keys.late.on.DP-4"),
                said("health.disk./var.most-used-percent"),
                said("code.core.long-files"),
            ],
            [
                Some(format!("{keys} on DP-4 at 60 Hz")),
                Some(format!("{keys} on DP-4 at 60 Hz and eDP-1 at 120 Hz")),
                Some(format!("{keys} on DP-4")),
                Some("the most in use of the disk at /var".to_owned()),
                Some("core's long files that don't say why".to_owned()),
            ],
            "a screen, a disk or a repository wasn't said as what its figure is of"
        );
    }

    #[test]
    fn the_launchers_figures_list_where_axes_reads_the_launcher() {
        // On 12 November a drawer's 66 ms was filed on polish, while
        // `cadence axes` reads the launcher on responsiveness.
        for key in [
            "launcher.opening.presented",
            "launcher.opening.usable",
            "launcher.keys.presented",
            "launcher.keys.late",
            "launcher.keys.late.on.DP-4@60Hz",
            "launcher.changes.presented",
            "launcher.drawers.presented",
        ] {
            assert_eq!(
                about(key),
                Some(("responsiveness", LAUNCHER)),
                "{key} isn't read where the launcher is"
            );
        }
        let usable = measures().find(|m| m.key == "launcher.opening.usable");
        assert_eq!(
            usable.and_then(|m| m.bar),
            Some(30.0),
            "an opening isn't held to design.md's 30 ms from a chord to the keys"
        );
    }

    #[test]
    fn crashes_are_counted_by_program() {
        let json = r#"[{"time":1,"exe":"/nix/store/x-tec/bin/tec"},
            {"time":2,"exe":"/a/b/biome"},{"time":3,"exe":"/c/biome"},{"time":4}]"#;
        let named = crashes(json);
        assert_eq!(
            named.get("biome"),
            Some(&2),
            "two dumps of biome from two places weren't one program's: {named:?}"
        );
        assert_eq!(
            named.len(),
            2,
            "a dump with no program was counted: {named:?}"
        );
    }

    #[test]
    fn a_scopes_pid_is_left_out_of_its_name() {
        let lines = concat!(
            r#"{"USER_UNIT":"work-3631643-934417.scope","_UID":"1000"}"#,
            "\n",
            r#"{"USER_UNIT":"work-1137263-1378235.scope"}"#,
            "\n",
            r#"{"UNIT":"idle-sleep-follows-the-charger.service"}"#,
            "\n",
            r#"{"USER_UNIT":"zone-check-2af2dh.service"}"#,
            "\nnot json\n"
        );
        let named = failed_units(lines);
        assert_eq!(
            named.get("work-*-*.scope"),
            Some(&2),
            "two work scopes weren't one name: {named:?}"
        );
        assert!(
            named.contains_key("zone-check-2af2dh.service"),
            "a name with digits among its letters lost them: {named:?}"
        );
        assert_eq!(
            named.len(),
            3,
            "a line that isn't a unit was counted: {named:?}"
        );
    }

    #[test]
    fn a_kill_inside_a_cgroups_limit_isnt_machine_wide() {
        let kernel = "1.0 laptop kernel: oom-kill:constraint=CONSTRAINT_NONE,task=pi\n\
                      2.0 laptop kernel: oom-kill:constraint=CONSTRAINT_MEMCG,task=cc\n\
                      3.0 laptop kernel: oom-kill:constraint=CONSTRAINT_MEMCG,task=ld\n";
        assert_eq!(
            oom_kills(kernel),
            (1, 2),
            "a kill wasn't told machine-wide or in a cgroup"
        );
    }

    #[test]
    fn pressure_is_the_mean_of_the_days_records() {
        let record = |ended: i64, cpu: f64| {
            format!(
                r#"{{"ended":{ended},"stalls":{{"app":{{"cpu_some":{cpu},"cpu_full":0,
                "memory_some":0,"memory_full":0.5,"io_some":0,"io_full":0}}}}}}"#
            )
            .replace('\n', "")
        };
        let lines = [
            record(5, 9.0),
            record(15, 1.0),
            record(25, 3.0),
            record(40, 7.0),
        ]
        .join("\n");
        let read = pressure(&lines, 10, 30);
        assert_eq!(
            read.get("pressure.cpu.app"),
            Some(&2.0),
            "records outside the day were counted: {read:?}"
        );
        assert_eq!(
            read.get("pressure.memory.app"),
            Some(&0.5),
            "memory stalls weren't read beside CPU's: {read:?}"
        );
    }

    #[test]
    fn the_median_of_an_even_count_is_between_its_middle_two() {
        assert_eq!(
            median(&[4.0, 1.0, 3.0, 2.0]),
            Some(2.5),
            "the median was wrong"
        );
        assert_eq!(median(&[]), None, "no values had a median");
    }
}

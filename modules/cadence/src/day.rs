//! The day's read for `cadence day`: every figure the loop judges, each
//! parsed from the text its source writes, so the parsing is tested
//! without the machine. judge.rs holds a day against the ones before it.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// One day's figures, kept as `day/<date>.json` in cadence's folder so the
/// next days are held against it.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Day {
    /// The local date the day ends on, as `2026-11-12`.
    pub date: String,
    /// The day's span, in seconds since the epoch.
    pub from: i64,
    pub to: i64,
    /// Each measure's figure, by its key in `MEASURES`.
    pub figures: BTreeMap<String, f64>,
    /// Each kind of thing that went wrong, by its key in `KINDS`, and how
    /// many times each one did: a program's crashes, a unit's failures.
    pub named: BTreeMap<String, BTreeMap<String, u32>>,
}

/// A figure the loop reads each day, where more is always worse.
pub struct Measure {
    /// Its figure's key, or, with `*` in place of the repository, each
    /// repository's: `code.*.long-files` holds `code.core.long-files`.
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
const HEALTH: &str = "code-health's ~/.local/state/code-health/<date>.json and its -named.json";

/// How much worse than its median a figure must be to be a regression.
const WORSE: f64 = 1.2;

impl Measure {
    /// Each of a day's figures this measure holds, by its key, with whose
    /// it is when the measure is each repository's.
    pub fn read<'a>(
        &self,
        figures: &'a BTreeMap<String, f64>,
    ) -> Vec<(&'a str, Option<&'a str>, f64)> {
        figures
            .iter()
            .filter(|(key, _)| self.holds(key))
            .map(|(key, now)| (key.as_str(), self.whose(key), *now))
            .collect()
    }

    /// Whether this measure holds a figure's key.
    fn holds(&self, key: &str) -> bool {
        if self.key.contains('*') {
            self.whose(key).is_some()
        } else {
            key == self.key
        }
    }

    /// Whose a figure is, when this measure is each repository's.
    fn whose<'k>(&self, key: &'k str) -> Option<&'k str> {
        let (before, after) = self.key.split_once('*')?;
        let whose = key.strip_prefix(before)?.strip_suffix(after)?;
        (!whose.is_empty()).then_some(whose)
    }

    /// How many times its median a figure must be to be a regression: a
    /// fifth over it, except for a repository's figures, counts of what it
    /// let in that move only when someone adds or takes one away, so any
    /// rise past their floor is one.
    pub fn worse(&self) -> f64 {
        if self.key.contains('*') { 1.0 } else { WORSE }
    }
}

/// Every figure the day reads. The bars are docs/delivery.md's targets.
pub const MEASURES: &[Measure] = &[
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
        axis: "polish",
        evidence: STALLS,
        says: "CPU pressure on Joel's apps",
        unit: "%",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "pressure.cpu.session",
        axis: "polish",
        evidence: STALLS,
        says: "CPU pressure on Joel's session",
        unit: "%",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "pressure.memory.app",
        axis: "polish",
        evidence: STALLS,
        says: "memory stalls of Joel's apps",
        unit: "%",
        bar: Some(1.0),
        floor: 0.2,
    },
    Measure {
        key: "pressure.memory.session",
        axis: "polish",
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
    // The launcher's, against docs/design.md's 50 ms from a key or a
    // chord to its frame.
    Measure {
        key: "launcher.opening.presented",
        axis: "polish",
        evidence: LAUNCHER,
        says: "the launcher's median opening, chord to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 10.0,
    },
    Measure {
        key: "launcher.opening.usable",
        axis: "polish",
        evidence: LAUNCHER,
        says: "the launcher's median opening, chord to answering keys",
        unit: " ms",
        bar: None,
        floor: 30.0,
    },
    Measure {
        key: "launcher.keys.presented",
        axis: "polish",
        evidence: LAUNCHER,
        says: "a key's median time in the launcher, key to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 5.0,
    },
    Measure {
        key: "launcher.keys.late",
        axis: "polish",
        evidence: LAUNCHER,
        says: "the launcher's keys past 50 ms",
        unit: "%",
        bar: None,
        floor: 5.0,
    },
    Measure {
        key: "launcher.changes.presented",
        axis: "polish",
        evidence: LAUNCHER,
        says: "a daemon's change's median time in the launcher, line to frame",
        unit: " ms",
        bar: Some(50.0),
        floor: 5.0,
    },
    Measure {
        key: "launcher.drawers.presented",
        axis: "polish",
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
    // Each repository's, from code-health's figures (health.rs).
    Measure {
        key: "code.*.long-files",
        axis: "organization",
        evidence: HEALTH,
        says: "long files that don't say why",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.crowded-folders",
        axis: "organization",
        evidence: HEALTH,
        says: "crowded folders that don't say why",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.suppressions",
        axis: "quality",
        evidence: HEALTH,
        says: "lint suppressions",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.duplicated-lines",
        axis: "quality",
        evidence: HEALTH,
        says: "lines repeated from elsewhere",
        unit: "",
        bar: None,
        floor: 20.0,
    },
    Measure {
        key: "code.*.quarantined-tests",
        axis: "quality",
        evidence: HEALTH,
        says: "tests in quarantine",
        unit: "",
        bar: None,
        floor: 1.0,
    },
    Measure {
        key: "code.*.daemons-waiting",
        axis: "organization",
        evidence: HEALTH,
        says: "points of docs/daemons.md daemons still break",
        unit: "",
        bar: None,
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
}

pub const KINDS: &[Kind] = &[
    Kind {
        key: "crashes",
        axis: "quality",
        evidence: "coredumpctl list over the day, by program",
        says: "a crash of",
    },
    Kind {
        key: "failed",
        axis: "quality",
        evidence: "journalctl MESSAGE_ID=d9b373ed55a64feb8242e02dbe79a49c over the day, by unit",
        says: "a failure of",
    },
];

/// The axis a finding's key reads and where it's read from: its
/// measure's, or for `crashes:biome` its kind's.
#[must_use]
pub fn about(key: &str) -> Option<(&'static str, &'static str)> {
    let kind = key.split_once(':').map_or(key, |(kind, _)| kind);
    MEASURES
        .iter()
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

/// The day's switches, from `journalctl -u update-to-main.service -o
/// short-unix` up to `to`.
#[derive(Debug, PartialEq)]
pub struct Switches {
    /// Each switch's length in minutes, from the first ask after the last
    /// update ended to the step that switched.
    pub lengths: Vec<f64>,
    /// How long the update still building at `to` has run, in minutes: on
    /// 11 November one built the kernel on the laptop from 16:42 past
    /// 19:00, and a day read from finished switches alone missed it.
    pub under_way: Option<f64>,
    /// The pieces the day's updates built on the laptop.
    pub here: u32,
    /// Of them, those that aren't light: the ones meant for the builders
    /// that an update built here once the builders didn't.
    pub heavy: u32,
}

#[must_use]
pub fn switches(journal: &str, to: f64) -> Switches {
    #[derive(Deserialize)]
    struct Said {
        step: Option<String>,
    }
    let mut lengths = Vec::new();
    let mut here = 0;
    let mut heavy = 0;
    // The pieces the update building now sent to the builders.
    let mut away = 0;
    let mut asked: Option<f64> = None;
    // Whether the update asked for got as far as building, so an ask the
    // journal says no more of isn't taken for one under way.
    let mut building = false;
    for line in journal.lines() {
        let Some((stamp, rest)) = line.split_once(' ') else {
            continue;
        };
        let Ok(at) = stamp.parse::<f64>() else {
            continue;
        };
        if rest.contains("asks for an update") {
            asked.get_or_insert(at);
        } else if let Some(needs) = rest.split_once("the system needs ") {
            let (light, sent) = pieces_here(needs.1).unwrap_or_default();
            here += light;
            away = sent;
            building = asked.is_some();
        } else if let Some((_, again)) = rest.split_once("so they go where the first were sent: ") {
            // The second listing, just before the build here, sends again
            // what went missing since the first.
            away = sent_again(again).unwrap_or(away);
        } else if rest.contains(", so they build here") {
            // update-to-main's fallback: what the builders didn't build
            // builds here, light or not.
            here += away;
            heavy += away;
            away = 0;
        } else if let Some(brace) = rest.find('{')
            && let Ok(said) = serde_json::from_str::<Said>(&rest[brace..])
        {
            match said.step.as_deref() {
                Some("done") => {
                    if let Some(asked) = asked.take() {
                        lengths.push((at - asked) / 60.0);
                    }
                    building = false;
                }
                Some("unchanged" | "refused" | "failed") => {
                    asked = None;
                    building = false;
                }
                _ => {}
            }
        }
    }
    let under_way = asked.filter(|_| building).map(|asked| (to - asked) / 60.0);
    Switches {
        lengths,
        under_way,
        here,
        heavy,
    }
}

/// From "N pieces built: K kept apart, R on the builders, the rest here",
/// N less K and R, built here, and K and R, sent away.
fn pieces_here(said: &str) -> Option<(u32, u32)> {
    let number = |text: &str| text.trim().split(' ').next()?.parse::<u32>().ok();
    let (all, rest) = said.split_once(" pieces built: ")?;
    let (kept, rest) = rest.split_once(" kept apart, ")?;
    let (away, _) = rest.split_once(" on the builders")?;
    let sent = number(kept)? + number(away)?;
    Some((number(all)?.saturating_sub(sent), sent))
}

/// From "K kept apart, R on the builders", K and R.
fn sent_again(said: &str) -> Option<u32> {
    let number = |text: &str| text.trim().parse::<u32>().ok();
    let (kept, rest) = said.split_once(" kept apart, ")?;
    let (away, _) = rest.split_once(" on the builders")?;
    Some(number(kept)? + number(away)?)
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
            Some(("organization", HEALTH)),
            "a repository's figure didn't find the measure that holds it"
        );
        assert_eq!(
            about("code..long-files"),
            None,
            "a figure naming no repository was taken for one"
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
    fn a_switch_runs_from_its_first_ask_to_its_switch() {
        let journal = "\
100.0 laptop u[1]: uid 1000 asks for an update
160.0 laptop u[1]: uid 1000 asks for an update
200.0 laptop u[1]: the system needs 15 pieces built: 2 kept apart, 3 on the builders, the rest here
700.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"c\"}
800.0 laptop u[1]: uid 1000 asks for an update
900.0 laptop u[1]: {\"step\":\"refused\",\"why\":\"a call\"}
1000.0 laptop u[1]: uid 1000 asks for an update
1300.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"d\"}
";
        let read = switches(journal, 2000.0);
        assert_eq!(
            read.lengths,
            vec![10.0, 5.0],
            "a refused ask was counted into the next switch, or a later ask restarted one"
        );
        assert_eq!(
            read.here, 10,
            "the pieces built here weren't 15 less 2 and 3"
        );
        assert_eq!(
            read.under_way, None,
            "a day of finished switches had one under way"
        );
        assert_eq!(
            read.heavy, 0,
            "pieces built on the builders were heavy here"
        );
    }

    #[test]
    fn what_the_builders_didnt_build_and_the_laptop_did_is_heavy() {
        let journal = "\
100.0 laptop u[1]: uid 1000 asks for an update
110.0 laptop u[1]: the system needs 15 pieces built: 2 kept apart, 3 on the builders, the rest here
120.0 laptop u[1]: building the system on the machine's builders failed, so they build here, after:
700.0 laptop u[1]: {\"step\":\"done\",\"commit\":\"c\"}
";
        let read = switches(journal, 2000.0);
        assert_eq!(
            (read.here, read.heavy),
            (15, 5),
            "the pieces the builders didn't build weren't counted heavy here"
        );
        let again = "\
100.0 laptop u[1]: uid 1000 asks for an update
110.0 laptop u[1]: the system needs 15 pieces built: 0 kept apart, 3 on the builders, the rest here
130.0 laptop u[1]: before the build here, 1 pieces that aren't light are still to build, so they \
go where the first were sent: 0 kept apart, 1 on the builders
140.0 laptop u[1]: building the system on the machine's builders failed, so they build here, after:
";
        assert_eq!(
            switches(again, 2000.0).heavy,
            1,
            "what the second listing sent and the laptop built wasn't the heavy count"
        );
    }

    #[test]
    fn an_update_still_building_is_under_way_and_an_ask_alone_isnt() {
        let building = "\
100.0 laptop u[1]: uid 1000 asks for an update
160.0 laptop u[1]: the system needs 1 pieces built: 0 kept apart, 0 on the builders, the rest here
";
        assert_eq!(
            switches(building, 700.0).under_way,
            Some(10.0),
            "an update building at the day's end wasn't under way from its ask"
        );
        let asked = "100.0 laptop u[1]: uid 1000 asks for an update\n";
        assert_eq!(
            switches(asked, 700.0).under_way,
            None,
            "an ask the journal said no more of was taken for an update under way"
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

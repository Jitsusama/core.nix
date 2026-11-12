//! What kept the processors busy: each group of processes' processor time,
//! read from its cgroup every five minutes and kept by the hour and by
//! whose it was, so an hour's pressure can be laid at the door of the
//! quest whose work it was. The pressure counters say only how long
//! programs waited, and the slices' own counters only since each began.
//!
//! A pi session and every command it runs are named for the quest its
//! presence says it works on, `OL0ST3 pi` and `OL0ST3 jobs`; any other
//! group of the account's for its slice or service, and the machine's for
//! its service. A group that ends between looks loses what it used since
//! the last, as does the day's first five minutes, which the day before
//! has no look in.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// How often the groups are read, in microseconds.
pub const EVERY: i64 = 300_000_000;

/// The account's slices whose groups are each named for a quest: pi's
/// sessions, and the commands they run.
const BY_QUEST: [(&str, &str); 2] = [("agents.slice", "pi"), ("jobs.slice", "jobs")];

/// How deep beneath a slice its groups are looked for.
const DEEPEST: usize = 4;

/// How many are named an hour.
const SHOWN: usize = 3;

/// The processor time each group used, kept by the hour.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Busy {
    /// When the groups were last read, in microseconds since the epoch.
    pub looked: i64,
    /// Each group's processor time at that look, in microseconds, by its
    /// cgroup, so the next look counts only what came since.
    pub last: BTreeMap<String, u64>,
    /// The processor milliseconds used, by the hour of the look that
    /// counted them, as `14`, then by whose they were.
    pub hours: BTreeMap<String, BTreeMap<String, u64>>,
}

/// One group as a look found it: whose it is, and its processor time so
/// far, in microseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub whose: String,
    pub used: u64,
}

impl Busy {
    /// Whether it's time to look again at `at`.
    #[must_use]
    pub fn due(&self, at: i64) -> bool {
        at - self.looked >= EVERY
    }

    /// Count what each group used since the last look into `hour`. A
    /// group first seen counts from now, and one whose time went back
    /// began again under the same name, so neither is counted.
    pub fn saw(&mut self, at: i64, hour: &str, groups: BTreeMap<String, Group>) {
        for (path, group) in &groups {
            let Some(before) = self.last.get(path) else {
                continue;
            };
            let since = group.used.saturating_sub(*before) / 1_000;
            if since > 0 {
                *self
                    .hours
                    .entry(hour.to_owned())
                    .or_default()
                    .entry(group.whose.clone())
                    .or_default() += since;
            }
        }
        self.last = groups
            .into_iter()
            .map(|(path, group)| (path, group.used))
            .collect();
        self.looked = at;
    }

    /// The hour's busiest, most first, in core-minutes.
    #[must_use]
    pub fn busiest(&self, hour: &str) -> Vec<(String, f64)> {
        let mut used: Vec<(&String, &u64)> = self.hours.get(hour).into_iter().flatten().collect();
        used.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        used.into_iter()
            .map(|(whose, ms)| (whose.clone(), minutes(*ms)))
            .collect()
    }

    /// Each hour's busiest three, an hour a line, with all it used.
    #[must_use]
    pub fn said(&self) -> String {
        let mut out = String::new();
        for (hour, used) in &self.hours {
            let all: u64 = used.values().sum();
            let named: Vec<String> = self
                .busiest(hour)
                .into_iter()
                .take(SHOWN)
                .map(|(whose, minutes)| format!("{whose} {minutes:.0}"))
                .collect();
            let _ = writeln!(
                out,
                "{hour}:00  {:.0} core-minutes; the most: {}",
                minutes(all),
                named.join(", ")
            );
        }
        out
    }
}

#[expect(
    clippy::cast_precision_loss,
    reason = "a day's processor milliseconds are far below where an f64 loses whole ones"
)]
fn minutes(ms: u64) -> f64 {
    ms as f64 / 60_000.0
}

/// Every group worth naming beneath `cgroups`: the account manager's at
/// `manager`, each of pi's and its commands' named for its quest from the
/// presences in `presences`, and each of the machine's services.
#[must_use]
pub fn look(
    cgroups: &Path,
    manager: &str,
    presences: &Path,
    proc: &Path,
) -> BTreeMap<String, Group> {
    let quests = Quests::read(presences);
    let mut groups = BTreeMap::new();
    let mut keep = |path: &Path, whose: String| {
        if let Some(used) = used(path) {
            let name = path.strip_prefix(cgroups).unwrap_or(path);
            groups.insert(name.to_string_lossy().into_owned(), Group { whose, used });
        }
    };
    for slice in children(&cgroups.join(manager)) {
        let name = file_name(&slice);
        match BY_QUEST.iter().find(|(by, _)| *by == name) {
            Some((_, part)) => {
                for leaf in leaves(&slice, DEEPEST) {
                    let whose = match quests.of(&leaf, proc) {
                        Some(quest) => format!("{quest} {part}"),
                        // A scope's name is only numbers; a service's
                        // says what it is.
                        None if leaf.extension().is_some_and(|kind| kind == "scope") => {
                            format!("no quest's {part}")
                        }
                        None => file_name(&leaf),
                    };
                    keep(&leaf, whose);
                }
            }
            None => keep(&slice, name),
        }
    }
    for service in children(&cgroups.join("system.slice")) {
        let name = file_name(&service);
        keep(&service, name);
    }
    groups
}

/// The account manager's group, beneath the cgroup root, from a process's
/// own `cgroup` file, when that process runs under it, as
/// `user.slice/user-1000.slice/user@1000.service`.
#[must_use]
pub fn manager_of(cgroup: &str) -> Option<String> {
    let path = cgroup.lines().find_map(|line| line.strip_prefix("0::/"))?;
    let end = path.find(".service")? + ".service".len();
    let manager = &path[..end];
    manager
        .rsplit('/')
        .next()
        .is_some_and(|last| last.starts_with("user@"))
        .then(|| manager.to_owned())
}

/// A group's processor time so far, in microseconds.
fn used(group: &Path) -> Option<u64> {
    fs::read_to_string(group.join("cpu.stat"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("usage_usec "))?
        .trim()
        .parse()
        .ok()
}

/// A group's own groups.
fn children(group: &Path) -> Vec<std::path::PathBuf> {
    let mut found: Vec<_> = fs::read_dir(group)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// The groups beneath `group` with none of their own, to `depth` down;
/// only these hold processes.
fn leaves(group: &Path, depth: usize) -> Vec<std::path::PathBuf> {
    let within = children(group);
    if within.is_empty() || depth == 0 {
        return vec![group.to_owned()];
    }
    within
        .iter()
        .flat_map(|child| leaves(child, depth - 1))
        .collect()
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Which quest each session works on, by its id and by its process.
struct Quests {
    by_session: BTreeMap<String, String>,
    by_pid: BTreeMap<String, String>,
}

impl Quests {
    /// The presences in `folder`, each a session's, named for it.
    fn read(folder: &Path) -> Self {
        let mut quests = Self {
            by_session: BTreeMap::new(),
            by_pid: BTreeMap::new(),
        };
        for entry in fs::read_dir(folder).into_iter().flatten().flatten() {
            let path = entry.path();
            let Some(said) = fs::read(&path)
                .ok()
                .and_then(|said| serde_json::from_slice::<Value>(&said).ok())
            else {
                continue;
            };
            let Some(quest) = said["quest"].as_str().map(short).filter(|q| !q.is_empty()) else {
                continue;
            };
            if let Some(pid) = said["owner"]["pid"].as_u64() {
                quests.by_pid.insert(pid.to_string(), quest.clone());
            }
            if let Some(session) = path.file_stem() {
                quests
                    .by_session
                    .insert(session.to_string_lossy().into_owned(), quest);
            }
        }
        quests
    }

    /// The quest a group's work is for: from its slice's name, as the
    /// commands' slices are named for theirs, else from a process in it
    /// that is a session's, or that a session started, as its
    /// environment says.
    fn of(&self, group: &Path, proc: &Path) -> Option<String> {
        let named = group.iter().find_map(|part| {
            let part = part.to_string_lossy();
            let rest = part
                .strip_prefix("jobs-work-qest")?
                .strip_suffix(".slice")?;
            // The quest's id after its date, as `20261107ol0st3`.
            rest.get(8..).map(str::to_uppercase)
        });
        if named.is_some() {
            return named;
        }
        let procs = fs::read_to_string(group.join("cgroup.procs")).unwrap_or_default();
        procs.split_whitespace().find_map(|pid| {
            if let Some(quest) = self.by_pid.get(pid) {
                return Some(quest.clone());
            }
            let environ = fs::read(proc.join(pid).join("environ")).ok()?;
            environ
                .split(|byte| *byte == 0)
                .find_map(|pair| pair.strip_prefix(b"PI_MESH_SESSION="))
                .and_then(|session| self.by_session.get(&*String::from_utf8_lossy(session)))
                .cloned()
        })
    }
}

/// A quest's short name, the last part of its id, as `QEST-20261107-OL0ST3`
/// is `OL0ST3`.
fn short(quest: &str) -> String {
    quest.rsplit('-').next().unwrap_or(quest).to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cgroup at `path` beneath `root`, holding `pids`, `used`
    /// microseconds so far.
    fn group(root: &Path, path: &str, used: u64, pids: &[&str]) {
        let at = root.join(path);
        fs::create_dir_all(&at).expect("a group's folder");
        fs::write(
            at.join("cpu.stat"),
            format!("usage_usec {used}\nuser_usec 1\n"),
        )
        .expect("a group's processor time");
        fs::write(at.join("cgroup.procs"), pids.join("\n")).expect("a group's processes");
    }

    #[test]
    fn each_group_is_named_for_its_quest_its_slice_or_its_service() {
        let root = tempfile::tempdir().expect("a folder");
        let (cgroups, proc, presences) = (
            root.path().join("cg"),
            root.path().join("proc"),
            root.path().join("presence"),
        );
        let manager = "user.slice/user-1000.slice/user@1000.service";
        let user = format!("{manager}/");
        group(
            &cgroups,
            &format!("{user}agents.slice/kitty-1-0.scope"),
            5,
            &["11", "12"],
        );
        group(
            &cgroups,
            &format!("{user}agents.slice/kitty-2-0.scope"),
            6,
            &["21"],
        );
        group(
            &cgroups,
            &format!("{user}jobs.slice/jobs-work.slice/jobs-work-qest20261107ol0st3.slice/a"),
            7,
            &[],
        );
        group(
            &cgroups,
            &format!("{user}jobs.slice/jobs-work.slice/work-9-1.scope"),
            8,
            &["31"],
        );
        group(&cgroups, &format!("{user}app.slice"), 9, &[]);
        group(
            &cgroups,
            &format!("{user}jobs.slice/jobs-timed.slice/keeps.service"),
            4,
            &[],
        );
        group(&cgroups, "system.slice/nix-daemon.service", 10, &[]);
        fs::create_dir_all(&presences).expect("the presences' folder");
        fs::write(
            presences.join("01a1.json"),
            r#"{"quest":"QEST-20261105-80ECO2","owner":{"pid":12}}"#,
        )
        .expect("a presence");
        fs::create_dir_all(proc.join("31")).expect("a process");
        fs::write(proc.join("31/environ"), b"HOME=/h\0PI_MESH_SESSION=01a1\0")
            .expect("its environment");
        let found: Vec<(String, String, u64)> = look(&cgroups, manager, &presences, &proc)
            .into_iter()
            .map(|(path, group)| (file_name(Path::new(&path)), group.whose, group.used))
            .collect();
        let expected = [
            ("kitty-1-0.scope", "80ECO2 pi", 5),
            ("kitty-2-0.scope", "no quest's pi", 6),
            ("app.slice", "app.slice", 9),
            ("keeps.service", "keeps.service", 4),
            ("a", "OL0ST3 jobs", 7),
            ("work-9-1.scope", "80ECO2 jobs", 8),
            ("nix-daemon.service", "nix-daemon.service", 10),
        ];
        let mut expected: Vec<(String, String, u64)> = expected
            .iter()
            .map(|(name, whose, used)| ((*name).to_owned(), (*whose).to_owned(), *used))
            .collect();
        let mut found = found;
        found.sort();
        expected.sort();
        assert_eq!(
            found, expected,
            "a group was named for the wrong quest, slice or service"
        );
    }

    #[test]
    fn the_account_manager_is_read_from_a_process_under_it() {
        let under = "0::/user.slice/user-1000.slice/user@1000.service/background.slice/h.service\n";
        assert_eq!(
            manager_of(under).as_deref(),
            Some("user.slice/user-1000.slice/user@1000.service"),
            "the account manager wasn't found above health's own service"
        );
        assert_eq!(
            manager_of("0::/system.slice/health.service\n"),
            None,
            "a machine's service was taken for the account manager"
        );
    }

    #[test]
    fn an_hour_counts_only_what_each_group_used_since_the_last_look() {
        let seen = |groups: &[(&str, &str, u64)]| {
            groups
                .iter()
                .map(|(path, whose, used)| {
                    (
                        (*path).to_owned(),
                        Group {
                            whose: (*whose).to_owned(),
                            used: *used,
                        },
                    )
                })
                .collect()
        };
        let mut busy = Busy::default();
        busy.saw(
            1,
            "13",
            seen(&[("a", "OL0ST3 jobs", 60_000_000), ("b", "app.slice", 5)]),
        );
        assert!(
            busy.hours.is_empty(),
            "a first look counted what came before it"
        );
        busy.saw(
            EVERY + 1,
            "14",
            seen(&[
                ("a", "OL0ST3 jobs", 240_000_000),
                ("c", "OL0ST3 jobs", 900_000_000),
                ("b", "app.slice", 60_000_005),
                ("d", "80ECO2 pi", 30_000_000),
            ]),
        );
        busy.saw(
            2 * EVERY + 1,
            "14",
            seen(&[("a", "OL0ST3 jobs", 1_000), ("d", "80ECO2 pi", 90_000_000)]),
        );
        let busiest: Vec<String> = busy
            .busiest("14")
            .into_iter()
            .map(|(whose, minutes)| format!("{whose} {minutes}"))
            .collect();
        assert_eq!(
            busiest,
            ["OL0ST3 jobs 3", "80ECO2 pi 1", "app.slice 1"],
            "an hour counted a group's time from before its first look, or one begun again"
        );
        assert_eq!(
            busy.said(),
            "14:00  5 core-minutes; the most: OL0ST3 jobs 3, 80ECO2 pi 1, app.slice 1\n",
            "an hour's busiest weren't said"
        );
        assert!(
            !busy.due(2 * EVERY + 2),
            "a look came again before five minutes"
        );
        assert!(
            busy.due(3 * EVERY + 1),
            "a look didn't come after five minutes"
        );
    }
}

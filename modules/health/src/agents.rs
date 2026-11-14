//! The agents that died: a pi session whose presence still says it's busy
//! or listening, though the process that wrote it is gone. A session put
//! down parks, and says so; one that crashed, was killed or lost its
//! window leaves its last word behind, and nothing else tells of it.

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::memory;

/// An agent that died, as its presence last said.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Died {
    /// What it was doing: `busy` mid-turn, or `listening` between turns.
    pub state: String,
    /// When its presence was last written, in microseconds since the
    /// epoch: as near as can be told to when it died.
    pub at: i64,
    /// The quest it worked on, if any.
    pub quest: String,
    /// Its window's title.
    pub title: String,
}

/// The agents gone without parking: those at the last look, and every one
/// that died in the day, each by session.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Agents {
    /// When they were last looked for, in microseconds since the epoch.
    pub looked: i64,
    /// Gone at the last look, whenever each died.
    pub gone: BTreeMap<String, Died>,
    /// Each that died in the day, kept though its presence goes.
    pub died: BTreeMap<String, Died>,
}

impl Agents {
    /// Keep what a look at `at` found gone, those that died on `date`
    /// among them.
    pub fn saw(&mut self, at: i64, gone: BTreeMap<String, Died>, died_today: impl Fn(i64) -> bool) {
        self.looked = at;
        for (session, died) in &gone {
            if died_today(died.at) {
                self.died.insert(session.clone(), died.clone());
            }
        }
        self.gone = gone;
    }
}

/// Look in `presences` for sessions whose owner is gone from `proc` and
/// that didn't park, by session.
#[must_use]
pub fn look(presences: &Path, proc: &Path) -> BTreeMap<String, Died> {
    let boot = fs::read_to_string(proc.join("sys/kernel/random/boot_id")).unwrap_or_default();
    gone_from(presences, |owner| alive(owner, proc, boot.trim()))
}

/// Look in `presences` for sessions whose owner `ps` no longer shows and
/// that didn't park, by session, where there's no `/proc`, as on a Mac.
#[must_use]
pub fn asked(presences: &Path, now: i64) -> BTreeMap<String, Died> {
    let said = |program: &str, args: &[&str]| {
        Command::new(program)
            .args(args)
            .output()
            .ok()
            .filter(|said| said.status.success())
            .map(|said| String::from_utf8_lossy(&said.stdout).into_owned())
    };
    // Without what ps says, every session would look dead, so none is.
    let Some(processes) = said("/bin/ps", &["-axo", "pid=,etime="]) else {
        return BTreeMap::new();
    };
    let boot = said("/usr/sbin/sysctl", &["-n", "kern.bootsessionuuid"]).unwrap_or_default();
    gone_by_ps(presences, &processes, now, boot.trim())
}

/// The sessions in `presences` gone without parking, by what
/// `ps -o pid=,etime=` said at `now`, in microseconds since the epoch, in
/// the boot `boot`.
fn gone_by_ps(presences: &Path, processes: &str, now: i64, boot: &str) -> BTreeMap<String, Died> {
    let now = now / 1_000_000;
    let started: BTreeMap<u64, i64> = processes
        .lines()
        .filter_map(|line| {
            let (pid, ran) = line.trim().split_once(' ')?;
            let ran = i64::try_from(memory::seconds_of(ran.trim())?).ok()?;
            Some((pid.parse().ok()?, now - ran))
        })
        .collect();
    gone_from(presences, |owner| {
        let same_boot = owner["bootId"]
            .as_str()
            .is_none_or(|id| boot.is_empty() || id.eq_ignore_ascii_case(boot));
        // ps says how long a process has run to the second, and a second
        // may tick between it and now, so its start is known to two.
        let began = owner["pid"].as_u64().and_then(|pid| started.get(&pid));
        let said = owner["startedAt"].as_i64().map(|at| at / 1_000);
        same_boot && matches!((began, said), (Some(began), Some(said)) if (began - said).abs() <= 2)
    })
}

/// The sessions in `presences` that didn't park and whose owner isn't
/// `alive`, by session; one with no owner here is left unjudged.
fn gone_from(presences: &Path, alive: impl Fn(&Value) -> bool) -> BTreeMap<String, Died> {
    fs::read_dir(presences)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|presence| {
            let said: Value = serde_json::from_slice(&fs::read(presence.path()).ok()?).ok()?;
            let state = said["state"].as_str()?;
            // A session held on another machine is heard here with no
            // owner, since its process isn't this machine's to look for.
            let owner = said.get("owner").filter(|owner| owner.is_object())?;
            if state == "parked" || alive(owner) {
                return None;
            }
            let died = Died {
                state: state.to_owned(),
                at: said["at"]
                    .as_str()?
                    .parse::<Timestamp>()
                    .ok()?
                    .as_microsecond(),
                quest: said["quest"].as_str().unwrap_or_default().to_owned(),
                title: said["title"].as_str().unwrap_or_default().to_owned(),
            };
            Some((said["session"].as_str()?.to_owned(), died))
        })
        .collect()
}

/// Whether a presence's owner still runs: the same boot, and a process of
/// its pid that started at the same tick, so a pid used again isn't taken
/// for it.
fn alive(owner: &Value, proc: &Path, boot: &str) -> bool {
    let same_boot = owner["bootId"]
        .as_str()
        .is_none_or(|id| boot.is_empty() || id == boot);
    let started = owner["pid"].as_u64().and_then(|pid| {
        let stat = fs::read_to_string(proc.join(pid.to_string()).join("stat")).ok()?;
        let (_, after) = stat.rsplit_once(')')?;
        after.split_whitespace().nth(19)?.parse::<u64>().ok()
    });
    same_boot && started.is_some() && started == owner["startTicks"].as_u64()
}

/// The most deaths in any ten minutes, of those at `times`.
#[must_use]
pub fn most_in_ten_minutes(times: &[i64]) -> usize {
    const TEN_MINUTES: i64 = 600_000_000;
    let mut sorted = times.to_vec();
    sorted.sort_unstable();
    (0..sorted.len())
        .map(|first| {
            sorted[first..]
                .iter()
                .take_while(|&&at| at - sorted[first] < TEN_MINUTES)
                .count()
        })
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOOT: &str = "382e9492-069f-4e8b-bb1b-d9ad097ff34b";

    /// A presence for `session`, owned by `pid` started at `ticks`.
    fn presence(folder: &Path, session: &str, state: &str, pid: u32, ticks: u64, boot: &str) {
        let said = format!(
            r#"{{"session":"{session}","state":"{state}","at":"2026-11-12T05:16:23.062Z",
                "quest":"QEST-1","title":"π - Git upkeep",
                "owner":{{"pid":{pid},"startTicks":{ticks},"bootId":"{boot}"}}}}"#
        );
        fs::write(folder.join(format!("{session}.json")), said).expect("a presence");
    }

    /// A process in a stand-in `/proc`, started at `ticks`.
    fn process(proc: &Path, pid: u32, ticks: u64) {
        let path = proc.join(pid.to_string());
        fs::create_dir_all(&path).expect("a process");
        let fields = format!("S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 {ticks} 0 0");
        fs::write(path.join("stat"), format!("{pid} (pi) {fields}\n")).expect("stat");
    }

    #[test]
    fn an_agent_gone_without_parking_died() {
        let presences = tempfile::tempdir().expect("the presences");
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        fs::create_dir_all(proc.path().join("sys/kernel/random")).expect("the kernel's");
        fs::write(
            proc.path().join("sys/kernel/random/boot_id"),
            format!("{BOOT}\n"),
        )
        .expect("the boot");
        process(proc.path(), 10, 500);
        process(proc.path(), 11, 999);
        presence(presences.path(), "running", "busy", 10, 500, BOOT);
        presence(presences.path(), "parked", "parked", 12, 1, BOOT);
        presence(presences.path(), "mid-turn", "busy", 13, 1, BOOT);
        presence(presences.path(), "pid-again", "listening", 11, 7, BOOT);
        presence(
            presences.path(),
            "last-boot",
            "listening",
            10,
            500,
            "c6d02106",
        );
        let died = look(presences.path(), proc.path());
        assert_eq!(
            died.keys().map(String::as_str).collect::<Vec<_>>(),
            ["last-boot", "mid-turn", "pid-again"],
            "neither the running nor the parked: {died:?}"
        );
        assert_eq!(
            died["mid-turn"],
            Died {
                state: "busy".into(),
                at: 1_794_460_583_062_000,
                quest: "QEST-1".into(),
                title: "π - Git upkeep".into()
            }
        );
    }

    /// A presence for `session` as a Mac writes it, owned by `pid` started
    /// at `started`, in milliseconds since the epoch.
    fn presence_on_a_mac(folder: &Path, session: &str, state: &str, pid: u32, started: u64) {
        let said = format!(
            r#"{{"session":"{session}","state":"{state}","at":"2026-11-12T05:16:23.062Z",
                "owner":{{"pid":{pid},"startedAt":{started},"bootId":"F9D442DF-0001"}}}}"#
        );
        fs::write(folder.join(format!("{session}.json")), said).expect("a presence");
    }

    #[test]
    fn an_agent_ps_no_longer_shows_since_it_started_died() {
        let presences = tempfile::tempdir().expect("the presences");
        // Now is 1,794,600,000 s; pid 10 started 4 days, 20:53:28 before,
        // and pid 11 started an hour ago.
        let now = 1_794_600_000_000_000;
        let started = 1_794_600_000 - (4 * 86_400 + 20 * 3_600 + 53 * 60 + 28);
        let processes = concat!("   10 04-20:53:28\n", "   11    01:00:00\n", "junk\n");
        presence_on_a_mac(presences.path(), "running", "busy", 10, started * 1_000);
        presence_on_a_mac(presences.path(), "parked", "parked", 12, 1_000);
        presence_on_a_mac(presences.path(), "gone", "busy", 13, started * 1_000);
        presence_on_a_mac(
            presences.path(),
            "pid-again",
            "listening",
            11,
            started * 1_000,
        );
        let died = gone_by_ps(presences.path(), processes, now, "F9D442DF-0001");
        assert_eq!(
            died.keys().map(String::as_str).collect::<Vec<_>>(),
            ["gone", "pid-again"],
            "neither the running nor the parked: {died:?}"
        );
        let after_a_restart = gone_by_ps(presences.path(), processes, now, "0A1B2C3D-0002");
        assert!(
            after_a_restart.contains_key("running"),
            "an owner of another boot is gone: {after_a_restart:?}"
        );
    }

    #[test]
    fn a_presence_with_no_owner_here_is_not_judged() {
        let presences = tempfile::tempdir().expect("the presences");
        // A session held on another machine is heard here with no owner,
        // since its process isn't this machine's to look for.
        let said = concat!(
            r#"{"session":"elsewhere","state":"listening","#,
            r#""at":"2026-11-12T01:30:48.415Z","machine":"laptop","heldAs":"4dbae29c"}"#
        );
        fs::write(presences.path().join("elsewhere.json"), said).expect("a presence");
        let proc = tempfile::tempdir().expect("a stand-in /proc");
        let looked = look(presences.path(), proc.path());
        assert!(looked.is_empty(), "none gone from /proc: {looked:?}");
        let asked = gone_by_ps(
            presences.path(),
            "   10 01:00:00\n",
            1_794_600_000_000_000,
            "",
        );
        assert!(asked.is_empty(), "none gone from ps: {asked:?}");
    }

    #[test]
    fn deaths_are_counted_in_ten_minutes_at_most() {
        let minute = 60_000_000;
        assert_eq!(most_in_ten_minutes(&[]), 0);
        assert_eq!(
            most_in_ten_minutes(&[0, 30 * minute, 31 * minute, 2 * minute, 39 * minute]),
            3,
            "the three from 30 to 39 minutes"
        );
        assert_eq!(
            most_in_ten_minutes(&[0, 10 * minute]),
            1,
            "ten minutes apart"
        );
    }
}

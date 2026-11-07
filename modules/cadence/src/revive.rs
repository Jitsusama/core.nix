//! Reviving sessions that died: which sessions are watched, whether one
//! that is gone died or was ended, and whether it may be started again
//! now. The orchestrators and the steward's own sessions are watched, so
//! a crash overnight costs minutes, not the night. Everything here is a
//! decision over what was read; main.rs reads and starts.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// No second start of one session within this long, so a slow start
/// isn't doubled.
pub const QUIET_FOR: i64 = 10 * 60;
/// Past this many starts of one session in an hour, something kills it
/// as it starts, and starting it again only repeats that.
pub const PER_HOUR: usize = 3;
/// How long after sign-in nothing is started. The user's manager lingers,
/// so cadence ticks from boot, before there is a desktop to open a window
/// on; and once there is one, the sessions Joel had are reopened together,
/// which a revive would race into a second window each.
pub const SETTLING: i64 = 15 * 60;

/// Whether the desktop has been up long enough, since `since` (when the
/// graphical session began, if it has), to start sessions at `now`.
#[must_use]
pub fn settled(since: Option<i64>, now: i64) -> bool {
    since.is_some_and(|since| now - since >= SETTLING)
}

/// A session as its presence describes it, and whether its process runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Seen {
    pub address: String,
    pub session: String,
    pub orchestrated_by: Option<String>,
    pub quest: Option<String>,
    pub title: String,
    pub at: String,
    pub running: bool,
    pub parked: bool,
}

/// What a quest's README says of itself and of its sessions.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Quest {
    pub status: String,
    pub sessions: Vec<QuestSession>,
}

/// One session entry of a quest: where it ran, whether it is still
/// attached (a pi that quits cleanly detaches itself; a crash leaves it
/// active) and the process it last ran as.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestSession {
    pub id: String,
    pub cwd: String,
    pub status: String,
    pub pid: Option<u32>,
}

/// How a watched session stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Alive,
    /// Gone, but left gone, for the reason given.
    Left(String),
    /// Died, and is to be started again in this folder.
    Died {
        cwd: String,
    },
}

/// Whether a died session may be started now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    Go,
    /// Started this many minutes ago, and given time to come up.
    TooSoon(i64),
    /// Started too often this hour; `report` once an hour.
    Defect {
        starts: usize,
        report: bool,
    },
}

/// What reviving remembers between ticks: the workspace each session's
/// window was last seen on, when each was started, and the hour a defect
/// was last reported in.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Memory {
    #[serde(default)]
    pub homes: BTreeMap<String, u32>,
    #[serde(default)]
    pub starts: BTreeMap<String, Vec<i64>>,
    #[serde(default)]
    pub defects: BTreeMap<String, i64>,
}

/// The sessions to watch: every orchestrator, those named and any that
/// runs with no orchestrator above it while another names it as theirs,
/// and every session of the steward's. One per address, a running one
/// over a gone one and then the newest, since a session that was
/// replaced leaves its old presence behind.
#[must_use]
pub fn watched<'a>(
    seen: &'a [Seen],
    orchestrators: &[String],
    steward: &str,
    skip: &[String],
) -> Vec<&'a Seen> {
    let named: Vec<&str> = seen
        .iter()
        .filter_map(|s| s.orchestrated_by.as_deref())
        .collect();
    let is_orchestrator = |s: &Seen| {
        orchestrators.contains(&s.address)
            || (s.running && s.orchestrated_by.is_none() && named.contains(&s.address.as_str()))
    };
    let mut by_address: BTreeMap<&str, &Seen> = BTreeMap::new();
    for s in seen {
        if skip.contains(&s.address)
            || !(is_orchestrator(s) || s.orchestrated_by.as_deref() == Some(steward))
        {
            continue;
        }
        let better = by_address
            .get(s.address.as_str())
            .is_none_or(|old| (s.running, &s.at) > (old.running, &old.at));
        if better {
            by_address.insert(&s.address, s);
        }
    }
    by_address.into_values().collect()
}

/// How a watched session stands, given its quest and whether a process
/// is running as it under another pid.
#[must_use]
pub fn judge(
    seen: &Seen,
    quest: Option<&Quest>,
    runs: impl Fn(u32) -> bool,
    named: bool,
) -> Verdict {
    if seen.running || seen.parked {
        return Verdict::Alive;
    }
    let Some(quest) = quest else {
        return Verdict::Left("it has no quest".into());
    };
    if !matches!(quest.status.as_str(), "active" | "paused" | "blocked") {
        return Verdict::Left(format!("its quest is {}", quest.status));
    }
    let Some(entry) = quest.sessions.iter().find(|e| e.id == seen.session) else {
        return Verdict::Left("its quest doesn't list it".into());
    };
    if entry.status != "active" {
        return Verdict::Left(format!("it was ended ({})", entry.status));
    }
    if named || entry.pid.is_some_and(runs) {
        return Verdict::Alive;
    }
    Verdict::Died {
        cwd: entry.cwd.clone(),
    }
}

/// Whether `session` may be started at `now`, by when it was started
/// before.
#[must_use]
pub fn gate(memory: &Memory, session: &str, now: i64) -> Gate {
    let starts: Vec<i64> = memory
        .starts
        .get(session)
        .map(|all| all.iter().copied().filter(|t| now - t < 3600).collect())
        .unwrap_or_default();
    if let Some(last) = starts.last()
        && now - last < QUIET_FOR
    {
        return Gate::TooSoon((now - last) / 60);
    }
    if starts.len() >= PER_HOUR {
        let hour = now / 3600;
        let report = memory.defects.get(session) != Some(&hour);
        return Gate::Defect {
            starts: starts.len(),
            report,
        };
    }
    Gate::Go
}

/// Notes a start of `session` at `now`, keeping only the last hour's.
pub fn started(memory: &mut Memory, session: &str, now: i64) {
    let starts = memory.starts.entry(session.to_owned()).or_default();
    starts.retain(|t| now - t < 3600);
    starts.push(now);
}

/// A quest README's front matter, as far as reviving reads it.
#[must_use]
pub fn quest(readme: &str) -> Quest {
    let mut quest = Quest::default();
    let head = readme.strip_prefix("---\n").unwrap_or(readme);
    let head = head.split("\n---").next().unwrap_or_default();
    let mut in_sessions = false;
    for line in head.lines() {
        let value = |key: &str| line.strip_prefix(key).map(|v| v.trim().to_owned());
        if !line.starts_with(' ') {
            in_sessions = line.starts_with("sessions:");
            if let Some(status) = value("status:") {
                quest.status = status;
            }
            continue;
        }
        if !in_sessions {
            continue;
        }
        if let Some(id) = value("  - id:") {
            quest.sessions.push(QuestSession {
                id,
                ..QuestSession::default()
            });
        } else if let Some(entry) = quest.sessions.last_mut() {
            if let Some(cwd) = value("    cwd:") {
                entry.cwd = cwd;
            } else if let Some(status) = value("    status:") {
                entry.status = status;
            } else if let Some(pid) = value("      pid:") {
                entry.pid = pid.parse().ok();
            }
        }
    }
    quest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_starts_before_sign_in_or_while_the_desktop_settles() {
        assert!(!settled(None, 10_000), "with no desktop, nothing starts");
        assert!(
            !settled(Some(10_000), 10_000 + SETTLING - 1),
            "sessions are still being reopened"
        );
        assert!(settled(Some(10_000), 10_000 + SETTLING));
    }

    fn seen(address: &str, by: Option<&str>, running: bool, at: &str) -> Seen {
        Seen {
            address: address.into(),
            session: format!("s-{address}-{at}"),
            orchestrated_by: by.map(Into::into),
            quest: Some(format!("Q-{address}")),
            title: String::new(),
            at: at.into(),
            running,
            parked: false,
        }
    }

    fn strings(all: &[&str]) -> Vec<String> {
        all.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn orchestrators_and_the_stewards_sessions_are_watched_once_each() {
        let all = vec![
            seen("STEW", None, true, "1"),
            seen("ORCH", None, true, "1"),
            seen("MINE", Some("STEW"), false, "1"),
            seen("MINE", Some("STEW"), true, "0"),
            seen("THEIRS", Some("ORCH"), false, "1"),
            seen("LONER", None, true, "1"),
            seen("IDLE", Some("STEW"), false, "1"),
        ];
        let watched = watched(&all, &strings(&["STEW"]), "STEW", &strings(&["IDLE"]));
        let names: Vec<(&str, bool)> = watched
            .iter()
            .map(|s| (s.address.as_str(), s.running))
            .collect();
        assert_eq!(names, [("MINE", true), ("ORCH", true), ("STEW", true)]);
    }

    fn quest_with(status: &str, entry: &str, pid: Option<u32>) -> Quest {
        Quest {
            status: status.into(),
            sessions: vec![QuestSession {
                id: "s-A-1".into(),
                cwd: "/work".into(),
                status: entry.into(),
                pid,
            }],
        }
    }

    #[test]
    fn a_crash_is_revived_and_an_end_on_purpose_is_left() {
        let gone = seen("A", None, false, "1");
        let died = Verdict::Died {
            cwd: "/work".into(),
        };
        let q = quest_with("active", "active", Some(9));
        assert_eq!(judge(&gone, Some(&q), |_| false, false), died);
        let detached = quest_with("active", "detached", None);
        assert_eq!(
            judge(&gone, Some(&detached), |_| false, false),
            Verdict::Left("it was ended (detached)".into())
        );
        let done = quest_with("concluded", "active", None);
        assert_eq!(
            judge(&gone, Some(&done), |_| false, false),
            Verdict::Left("its quest is concluded".into())
        );
    }

    #[test]
    fn a_session_running_under_another_pid_or_parked_is_alive() {
        let gone = seen("A", None, false, "1");
        let q = quest_with("active", "active", Some(9));
        assert_eq!(
            judge(&gone, Some(&q), |pid| pid == 9, false),
            Verdict::Alive
        );
        assert_eq!(judge(&gone, Some(&q), |_| false, true), Verdict::Alive);
        let parked = Seen {
            parked: true,
            ..gone
        };
        assert_eq!(judge(&parked, None, |_| false, false), Verdict::Alive);
    }

    #[test]
    fn starts_wait_ten_minutes_and_stop_past_three_an_hour() {
        let mut memory = Memory::default();
        assert_eq!(gate(&memory, "s", 10_000), Gate::Go);
        started(&mut memory, "s", 10_000);
        assert_eq!(gate(&memory, "s", 10_120), Gate::TooSoon(2));
        started(&mut memory, "s", 10_700);
        started(&mut memory, "s", 11_400);
        assert_eq!(
            gate(&memory, "s", 12_100),
            Gate::Defect {
                starts: 3,
                report: true
            }
        );
        memory.defects.insert("s".into(), 12_100 / 3600);
        assert_eq!(
            gate(&memory, "s", 12_200),
            Gate::Defect {
                starts: 3,
                report: false
            }
        );
        assert_eq!(gate(&memory, "s", 14_000), Gate::Go);
    }

    #[test]
    fn a_readme_gives_its_status_and_sessions() {
        let readme = "---\nid: Q\nstatus: active\nsessions:\n  - id: one\n    cwd: /a\n    \
                      status: detached\n    process:\n      pid: 12\n  - id: two\n    cwd: /b\n    \
                      status: active\n      pid: 34\nlinks: []\n---\n\n# Title\nstatus: no\n";
        let q = quest(readme);
        assert_eq!(q.status, "active");
        assert_eq!(
            q.sessions,
            [
                QuestSession {
                    id: "one".into(),
                    cwd: "/a".into(),
                    status: "detached".into(),
                    pid: Some(12)
                },
                QuestSession {
                    id: "two".into(),
                    cwd: "/b".into(),
                    status: "active".into(),
                    pid: Some(34)
                },
            ]
        );
    }
}

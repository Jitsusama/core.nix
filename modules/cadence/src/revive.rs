//! Reviving sessions that died: which sessions are watched, whether one
//! that is gone died or was ended, and whether it may be started again
//! now. The orchestrators and every orchestrator's sessions are watched,
//! so a crash overnight costs minutes, not the night. Everything here is a
//! decision over what was read; main.rs reads and starts.

use std::collections::BTreeMap;

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
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
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a separate fact a presence says of its session"
)]
pub struct Seen {
    pub address: String,
    pub session: String,
    pub orchestrated_by: Option<String>,
    pub quest: Option<String>,
    pub title: String,
    pub at: String,
    pub running: bool,
    pub parked: bool,
    /// Whether its pi ran during this boot, by the boot its presence was
    /// written in. One that didn't ended with the machine, not by a crash.
    pub this_boot: bool,
    /// Whether it hears its own orchestration's group, as every
    /// orchestrator does, whether or not it has sessions yet.
    pub orchestrates: bool,
    /// How its pi last ended on purpose, by pi's own record of the
    /// session (`ended_on_purpose`), if it did.
    pub ended: Option<String>,
}

/// How a session's pi ended on purpose, by the record pi keeps of each
/// session: the ending its newest process stamped, at the record's top,
/// when that came after the process started. `endings` holds only the
/// endings of the processes before it, moved there as each reopened: on
/// 8 November reading the newest of those missed a `mesh_end`. A quit or
/// a signal (`mesh_end`'s, or a window closing) is on purpose; a crash
/// or a `kill -9` stamps none, and a swap may leave its process running
/// the session, so it counts for nothing. A start time that can't be
/// read says nothing, as before.
#[must_use]
pub fn ended_on_purpose(record: &str, zone: &TimeZone) -> Option<String> {
    let record: serde_json::Value = serde_json::from_str(record).ok()?;
    let reason = record.get("endReason")?.as_str()?;
    if !matches!(reason, "quit" | "signalled") {
        return None;
    }
    let closed: Timestamp = record.get("closedAt")?.as_str()?.parse().ok()?;
    // The start is the process's own clock line, such as
    // "Sat Nov  7 23:15:03 2026", in the machine's time zone.
    let token = record.pointer("/process/startToken")?.as_str()?;
    let token = token.split_whitespace().collect::<Vec<_>>().join(" ");
    let started = DateTime::strptime("%a %b %d %H:%M:%S %Y", &token)
        .ok()?
        .to_zoned(zone.clone())
        .ok()?
        .timestamp();
    (closed >= started).then(|| reason.to_owned())
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

/// The sessions to watch: every orchestrator, and every session of one.
/// An orchestrator is one named, or one with no orchestrator above it
/// that hears its own orchestration's group or that another names as
/// theirs, running or gone: on 7 November one found only while running
/// would have been left down once it died, and two orchestrators with no
/// sessions yet were found by neither. One per address, a running one
/// over a gone one and then the newest, since a session that was
/// replaced leaves its old presence behind; `judge` leaves those an
/// earlier boot ended.
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
            || s.address == steward
            || (s.orchestrated_by.is_none()
                && (s.orchestrates || named.contains(&s.address.as_str())))
    };
    let leads: Vec<&str> = seen
        .iter()
        .filter(|s| is_orchestrator(s))
        .map(|s| s.address.as_str())
        .chain(std::iter::once(steward))
        .collect();
    let mut by_address: BTreeMap<&str, &Seen> = BTreeMap::new();
    for s in seen {
        let led = s
            .orchestrated_by
            .as_deref()
            .is_some_and(|by| leads.contains(&by));
        if skip.contains(&s.address) || !(is_orchestrator(s) || led) {
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
    // On 7 November a session its orchestrator ended with `mesh_end`
    // was started again two minutes later: a revived pi doesn't take
    // its quest entry back, so its clean end left that entry active.
    if let Some(how) = &seen.ended {
        return Verdict::Left(format!("it was ended on purpose ({how})"));
    }
    // Joel reopens what a boot ended when he asks, not cadence: on 7
    // November he said he never wanted sessions back by themselves.
    if !seen.this_boot {
        return Verdict::Left("it ended with an earlier boot".into());
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

/// A holder the machine's node keeps a session in, as a line of
/// `mycelium held` says: the command it holds, the pi it runs (0 while
/// parked or gone), and the windows it shows in.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Holder {
    pub command: Vec<String>,
    #[serde(default)]
    pub pid: u32,
    #[serde(default)]
    pub parked: bool,
    #[serde(default)]
    pub windows: Vec<HeldWindow>,
}

/// A window a holder shows its session in: its terminal, its id there,
/// and the socket that terminal listens on.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct HeldWindow {
    pub terminal: String,
    pub id: String,
    pub within: String,
}

/// How the node holds a session.
#[derive(Debug, PartialEq, Eq)]
pub enum Held {
    /// A holder runs its pi.
    Running,
    /// A holder keeps it parked, to start again on a key in its window.
    Parked,
    /// Only holders whose pi is gone, each window of theirs left showing
    /// a dead session.
    Dead(Vec<HeldWindow>),
    /// No holder names it.
    Not,
}

/// The holders in `mycelium held`'s output, a JSON line each, skipping
/// lines that aren't one.
#[must_use]
pub fn holders(said: &str) -> Vec<Holder> {
    said.lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// The session a command opens, after `--session` or `--session-id`,
/// given as its id or as its log's path, whose name ends `_<id>.jsonl`.
#[must_use]
pub fn session_of(command: &[String]) -> Option<&str> {
    let at = command
        .iter()
        .position(|w| w == "--session" || w == "--session-id")?;
    let named = command.get(at + 1)?;
    Some(
        named
            .strip_suffix(".jsonl")
            .and_then(|stem| stem.rsplit_once('_'))
            .map_or(named.as_str(), |(_, id)| id),
    )
}

/// How the node holds `session`, given whether a pid is a live pi. A
/// holder whose pi is gone and isn't parked, its record left behind,
/// holds nothing, and mustn't keep the session from being started again;
/// its windows are what a restart closes first.
#[must_use]
pub fn held(holders: &[Holder], session: &str, runs: impl Fn(u32) -> bool) -> Held {
    let its: Vec<&Holder> = holders
        .iter()
        .filter(|h| session_of(&h.command) == Some(session))
        .collect();
    if its.is_empty() {
        Held::Not
    } else if its.iter().any(|h| h.pid > 0 && runs(h.pid)) {
        Held::Running
    } else if its.iter().any(|h| h.parked) {
        Held::Parked
    } else {
        Held::Dead(its.iter().flat_map(|h| h.windows.clone()).collect())
    }
}

/// The realm the restart words hold a session in, after `--realm`.
#[must_use]
pub fn realm_of(restart: &str) -> Option<&str> {
    let mut words = restart.split_whitespace();
    words.find(|w| *w == "--realm")?;
    words.next()
}

/// The kitty user variable the harness marks a session's window with.
pub const SESSION_MARK: &str = "pi_session";

/// The windows in a `kitty @ ls` listing marked as `session`'s, each
/// with the process kitty started in it.
#[must_use]
pub fn marked(listing: &serde_json::Value, session: &str) -> Vec<(String, Option<u32>)> {
    let each = |v: &serde_json::Value, key: &str| -> Vec<serde_json::Value> {
        v.get(key)
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default()
    };
    let mut found = Vec::new();
    for os_window in listing.as_array().into_iter().flatten() {
        for tab in each(os_window, "tabs") {
            for window in each(&tab, "windows") {
                let mark = window
                    .pointer(&format!("/user_vars/{SESSION_MARK}"))
                    .and_then(serde_json::Value::as_str);
                if mark == Some(session)
                    && let Some(id) = window.get("id").and_then(serde_json::Value::as_u64)
                {
                    let pid = window
                        .get("pid")
                        .and_then(serde_json::Value::as_u64)
                        .and_then(|p| u32::try_from(p).ok());
                    found.push((id.to_string(), pid));
                }
            }
        }
    }
    found
}

/// What each peer is told of a look's revivals, `told` holding whom
/// each line is for, in the order they were started: one message each,
/// however many sessions it started. A harness rollover at 03:22 on
/// 11 November ended about 40 sessions together, and telling of each
/// alone sent the steward 40 messages and the watcher 40 more.
#[must_use]
pub fn gathered(told: &[(String, String)]) -> Vec<(String, String)> {
    let mut each: Vec<(String, Vec<&str>)> = Vec::new();
    for (to, line) in told {
        match each.iter_mut().find(|(peer, _)| peer == to) {
            Some((_, lines)) => lines.push(line),
            None => each.push((to.clone(), vec![line])),
        }
    }
    each.into_iter()
        .map(|(to, lines)| {
            let text = match lines.as_slice() {
                [one] => (*one).to_owned(),
                all => format!(
                    "{} sessions were started again: {}",
                    all.len(),
                    all.join(" ")
                ),
            };
            (to, text)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_that_starts_several_sessions_tells_each_peer_once() {
        let told: Vec<(String, String)> = [
            ("steward", "A died; started again."),
            ("watcher", "A died; started again."),
            ("steward", "B died; started again."),
            ("orchestrator", "B died; started again."),
            ("watcher", "B died; started again."),
        ]
        .map(|(to, line)| (to.to_owned(), line.to_owned()))
        .into();
        let both = "2 sessions were started again: A died; started again. B died; started again.";
        assert_eq!(
            gathered(&told),
            [
                ("steward".to_owned(), both.to_owned()),
                ("watcher".to_owned(), both.to_owned()),
                (
                    "orchestrator".to_owned(),
                    "B died; started again.".to_owned()
                ),
            ],
            "each peer hears one message, with its own sessions' lines"
        );
        assert!(
            gathered(&[]).is_empty(),
            "a look that started nothing says nothing"
        );
    }

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
            this_boot: true,
            orchestrates: false,
            ended: None,
        }
    }

    fn strings(all: &[&str]) -> Vec<String> {
        all.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn a_holder_whose_pi_died_leaves_its_windows_to_close() {
        let said = [
            r#"{"command":["pi","--session","a"],"pid":7,"windows":[]}"#,
            "not a holder",
            r#"{"command":["pi","--session","/s/2026_b.jsonl"],"pid":0,"parked":true}"#,
            r#"{"command":["pi","--session","c"],"pid":9,"windows":"#,
            r#"[{"terminal":"kitty","id":"2","within":"unix:@kitty-1"}]}"#,
        ];
        let said = format!(
            "{}\n{}\n{}\n{}{}",
            said[0], said[1], said[2], said[3], said[4]
        );
        let all = holders(&said);
        assert_eq!(all.len(), 3, "a line that isn't a holder is skipped");
        let runs = |pid: u32| pid == 7;
        assert_eq!(held(&all, "a", runs), Held::Running);
        assert_eq!(held(&all, "b", runs), Held::Parked, "named by its log");
        assert_eq!(
            held(&all, "c", runs),
            Held::Dead(vec![HeldWindow {
                terminal: "kitty".into(),
                id: "2".into(),
                within: "unix:@kitty-1".into(),
            }]),
            "its pi 9 is gone, so its window shows a dead session"
        );
        assert_eq!(held(&all, "d", runs), Held::Not);
    }

    #[test]
    fn the_realm_is_read_from_the_restart_words() {
        assert_eq!(realm_of("mycelium --realm work hold -- pi"), Some("work"));
        assert_eq!(realm_of("mycelium hold -- pi"), None);
    }

    #[test]
    fn a_kitty_window_marked_as_the_session_is_found() {
        let listing = serde_json::json!([{ "tabs": [{ "windows": [
            { "id": 4, "pid": 40, "user_vars": { "pi_session": "s" } },
            { "id": 5, "pid": 50, "user_vars": { "pi_session": "t" } },
            { "id": 6, "user_vars": {} },
        ] }] }]);
        assert_eq!(marked(&listing, "s"), vec![("4".to_owned(), Some(40))]);
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
        assert_eq!(
            names,
            [
                ("MINE", true),
                ("ORCH", true),
                ("STEW", true),
                ("THEIRS", false)
            ]
        );
    }

    #[test]
    fn an_orchestrator_found_while_running_is_still_watched_once_gone() {
        let all = vec![
            seen("ORCH", None, false, "1"),
            seen("THEIRS", Some("ORCH"), false, "1"),
            Seen {
                orchestrates: true,
                ..seen("ALONE", None, false, "1")
            },
            seen("LONER", None, false, "1"),
        ];
        let watched = watched(&all, &[], "STEW", &[]);
        let names: Vec<&str> = watched.iter().map(|s| s.address.as_str()).collect();
        assert_eq!(names, ["ALONE", "ORCH", "THEIRS"]);
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

    /// A record as pi writes it: `ending` at its top, as its newest
    /// process stamped it, and an older one in `endings`.
    fn record(ending: &str) -> String {
        format!(
            r#"{{"sessionId": "s-A-1", {ending}
                "endings": [{{"closedAt": "2026-11-08T00:00:00.000Z", "endReason": "quit"}}],
                "process": {{"pid": 9, "startToken": "Sat Nov  7 23:15:03 2026"}}}}"#
        )
    }

    #[test]
    fn a_session_ended_through_mesh_end_stays_down_and_a_kill_does_not() {
        let utc = TimeZone::UTC;
        let by_mesh_end =
            record(r#""closedAt": "2026-11-08T05:02:56.924Z", "endReason": "signalled","#);
        let ended = ended_on_purpose(&by_mesh_end, &utc);
        assert_eq!(ended.as_deref(), Some("signalled"));
        let gone = Seen {
            ended,
            ..seen("A", None, false, "1")
        };
        let q = quest_with("active", "active", Some(9));
        assert_eq!(
            judge(&gone, Some(&q), |_| false, false),
            Verdict::Left("it was ended on purpose (signalled)".into())
        );
        // A kill stamps no ending, though one an earlier pi stamped
        // stays in `endings`; one stamped before this process started
        // belongs to the pi it replaced; and a reader stamps `died`.
        assert_eq!(ended_on_purpose(&record(""), &utc), None);
        let before = record(r#""closedAt": "2026-11-07T23:13:39.371Z", "endReason": "signalled","#);
        assert_eq!(ended_on_purpose(&before, &utc), None);
        let crashed = record(r#""closedAt": "2026-11-08T01:00:00.000Z", "endReason": "died","#);
        assert_eq!(ended_on_purpose(&crashed, &utc), None);
    }

    #[test]
    fn a_session_the_last_boot_ended_is_left_for_joel_to_reopen() {
        let ended = Seen {
            this_boot: false,
            ..seen("A", None, false, "1")
        };
        let q = quest_with("active", "active", Some(9));
        assert_eq!(
            judge(&ended, Some(&q), |_| false, false),
            Verdict::Left("it ended with an earlier boot".into())
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

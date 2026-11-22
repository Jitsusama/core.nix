//! Reviving sessions that died: which sessions are watched, whether one
//! that is gone died or was ended, and whether it may be started again
//! now. The orchestrators and every orchestrator's sessions are watched,
//! so a crash overnight costs minutes, not the night. Everything here is a
//! decision over what was read; main.rs reads and starts.
//!
//! Long because its cases are over a third of it: each way a gone session
//! is judged (crashed, ended, hosted, stranded, held, gated) is proven
//! beside the decision it proves, over private fixtures.

use std::collections::BTreeMap;
use std::io;

use jiff::Timestamp;
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use crate::mesh::Rosters;

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
    /// The shared pi host its presence says it runs in, if one does.
    pub hosted: Option<Hosting>,
}

/// A hosted session's host, as far as reviving asks of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hosting {
    pub host: String,
    /// Whether the host's roster holds it, so the host resumes it.
    pub holds: bool,
    /// Whether the host runs, or is starting again or waits to.
    pub back: bool,
}

/// Whether a host's unit, by its `ActiveState` and the jobs systemd has
/// waiting on it, runs or is coming back: as pi-host-down judges it, and
/// deactivating too, since a host whose process died is deactivating
/// while systemd stops what is left of it, before it starts again.
#[must_use]
pub fn back(state: &str, jobs: &str) -> bool {
    matches!(
        state,
        "active" | "activating" | "reloading" | "deactivating"
    ) || !jobs.trim().is_empty()
}

/// Whether `id` is one pi or a pi host gives, letters, digits and
/// hyphens after a letter or digit, and so safe in a shell's words, as
/// an option's value, in a unit's name or a line sent under cadence's
/// name: a presence is a file any of Joel's processes may write.
#[must_use]
pub fn plain(id: &str) -> bool {
    id.starts_with(|c: char| c.is_ascii_alphanumeric())
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// What a host whose name isn't one pi gives is called in what cadence
/// says, since the name itself mustn't reach a line sent under its name.
pub const ODD_HOST: &str = "a host named oddly";

/// The host a gone session is left to, if any, by the rosters, the host
/// its presence `named`, and whether a host is `back`. A roster that
/// holds it decides over the presence, a plain host's over an odd one's
/// and one whose host is back over one whose host is down; then a roster
/// torn now, which reads again within a look or two, so is waited on
/// rather than advised past; then the presence. A host named oddly is
/// never asked after, and is said as stranded for someone to look at.
pub fn hosting(
    rosters: &Rosters,
    session: &str,
    named: Option<&str>,
    mut back: impl FnMut(&str) -> bool,
) -> Option<Hosting> {
    let listing = rosters.holders(session);
    let host = listing
        .iter()
        .copied()
        .find(|h| plain(h) && back(h))
        .or_else(|| listing.iter().copied().find(|h| plain(h)))
        .or_else(|| listing.first().copied())
        .or_else(|| rosters.torn().first().copied())
        .or(named)?;
    if !plain(host) {
        return Some(Hosting {
            host: ODD_HOST.into(),
            holds: false,
            back: false,
        });
    }
    Some(Hosting {
        host: host.to_owned(),
        holds: rosters.holds(host, session),
        back: back(host),
    })
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
    /// Died in a host that won't bring it back, for the reason given, and
    /// left for its orchestrator to start in a host again, never here.
    Stranded(String),
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
    /// The hour each session left down in a host was last said in.
    #[serde(default)]
    pub stranded: BTreeMap<String, i64>,
}

/// Whether `session`, left down in a host, is still to be said this hour,
/// noting it said when it is.
pub fn strand(memory: &mut Memory, session: &str, now: i64) -> bool {
    let hour = now / 3600;
    // An hour past a day is kept by nothing, so the memory stays small.
    memory.stranded.retain(|_, said| hour - *said < 24);
    memory.stranded.insert(session.to_owned(), hour) != Some(hour)
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
    orchestrator: &str,
    skip: &[String],
) -> Vec<&'a Seen> {
    let named: Vec<&str> = seen
        .iter()
        .filter_map(|s| s.orchestrated_by.as_deref())
        .collect();
    let is_orchestrator = |s: &Seen| {
        orchestrators.contains(&s.address)
            || s.address == orchestrator
            || (s.orchestrated_by.is_none()
                && (s.orchestrates || named.contains(&s.address.as_str())))
    };
    let leads: Vec<&str> = seen
        .iter()
        .filter(|s| is_orchestrator(s))
        .map(|s| s.address.as_str())
        .chain(std::iter::once(orchestrator))
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
    // A host resumes what its roster holds when it starts again, so one
    // started here as well runs twice on one log: on 18 November nine
    // did, after the background host's 03:23 end.
    if let Some(h) = seen.hosted.as_ref().filter(|h| h.holds && h.back) {
        return Verdict::Left(format!("its host, {}, resumes it", h.host));
    }
    // Its id goes into a login shell's words.
    if !plain(&seen.session) {
        return Verdict::Left("its id isn't one pi gives".into());
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
    // Joel ruled that a hosted conversation comes back in a host with no
    // window, so one its host won't resume is its orchestrator's to place.
    if let Some(h) = &seen.hosted {
        // Started in a host while its own still holds it, it would run
        // twice once that host is back, so the host is what is asked for.
        if h.host == ODD_HOST {
            return Verdict::Stranded(
                "its host's name isn't one pi gives, so cadence can't ask after it: \
                 see whether a host still holds it"
                    .into(),
            );
        }
        let why = if h.holds {
            "is down or gone: bring the host back, which resumes it"
        } else {
            "no longer holds it: start it in a host again"
        };
        return Verdict::Stranded(format!("its host, {}, {why}", h.host));
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
        return Gate::Defect {
            starts: starts.len(),
            report: report_due(memory, session, now),
        };
    }
    Gate::Go
}

/// Whether what keeps `session` down is still to be said this hour.
#[must_use]
pub fn report_due(memory: &Memory, session: &str, now: i64) -> bool {
    memory.defects.get(session) != Some(&(now / 3600))
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
/// `mycelium holds` says: the command it holds, the pi it runs (0 while
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

/// The holders in `mycelium holds`' output, a JSON line each, skipping
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

impl Held {
    /// Whether it keeps its session from being started again: a holder
    /// runs it, or keeps it parked to start on a key.
    #[must_use]
    pub fn names(&self) -> bool {
        matches!(self, Self::Running | Self::Parked)
    }
}

/// Whether a command, by its words, is a hosted session's viewer: node
/// running the harness's `scripts/host-join.ts`, as the launcher's
/// waiting viewers do inside a shell that titles their window, or
/// `mycelium attach` (or `holds ID attach`, as mycelium spells it now),
/// inside that shell too, as the launcher runs every viewer.
#[must_use]
pub fn views<W: AsRef<[u8]>>(words: &[W]) -> bool {
    let mycelium = words
        .iter()
        .position(|word| word.as_ref().rsplit(|&b| b == b'/').next() == Some(b"mycelium"));
    words
        .iter()
        .any(|word| word.as_ref().ends_with(b"/scripts/host-join.ts"))
        || mycelium.is_some_and(|at| {
            words
                .iter()
                .skip(at + 1)
                .any(|word| word.as_ref() == b"attach")
        })
}

/// How `seen` is held: by the node's `holders` where it lists them, else
/// by a process here naming it (`named_here`, told which processes
/// count); a holder here is no pi itself, since its pi runs under it.
/// A hosted session's viewer only shows what its host runs, so with the
/// host down it keeps nothing alive: counted, it hid a host left down,
/// since after a crash the launcher opens each hosted session's viewer
/// to wait for its host. So for a session a host runs, a viewer's holder
/// is left out altogether, known by its command while its viewer is down
/// and its holder parked, or by its process (`is_viewer`); a holder whose
/// process is a pi counts whatever its words, as a pi always does. For one no
/// host is known to run, a viewer counts as a pi does: a pi started
/// beside a host's would run twice on one log.
pub fn holding(
    seen: &Seen,
    holders: Option<&[Holder]>,
    is_pi: impl Fn(u32) -> bool,
    is_viewer: impl Fn(u32) -> bool,
    named_here: impl Fn(&dyn Fn(u32) -> bool) -> bool,
) -> Held {
    let hosted = seen.hosted.is_some();
    match holders {
        Some(all) if hosted => {
            let counted: Vec<Holder> = all
                .iter()
                .filter(|h| {
                    let live = h.pid > 0;
                    (live && is_pi(h.pid)) || !(views(&h.command) || (live && is_viewer(h.pid)))
                })
                .cloned()
                .collect();
            held(&counted, &seen.session, is_pi)
        }
        Some(all) => held(all, &seen.session, |pid| is_pi(pid) || is_viewer(pid)),
        None if named_here(&|pid| !(is_pi(pid) || (hosted && is_viewer(pid)))) => Held::Running,
        None => Held::Not,
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
/// alone sent the orchestrator 40 messages and the watcher 40 more.
#[must_use]
pub fn gathered(told: &[(String, String)], what: &str) -> Vec<(String, String)> {
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
                all => format!("{} sessions {what}: {}", all.len(), all.join(" ")),
            };
            (to, text)
        })
        .collect()
}

/// What a look has to say: defects for the orchestrator, and the stranded
/// and started-again lines, each with whom it is for.
pub struct Said<'a> {
    pub defects: &'a [String],
    pub stranded: &'a [(String, String)],
    pub told: &'a [(String, String)],
}

/// Sends what a look has to say, gathered per peer, the watcher's
/// started-again lines marked as revivals. Every line is tried, so one
/// that can't be sent keeps none of the others from their peers; the
/// first failure is still said.
///
/// # Errors
///
/// The first send that failed.
pub fn say(
    orchestrator: &str,
    watcher: Option<&str>,
    said: &Said,
    send: &dyn Fn(&str, &str, String) -> io::Result<()>,
) -> io::Result<()> {
    let mut failed = Ok(());
    for line in said.defects {
        failed = failed.and(send(orchestrator, "finding", line.clone()));
    }
    for (to, text) in gathered(said.stranded, "are left down in a host") {
        failed = failed.and(send(&to, "finding", text));
    }
    for (to, text) in gathered(said.told, "were started again") {
        let text = if watcher == Some(to.as_str()) {
            format!("Revived: {text}")
        } else {
            text
        };
        failed = failed.and(send(&to, "fyi", text));
    }
    failed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_that_starts_several_sessions_tells_each_peer_once() {
        let told: Vec<(String, String)> = [
            ("top", "A died; started again."),
            ("watcher", "A died; started again."),
            ("top", "B died; started again."),
            ("orchestrator", "B died; started again."),
            ("watcher", "B died; started again."),
        ]
        .map(|(to, line)| (to.to_owned(), line.to_owned()))
        .into();
        let both = "2 sessions were started again: A died; started again. B died; started again.";
        assert_eq!(
            gathered(&told, "were started again"),
            [
                ("top".to_owned(), both.to_owned()),
                ("watcher".to_owned(), both.to_owned()),
                (
                    "orchestrator".to_owned(),
                    "B died; started again.".to_owned()
                ),
            ],
            "each peer hears one message, with its own sessions' lines"
        );
        assert!(
            gathered(&[], "were started again").is_empty(),
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
            hosted: None,
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
        assert_eq!(
            realm_of("mycelium --realm work holds add -- pi"),
            Some("work")
        );
        assert_eq!(realm_of("mycelium --realm work hold -- pi"), Some("work"));
        assert_eq!(realm_of("mycelium holds add -- pi"), None);
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
    fn orchestrators_and_the_orchestrators_sessions_are_watched_once_each() {
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
        let forged = Seen {
            session: "s-A-1; rm -rf ~".into(),
            ..gone
        };
        let forged_quest = Quest {
            status: "active".into(),
            sessions: vec![QuestSession {
                id: forged.session.clone(),
                cwd: "/work".into(),
                status: "active".into(),
                pid: None,
            }],
        };
        assert_eq!(
            judge(&forged, Some(&forged_quest), |_| false, false),
            Verdict::Left("its id isn't one pi gives".into()),
            "an id that isn't pi's never reaches the shell"
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
    fn a_hosted_conversation_is_left_to_its_host() {
        // At 03:23 on 18 November the background host ran out of memory;
        // cadence started 33 of its conversations again as pi processes of
        // their own, in windows, and the host's own restart resumed them
        // too, so nine had two writers on one log.
        let hosted = |holds, back| Seen {
            hosted: Some(Hosting {
                host: "work-background".into(),
                holds,
                back,
            }),
            ..seen("A", None, false, "1")
        };
        let q = quest_with("active", "active", Some(9));
        assert_eq!(
            judge(&hosted(true, true), Some(&q), |_| false, false),
            Verdict::Left("its host, work-background, resumes it".into()),
            "a host that runs or restarts resumes what its roster holds"
        );
        assert_eq!(
            judge(&hosted(true, false), Some(&q), |_| false, false),
            Verdict::Stranded(
                "its host, work-background, is down or gone: bring the host back, which resumes it"
                    .into()
            ),
            "a host left down resumes nothing, and no window opens for it"
        );
        assert_eq!(
            judge(&hosted(false, true), Some(&q), |_| false, false),
            Verdict::Stranded(
                "its host, work-background, no longer holds it: start it in a host again".into()
            ),
            "a presence the host left behind isn't what it resumes"
        );
        let done = quest_with("concluded", "active", None);
        assert_eq!(
            judge(&hosted(false, false), Some(&done), |_| false, false),
            Verdict::Left("its quest is concluded".into()),
            "one that would be left anyway isn't reported"
        );
        let earlier = Seen {
            this_boot: false,
            ..hosted(true, false)
        };
        assert_eq!(
            judge(&earlier, Some(&q), |_| false, false),
            Verdict::Left("it ended with an earlier boot".into()),
            "one an earlier boot ended is Joel's to reopen, not stranded"
        );
        assert_eq!(
            judge(&hosted(true, false), Some(&q), |pid| pid == 9, false),
            Verdict::Alive,
            "one its quest's pid still runs is alive, wherever its host is"
        );
    }

    /// After a crash the launcher opens each hosted conversation's viewer
    /// to wait for its host, inside a shell titling its window, so one
    /// whose host stays down has a holder running a viewer and nothing
    /// else, its window open: that holder, the conversation hosted, and
    /// the same one no host runs.
    fn viewed() -> (Vec<Holder>, Seen, Seen) {
        let all = holders(concat!(
            r#"{"command":["sh","-c","exec \"$@\"","Reef","node","/lasts/scripts/host-join.ts","#,
            r#""--view","--session","s-A-1"],"pid":7,"windows":"#,
            r#"[{"terminal":"kitty","id":"4","within":"unix:@kitty-1"}]}"#
        ));
        let hosted = Seen {
            hosted: Some(Hosting {
                host: "work-background".into(),
                holds: true,
                back: false,
            }),
            ..seen("A", None, false, "1")
        };
        (all, hosted, seen("A", None, false, "1"))
    }

    #[test]
    fn a_viewer_holds_a_conversation_only_where_no_host_is_known_to_run_it() {
        let (all, hosted, _) = viewed();
        let viewer = |pid: u32| pid == 7;
        let pi = |pid: u32| pid == 9;
        // Where the node can't be asked, the viewer is the process here
        // naming the session.
        let here = |counts: &dyn Fn(u32) -> bool| counts(7);
        let held_hosted = holding(&hosted, Some(&all), pi, viewer, here);
        assert_eq!(
            held_hosted,
            Held::Not,
            "a viewer only shows what its host runs, so it holds nothing"
        );
        // Once its host stays away past attach's patience, the holder
        // keeps the window with its viewer down, parked.
        let away = holders(concat!(
            r#"{"command":["node","/lasts/scripts/host-join.ts","--session","s-A-1"],"#,
            r#""pid":0,"parked":true}"#
        ));
        assert_eq!(
            holding(&hosted, Some(&away), pi, viewer, here),
            Held::Not,
            "nor does one kept parked while its host is away, known by its command"
        );
        // One a viewer runs under words that don't say so, its process
        // does.
        let unsaid = holders(r#"{"command":["view-it","--session","s-A-1"],"pid":7}"#);
        assert_eq!(
            holding(&hosted, Some(&unsaid), pi, viewer, here),
            Held::Not,
            "nor one whose process is a viewer"
        );
        let named = held_hosted.names();
        let q = quest_with("active", "active", None);
        assert_eq!(
            judge(&hosted, Some(&q), |_| false, named),
            Verdict::Stranded(
                "its host, work-background, is down or gone: bring the host back, which resumes it"
                    .into()
            ),
            "so a host left down is still said, and nothing judged dead is closed for it"
        );
        assert_eq!(
            holding(&hosted, None, pi, viewer, here),
            Held::Not,
            "nor where the node can't be asked"
        );
    }

    #[test]
    fn a_conversations_own_holder_or_a_pi_still_holds_it() {
        let (all, hosted, plain) = viewed();
        let viewer = |pid: u32| pid == 7;
        let pi = |pid: u32| pid == 9;
        let here = |counts: &dyn Fn(u32) -> bool| counts(7);
        // A hosted conversation's own holder, parked or here, still holds
        // it.
        let parked = holders(r#"{"command":["pi","--session","s-A-1"],"pid":0,"parked":true}"#);
        assert_eq!(
            holding(&hosted, Some(&parked), pi, viewer, here),
            Held::Parked,
            "a hosted one parked in a holder of its own is held"
        );
        assert!(
            Held::Parked.names() && Held::Running.names(),
            "and one parked or running keeps a second pi from starting on its log"
        );
        // A pi keeps its session running whatever its holder's words say,
        // as one carried on as plain pi in a viewer's window would.
        let became = holders(concat!(
            r#"{"command":["node","/lasts/scripts/host-join.ts","--session","s-A-1"],"#,
            r#""pid":9}"#
        ));
        assert_eq!(
            holding(&hosted, Some(&became), pi, viewer, here),
            Held::Running,
            "a pi under a viewer's words still holds it"
        );
        assert_eq!(
            holding(&hosted, None, pi, viewer, |counts: &dyn Fn(u32) -> bool| {
                counts(8)
            }),
            Held::Running,
            "and so is one a holder here names, where the node can't be asked"
        );
        assert_eq!(
            holding(&plain, Some(&all), pi, viewer, here),
            Held::Running,
            "one no host runs keeps its window while a viewer holds it"
        );
        let neither = holders(r#"{"command":["pi","--session","s-A-1"],"pid":8}"#);
        assert_eq!(
            holding(&plain, Some(&neither), pi, viewer, here),
            Held::Dead(Vec::new()),
            "one no host runs, held by a process that's neither a pi nor a viewer, is dead"
        );
        assert_eq!(
            holding(&plain, None, pi, viewer, here),
            Held::Running,
            "the same where the node can't be asked"
        );
        let with_pi = holders(r#"{"command":["pi","--session","s-A-1"],"pid":9}"#);
        assert_eq!(
            holding(&hosted, Some(&with_pi), pi, viewer, here),
            Held::Running,
            "a pi keeps any session running"
        );
        assert_eq!(
            holding(&plain, None, pi, viewer, |counts: &dyn Fn(u32) -> bool| {
                counts(9)
            }),
            Held::Not,
            "a pi naming it here is no holder of it"
        );
    }

    #[test]
    fn a_host_restarting_after_a_crash_is_coming_back() {
        assert!(back("active", ""), "a running host is back");
        assert!(
            back("deactivating", ""),
            "a host whose process died is stopped before it starts again"
        );
        assert!(plain("work-background"), "a host's name is plain");
        assert!(!plain("work-*"), "a glob isn't");
        assert!(!plain(""), "nor is nothing");
        assert!(!plain("-c"), "nor an option");
        assert!(back("activating", ""), "systemd's restart after a crash");
        assert!(
            back("failed", "4 pi-host-work-background.service start waiting"),
            "a start is waiting"
        );
        assert!(!back("failed", ""), "left down past its start limit");
        assert!(!back("inactive", "\n"), "stopped, or no such unit");
    }

    #[test]
    fn a_roster_names_the_host_over_a_presence_and_a_torn_one_rules_nothing_out() {
        let rosters = |all: &[(&str, Option<&[&str]>)]| {
            Rosters(
                all.iter()
                    .map(|(host, held)| {
                        let held = held.map(|h| h.iter().map(|s| (*s).to_owned()).collect());
                        ((*host).to_owned(), held)
                    })
                    .collect(),
            )
        };
        let up = |h: &str| h != "down";
        let host =
            |r: &Rosters, named| hosting(r, "s", named, up).map(|h| (h.host, h.holds, h.back));
        let listed = rosters(&[("work-background", Some(&["s"]))]);
        assert_eq!(
            host(&listed, None),
            Some(("work-background".into(), true, true)),
            "a roster holding it names its host when its presence names none"
        );
        assert_eq!(
            host(&listed, Some("elsewhere")),
            Some(("work-background".into(), true, true)),
            "and over a host its presence names"
        );
        let both = rosters(&[("down", Some(&["s"])), ("work-background", Some(&["s"]))]);
        assert_eq!(
            host(&both, None),
            Some(("work-background".into(), true, true)),
            "of two rosters, the one whose host is back"
        );
        let torn = rosters(&[("work-background", None)]);
        assert_eq!(
            host(&torn, None),
            Some(("work-background".into(), true, true)),
            "a torn roster keeps it from a standalone start until it reads"
        );
        assert_eq!(
            host(&torn, Some("elsewhere")),
            Some(("work-background".into(), true, true)),
            "and is waited on over a host a stale presence names"
        );
        let odd_first = rosters(&[("-odd", Some(&["s"])), ("down", Some(&["s"]))]);
        assert_eq!(
            host(&odd_first, None),
            Some(("down".into(), true, false)),
            "a plain host that is down is named over an odd one"
        );
        let none = rosters(&[("work-background", Some(&["t"]))]);
        assert_eq!(host(&none, None), None, "one no host holds is standalone");
        assert_eq!(
            host(&none, Some("work-background")),
            Some(("work-background".into(), false, true)),
            "a presence a host left behind names a host that no longer holds it"
        );
        let mut asked = Vec::new();
        assert_eq!(
            hosting(&none, "s", Some("work-*"), |h| {
                asked.push(h.to_owned());
                true
            })
            .map(|h| (h.host, h.holds, h.back)),
            Some((ODD_HOST.into(), false, false)),
            "an odd name is never taken as resuming it"
        );
        assert!(asked.is_empty(), "nor is its unit asked after");
    }

    #[test]
    fn a_send_that_fails_keeps_no_other_line_from_its_peer() {
        let heard = std::cell::RefCell::new(Vec::new());
        let send = |to: &str, intent: &str, text: String| {
            heard.borrow_mut().push(format!("{to} {intent} {text}"));
            if to == "O" {
                Err(io::Error::other("the orchestrator's send failed"))
            } else {
                Ok(())
            }
        };
        let said = Said {
            defects: &["A died again".into()],
            stranded: &[("Q".into(), "B is left down".into())],
            told: &[("W".into(), "C was started".into())],
        };
        let failed = say("O", Some("W"), &said, &send);
        assert_eq!(
            failed.map_err(|e| e.to_string()),
            Err("the orchestrator's send failed".into()),
            "the failure is still said"
        );
        assert_eq!(
            heard.into_inner(),
            [
                "O finding A died again",
                "Q finding B is left down",
                "W fyi Revived: C was started",
            ],
            "every line is tried after the first fails"
        );
    }

    #[test]
    fn a_session_whose_host_is_named_oddly_is_said_as_stranded() {
        let mut odd = seen("A", None, false, "1");
        odd.hosted = Some(Hosting {
            host: ODD_HOST.into(),
            holds: false,
            back: false,
        });
        let q = quest_with("active", "active", None);
        assert!(
            matches!(
                judge(&odd, Some(&q), |_| false, false),
                Verdict::Stranded(why) if why.contains("isn't one pi gives")
            ),
            "someone is told, and the odd name isn't said"
        );
    }

    #[test]
    fn a_session_left_down_in_a_host_is_said_once_an_hour_apart_from_defects() {
        let mut memory = Memory::default();
        assert!(strand(&mut memory, "s", 7_200), "said the first time");
        assert!(!strand(&mut memory, "s", 7_300), "not again that hour");
        assert!(
            strand(&mut memory, "t", 7_300),
            "another session is its own"
        );
        assert!(
            gate(&memory, "s", 7_300) == Gate::Go && memory.defects.is_empty(),
            "it takes no defect's hour"
        );
        assert!(strand(&mut memory, "s", 10_800), "said again the next hour");
        // t was said in hour 2 and s last in hour 3.
        assert!(strand(&mut memory, "u", 25 * 3600), "u is its own");
        assert!(
            memory.stranded.contains_key("t"),
            "an hour 23 hours gone is kept"
        );
        assert!(strand(&mut memory, "u", 26 * 3600), "u again a new hour");
        assert!(
            !memory.stranded.contains_key("t") && memory.stranded.contains_key("s"),
            "an hour a day gone is forgotten, and one 23 hours gone isn't"
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

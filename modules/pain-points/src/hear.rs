//! `pain-points hear`: the listener. Each run reads what's new in pi's
//! session logs, hears each message Joel said that says something hurt
//! (sore.rs), keeps it with his words and what the agent had just done
//! (said.rs), and sends each pain no one has yet as a finding to the cell
//! that was working on it (route.rs), once, ranked by how often he hits
//! the same thing (heard.rs). His orchestrator's asks and misses ledgers
//! are read: a pain they carry goes to no one again, and the keeper of
//! the ledgers hears of each new one. Each pain sent is written into his
//! misses ledger too (missed.rs), and stays open there and here until a
//! landing a switch carried closes it. Nothing is ever sent to the void,
//! and nothing is posted anywhere.
//!
//! Unlike `read`, which keeps kinds and timings alone, `hear` keeps his
//! words, since a finding without them can't be acted on. They stay in
//! its own state (heard.jsonl, a fortnight of it, and each pain sent past
//! that until its fix runs) and in his misses ledger, where the cells
//! that keep his misses read them, and go only over the machine's own
//! mycelium socket to the cell that owns what hurt. For the
//! moment a send runs they are in mycelium's command line too, which any
//! account here can read, since `mycelium send` takes its text no other
//! way.
//!
//! It writes hour.json each run, for cadence's hourly card: the pains
//! heard and routed in the last hour, those still open however old, the
//! one he hit most, and the turns that stalled him.

mod counted;
pub mod heard;
pub mod landed;
pub mod missed;
pub mod route;
pub mod said;
pub mod sore;
pub mod stalled;
pub mod troubles;

use counted::hour_of;
use heard::Heard;
use route::{Cell, Routes};
use said::Carried;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::TryLockError;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, UNIX_EPOCH};
use troubles::{CELLLESS_AT_MOST_MS, cellless_since, troubles};

/// How far back a first run hears, and how long a pain whose thread was
/// never seen waits before it counts as lost.
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// A pain first heard longer than this after he said it goes to the
/// ledgers' keeper alone: its cell has moved on, and a first run over a
/// day of logs would otherwise send every cell a day of old pains.
pub const FRESH_MS: i64 = 2 * HOUR_MS;

/// The longest a send may take: the node answers in well under a second,
/// and a run every five minutes must end long before the next begins.
const SEND_AT_MOST: Duration = Duration::from_secs(20);

/// How long a quick question of the machine, its time zone, may take.
const ASK_AT_MOST: Duration = Duration::from_secs(5);

/// How many times a pain's finding is sent that the node never took, and
/// again its keeper's note, before the run gives up on it and says so: a
/// send that went but read as failed would otherwise ask a cell the same
/// question every five minutes.
pub const SEND_TRIES: u32 = 3;

/// Where a run listens and what it knows of the time.
pub struct Ears {
    pub sessions: PathBuf,
    pub state: PathBuf,
    /// The asks and misses ledgers, read.
    pub ledgers: Vec<PathBuf>,
    /// The misses ledger, which each pain sent is written into, and when
    /// the first it holds was sent.
    pub misses: PathBuf,
    pub misses_from: i64,
    /// The void's folder: a session begun there is the void.
    pub void: PathBuf,
    /// The mesh's messages, for the threads of the questions it sent.
    pub messages: PathBuf,
    /// Integration's switches, for the core the machine runs, and a git
    /// of core to ask whether a landing's commit is in it.
    pub switches: PathBuf,
    pub core: PathBuf,
    pub git: PathBuf,
    pub at: i64,
    /// His offset from UTC, in seconds, to say his own times.
    pub offset_s: i64,
}

/// How each session log stands between runs.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Following {
    offset: u64,
    session: String,
    void: bool,
    lab: bool,
    carried: Carried,
}

/// What hour.json says.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hour {
    pub at: i64,
    pub heard: usize,
    pub routed: usize,
    /// Pains nobody has taken up, however old, said where he wasn't in
    /// the void; a lost one isn't among them.
    pub open: usize,
    /// Those past a day nobody took up whose question's thread was never
    /// seen, which only a landing naming its entry closes.
    #[serde(default)]
    pub lost: usize,
    /// Those a peer took up whose fix isn't on the machine yet.
    #[serde(default)]
    pub taken: usize,
    /// Those of the last day a landing a switch carried closed.
    #[serde(default)]
    pub closed: usize,
    pub worst: Option<String>,
    pub worst_hits: usize,
    /// Turns that stalled him in the hour, heard though he said nothing.
    #[serde(default)]
    pub stalled: usize,
    /// What kept the run from routing, so a broken listener doesn't read
    /// as a quiet hour.
    #[serde(default)]
    pub troubles: Vec<String>,
    /// Since when the node has named no sessions while pains waited.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cellless_since: Option<i64>,
    /// Why no pain can close, when the core the machine runs or the git
    /// to ask of it can't be read: said, though it fails no run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unclosed: Option<String>,
}

/// Hears what's new since the last run, sends each new pain to its owner
/// through the mycelium, and says the hour in a line for the journal,
/// with whether the run had troubles, so a run that couldn't route exits
/// failed where `systemctl --user --failed` shows it.
///
/// # Errors
///
/// When its state can't be written, or another run is hearing.
pub fn listen(
    home: &Path,
    given: &mut BTreeMap<&'static str, String>,
) -> Result<(String, bool), String> {
    let mut take = |flag: &str| given.remove(flag);
    let state = crate::state_home();
    let own = state.join("pain-points");
    fs::create_dir_all(&own).map_err(|e| format!("{}: {e}", own.display()))?;
    // One run at a time: the timer's and one typed in a terminal would
    // each send the same findings and write over each other's state.
    let lock = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(own.join("hear.lock"))
        .map_err(|e| format!("hear.lock: {e}"))?;
    match lock.try_lock() {
        Ok(()) => {}
        // Stepping aside is the lock doing its work, so the unit isn't
        // marked failed for a run typed in a terminal beside it.
        Err(TryLockError::WouldBlock) => {
            return Ok((
                "pain-points: another run is hearing, so this one stepped aside".into(),
                false,
            ));
        }
        Err(TryLockError::Error(e)) => return Err(format!("hear.lock: {e}")),
    }
    let mission = take("--ledgers").map_or_else(|| state.join("mission"), PathBuf::from);
    let mycelium = take("--mycelium").map_or_else(|| "mycelium".into(), PathBuf::from);
    let date = take("--date").map_or_else(|| "date".into(), PathBuf::from);
    let ears = Ears {
        sessions: take("--sessions").map_or_else(|| home.join(".pi/agent/sessions"), PathBuf::from),
        state: own,
        ledgers: vec![mission.join("asks.jsonl"), mission.join("misses.jsonl")],
        misses: mission.join("misses.jsonl"),
        misses_from: missed::FROM_MS,
        void: take("--void").map_or_else(|| state.join("void"), PathBuf::from),
        messages: take("--messages").map_or_else(
            || state.join("pi/agentic-harness.pi/mesh-workflow/messages"),
            PathBuf::from,
        ),
        switches: take("--switches")
            .map_or_else(|| state.join("integration/switches.jsonl"), PathBuf::from),
        core: take("--core").map_or_else(|| state.join("cadence/core.git"), PathBuf::from),
        git: take("--git").map_or_else(|| "git".into(), PathBuf::from),
        at: take("--at")
            .and_then(|at| at.parse().ok())
            .unwrap_or_else(crate::millis_now),
        offset_s: output_within(Command::new(&date).arg("+%z"), ASK_AT_MOST)
            .as_deref()
            .and_then(crate::offset_of)
            .unwrap_or(0),
    };
    // A node that never answers leaves the run with no cells, said as a
    // trouble, rather than holding every later run behind this one.
    let cells =
        output_within(Command::new(&mycelium).arg("sessions"), SEND_AT_MOST).unwrap_or_default();
    let from = Routes::read(&ears.state).from;
    let hour = hear(&ears, &cells, &mut |to, text| {
        let Some(from) = &from else { return false };
        // A node that never answers would hold every later run behind this
        // one, so a send that hangs is stopped and tried again next run.
        output_within(
            Command::new(&mycelium)
                .args([
                    // A question, as a finding is read only on a cell's
                    // next turn and wants nothing back, and each owner is
                    // asked to say when its fix lands.
                    "send", "--from", from, "--intent", "question", "--text", text,
                ])
                .arg(format!("peer.{}", to.to_lowercase())),
            SEND_AT_MOST,
        )
        .is_some_and(|answer| node_took(&answer))
    })?;
    drop(lock);
    Ok((said(&hour), !hour.troubles.is_empty()))
}

/// The hour in a line for the journal.
fn said(hour: &Hour) -> String {
    format!(
        "pain-points: {} pains heard and {} routed in the hour, {} turns stalled him, {} open\
         {}{}{}{}{}",
        hour.heard,
        hour.routed,
        hour.stalled,
        hour.open,
        match hour.lost {
            0 => String::new(),
            lost => format!(", {lost} lost, as no thread of theirs was seen"),
        },
        hour.worst.as_ref().map_or(String::new(), |_| format!(
            "; the most hit, {} times",
            hour.worst_hits
        )),
        hour.troubles
            .iter()
            .map(|t| ["; ", t].concat())
            .collect::<String>(),
        // A node silent for less than the trouble's limit is still said,
        // so the grace isn't a quiet hour either.
        hour.cellless_since
            .filter(|since| hour.at - since < CELLLESS_AT_MOST_MS)
            .map_or(String::new(), |since| format!(
                "; the mycelium has named no sessions for {} minutes, so pains wait",
                (hour.at - since) / 60_000
            )),
        hour.unclosed.as_ref().map_or(String::new(), |why| format!(
            "; no pain can close, as {why}"
        ))
    )
}

/// Whether the node's answer to a send says it took it: the mycelium
/// exits well on a send it refused, and says so only in its answer.
fn node_took(answer: &str) -> bool {
    answer.trim_start().starts_with(r#"{"Ok""#)
}

/// What a command printed, when it ended within `limit`.
fn output_within(command: &mut Command, limit: Duration) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut out = child.stdout.take()?;
    // Read beside the wait, so a command printing more than a pipe holds
    // isn't stopped waiting on its reader; and read with the same limit,
    // as a child it left behind may hold the pipe open long after.
    let started = Instant::now();
    let (read, reading) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        let _ = read.send(out.read_to_string(&mut text).ok().map(|_| text));
    });
    if !ended_within(&mut child, limit) {
        return None;
    }
    reading
        .recv_timeout(limit.saturating_sub(started.elapsed()))
        .ok()
        .flatten()
        .filter(|s| !s.is_empty())
}

/// Whether a child ended well within `limit`; one still running then is
/// killed and counts as failed.
fn ended_within(child: &mut std::process::Child, limit: Duration) -> bool {
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if started.elapsed() < limit => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

/// Writes what it heard whole, so what was sent stays marked sent however
/// the run ends.
fn keep(heard_at: &Path, kept: &[Heard]) -> Result<(), String> {
    let lines: String = kept
        .iter()
        .filter_map(|h| serde_json::to_string(h).ok())
        .map(|line| line + "\n")
        .collect();
    crate::write_whole(heard_at, &lines)
}

/// Hears what's new, sends what no one has, and says the hour. `send`
/// takes an address and a finding's text and says whether it went.
///
/// # Errors
///
/// When its state can't be written.
pub fn hear(
    ears: &Ears,
    cells_printed: &str,
    send: &mut dyn FnMut(&str, &str) -> bool,
) -> Result<Hour, String> {
    fs::create_dir_all(&ears.state).map_err(|e| format!("{}: {e}", ears.state.display()))?;
    let since = ears.at - DAY_MS;
    let routes = Routes::read(&ears.state);
    let following_at = ears.state.join("hear.json");
    let mut following: BTreeMap<String, Following> = fs::read(&following_at)
        .ok()
        .and_then(|text| serde_json::from_slice(&text).ok())
        .unwrap_or_default();
    let heard_at = ears.state.join("heard.jsonl");
    let mut kept: Vec<Heard> = fs::read_to_string(&heard_at)
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .filter(|h: &Heard| {
            h.at >= ears.at - heard::KEPT_MS
                || (h.is_miss() && h.closed_by.is_none())
                || h.closed_at.is_some_and(|at| at >= ears.at - DAY_MS)
        })
        .collect();
    let ledgers: Vec<String> = ears
        .ledgers
        .iter()
        .flat_map(|path| heard::ledger(&fs::read_to_string(path).unwrap_or_default()))
        .collect();
    let cells = route::cells(cells_printed);
    // A pain the keeper has carried since it was heard is closed, though
    // it was open on an earlier run, and one still open is looked for
    // however old, as the keeper may carry it any day.
    for h in &mut kept {
        let looked = h.at >= ears.at - DAY_MS || h.closed_by.is_none();
        if !h.carried && !stalled::is(h) && looked {
            h.carried = heard::carried(&h.words, &ledgers);
        }
    }

    hear_logs(
        ears,
        &mut following,
        &mut kept,
        (&ledgers, &cells, &routes),
        since,
    );
    keep_stalls(ears, &following, &mut kept, since);

    // What it heard is kept before anything is sent, and again after each
    // send, so a run stopped partway sends nothing twice but the one it
    // was sending.
    keep(&heard_at, &kept)?;
    crate::write_whole(
        &following_at,
        &serde_json::to_string(&following).map_err(|e| e.to_string())?,
    )?;
    // Nothing reaches a cell the routes say never to, whichever way it
    // was found, and each send that failed is counted for the hour.
    let mut failed = 0;
    let mut guarded = |to: &str, text: &str| {
        if routes.refuses(to) {
            return false;
        }
        let went = send(to, text);
        failed += usize::from(!went);
        went
    };
    let owned = route_waiting(ears, &routes, &cells, &mut kept, &mut guarded, &heard_at);
    note_for_keeper(
        ears,
        &routes,
        &cells,
        &mut kept,
        &owned,
        &mut guarded,
        &heard_at,
    );
    let read = landed::last_read(&ears.state);
    let stood = landed::stand(ears, routes.from.as_deref(), &mut kept, read.as_ref());
    keep(&heard_at, &kept)?;
    if let Some(read) = &stood.read {
        landed::keep_read(&ears.state, read)?;
    }
    // A listener nobody set up writes nothing of his.
    let ledgered = routes
        .sends()
        .then(|| missed::write(&ears.misses, &kept, ears.offset_s, ears.misses_from).err())
        .flatten();
    let mut hour = hour_of(&kept, ears.at);
    hour.unclosed = stood.unclosed;
    let (waiting, given_up) = unsent(&kept, ears.at);
    hour.cellless_since = cellless_since(&ears.state, ears.at, cells.is_empty() && waiting);
    hour.troubles = troubles(
        &ears.state,
        &routes,
        (failed, given_up),
        hour.cellless_since
            .is_some_and(|since| ears.at - since >= CELLLESS_AT_MOST_MS),
    );
    hour.troubles.extend(ledgered);
    crate::write_whole(
        &ears.state.join("hour.json"),
        &serde_json::to_string(&hour).map_err(|e| e.to_string())?,
    )?;
    Ok(hour)
}

/// Hears what's new in each session log written since `since`, keeping
/// each pain he said, takes up those its session's cell answered him on,
/// and forgets the logs no longer there.
fn hear_logs(
    ears: &Ears,
    following: &mut BTreeMap<String, Following>,
    kept: &mut Vec<Heard>,
    (ledgers, cells, routes): (&[String], &BTreeMap<String, Cell>, &Routes),
    since: i64,
) {
    let mut present = Vec::new();
    for (name, path) in logs(&ears.sessions, since) {
        present.push(name.clone());
        let log = following.entry(name).or_default();
        let said = follow(&path, &ears.void, log);
        for said in said {
            let key = format!("{}/{}", log.session, said.entry);
            let kinds = sore::kinds(&said.words, said.at, &said.before);
            if log.lab || said.at < since || kinds.is_empty() {
                continue;
            }
            // A session carried on into a new log repeats what was said
            // in the old one, at the same time, so it is heard once, and
            // owned by the later log's session, which carries it on.
            if let Some(h) = kept
                .iter_mut()
                .find(|h| h.key == key || (h.at == said.at && h.words == said.words))
            {
                carry_on(h, &key, log);
                continue;
            }
            let hits = heard::hits(&said.words, kept);
            kept.push(Heard {
                key,
                session: log.session.clone(),
                entry: said.entry,
                at: said.at,
                void: log.void,
                carried: heard::carried(&said.words, ledgers),
                words: said.words,
                kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
                looking_at: said.looking_at,
                tools: said.tools,
                failed: said.failed,
                hits,
                to: None,
                routed_at: None,
                tries: 0,
                taken_by: None,
                answered_at: None,
                closed_by: None,
                closed_at: None,
                thread: Vec::new(),
                shared: false,
                landed: Vec::new(),
            });
        }
        // An answer read while the node names no cell for the session
        // waits in its reading for a run that does; one by a cell the
        // routes refuse, as the void begun elsewhere, takes nothing up.
        if let Some(cell) = cells.get(&log.session)
            && let Some(at) = log.carried.answered.take()
            && let Some(asked) = log.carried.asked.take()
            && !routes.refuses(&cell.address)
        {
            answered(kept, &log.session, (asked, at), &cell.address);
        }
    }
    following.retain(|name, _| present.contains(name));
}

/// Takes up each pain said in `session` before its cell, `by`, answered
/// him there `at`. The cell a pain goes to is the session's own, and one
/// that answered him where he said it has taken it up though it never
/// replies to the question, as one did on 20 November after a restart cut
/// its first turn short. A stall is no words of his to answer, the void
/// has no cell to take anything, and a pain a carried-on log moved to
/// another session waits for that one's answer. Only the pains said from
/// the message it answered, `asked`, are taken, so one a failed turn left
/// open before isn't taken by an answer to something else.
fn answered(kept: &mut [Heard], session: &str, (asked, at): (i64, i64), by: &str) {
    for h in kept
        .iter_mut()
        .filter(|h| h.session == session && !h.void && !stalled::is(h))
    {
        if (asked..at).contains(&h.at) && h.taken_by.is_none() {
            h.taken_by = Some(by.to_owned());
            h.answered_at = Some(at);
        }
    }
}

/// Whether any pain of the day waits to be sent, since a node naming no
/// sessions is a trouble only then, and how many were given up on: their
/// owner's tries spent while they were fresh, or the keeper's after them.
fn unsent(kept: &[Heard], at: i64) -> (bool, usize) {
    let unsent = || {
        kept.iter()
            .filter(move |h| h.routed_at.is_none() && h.at >= at - DAY_MS)
    };
    let waiting = unsent().any(|h| !h.carried);
    let given_up = unsent()
        .filter(|h| h.tries >= 2 * SEND_TRIES || (h.tries >= SEND_TRIES && h.at >= at - FRESH_MS))
        .count();
    (waiting, given_up)
}

/// Points a pain heard again in a carried-on log at the session carrying
/// it on, while it hasn't gone, since the old one has ended or moved.
fn carry_on(h: &mut Heard, key: &str, log: &Following) {
    if h.key != key && h.routed_at.is_none() {
        h.session.clone_from(&log.session);
        h.void = log.void;
    }
}

/// Keeps each turn read timed as stalling him in a followed session, so
/// what slowed him goes to the session's cell too, though he said nothing.
fn keep_stalls(
    ears: &Ears,
    following: &BTreeMap<String, Following>,
    kept: &mut Vec<Heard>,
    since: i64,
) {
    let timed = stalled::logs(&ears.state);
    for (name, log) in following {
        let Some(turns) = timed.get(name).filter(|_| !log.lab) else {
            continue;
        };
        for stall in stalled::stalls(turns, &log.session, log.void, since, ears.at) {
            // A stall is the same one when its turn began at the same
            // moment and lost him as long, though a carried-on log names
            // it again; another session's in the same second is its own.
            if let Some(h) = kept.iter_mut().find(|h| {
                h.key == stall.key || (stalled::is(h) && h.at == stall.at && h.words == stall.words)
            }) {
                carry_on(h, &stall.key, log);
            } else {
                kept.push(stall);
            }
        }
    }
}

/// Sends each pain of the last two hours not yet sent, nor carried, to its
/// owner, one finding a cell, worst first, and gives the keys of those
/// that went.
fn route_waiting(
    ears: &Ears,
    routes: &Routes,
    cells: &BTreeMap<String, Cell>,
    kept: &mut [Heard],
    send: &mut dyn FnMut(&str, &str) -> bool,
    heard_at: &Path,
) -> Vec<String> {
    // With no one named to send as, or no cell named never to send to,
    // nothing is sent: a listener nobody set up hears and counts alone.
    if !routes.sends() {
        return Vec::new();
    }
    let mut by_owner: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (n, pain) in kept.iter().enumerate() {
        if pain.carried
            || pain.routed_at.is_some()
            || pain.tries >= SEND_TRIES
            || pain.at < ears.at - FRESH_MS
        {
            continue;
        }
        if let Some(owner) = route::owner(&pain.session, pain.void, cells, routes) {
            by_owner.entry(owner).or_default().push(n);
        }
    }
    let mut owned = Vec::new();
    for (owner, mut pains) in by_owner {
        pains.sort_by_key(|n| (std::cmp::Reverse(kept[*n].hits), kept[*n].at));
        pains.truncate(route::NAMED_AT_MOST);
        let named: Vec<&Heard> = pains.iter().map(|n| &kept[*n]).collect();
        if send(&owner, &route::finding(&named, cells, ears.offset_s)) {
            for n in pains {
                kept[n].to = Some(owner.clone());
                kept[n].routed_at = Some(ears.at);
                owned.push(kept[n].key.clone());
            }
        } else {
            for n in pains {
                kept[n].tries += 1;
            }
        }
        let _ = keep(heard_at, kept);
    }
    owned
}

/// Sends the ledgers' keeper the pains just sent and those heard too late
/// for their owners, and marks the late ones as gone to the keeper once
/// the note went.
fn note_for_keeper(
    ears: &Ears,
    routes: &Routes,
    cells: &BTreeMap<String, Cell>,
    kept: &mut [Heard],
    owned: &[String],
    send: &mut dyn FnMut(&str, &str) -> bool,
    heard_at: &Path,
) {
    let Some(keeper) = routes.ledgers.as_ref().filter(|_| routes.sends()) else {
        return;
    };
    let late: Vec<usize> = (0..kept.len())
        .filter(|n| {
            let pain = &kept[*n];
            // A stall is no ask or miss of his, and one heard late has
            // nothing for the keeper to carry.
            // One from a session this realm doesn't know waits, as it
            // would for its owner.
            !pain.carried
                && !stalled::is(pain)
                && route::owner(&pain.session, pain.void, cells, routes).is_some()
                && pain.routed_at.is_none()
                && pain.tries < 2 * SEND_TRIES
                && pain.at >= ears.at - DAY_MS
                && pain.at < ears.at - FRESH_MS
        })
        .collect();
    let noted: Vec<&Heard> = kept
        .iter()
        .enumerate()
        .filter(|(n, h)| {
            // The keeper isn't told again of what it was itself just sent.
            let to_keeper =
                h.to.as_ref()
                    .is_some_and(|to| to.eq_ignore_ascii_case(keeper));
            (owned.contains(&h.key) && !stalled::is(h) && !to_keeper) || late.contains(n)
        })
        .map(|(_, h)| h)
        .collect();
    if noted.is_empty() {
        return;
    }
    // The note of pains already sent is said once: a keeper that misses it
    // still has each from its owner's side, and heard.jsonl. Late ones
    // wait for the next run until the note goes.
    // A late pain has its keeper's own tries, after any its owner had.
    let went = send(keeper, &route::ledger_note(&noted, cells, ears.offset_s));
    for n in late {
        if went {
            kept[n].to = Some(keeper.clone());
            kept[n].routed_at = Some(ears.at);
        } else {
            kept[n].tries = kept[n].tries.max(SEND_TRIES) + 1;
        }
    }
    let _ = keep(heard_at, kept);
}

/// Each session log, by its file's name, written since `since`.
fn logs(sessions: &Path, since: i64) -> Vec<(String, PathBuf)> {
    let mut found = Vec::new();
    for folder in fs::read_dir(sessions).into_iter().flatten().flatten() {
        for entry in fs::read_dir(folder.path()).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.extension().is_none_or(|e| e != "jsonl") {
                continue;
            }
            let written = entry
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX));
            if written < since {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                found.push((name.to_string(), path.clone()));
            }
        }
    }
    found.sort();
    found
}

/// Reads a log on from where it was left, whole lines alone, giving what
/// he said in it; when the agent last answered him is kept in its
/// reading. A log shorter than where it was left is read anew.
fn follow(path: &Path, void: &Path, log: &mut Following) -> Vec<said::Said> {
    let Ok(file) = fs::File::open(path) else {
        return Vec::new();
    };
    let length = file.metadata().map_or(0, |m| m.len());
    if length < log.offset {
        *log = Following::default();
    }
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::Start(log.offset)).is_err() {
        return Vec::new();
    }
    let mut found = Vec::new();
    let mut line = Vec::new();
    loop {
        line.clear();
        match reader.read_until(b'\n', &mut line) {
            Ok(0) | Err(_) => break,
            Ok(n) if line.last() != Some(&b'\n') => {
                // Half a line: it is read whole on the next run.
                let _ = n;
                break;
            }
            Ok(n) => log.offset += n as u64,
        }
        let Ok(text) = std::str::from_utf8(&line) else {
            continue;
        };
        if log.session.is_empty() {
            if let Ok(header) = serde_json::from_str::<Header>(text)
                && header.kind == "session"
            {
                log.session = header.id;
                log.void = header.cwd == void;
                log.lab = crate::a_lab(&header.cwd);
            }
            continue;
        }
        if let Some(said) = said::line(text, &mut log.carried) {
            found.push(said);
        }
    }
    found
}

/// A session log's first line.
#[derive(Deserialize)]
struct Header {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    id: String,
    #[serde(default)]
    cwd: PathBuf,
}

#[cfg(test)]
mod tests;

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
//!
//! Long because a run's sends, to owners and to the ledgers' keeper,
//! each keep the heard state before the next, and they share it here.

mod counted;
pub mod heard;
pub mod landed;
pub mod missed;
mod node;
pub mod route;
pub mod said;
pub mod sore;
pub mod stalled;
pub mod troubles;

use counted::hour_of;
use heard::Heard;
pub use node::Sent;
use node::{Node, SEND_AT_MOST, SENDS_END, Silence, output_within};
use route::{Cell, Routes};
use said::Carried;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::fs::TryLockError;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant, UNIX_EPOCH};
use troubles::{CELLLESS_AT_MOST_MS, cellless_since, troubles};

/// How far back a first run hears, and how long a pain whose thread was
/// never seen waits before it counts as lost.
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// A pain first heard longer than this after he said it goes to the
/// ledgers' keeper alone: its cell has moved on, and a first run over a
/// day of logs would otherwise send every cell a day of old pains. One
/// said in a cell kept apart, which has no keeper, has `APART_MS`.
pub const FRESH_MS: i64 = 2 * HOUR_MS;

/// How long after he said it a pain in a cell kept apart from the keeper
/// is still sent to that cell, since no keeper carries it after: longer,
/// it's given up and said so. Bounded, so a first run, or a listener
/// back after hours down, sends no such cell a day of old pains. A stall
/// keeps `FRESH_MS`, as anywhere else, since one heard late has nothing
/// for anyone to carry.
pub const APART_MS: i64 = 6 * HOUR_MS;

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
    /// When the run stops asking that git, so it writes what it heard
    /// within its unit's time.
    pub asks_end: Instant,
    pub at: i64,
    /// His offset from UTC, in seconds, to say his own times.
    pub offset_s: i64,
}

/// How far into a run its asks of git may go, half a minute after its
/// sends and half a minute short of the four minutes its unit allows.
const ASKS_END: Duration = Duration::from_secs(210);

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

/// How a send is put to a cell: asked, wanting an answer, or told, read
/// with its next turn and wanting none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Put {
    Asked,
    Told,
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
    let begun = Instant::now();
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
        asks_end: begun + ASKS_END,
        at: take("--at")
            .and_then(|at| at.parse().ok())
            .unwrap_or_else(crate::millis_now),
        offset_s: output_within(Command::new(&date).arg("+%z"), ASK_AT_MOST)
            .as_deref()
            .and_then(crate::offset_of)
            .unwrap_or(0),
    };
    // A node that never answers would hold every later run behind this
    // one, so an ask that hangs is stopped, and the run's sends after it
    // wait for the next run, their tries kept; one that never names its
    // sessions leaves the run with no cells, said as a trouble.
    let from = Routes::read(&ears.state).from;
    let mut node = Node::new(mycelium, from, begun + SENDS_END, SEND_AT_MOST);
    let cells = node.sessions();
    let hour = hear_putting(&ears, &cells, &mut |to, text, put| node.send(to, text, put))?;
    drop(lock);
    // Sends held by a node that stopped answering are no trouble of their
    // own, as the ask that found it so failed, nor are those held for a
    // routes.json read as it gained who to send as, which the next run
    // reads whole; but each is said, so a quiet line isn't read as nothing
    // to send. Those held as the run ran out of time, or as the mycelium
    // couldn't be started, are among the hour's troubles, in hour.json
    // too, so cadence's card says why; the journal names the error.
    Ok((
        [said(&hour), waits(&node)].concat(),
        !hour.troubles.is_empty(),
    ))
}

/// What the journal says of the sends `node` held, each by why.
fn waits(node: &Node) -> String {
    use std::fmt::Write;
    let mut said = String::new();
    let (held, fromless) = (node.held, node.fromless);
    if held > 0 {
        let _ = write!(
            said,
            "; {held} sends wait for the next run, as the mycelium stopped answering"
        );
    }
    if let Some(Silence::Unstarted(kind)) = node.silent {
        let _ = write!(said, "; the mycelium couldn't be started: {kind}");
    }
    if fromless > 0 {
        let _ = write!(
            said,
            "; {fromless} sends wait for the next run, as routes.json named no one to send as \
             when it began"
        );
    }
    said
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

/// `hear_putting`, for the tests that read every send alike, each going
/// or failing.
#[cfg(test)]
pub fn hear(
    ears: &Ears,
    cells_printed: &str,
    send: &mut dyn FnMut(&str, &str) -> bool,
) -> Result<Hour, String> {
    hear_putting(ears, cells_printed, &mut |to, text, _| {
        if send(to, text) {
            Sent::Went
        } else {
            Sent::Failed
        }
    })
}

/// Hears what's new, sends what no one has, and says the hour. `send`
/// takes an address, a finding's text and how it's put, and says how it
/// came out.
///
/// # Errors
///
/// When its state can't be written.
pub fn hear_putting(
    ears: &Ears,
    cells_printed: &str,
    send: &mut dyn FnMut(&str, &str, Put) -> Sent,
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
    let ledgers: Vec<heard::Line> = ears
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
    // was found: it waits, its tries kept, as the hour's troubles say of a
    // fallback or keeper that is one. Each send that failed, or waits as
    // the run's time for sends ran out, is counted for the hour.
    let (mut failed, mut out_of_time, mut unstarted) = (0, 0, 0);
    let mut guarded = |to: &str, text: &str, put: Put| {
        if routes.refuses(to) {
            return Sent::Held;
        }
        let answer = send(to, text, put);
        failed += usize::from(answer == Sent::Failed);
        out_of_time += usize::from(answer == Sent::OutOfTime);
        unstarted += usize::from(answer == Sent::Unstarted);
        answer
    };
    let mut asked = |to: &str, text: &str| guarded(to, text, Put::Asked);
    let owned = route_waiting(ears, &routes, &cells, &mut kept, &mut asked, &heard_at);
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
    let (offset_s, from) = (ears.offset_s, ears.misses_from);
    let owning = missed::owning(&ledgers, &routes);
    let ledgered = routes
        .sends()
        .then(|| missed::write(&ears.misses, &kept, &owning, offset_s, from).err())
        .flatten();
    let mut hour = hour_of(&kept, ears.at);
    hour.unclosed = stood.unclosed;
    let apart = |h: &Heard| route::apart(&h.session, h.void, &cells, &routes);
    let (waiting, given_up) = unsent(&kept, ears.at, &apart);
    hour.cellless_since = cellless_since(&ears.state, ears.at, cells.is_empty() && waiting);
    hour.troubles = troubles(
        &ears.state,
        &routes,
        (failed, out_of_time, unstarted, given_up),
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
    (ledgers, cells, routes): (&[heard::Line], &BTreeMap<String, Cell>, &Routes),
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
                unnoted_since: None,
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
        // An answer kept before answers held their turn's opening takes,
        // as it did then, the pains from the message it answered, or
        // every pain said before it where none was kept.
        if let Some(cell) = cells.get(&log.session) {
            let asked = log.carried.asked.take().unwrap_or(i64::MIN);
            let older = log.carried.answered.take().map(|at| (asked, at));
            let answers = std::mem::take(&mut log.carried.answers);
            if !routes.refuses(&cell.address) {
                for turn in older.into_iter().chain(answers) {
                    answered(kept, &log.session, turn, &cell.address);
                }
            }
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
/// another session waits for that one's answer. Only the pains said in
/// the turn it answered, from when he opened it, `from`, are taken, so one
/// a failed turn left open isn't taken by an answer to something else.
fn answered(kept: &mut [Heard], session: &str, (from, at): (i64, i64), by: &str) {
    for h in kept
        .iter_mut()
        .filter(|h| h.session == session && !h.void && !stalled::is(h))
    {
        if (from..at).contains(&h.at) && h.taken_by.is_none() {
            h.taken_by = Some(by.to_owned());
            h.answered_at = Some(at);
        }
    }
}

/// Whether any pain of the day waits to be sent, since a node naming no
/// sessions is a trouble only then, and how many were given up on: their
/// owner's tries spent while they were fresh, or the keeper's after them.
/// One said in a cell kept `apart` from the keeper has no keeper after
/// its owner, so it's given up once its owner's tries are spent or its
/// time to be sent has passed.
fn unsent(kept: &[Heard], at: i64, apart: &dyn Fn(&Heard) -> bool) -> (bool, usize) {
    let unsent = || {
        kept.iter()
            .filter(move |h| h.routed_at.is_none() && h.at >= at - DAY_MS)
    };
    let waiting = unsent().any(|h| !h.carried);
    let given_up = unsent()
        .filter(|h| {
            if apart(h) {
                let aged = !h.carried && !stalled::is(h) && h.at < at - fresh_for(h, true);
                h.tries >= SEND_TRIES || aged
            } else {
                h.tries >= 2 * SEND_TRIES || (h.tries >= SEND_TRIES && h.at >= at - FRESH_MS)
            }
        })
        .count();
    (waiting, given_up)
}

/// How long after he said it `pain` is still sent to its own cell: while
/// fresh, or for a pain said in a cell kept `apart`, `APART_MS`, as no
/// keeper carries it after. A stall is sent only while fresh either way.
fn fresh_for(pain: &Heard, apart: bool) -> i64 {
    if apart && !stalled::is(pain) {
        APART_MS
    } else {
        FRESH_MS
    }
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
/// that went. A pain said in a cell kept apart is sent for `APART_MS`.
/// Each is tried `SEND_TRIES` times at most, so a send that went but read
/// as failed asks no cell the same question more than that.
fn route_waiting(
    ears: &Ears,
    routes: &Routes,
    cells: &BTreeMap<String, Cell>,
    kept: &mut [Heard],
    send: &mut dyn FnMut(&str, &str) -> Sent,
    heard_at: &Path,
) -> Vec<String> {
    // With no one named to send as, or no cell named never to send to,
    // nothing is sent: a listener nobody set up hears and counts alone.
    if !routes.sends() {
        return Vec::new();
    }
    let mut by_owner: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (n, pain) in kept.iter().enumerate() {
        if pain.carried || pain.routed_at.is_some() {
            continue;
        }
        let Some(owner) = route::owner(&pain.session, pain.void, cells, routes) else {
            continue;
        };
        let fresh = fresh_for(pain, route::apart(&pain.session, pain.void, cells, routes));
        if pain.tries < SEND_TRIES && pain.at >= ears.at - fresh {
            by_owner.entry(owner).or_default().push(n);
        }
    }
    let mut owned = Vec::new();
    for (owner, mut pains) in by_owner {
        pains.sort_by_key(|n| (std::cmp::Reverse(kept[*n].hits), kept[*n].at));
        pains.truncate(route::NAMED_AT_MOST);
        let named: Vec<&Heard> = pains.iter().map(|n| &kept[*n]).collect();
        match send(&owner, &route::finding(&named, cells, ears.offset_s)) {
            Sent::Went => {
                for n in pains {
                    kept[n].to = Some(owner.clone());
                    kept[n].routed_at = Some(ears.at);
                    owned.push(kept[n].key.clone());
                }
            }
            Sent::Failed => {
                for n in pains {
                    kept[n].tries += 1;
                }
            }
            // Not tried, so its tries stand for the next run.
            Sent::Held | Sent::OutOfTime | Sent::Unstarted => {}
        }
        let _ = keep(heard_at, kept);
    }
    owned
}

/// Asks the ledgers' keeper of the pains just sent and those heard too
/// late for their owners, and marks the late ones as gone to the keeper
/// once the note went. Of those sent to cells its list leaves to them,
/// it tells the keeper in a note of their own, wanting no answer.
fn note_for_keeper(
    ears: &Ears,
    routes: &Routes,
    cells: &BTreeMap<String, Cell>,
    kept: &mut [Heard],
    owned: &[String],
    send: &mut dyn FnMut(&str, &str, Put) -> Sent,
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
                && !route::apart(&pain.session, pain.void, cells, routes)
                && route::owner(&pain.session, pain.void, cells, routes).is_some()
                && pain.routed_at.is_none()
                && pain.tries < 2 * SEND_TRIES
                && pain.at >= ears.at - DAY_MS
                && pain.at < ears.at - FRESH_MS
        })
        .collect();
    let (told, noted): (Vec<_>, Vec<_>) = kept
        .iter()
        .enumerate()
        .filter(|(n, h)| {
            // The keeper isn't told again of what it was itself just sent.
            let to_keeper =
                h.to.as_ref()
                    .is_some_and(|to| to.eq_ignore_ascii_case(keeper));
            let waits = owned.contains(&h.key) || h.unnoted_since.is_some();
            // Nor of what was said in, or went to, a cell whose work the
            // ledgers don't hold.
            let apart = route::apart(&h.session, h.void, cells, routes)
                || h.to.as_deref().is_some_and(|to| routes.keeps_apart(to));
            (waits && !stalled::is(h) && !to_keeper && !apart) || late.contains(n)
        })
        // A late one went to no owner, so the keeper is asked of it,
        // whoever's cell it was heard in.
        .partition(|(n, h)| {
            !late.contains(n) && h.to.as_deref().is_some_and(|to| routes.only_tells(to))
        });
    let told: Vec<usize> = told.into_iter().map(|(n, _)| n).collect();
    let noted: Vec<usize> = noted.into_iter().map(|(n, _)| n).collect();
    // Told once, as the note of pains already sent is: a failed send is
    // counted for the hour, and each is in heard.jsonl. One held, or out
    // of the run's time, goes with the next run's note.
    if !told.is_empty() {
        let note = ledger_note_of(kept, &told, cells, ears.offset_s, false);
        let answer = send(keeper, &note, Put::Told);
        wait_for_note(kept, &told, answer, ears.at);
    }
    if noted.is_empty() {
        return;
    }
    // The note of pains already sent is said once: a keeper that misses it
    // still has each from its owner's side, and heard.jsonl. Late ones
    // wait for the next run until the note goes.
    // A late pain has its keeper's own tries, after any its owner had.
    let note = ledger_note_of(kept, &noted, cells, ears.offset_s, true);
    let answer = send(keeper, &note, Put::Asked);
    let owners: Vec<usize> = noted.into_iter().filter(|n| !late.contains(n)).collect();
    wait_for_note(kept, &owners, answer, ears.at);
    for n in late {
        match answer {
            Sent::Went => {
                kept[n].to = Some(keeper.clone());
                kept[n].routed_at = Some(ears.at);
            }
            Sent::Failed => kept[n].tries = kept[n].tries.max(SEND_TRIES) + 1,
            Sent::Held | Sent::OutOfTime | Sent::Unstarted => {}
        }
    }
    let _ = keep(heard_at, kept);
}

/// The keeper's note of the pains at `of` in `kept`, asked or told.
fn ledger_note_of(
    kept: &[Heard],
    of: &[usize],
    cells: &BTreeMap<String, Cell>,
    offset_s: i64,
    asked: bool,
) -> String {
    let pains: Vec<&Heard> = of.iter().map(|&n| &kept[n]).collect();
    route::ledger_note(&pains, cells, offset_s, asked)
}

/// Marks how a keeper's note of the pains at `of` came out: one held, or
/// out of the run's time, waits for the next run's note, and one said, or
/// tried and lost, is said no more.
fn wait_for_note(kept: &mut [Heard], of: &[usize], sent: Sent, at: i64) {
    for &n in of {
        kept[n].unnoted_since = match sent {
            Sent::Held | Sent::OutOfTime | Sent::Unstarted => kept[n].unnoted_since.or(Some(at)),
            Sent::Went | Sent::Failed => None,
        };
    }
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

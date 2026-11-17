//! `pain-points hear`: the listener. Each run reads what's new in pi's
//! session logs, hears each message Joel said that says something hurt
//! (sore.rs), keeps it with his words and what the agent had just done
//! (said.rs), and sends each pain no one has yet as a finding to the cell
//! that was working on it (route.rs), once, ranked by how often he hits
//! the same thing (heard.rs). His orchestrator's asks and misses ledgers
//! are read, never written: a pain they carry goes to no one again, and
//! the keeper of the ledgers hears of each new one as it goes to its
//! owner, and of each day in a count once it ends (day.rs). Nothing is
//! ever sent to the void, and nothing is posted anywhere.
//!
//! Unlike `read`, which keeps kinds and timings alone, `hear` keeps his
//! words, since a finding without them can't be acted on. They stay in
//! its own state (heard.jsonl, a fortnight of it) and go only over the
//! machine's own mycelium socket to the cell that owns what hurt. For the
//! moment a send runs they are in mycelium's command line too, which any
//! account here can read, since `mycelium send` takes its text no other
//! way.
//!
//! It writes hour.json each run, for cadence's hourly card: the pains
//! heard and routed in the last hour, those of the last day still open,
//! the one he hit most, the turns that stalled him, and the times today
//! he asked how something stands.

pub mod day;
pub mod heard;
pub mod route;
pub mod said;
pub mod sore;
pub mod stalled;

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

/// How far back a first run hears, and how long a pain stays open.
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// A pain first heard longer than this after he said it goes to no one,
/// and is said only in its day's count: its cell has moved on, the keeper
/// asked to hear only what's new, and a first run over a day of logs
/// would otherwise send every cell a day of old pains.
pub const FRESH_MS: i64 = 2 * HOUR_MS;

/// The longest a send may take: the node answers in well under a second,
/// and a run every five minutes must end long before the next begins.
const SEND_AT_MOST: Duration = Duration::from_secs(20);

/// How long a quick question of the machine, its time zone, may take.
const ASK_AT_MOST: Duration = Duration::from_secs(5);

/// How many times a pain's finding is sent that the node never took
/// before the run gives up on it and says so: a
/// send that went but read as failed would otherwise ask a cell the same
/// question every five minutes.
pub const SEND_TRIES: u32 = 3;

/// Where a run listens and what it knows of the time.
pub struct Ears {
    pub sessions: PathBuf,
    pub state: PathBuf,
    /// The asks and misses ledgers, read alone.
    pub ledgers: Vec<PathBuf>,
    /// The void's folder: a session begun there is the void.
    pub void: PathBuf,
    pub at: i64,
    /// His offset from UTC, in seconds, to say his own times.
    pub offset_s: i64,
    /// Whether the offset was read, rather than taken as UTC because
    /// `date` couldn't say it.
    pub zone_known: bool,
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
    pub open: usize,
    pub worst: Option<String>,
    pub worst_hits: usize,
    /// Turns that stalled him in the hour, heard though he said nothing.
    #[serde(default)]
    pub stalled: usize,
    /// Times he asked how something stands today where he is: status
    /// should reach him unasked, so this is aimed at none.
    #[serde(default)]
    pub status_asks: usize,
    /// What kept the run from routing, so a broken listener doesn't read
    /// as a quiet hour.
    #[serde(default)]
    pub troubles: Vec<String>,
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
    let offset = output_within(Command::new(&date).arg("+%z"), ASK_AT_MOST)
        .as_deref()
        .and_then(crate::offset_of);
    let ears = Ears {
        sessions: take("--sessions").map_or_else(|| home.join(".pi/agent/sessions"), PathBuf::from),
        state: own,
        ledgers: vec![mission.join("asks.jsonl"), mission.join("misses.jsonl")],
        void: take("--void").map_or_else(|| state.join("void"), PathBuf::from),
        at: take("--at")
            .and_then(|at| at.parse().ok())
            .unwrap_or_else(crate::millis_now),
        offset_s: offset.unwrap_or(0),
        zone_known: offset.is_some(),
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
    let said = format!(
        "pain-points: {} pains heard and {} routed in the hour, {} turns stalled him, {} open \
         in the day, {} status asks today{}{}",
        hour.heard,
        hour.routed,
        hour.stalled,
        hour.open,
        hour.status_asks,
        hour.worst.as_ref().map_or(String::new(), |_| format!(
            "; the most hit, {} times",
            hour.worst_hits
        )),
        hour.troubles
            .iter()
            .map(|t| ["; ", t].concat())
            .collect::<String>()
    );
    Ok((said, !hour.troubles.is_empty()))
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
        .filter(|h: &Heard| h.at >= ears.at - heard::KEPT_MS)
        .collect();
    let ledgers: Vec<String> = ears
        .ledgers
        .iter()
        .flat_map(|path| heard::ledger(&fs::read_to_string(path).unwrap_or_default()))
        .collect();
    let cells = route::cells(cells_printed);
    // A pain the keeper has carried since it was heard is closed, though
    // it was open on an earlier run.
    for h in &mut kept {
        if !h.carried && !stalled::is(h) && h.at >= ears.at - DAY_MS {
            h.carried = heard::carried(&h.words, &ledgers);
        }
    }

    hear_logs(ears, &mut following, &mut kept, &ledgers, since);
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
    note_for_keeper(ears, &routes, &cells, &kept, &owned, &mut guarded);
    keep(&heard_at, &kept)?;
    // A run that took UTC for want of his zone would split his days at
    // the wrong hour, so it counts none and waits for one that knows.
    let mut counted = Vec::new();
    if !ears.zone_known {
        counted.push(
            "his time zone couldn't be read, so status asks are counted from UTC midnight and \
             no day was counted"
                .into(),
        );
    } else if let Some(keeper) = routes.ledgers.clone().filter(|_| routes.sends()) {
        counted = day::count(&ears.state, &kept, ears.at, ears.offset_s, &mut |text| {
            guarded(&keeper, text)
        });
    }
    let mut hour = hour_of(&kept, ears.at);
    hour.status_asks = day::asked_since(&kept, crate::local_day(ears.at, ears.offset_s).1);
    let (waiting, given_up) = unsent(&kept, ears.at);
    hour.troubles = troubles(
        &ears.state,
        &routes,
        (failed, given_up),
        cells.is_empty() && waiting,
    );
    hour.troubles.extend(counted);
    crate::write_whole(
        &ears.state.join("hour.json"),
        &serde_json::to_string(&hour).map_err(|e| e.to_string())?,
    )?;
    Ok(hour)
}

/// Hears what's new in each session log written since `since`, keeping
/// each pain he said, and forgets the logs no longer there.
fn hear_logs(
    ears: &Ears,
    following: &mut BTreeMap<String, Following>,
    kept: &mut Vec<Heard>,
    ledgers: &[String],
    since: i64,
) {
    let mut present = Vec::new();
    for (name, path) in logs(&ears.sessions, since) {
        present.push(name.clone());
        let log = following.entry(name).or_default();
        for said in follow(&path, &ears.void, log) {
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
            });
        }
    }
    following.retain(|name, _| present.contains(name));
}

/// Whether any fresh pain waits to be sent, since a node naming no
/// sessions is a trouble only then, and how many fresh ones were given up
/// on, their owner's tries spent. Past fresh, a pain given up on is said
/// only in its day's count, so a short outage of the node fails the
/// listener no longer than a pain stays fresh. One heard late waits for
/// no one, since it goes only into its day's count.
fn unsent(kept: &[Heard], at: i64) -> (bool, usize) {
    let unsent = || {
        kept.iter()
            .filter(move |h| h.routed_at.is_none() && h.at >= at - FRESH_MS)
    };
    let waiting = unsent().any(|h| !h.carried && h.tries < SEND_TRIES);
    let given_up = unsent().filter(|h| h.tries >= SEND_TRIES).count();
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

/// What kept a run from routing as it should, so a broken listener doesn't
/// read as a quiet hour: sends that failed this run, and pains given up on.
fn troubles(
    state: &Path,
    routes: &Routes,
    (failed, given_up): (usize, usize),
    no_cells: bool,
) -> Vec<String> {
    let mut troubles: Vec<String> = Routes::trouble(state).into_iter().collect();
    troubles.extend(refused(routes));
    if failed > 0 {
        troubles.push(format!("{failed} sends failed"));
    }
    if given_up > 0 {
        troubles.push(format!(
            "gave up on {given_up} pains the node never took, after {SEND_TRIES} sends each"
        ));
    }
    if routes.sends() && no_cells {
        troubles.push("the mycelium named no sessions, so none could be routed".into());
    }
    troubles
}

/// What the routes say that keeps a run from sending as it should: no
/// cell named never to send to, or a fallback or keeper that is one.
fn refused(routes: &Routes) -> Vec<String> {
    let mut troubles = Vec::new();
    if routes.from.is_some() && routes.never.is_empty() {
        troubles.push("routes.json names no cell never to send to, so nothing was sent".into());
    }
    for (what, to) in [("fallback", &routes.fallback), ("keeper", &routes.ledgers)] {
        if let Some(to) = to.as_ref().filter(|to| routes.refuses(to)) {
            troubles.push(format!(
                "routes.json's {what} {to} is one never to send to, so what goes to it waits"
            ));
        }
    }
    troubles
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

/// Sends the ledgers' keeper the pains just sent to their owners: only
/// what's new, each already routed, as the keeper asked. A stall is no
/// ask or miss of his, so it isn't noted.
fn note_for_keeper(
    ears: &Ears,
    routes: &Routes,
    cells: &BTreeMap<String, Cell>,
    kept: &[Heard],
    owned: &[String],
    send: &mut dyn FnMut(&str, &str) -> bool,
) {
    let Some(keeper) = routes.ledgers.as_ref().filter(|_| routes.sends()) else {
        return;
    };
    let noted: Vec<&Heard> = kept
        .iter()
        .filter(|h| {
            // The keeper isn't told again of what it was itself just sent.
            let to_keeper =
                h.to.as_ref()
                    .is_some_and(|to| to.eq_ignore_ascii_case(keeper));
            owned.contains(&h.key) && !stalled::is(h) && !to_keeper
        })
        .collect();
    if noted.is_empty() {
        return;
    }
    // The note is said once: a keeper that misses it still has each from
    // its owner's side, heard.jsonl, and the day's count.
    send(keeper, &route::ledger_note(&noted, cells, ears.offset_s));
}

/// The hour as cadence's card says it.
fn hour_of(kept: &[Heard], at: i64) -> Hour {
    let in_hour = |t: i64| t >= at - HOUR_MS;
    let said: Vec<&Heard> = kept.iter().filter(|h| !stalled::is(h)).collect();
    let worst = said
        .iter()
        .filter(|h| h.at >= at - DAY_MS && !h.carried)
        .max_by_key(|h| (h.hits, h.at));
    Hour {
        at,
        heard: said.iter().filter(|h| in_hour(h.at)).count(),
        stalled: kept
            .iter()
            .filter(|h| stalled::is(h) && in_hour(h.at))
            .count(),
        routed: kept
            .iter()
            .filter(|h| h.routed_at.is_some_and(in_hour))
            .count(),
        open: said
            .iter()
            .filter(|h| h.at >= at - DAY_MS && !h.carried)
            .count(),
        worst: worst.map(|h| h.words.clone()),
        worst_hits: worst.map_or(0, |h| h.hits),
        status_asks: 0,
        troubles: Vec::new(),
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
/// he said in it. A log shorter than where it was left is read anew.
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

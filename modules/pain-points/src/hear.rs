//! `pain-points hear`: the listener. Each run reads what's new in pi's
//! session logs, hears each message Joel said that says something hurt
//! (sore.rs), keeps it with his words and what the agent had just done
//! (said.rs), and sends each pain no one has yet as a finding to the cell
//! that was working on it (route.rs), once, ranked by how often he hits
//! the same thing (heard.rs). His orchestrator's asks and misses ledgers
//! are read, never written: a pain they carry goes to no one again, and
//! the keeper of the ledgers hears of each new one. Nothing is ever sent
//! to the void, and nothing is posted anywhere.
//!
//! Unlike `read`, which keeps kinds and timings alone, `hear` keeps his
//! words, since a finding without them can't be acted on. They stay in
//! its own state (heard.jsonl, a fortnight of it) and go only over the
//! machine's own mycelium socket to the cell that owns what hurt.
//!
//! It writes hour.json each run, for cadence's hourly card: the pains
//! heard and routed in the last hour, those of the last day still open,
//! and the one he hit most.

pub mod heard;
pub mod route;
pub mod said;
pub mod sore;

use heard::Heard;
use route::{Cell, Routes};
use said::Carried;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

/// How far back a first run hears, and how long a pain stays open.
pub const DAY_MS: i64 = 86_400_000;
const HOUR_MS: i64 = 3_600_000;

/// A pain first heard longer than this after he said it goes to the
/// ledgers' keeper alone: its cell has moved on, and a first run over a
/// day of logs would otherwise send every cell a day of old pains.
pub const FRESH_MS: i64 = 2 * HOUR_MS;

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
}

/// Hears what's new since the last run, sends each new pain to its owner
/// through the mycelium, and says the hour in a line for the journal.
///
/// # Errors
///
/// When its state can't be written.
pub fn listen(home: &Path, given: &mut BTreeMap<&'static str, String>) -> Result<String, String> {
    let mut take = |flag: &str| given.remove(flag);
    let state = crate::state_home();
    let mission = take("--ledgers").map_or_else(|| state.join("mission"), PathBuf::from);
    let mycelium = take("--mycelium").map_or_else(|| "mycelium".into(), PathBuf::from);
    let date = take("--date").map_or_else(|| "date".into(), PathBuf::from);
    let ears = Ears {
        sessions: take("--sessions").map_or_else(|| home.join(".pi/agent/sessions"), PathBuf::from),
        state: state.join("pain-points"),
        ledgers: vec![mission.join("asks.jsonl"), mission.join("misses.jsonl")],
        void: take("--void").map_or_else(|| state.join("void"), PathBuf::from),
        at: take("--at")
            .and_then(|at| at.parse().ok())
            .unwrap_or_else(crate::millis_now),
        offset_s: crate::output(Command::new(&date).arg("+%z"))
            .as_deref()
            .and_then(crate::offset_of)
            .unwrap_or(0),
    };
    let cells = crate::output(Command::new(&mycelium).arg("sessions")).unwrap_or_default();
    let from = Routes::read(&ears.state).from;
    let hour = hear(&ears, &cells, &mut |to, text| {
        let Some(from) = &from else { return false };
        Command::new(&mycelium)
            .args([
                "send", "--from", from, "--intent", "finding", "--text", text,
            ])
            .arg(format!("peer.{}", to.to_lowercase()))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    })?;
    Ok(format!(
        "pain-points: {} pains heard and {} routed in the hour, {} open in the day{}",
        hour.heard,
        hour.routed,
        hour.open,
        hour.worst.as_ref().map_or(String::new(), |_| format!(
            "; the most hit, {} times",
            hour.worst_hits
        ))
    ))
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
    let ledgers: Vec<(String, bool)> = ears
        .ledgers
        .iter()
        .flat_map(|path| heard::ledger(&fs::read_to_string(path).unwrap_or_default()))
        .collect();
    let cells = route::cells(cells_printed);

    let mut present = Vec::new();
    for (name, path) in logs(&ears.sessions, since) {
        present.push(name.clone());
        let log = following.entry(name).or_default();
        for said in follow(&path, &ears.void, log) {
            let key = format!("{}/{}", log.session, said.entry);
            let kinds = sore::kinds(&said.words, said.at, &said.before);
            if log.lab || said.at < since || kinds.is_empty() || kept.iter().any(|h| h.key == key) {
                continue;
            }
            let hits = heard::hits(&said.words, &kept);
            kept.push(Heard {
                key,
                session: log.session.clone(),
                entry: said.entry,
                at: said.at,
                void: log.void,
                carried: heard::carried(&said.words, &ledgers),
                words: said.words,
                kinds: kinds.iter().map(|k| (*k).to_string()).collect(),
                looking_at: said.looking_at,
                tools: said.tools,
                failed: said.failed,
                hits,
                to: None,
                routed_at: None,
            });
        }
    }
    following.retain(|name, _| present.contains(name));

    let owned = route_waiting(ears, &routes, &cells, &mut kept, send);
    note_for_keeper(ears, &routes, &cells, &mut kept, &owned, send);

    let lines: String = kept
        .iter()
        .filter_map(|h| serde_json::to_string(h).ok())
        .map(|line| line + "\n")
        .collect();
    crate::write_whole(&heard_at, &lines)?;
    crate::write_whole(
        &following_at,
        &serde_json::to_string(&following).map_err(|e| e.to_string())?,
    )?;
    let hour = hour_of(&kept, ears.at);
    crate::write_whole(
        &ears.state.join("hour.json"),
        &serde_json::to_string(&hour).map_err(|e| e.to_string())?,
    )?;
    Ok(hour)
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
) -> Vec<String> {
    // With no one named to send as, nothing is sent: a listener nobody
    // set up hears and counts alone.
    if routes.from.is_none() {
        return Vec::new();
    }
    let mut by_owner: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (n, pain) in kept.iter().enumerate() {
        if pain.carried || pain.routed_at.is_some() || pain.at < ears.at - FRESH_MS {
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
        }
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
) {
    let (Some(keeper), Some(_)) = (&routes.ledgers, &routes.from) else {
        return;
    };
    let late: Vec<usize> = (0..kept.len())
        .filter(|n| {
            let pain = &kept[*n];
            !pain.carried
                && pain.routed_at.is_none()
                && pain.at >= ears.at - DAY_MS
                && pain.at < ears.at - FRESH_MS
        })
        .collect();
    if owned.is_empty() && late.is_empty() {
        return;
    }
    let noted: Vec<&Heard> = kept
        .iter()
        .enumerate()
        .filter(|(n, h)| owned.contains(&h.key) || late.contains(n))
        .map(|(_, h)| h)
        .collect();
    // The note of pains already sent is said once: a keeper that misses it
    // still has each from its owner's side, and heard.jsonl. Late ones
    // wait for the next run until the note goes.
    if send(keeper, &route::ledger_note(&noted, cells, ears.offset_s)) {
        for n in late {
            kept[n].to = Some(keeper.clone());
            kept[n].routed_at = Some(ears.at);
        }
    }
}

/// The hour as cadence's card says it.
fn hour_of(kept: &[Heard], at: i64) -> Hour {
    let in_hour = |t: i64| t >= at - HOUR_MS;
    let worst = kept
        .iter()
        .filter(|h| h.at >= at - DAY_MS && !h.carried)
        .max_by_key(|h| (h.hits, h.at));
    Hour {
        at,
        heard: kept.iter().filter(|h| in_hour(h.at)).count(),
        routed: kept
            .iter()
            .filter(|h| h.routed_at.is_some_and(in_hour))
            .count(),
        open: kept
            .iter()
            .filter(|h| h.at >= at - DAY_MS && !h.carried)
            .count(),
        worst: worst.map(|h| h.words.clone()),
        worst_hits: worst.map_or(0, |h| h.hits),
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

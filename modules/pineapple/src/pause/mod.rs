//! A stop on everything that Joel says himself, for a time when nothing of
//! the agents' may run, as through a demo: `pineapple stop all`.
//!
//! The stop's record parks every session and holds every write, as any
//! stop on everything does (the harness's pineapple-workflow). What a
//! record can't reach is the machine's own: agents' starts, their timers,
//! and the work they left running. So the stop also refuses agents' starts
//! (`mycelium starts stop`), stops the timers the machine names as
//! agents', freezes each unit of agents' work in the jobs slice, and asks
//! each other machine running a session to stop the same way. The jobs
//! slice's awaited part is left running, since that is where Joel's own
//! terminal puts its busy work (../slices/jobs-sorter).
//!
//! What was paused is written beside the stops, under `paused/`, before
//! any of it is done, so a resume undoes exactly that and no more: a timer
//! that wasn't running stays stopped, and starts Joel had refused already
//! stay refused. Undoing a step that never happened does no harm, so a
//! stop cut short is still resumed whole, and a step a resume couldn't
//! take is tried again each minute until it can. A unit the machine names
//! to keep is never frozen, such as the compile cache Joel's own builds
//! read, or anything started for the demo itself; and while the stop
//! stands, [`tend`] freezes agents' work started since, as `pineapple
//! serve` asks it to each time it looks.

mod cgroup;
mod reach;
mod system;
mod units;

use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

pub use cgroup::agents_cgroup;
pub use reach::{script, session_ids, sessions};
pub use system::{Runs, System};

use reach::Reached;

use crate::record;
use crate::resume;

/// The folder under the state folder where what each stop paused is kept.
pub const PAUSED: &str = "paused";

/// How long a resume that couldn't take a step waits to try it again.
const RETRY_MS: u64 = 60_000;

/// What a machine pauses for a stop on everything, as its config names it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    /// systemd's own command, run as `--user`.
    pub systemctl: PathBuf,
    /// mycelium's command, for starts and the other machines; none, neither.
    pub mycelium: Option<PathBuf>,
    /// The agents' timers, as names or systemd's patterns (`cadence*.timer`).
    pub timers: Vec<String>,
    /// Other machines that stop with this one, by mycelium's names, beside
    /// each it lists a session on.
    pub machines: Vec<String>,
    /// The realms this machine's mycelium joins, each with agents of its
    /// own; none, mycelium's default alone.
    pub realms: Vec<String>,
    /// The units never frozen, as names or patterns (`*demo*`).
    pub keep: Vec<String>,
}

impl Plan {
    /// The plan in `text`, the JSON a machine's config writes.
    ///
    /// # Errors
    ///
    /// What the text lacks, or why it isn't JSON.
    pub fn from_json(text: &str) -> Result<Plan, String> {
        let value: Value = serde_json::from_str(text).map_err(|error| error.to_string())?;
        let systemctl = value
            .get("systemctl")
            .and_then(Value::as_str)
            .ok_or("it names no systemctl")?;
        Ok(Plan {
            systemctl: PathBuf::from(systemctl),
            mycelium: value
                .get("mycelium")
                .and_then(Value::as_str)
                .map(PathBuf::from),
            timers: names(&value, "timers"),
            machines: names(&value, "machines"),
            realms: names(&value, "realms"),
            keep: names(&value, "keep"),
        })
    }
}

/// How one machine took a stop or its resume: done, or why not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Took {
    /// The machine, by its name.
    pub machine: String,
    /// Each step it couldn't take, said plainly; none when all were taken.
    pub failed: Vec<String>,
    /// Whether the stop's record stands there, so its sessions park.
    pub recorded: bool,
}

/// What [`tend`] did: the work it froze, and how each machine took a
/// resume it finished.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tended {
    /// Agents' work started since the stop, frozen now.
    pub froze: Vec<String>,
    /// How each machine took a resume.
    pub took: Vec<Took>,
}

/// A stop on everything said by Joel at `host`, at `at` in milliseconds.
#[must_use]
pub fn record(id: &str, host: &str, at: u64) -> Value {
    json!({
        "id": id,
        "shadow": false,
        "said_by": "joel",
        "where": format!("pineapple stop all on {host}"),
        "words": "pineapple stop all",
        "heard_at": at,
        "by_joel": true,
        "reaches": ["everything"],
        "fell_back": false,
        "resumed": Value::Null,
    })
}

/// The stop's id, named for when it was said.
#[must_use]
pub fn id(at: u64) -> String {
    format!("stop-all-{at}")
}

/// Says the stop `id` on this machine, `host`, and on each other machine
/// it reaches, under the state folder `state`: its record, then what it
/// pauses, written down before any of it is done.
///
/// # Errors
///
/// Why the record or what it paused couldn't be written; the steps after
/// are taken whether or not each succeeds, and each one that fails is
/// said in the answer.
pub fn stop(
    plan: &Plan,
    state: &Path,
    id: &str,
    host: &str,
    at: u64,
    runs: &mut dyn Runs,
) -> io::Result<Vec<Took>> {
    let _held = lock(state)?;
    let said = record(id, host, at);
    record::keep(&state.join("stops"), id, &said)?;
    let mut paused = json!({
        "id": id,
        "at": at,
        "timers": [],
        "froze": [],
        "machines": [],
        "starts": [],
        "undone": Value::Null,
        "retry_at": Value::Null,
    });
    // Written at once, so a second stop finds this one standing.
    write_paused(state, id, &paused)?;
    let mut failed = Vec::new();
    let timers = units::active_timers(plan, runs).unwrap_or_else(|why| {
        failed.push(couldnt("list agents' timers", &why));
        Vec::new()
    });
    let work: Vec<String> = units::agents_work(plan, runs).map_or_else(
        |why| {
            failed.push(couldnt("list agents' work", &why));
            Vec::new()
        },
        |work| work.into_iter().map(|unit| unit.id).collect(),
    );
    let machines = reach::others(plan, host, runs, &mut failed);
    paused["timers"] = json!(timers);
    paused["froze"] = json!(work);
    paused["machines"] = json!(machines.iter().map(Reached::kept).collect::<Vec<_>>());
    write_paused(state, id, &paused)?;
    paused["starts"] = json!(reach::stop_starts(plan, runs, &mut failed));
    write_paused(state, id, &paused)?;
    if !timers.is_empty() {
        let asked = units::user(&["stop"], &timers);
        step(
            runs,
            &plan.systemctl,
            &asked,
            "stop agents' timers",
            &mut failed,
        );
    }
    let running = units::settle(plan, runs, "freeze", &work);
    if !running.is_empty() {
        failed.push(format!("couldn't freeze {}", running.join(", ")));
    }
    let mut took = vec![Took {
        machine: host.to_owned(),
        failed,
        recorded: true,
    }];
    for machine in &machines {
        took.push(reach::elsewhere(plan, runs, machine, &said, "stop"));
    }
    Ok(took)
}

/// Undoes what the stop `id` paused here and on each machine it reached,
/// once its record is resumed, and marks what it paused undone; what it
/// couldn't undo is kept, to be tried again after a minute.
fn undo(
    plan: &Plan,
    state: &Path,
    paused: &mut Value,
    host: &str,
    at: u64,
    runs: &mut dyn Runs,
) -> io::Result<Vec<Took>> {
    let id = paused["id"].as_str().unwrap_or_default().to_owned();
    let (froze, timers, starts) = (
        names(paused, "froze"),
        names(paused, "timers"),
        names(paused, "starts"),
    );
    let machines: Vec<Reached> = paused["machines"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Reached::from_kept)
        .collect();
    let mut failed = Vec::new();
    let frozen = units::settle(plan, runs, "thaw", &froze);
    if !frozen.is_empty() {
        failed.push(format!("couldn't thaw {}", frozen.join(", ")));
    }
    let asked = units::user(&["start"], &timers);
    let timers_left = !timers.is_empty()
        && !step(
            runs,
            &plan.systemctl,
            &asked,
            "start agents' timers",
            &mut failed,
        );
    let starts_left = reach::resume_starts(plan, runs, &starts, &mut failed);
    let mut took = vec![Took {
        machine: host.to_owned(),
        failed,
        recorded: true,
    }];
    let said = fs::read_to_string(state.join("stops").join(format!("{}.json", file_of(&id))))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let mut machines_left = Vec::new();
    for machine in &machines {
        let one = match &said {
            Some(said) => reach::elsewhere(plan, runs, machine, said, "resume"),
            None => Took {
                machine: machine.machine.clone(),
                failed: vec!["its stop's record here can't be read".to_owned()],
                recorded: false,
            },
        };
        if !one.failed.is_empty() {
            machines_left.push(machine.kept());
        }
        took.push(one);
    }
    if took.iter().all(|one| one.failed.is_empty()) {
        paused["undone"] = json!(at);
        paused["retry_at"] = Value::Null;
    } else {
        paused["froze"] = json!(frozen);
        paused["timers"] = json!(if timers_left { timers } else { Vec::new() });
        paused["starts"] = json!(starts_left);
        paused["machines"] = json!(machines_left);
        paused["retry_at"] = json!(at + RETRY_MS);
    }
    write_paused(state, &id, paused)?;
    Ok(took)
}

/// What `pineapple serve` does each time it looks, unless a stop or a
/// resume is under way: while a stop-all stands, freezes agents' work and
/// stops agents' timers started since, adding them to what it paused so
/// its resume undoes them too; once its record is resumed, however that
/// was, undoes what it paused, as [`settle`] does, but never while a
/// newer stop-all stands, whose hold that undo would let go.
///
/// # Errors
///
/// Why what a stop paused couldn't be read or written.
pub fn tend(
    plan: &Plan,
    state: &Path,
    host: &str,
    at: u64,
    runs: &mut dyn Runs,
) -> io::Result<Tended> {
    let Some(_held) = try_lock(state)? else {
        return Ok(Tended::default());
    };
    let standing = standing_ids(state);
    let held = stands(state).is_some();
    let mut tended = Tended::default();
    for (_, mut paused) in pauses(state) {
        let id = paused["id"].as_str().unwrap_or_default().to_owned();
        if !paused["undone"].is_null() {
            continue;
        }
        if standing.contains(&id) {
            tended.froze.extend(hold(plan, state, &mut paused, runs)?);
        } else if !held && paused["retry_at"].as_u64().is_none_or(|retry| retry <= at) {
            tended
                .took
                .extend(undo(plan, state, &mut paused, host, at, runs)?);
        }
    }
    Ok(tended)
}

/// Freezes agents' work and stops agents' timers running again since the
/// stop `paused`, which a restart of the user's manager or a switch may
/// have started, and adds them to what it paused; answers those that are
/// new to it.
///
/// Every unit of agents' work is frozen again each time, not only those
/// systemd says run: the jobs sorter thaws a scope by writing its cgroup
/// itself, and systemd goes on saying it is frozen.
fn hold(
    plan: &Plan,
    state: &Path,
    paused: &mut Value,
    runs: &mut dyn Runs,
) -> io::Result<Vec<String>> {
    let listed: Vec<String> = units::agents_work(plan, runs)
        .unwrap_or_default()
        .into_iter()
        .map(|unit| unit.id)
        .collect();
    let froze = names(paused, "froze");
    let work: Vec<String> = listed
        .iter()
        .filter(|unit| !froze.contains(unit))
        .cloned()
        .collect();
    let timers = units::active_timers(plan, runs).unwrap_or_default();
    if work.is_empty() && timers.is_empty() {
        // What still runs is looked at again next time.
        let _ = units::settle(plan, runs, "freeze", &listed);
        return Ok(Vec::new());
    }
    let id = paused["id"].as_str().unwrap_or_default().to_owned();
    for (key, more) in [("froze", &work), ("timers", &timers)] {
        let mut all = names(paused, key);
        all.extend(
            more.iter()
                .filter(|one| !all.contains(one))
                .cloned()
                .collect::<Vec<_>>(),
        );
        paused[key] = json!(all);
    }
    write_paused(state, &id, paused)?;
    if !timers.is_empty() {
        let _ = runs.run(&plan.systemctl, &units::user(&["stop"], &timers));
    }
    // Written down first, so a resume thaws whatever this freezes.
    let _ = units::settle(plan, runs, "freeze", &listed);
    Ok(work.into_iter().chain(timers).collect())
}

/// Undoes what each stop-all under `state` paused once its record is
/// resumed, however that was: by Joel's command, or his Resume in the
/// launcher; waiting for a stop or a tend under way to finish. Answers
/// how each machine took it.
///
/// # Errors
///
/// Why what a stop paused couldn't be read or marked undone.
pub fn settle(
    plan: &Plan,
    state: &Path,
    host: &str,
    at: u64,
    runs: &mut dyn Runs,
) -> io::Result<Vec<Took>> {
    let _held = lock(state)?;
    // An older stop's undo would let go what a newer one, standing, holds.
    if stands(state).is_some() {
        return Ok(Vec::new());
    }
    let standing = standing_ids(state);
    let mut took = Vec::new();
    for (_, mut paused) in pauses(state) {
        let id = paused["id"].as_str().unwrap_or_default().to_owned();
        if paused["undone"].is_null() && !standing.contains(&id) {
            took.extend(undo(plan, state, &mut paused, host, at, runs)?);
        }
    }
    Ok(took)
}

/// What each stop-all resumed, but not yet undone, still holds, as Joel
/// is told it: what a tend that took the undo from his resume, or one
/// that couldn't finish it, left to be tried again.
#[must_use]
pub fn left(state: &Path) -> Vec<String> {
    let standing = standing_ids(state);
    let mut left = Vec::new();
    for (_, paused) in pauses(state) {
        let id = paused["id"].as_str().unwrap_or_default();
        if !paused["undone"].is_null() || standing.iter().any(|one| one == id) {
            continue;
        }
        for (key, held) in [("froze", "frozen"), ("timers", "stopped")] {
            let units = names(&paused, key);
            if !units.is_empty() {
                left.push(format!("{} {held}", units.join(", ")));
            }
        }
        if !names(&paused, "starts").is_empty() {
            left.push("agents' starts refused".to_owned());
        }
        let machines: Vec<String> = paused["machines"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Reached::from_kept)
            .map(|one| one.machine)
            .collect();
        if !machines.is_empty() {
            left.push(format!("{} not yet resumed", machines.join(", ")));
        }
    }
    left
}

/// The machines whose sessions park for a stop that `took`, each once.
#[must_use]
pub fn recorded(took: &[Took]) -> Vec<&str> {
    let mut machines: Vec<&str> = Vec::new();
    for one in took.iter().filter(|one| one.recorded) {
        if !machines.contains(&one.machine.as_str()) {
            machines.push(&one.machine);
        }
    }
    machines
}

/// The one line Joel is told of a stop or its resume, `act` as
/// "stopping" or "resuming": how many sessions, on which machines, and
/// each step a machine couldn't take, with why.
#[must_use]
pub fn said(act: &str, took: &[Took], sessions: Option<usize>) -> String {
    let reached = recorded(took);
    let count = match sessions {
        Some(1) => "1 session".to_owned(),
        Some(count) => format!("{count} sessions"),
        None => "every session".to_owned(),
    };
    let mut line = if reached.is_empty() {
        format!("{act} nothing fully")
    } else {
        format!("{act} {count} on {}", and(&reached))
    };
    let after = if act == "resuming" {
        ", tried again each minute"
    } else {
        ""
    };
    for one in took.iter().filter(|one| !one.failed.is_empty()) {
        let so = if one.recorded || act == "resuming" {
            after
        } else {
            ", so its agents may run on"
        };
        line = format!("{line}; {} {}{so}", one.machine, one.failed.join(", "));
    }
    line
}

fn and(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => (*one).to_owned(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Whether a stop-all stands under `state`: one whose record isn't resumed
/// and whose pause isn't undone.
#[must_use]
pub fn stands(state: &Path) -> Option<String> {
    let standing = standing_ids(state);
    pauses(state)
        .into_iter()
        .filter(|(_, paused)| paused["undone"].is_null())
        .filter_map(|(_, paused)| paused["id"].as_str().map(str::to_owned))
        .find(|id| standing.contains(id))
}

fn standing_ids(state: &Path) -> Vec<String> {
    resume::standing(&state.join("stops"))
        .into_iter()
        .map(|stop| stop.id)
        .collect()
}

/// What each stop-all under `state` paused, from its `.json` files only.
fn pauses(state: &Path) -> Vec<(PathBuf, Value)> {
    let Ok(entries) = fs::read_dir(state.join(PAUSED)) else {
        return Vec::new();
    };
    let mut found: Vec<(PathBuf, Value)> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|kind| kind == "json"))
        .filter_map(|path| {
            let paused = serde_json::from_str::<Value>(&fs::read_to_string(&path).ok()?).ok()?;
            paused["id"].as_str()?;
            Some((path, paused))
        })
        .collect();
    found.sort_by(|a, b| a.0.cmp(&b.0));
    found
}

fn names(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

fn couldnt(what: &str, why: &str) -> String {
    let why = why.lines().next().unwrap_or("it failed").trim();
    format!("couldn't {what}: {why}")
}

/// Runs `program`, noting in `failed` that it couldn't `what` when it
/// fails; answers whether it ran.
fn step(
    runs: &mut dyn Runs,
    program: &Path,
    args: &[String],
    what: &str,
    failed: &mut Vec<String>,
) -> bool {
    match runs.run(program, args) {
        Ok(_) => true,
        Err(why) => {
            failed.push(couldnt(what, &why));
            false
        }
    }
}

/// The pause folder held, waiting for whoever holds it, so a stop, a
/// resume and serve's tending never write what a stop paused at once.
fn lock(state: &Path) -> io::Result<File> {
    let file = lock_file(state)?;
    file.lock()?;
    Ok(file)
}

/// The pause folder held, or none while another holds it.
fn try_lock(state: &Path) -> io::Result<Option<File>> {
    let file = lock_file(state)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(fs::TryLockError::WouldBlock) => Ok(None),
        Err(fs::TryLockError::Error(error)) => Err(error),
    }
}

fn lock_file(state: &Path) -> io::Result<File> {
    let folder = state.join(PAUSED);
    fs::create_dir_all(&folder)?;
    File::options()
        .create(true)
        .truncate(false)
        .write(true)
        .open(folder.join(".lock"))
}

fn paused_file(state: &Path, id: &str) -> PathBuf {
    state.join(PAUSED).join(format!("{}.json", file_of(id)))
}

fn write_paused(state: &Path, id: &str, paused: &Value) -> io::Result<()> {
    let path = paused_file(state, id);
    if let Some(folder) = path.parent() {
        fs::create_dir_all(folder)?;
    }
    let written = path.with_extension("new");
    fs::write(&written, format!("{paused:#}\n"))?;
    fs::rename(&written, &path)
}

/// An id as the file its record is kept in, as [`record::keep`] names it.
fn file_of(id: &str) -> String {
    id.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

/// `text` as one word for sh, in single quotes.
fn quoted(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

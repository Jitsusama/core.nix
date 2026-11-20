//! A stop on everything that Joel says himself, for a time when nothing of
//! the agents' may run, as through a demo: `pineapple stop all`.
//!
//! The stop's record parks every session and holds every write, as any
//! stop on everything does (the harness's pineapple-workflow). What a
//! record can't reach is the machine's own: agents' starts, their timers,
//! and the work they left running. So the stop also refuses agents' starts
//! (`mycelium starts stop`), stops the timers the machine names as
//! agents', freezes each unit of agents' work in the jobs slice, and asks
//! each other machine named to stop the same way. The jobs slice's
//! awaited part is left running, since that is where Joel's own terminal
//! puts its busy work (../slices/jobs-sorter).
//!
//! What was paused is written beside the stops, under `paused/`, before
//! any of it is done, so a resume undoes exactly that and no more: a timer
//! that wasn't running stays stopped. Undoing a step that never happened
//! does no harm, so a stop cut short is still resumed whole. A unit the
//! machine names to keep is never frozen, such as the compile cache
//! Joel's own builds read, or anything started for the demo itself; and
//! while the stop stands, [`hold`] freezes agents' work started since,
//! as `pineapple serve` asks it to each time it looks.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use crate::record;
use crate::resume;

/// The folder under the state folder where what each stop paused is kept.
pub const PAUSED: &str = "paused";

/// The slice agents' work runs in, and the part of it that is Joel's.
const JOBS: &str = "jobs.slice";
const JOBS_WORK: &str = "jobs-work";
const AWAITED: &str = "jobs-awaited.slice";

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
        let names = |key: &str| -> Vec<String> {
            value
                .get(key)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        };
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
            timers: names("timers"),
            machines: names("machines"),
            keep: names("keep"),
        })
    }
}

/// Runs a program to its end: what it printed, or why it failed.
pub trait Runs {
    /// Runs `program` with `args`, answering its output when it exits 0.
    ///
    /// # Errors
    ///
    /// What it said, or why it couldn't run, when it didn't exit 0.
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String>;
}

/// The machine's own programs, each given at most `limit` before it's
/// killed, so a machine that doesn't answer can't hold Joel's act up.
pub struct System {
    /// How long each program is given.
    pub limit: Duration,
}

/// How often a program running is looked at, to end it at its limit.
const LOOKED: Duration = Duration::from_millis(50);

impl Runs for System {
    fn run(&mut self, program: &Path, args: &[String]) -> Result<String, String> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| format!("{} can't run: {error}", program.to_string_lossy()))?;
        let began = Instant::now();
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if began.elapsed() >= self.limit => {
                    // Killed at its limit; it may have ended meanwhile,
                    // which is as good.
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(format!("no answer in {} s", self.limit.as_secs()));
                }
                Ok(None) => std::thread::sleep(LOOKED),
                Err(error) => return Err(error.to_string()),
            }
        }
        let output = child
            .wait_with_output()
            .map_err(|error| error.to_string())?;
        let out = String::from_utf8_lossy(&output.stdout).into_owned();
        if output.status.success() {
            Ok(out)
        } else {
            let err = String::from_utf8_lossy(&output.stderr);
            let said = if err.trim().is_empty() {
                out
            } else {
                err.into_owned()
            };
            Err(format!("{} ({})", said.trim(), output.status))
        }
    }
}

/// How one machine took a stop or its resume: done, or why not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Took {
    /// The machine, by its name.
    pub machine: String,
    /// Each step it couldn't take, said plainly; none when all were taken.
    pub failed: Vec<String>,
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

/// Says the stop `id` on this machine, `host`, and on each machine the plan
/// names, under the state folder `state`: its record, then what it pauses,
/// written down before any of it is done.
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
    let said = record(id, host, at);
    record::keep(&state.join("stops"), id, &said)?;
    let timers = active_timers(plan, runs);
    let work = agents_work(plan, runs);
    let machines = others(plan, host, runs);
    let paused = json!({
        "id": id,
        "at": at,
        "timers": timers,
        "froze": work,
        "machines": machines,
        "undone": Value::Null,
    });
    write_paused(state, id, &paused)?;
    let mut failed = Vec::new();
    if let Some(mycelium) = &plan.mycelium {
        step(
            runs,
            mycelium,
            &["starts", "stop"],
            "refuse agents' starts",
            &mut failed,
        );
    }
    if !timers.is_empty() {
        let args = user(&["stop"], &timers);
        step_with(
            runs,
            &plan.systemctl,
            &args,
            "stop agents' timers",
            &mut failed,
        );
    }
    if !work.is_empty() {
        let args = user(&["freeze"], &work);
        step_with(
            runs,
            &plan.systemctl,
            &args,
            "freeze agents' work",
            &mut failed,
        );
    }
    let mut took = vec![Took {
        machine: host.to_owned(),
        failed,
    }];
    for machine in &machines {
        took.push(elsewhere(plan, runs, machine, &said, "stop"));
    }
    Ok(took)
}

/// The other machines a stop reaches: those the plan names, and each one
/// mycelium lists a session on, since a machine running a session runs
/// agents; so no machine's name need be written down for it to stop.
fn others(plan: &Plan, host: &str, runs: &mut dyn Runs) -> Vec<String> {
    let mut machines = plan.machines.clone();
    let listed = plan
        .mycelium
        .as_ref()
        .and_then(|mycelium| runs.run(mycelium, &["sessions".to_owned()]).ok())
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    let sessions = listed
        .as_ref()
        .and_then(|value| value.pointer("/Ok/Sessions"))
        .and_then(Value::as_array);
    for machine in sessions
        .into_iter()
        .flatten()
        .filter_map(|one| one["machine"].as_str())
    {
        if machine != host && !machines.iter().any(|named| named == machine) {
            machines.push(machine.to_owned());
        }
    }
    machines
}

/// Undoes what the stop `id` paused here and on each machine it reached,
/// once its record is resumed, and marks what it paused undone; a stop
/// that paused nothing, as one a reader heard, answers none.
///
/// # Errors
///
/// Why what it paused couldn't be read or marked undone.
pub fn undo(
    plan: &Plan,
    state: &Path,
    id: &str,
    host: &str,
    at: u64,
    runs: &mut dyn Runs,
) -> io::Result<Vec<Took>> {
    let path = paused_file(state, id);
    let Ok(text) = fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let mut paused: Value = serde_json::from_str(&text)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if !paused.get("undone").is_none_or(Value::is_null) {
        return Ok(Vec::new());
    }
    let names = |key: &str| -> Vec<String> {
        paused
            .get(key)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    };
    let (froze, timers, machines) = (names("froze"), names("timers"), names("machines"));
    let mut failed = Vec::new();
    if !froze.is_empty() {
        let args = user(&["thaw"], &froze);
        step_with(
            runs,
            &plan.systemctl,
            &args,
            "thaw agents' work",
            &mut failed,
        );
    }
    if !timers.is_empty() {
        let args = user(&["start"], &timers);
        step_with(
            runs,
            &plan.systemctl,
            &args,
            "start agents' timers",
            &mut failed,
        );
    }
    if let Some(mycelium) = &plan.mycelium {
        step(
            runs,
            mycelium,
            &["starts", "resume"],
            "let agents start",
            &mut failed,
        );
    }
    let mut took = vec![Took {
        machine: host.to_owned(),
        failed,
    }];
    let said = fs::read_to_string(state.join("stops").join(format!("{}.json", file_of(id))))
        .ok()
        .and_then(|text| serde_json::from_str::<Value>(&text).ok());
    for machine in &machines {
        match &said {
            Some(said) => took.push(elsewhere(plan, runs, machine, said, "resume")),
            None => took.push(Took {
                machine: machine.clone(),
                failed: vec!["its stop's record here can't be read".to_owned()],
            }),
        }
    }
    if let Some(fields) = paused.as_object_mut() {
        fields.insert("undone".to_owned(), json!(at));
    }
    write_paused(state, id, &paused)?;
    Ok(took)
}

/// The script a machine runs for `act`, stop or resume, of the stop
/// `said`: its record written whole into that machine's stops, then its
/// starts refused or let go again.
#[must_use]
pub fn script(said: &Value, act: &str) -> String {
    let id = said.get("id").and_then(Value::as_str).unwrap_or_default();
    let starts = if act == "resume" { "resume" } else { "stop" };
    format!(
        "set -e\n\
         d=\"${{XDG_STATE_HOME:-$HOME/.local/state}}/pineapple/stops\"\n\
         mkdir -p \"$d\"\n\
         printf '%s\\n' {record} > \"$d/{file}.new\"\n\
         mv \"$d/{file}.new\" \"$d/{file}.json\"\n\
         mycelium starts {starts}\n",
        record = quoted(&said.to_string()),
        file = file_of(id),
    )
}

/// The stop `said` asked of `machine` through mycelium, for `act`.
fn elsewhere(plan: &Plan, runs: &mut dyn Runs, machine: &str, said: &Value, act: &str) -> Took {
    let Some(mycelium) = &plan.mycelium else {
        return Took {
            machine: machine.to_owned(),
            failed: vec!["this machine names no mycelium to reach it".to_owned()],
        };
    };
    let args = [
        "run".to_owned(),
        "--need".to_owned(),
        format!("machine={machine}"),
        "--".to_owned(),
        "sh".to_owned(),
        "-c".to_owned(),
        script(said, act),
    ];
    let mut failed = Vec::new();
    step_with(runs, mycelium, &args, &format!("{act} there"), &mut failed);
    Took {
        machine: machine.to_owned(),
        failed,
    }
}

/// Freezes the agents' work started since each stop-all standing under
/// `state` paused, and adds it to what that stop paused, so its resume
/// thaws it too; answers the units it froze.
///
/// # Errors
///
/// Why what a stop paused couldn't be written again.
pub fn hold(plan: &Plan, state: &Path, runs: &mut dyn Runs) -> io::Result<Vec<String>> {
    let Ok(entries) = fs::read_dir(state.join(PAUSED)) else {
        return Ok(Vec::new());
    };
    let mut standing = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.extension().is_none_or(|kind| kind != "json") {
            continue;
        }
        let Some(paused) = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        else {
            continue;
        };
        if paused.get("undone").is_none_or(Value::is_null) {
            standing.push(paused);
        }
    }
    let Some(mut paused) = standing.pop() else {
        return Ok(Vec::new());
    };
    let froze: Vec<String> = paused["froze"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let new: Vec<String> = agents_work(plan, runs)
        .into_iter()
        .filter(|unit| !froze.contains(unit))
        .collect();
    if new.is_empty() {
        return Ok(Vec::new());
    }
    let id = paused["id"].as_str().unwrap_or_default().to_owned();
    let mut all = froze;
    all.extend(new.iter().cloned());
    if let Some(fields) = paused.as_object_mut() {
        fields.insert("froze".to_owned(), json!(all));
    }
    write_paused(state, &id, &paused)?;
    let _ = runs.run(&plan.systemctl, &user(&["freeze"], &new));
    Ok(new)
}

/// Undoes what each stop-all under `state` paused once its stop no
/// longer stands, however it was resumed: by Joel's command, or his
/// Resume in the launcher. Answers how each machine took it.
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
    let Ok(entries) = fs::read_dir(state.join(PAUSED)) else {
        return Ok(Vec::new());
    };
    let standing: Vec<String> = resume::standing(&state.join("stops"))
        .into_iter()
        .map(|stop| stop.id)
        .collect();
    let mut took = Vec::new();
    for path in entries.flatten().map(|entry| entry.path()) {
        let Some(paused) = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
        else {
            continue;
        };
        let Some(id) = paused["id"].as_str() else {
            continue;
        };
        if paused.get("undone").is_none_or(Value::is_null) && !standing.iter().any(|s| s == id) {
            took.extend(undo(plan, state, id, host, at, runs)?);
        }
    }
    Ok(took)
}

/// How many sessions mycelium lists on the machines that took the stop,
/// or none when it can't say.
pub fn sessions(plan: &Plan, took: &[Took], runs: &mut dyn Runs) -> Option<usize> {
    let mycelium = plan.mycelium.as_ref()?;
    let listed = runs.run(mycelium, &["sessions".to_owned()]).ok()?;
    let value: Value = serde_json::from_str(&listed).ok()?;
    let sessions = value.pointer("/Ok/Sessions")?.as_array()?;
    Some(
        sessions
            .iter()
            .filter(|session| {
                let machine = session["machine"].as_str().unwrap_or_default();
                took.iter()
                    .any(|one| one.failed.is_empty() && one.machine == machine)
            })
            .count(),
    )
}

/// The one line Joel is told of a stop or its resume, `act` as
/// "stopping" or "resuming": how many sessions, on which machines, and
/// for each machine that refused, why and what to type there.
#[must_use]
pub fn said(act: &str, took: &[Took], sessions: Option<usize>) -> String {
    let took_it: Vec<&str> = took
        .iter()
        .filter(|one| one.failed.is_empty())
        .map(|one| one.machine.as_str())
        .collect();
    let count = match sessions {
        Some(1) => "1 session".to_owned(),
        Some(count) => format!("{count} sessions"),
        None => "every session".to_owned(),
    };
    let mut line = if took_it.is_empty() {
        format!("{act} nothing fully")
    } else {
        format!("{act} {count} on {}", and(&took_it))
    };
    let left = if act == "resuming" {
        "its agents stay stopped"
    } else {
        "its agents run on"
    };
    for one in took.iter().filter(|one| !one.failed.is_empty()) {
        line = format!(
            "{line}; {} couldn't: {}, so {left}",
            one.machine,
            one.failed.join(", "),
        );
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

/// Whether a stop-all stands under `state`: what one paused, not yet undone.
#[must_use]
pub fn stands(state: &Path) -> Option<String> {
    let entries = fs::read_dir(state.join(PAUSED)).ok()?;
    entries
        .flatten()
        .filter_map(|entry| fs::read_to_string(entry.path()).ok())
        .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
        .find(|paused| paused.get("undone").is_none_or(Value::is_null))
        .and_then(|paused| paused["id"].as_str().map(str::to_owned))
}

/// The plan's timers running now, by name, so a resume starts only those.
fn active_timers(plan: &Plan, runs: &mut dyn Runs) -> Vec<String> {
    if plan.timers.is_empty() {
        return Vec::new();
    }
    let args = user(
        &[
            "list-units",
            "--type=timer",
            "--state=active",
            "--plain",
            "--no-legend",
            "--no-pager",
        ],
        &plan.timers,
    );
    runs.run(&plan.systemctl, &args)
        .map(|listed| first_words(&listed))
        .unwrap_or_default()
}

/// Each unit of agents' work running now: every service and scope in the
/// jobs slice itself, and every slice of the jobs-work tiers, but not the
/// awaited part, which holds Joel's terminal's work, nor a unit the plan
/// keeps.
fn agents_work(plan: &Plan, runs: &mut dyn Runs) -> Vec<String> {
    let listed = user(
        &[
            "list-units",
            "--type=service,scope",
            "--state=running",
            "--plain",
            "--no-legend",
            "--no-pager",
        ],
        &[],
    );
    let Ok(units) = runs
        .run(&plan.systemctl, &listed)
        .map(|text| first_words(&text))
    else {
        return Vec::new();
    };
    if units.is_empty() {
        return Vec::new();
    }
    let shown = user(&["show", "--property=Id,Slice"], &units);
    let Ok(text) = runs.run(&plan.systemctl, &shown) else {
        return Vec::new();
    };
    let mut work = Vec::new();
    let mut tiers = Vec::new();
    for (unit, slice) in slices(&text) {
        if slice == JOBS {
            work.push(unit);
        } else if slice.starts_with(JOBS_WORK) && !tiers.contains(&JOBS_WORK) {
            tiers.push(JOBS_WORK);
        }
    }
    work.retain(|unit| unit != AWAITED && !plan.keep.iter().any(|kept| matches(kept, unit)));
    work.extend(tiers.into_iter().map(|tier| format!("{tier}.slice")));
    work
}

/// Whether `name` is `pattern`, where each `*` stands for any run of
/// characters.
fn matches(pattern: &str, name: &str) -> bool {
    let mut parts = pattern.split('*');
    let first = parts.next().unwrap_or_default();
    let Some(mut rest) = name.strip_prefix(first) else {
        return false;
    };
    let parts: Vec<&str> = parts.collect();
    let Some((last, middle)) = parts.split_last() else {
        return rest.is_empty();
    };
    for part in middle {
        let Some(at) = rest.find(part) else {
            return false;
        };
        rest = &rest[at + part.len()..];
    }
    rest.ends_with(last)
}

/// Each unit and its slice, from `systemctl show`'s blocks of `Id=` and
/// `Slice=` lines.
fn slices(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut id = None;
    for line in text.lines() {
        if let Some(unit) = line.strip_prefix("Id=") {
            id = Some(unit.to_owned());
        } else if let Some(slice) = line.strip_prefix("Slice=")
            && let Some(unit) = id.take()
        {
            found.push((unit, slice.to_owned()));
        }
    }
    found
}

fn first_words(listed: &str) -> Vec<String> {
    listed
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .map(str::to_owned)
        .collect()
}

fn user(act: &[&str], names: &[String]) -> Vec<String> {
    let mut args = vec!["--user".to_owned()];
    args.extend(act.iter().map(|word| (*word).to_owned()));
    args.extend(names.iter().cloned());
    args
}

fn step(runs: &mut dyn Runs, program: &Path, args: &[&str], what: &str, failed: &mut Vec<String>) {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    step_with(runs, program, &args, what, failed);
}

fn step_with(
    runs: &mut dyn Runs,
    program: &Path,
    args: &[String],
    what: &str,
    failed: &mut Vec<String>,
) {
    if let Err(why) = runs.run(program, args) {
        let why = why.lines().next().unwrap_or("it failed").trim().to_owned();
        failed.push(format!("couldn't {what}: {why}"));
    }
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

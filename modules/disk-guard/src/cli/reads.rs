//! The commands that read and answer: each guard's status, the journals,
//! the plan a pass would follow and why a path stays or would go. Each
//! answers text for Joel, or with `--json` the facts it read, naming its
//! schema.

use super::{Env, failed, read_config, said_path, wall};
use crate::band::Band;
use crate::daemon::{Explained, Guard, Places};
use crate::units::{Size, Span, shown as size};
use crate::words::span;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use terminal_kit::{Context, Exit, Failure, Format, Pen, Room, layout};

/// The bands from healthy to critically full, as each is typed.
const BANDS: [&str; 4] = ["healthy", "filling", "nearly-full", "critically-full"];

/// How many of the things a guard last cleared `status` shows; the
/// journal has the rest.
const CLEARED_SHOWN: usize = 3;

/// How many of the biggest holders `status` shows in words; the JSON has
/// the rest.
const HOLDERS_SHOWN: usize = 3;

/// How old a heartbeat may be before its guard is taken to have stopped.
const STOPPED_AFTER: f64 = 120.0;

/// The links to the systems running and booted, NixOS's own, whose
/// generations never expire.
pub(super) fn running() -> Vec<PathBuf> {
    ["/run/current-system", "/run/booted-system"]
        .map(PathBuf::from)
        .to_vec()
}

pub(super) fn status(env: &Env, context: &Context, out: &mut dyn Write) -> Result<(), Failure> {
    let read = |path: &Path| -> Option<Value> {
        serde_json::from_str(&fs::read_to_string(path).ok()?).ok()
    };
    let (user, system) = (read(&env.status), read(&env.system_status));
    if user.is_none() && system.is_none() {
        return Err(
            Failure::new(Exit::Unavailable, "no guard has written its status")
                .because("neither the account's guard nor the machine's is running")
                .try_this("systemctl --user start disk-guard"),
        );
    }
    if context.format().is_json() {
        // Each guard's status as it wrote it, which the desktop reads too.
        let both = json!({"schema": "disk-guard.status/1", "user": user, "system": system});
        return context.answer_to(out, &both).map_err(failed);
    }
    let now = wall();
    let guards: Vec<Vec<String>> = [("user", user), ("system", system)]
        .into_iter()
        .filter_map(|(name, status)| {
            Some(said_status(name, &status?, now, &context.out, context.room))
        })
        .map(|blocks| {
            layout::joined(blocks)
                .trim_end()
                .split('\n')
                .map(str::to_owned)
                .collect()
        })
        .collect();
    out.write_all(layout::joined(guards).as_bytes())
        .map_err(failed)
}

/// How far in a guard's status is set under its name.
const INSET: usize = 2;

/// One guard's status in words, as blocks: its name and when it was
/// heard from; what it says; each disk and its last pass, as facts; what
/// it cleared lately, by when; and what holds the most. Every block sits
/// two cells in, is set at the reading measure into a terminal, and
/// wraps under itself, so nothing folds at the terminal's edge; into a
/// pipe each line stays whole.
pub(super) fn said_status(
    name: &str,
    status: &Value,
    now: f64,
    pen: &Pen,
    room: Room,
) -> Vec<Vec<String>> {
    let prose = room.for_prose();
    let inner = prose.less(INSET);
    let inset = |lines: Vec<String>| -> Vec<String> {
        lines
            .into_iter()
            .map(|line| format!("{}{line}", " ".repeat(INSET)))
            .collect()
    };
    let age = now - status["heartbeat"].as_f64().unwrap_or(0.0);
    let heard = if age > STOPPED_AFTER {
        pen.role("warning", &format!("not heard from for {}", span(age)))
    } else {
        pen.role("muted", &format!("heard from {} ago", span(age)))
    };
    let mut blocks = vec![vec![format!(
        "{}  {heard}",
        pen.role("text", &format!("{name} guard"))
    )]];
    if let Some(says) = status["says"].as_str() {
        blocks.push(
            layout::paragraph(says, INSET, prose)
                .into_iter()
                .map(|line| pen.role("text", &line))
                .collect(),
        );
    }
    let disks = disk_facts(status, now);
    let set = |facts: &[(String, String)]| -> Vec<String> {
        let facts: Vec<(&str, Option<&str>)> = facts
            .iter()
            .map(|(label, value)| (label.as_str(), Some(value.as_str())))
            .collect();
        inset(layout::facts(&facts, "", pen, inner))
    };
    if !disks.is_empty() {
        blocks.push(set(&disks));
    }
    let cleared: Vec<(String, String)> = status["cleared"]
        .as_array()
        .into_iter()
        .flatten()
        .take(CLEARED_SHOWN)
        .map(|cleared| {
            let when = span(now - cleared["at"].as_f64().unwrap_or(0.0));
            (
                format!("{when} ago"),
                cleared["says"].as_str().unwrap_or("?").to_owned(),
            )
        })
        .collect();
    if !cleared.is_empty() {
        blocks.push(set(&cleared));
    }
    let holders: Vec<&str> = status["holders"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|holder| holder["says"].as_str())
        .take(HOLDERS_SHOWN)
        .collect();
    if !holders.is_empty() {
        let mut block = vec![format!(
            "{}{}",
            " ".repeat(INSET),
            pen.label("holds the most")
        )];
        for says in holders {
            // Each holder hangs: its first line at the label's inset, the
            // rest two further in, so where one ends and the next begins
            // reads at a glance.
            let hung = layout::hung(says, INSET + layout::GAP, prose);
            for (at, line) in hung.split('\n').enumerate() {
                let lead = if at == 0 { INSET } else { 0 };
                block.push(format!("{}{}", " ".repeat(lead), pen.role("text", line)));
            }
        }
        blocks.push(block);
    }
    blocks
}

/// Each disk a guard watches and its last pass, as facts: a disk's band,
/// what's free and when it reaches the next band, and the pass's when,
/// where, band, end and what it freed.
fn disk_facts(status: &Value, now: f64) -> Vec<(String, String)> {
    let mut disks: Vec<(String, String)> = Vec::new();
    for filesystem in status["filesystems"].as_array().into_iter().flatten() {
        let band = filesystem["band"].as_str().unwrap_or("unknown");
        let mut said = format!(
            "{band}{}{} free of {}",
            layout::ITEMS,
            size(filesystem["free"].as_u64().unwrap_or(0)),
            size(filesystem["total"].as_u64().unwrap_or(0)),
        );
        let next = BANDS
            .iter()
            .position(|named| *named == band)
            .and_then(|at| BANDS.get(at + 1));
        if let (Some(next), Some(forecast)) = (next, filesystem["forecast"].as_f64()) {
            // Writing to a String can't fail.
            _ = write!(said, "{}{next} in {}", layout::ITEMS, span(forecast));
        }
        disks.push((filesystem["path"].as_str().unwrap_or("?").to_owned(), said));
    }
    let last = &status["last"];
    if last.is_object() {
        let said = [
            format!(
                "{} ago on {}",
                span(now - last["at"].as_f64().unwrap_or(0.0)),
                last["path"].as_str().unwrap_or("?")
            ),
            last["band"].as_str().unwrap_or("?").to_owned(),
            last["end"].as_str().unwrap_or("?").to_owned(),
            format!("freed {}", size(last["freed"].as_u64().unwrap_or(0))),
        ];
        disks.push(("last pass".to_owned(), said.join(layout::ITEMS)));
    }
    disks
}

/// One line of a guard's journal, kept as it was written until it's said,
/// so a long journal is held as its text rather than as parsed values.
struct Written {
    at: f64,
    /// Whose guard wrote it: the account's or the machine's.
    whose: &'static str,
    line: String,
}

/// The lines of the account's guard's journal and the machine's, from
/// `cut` on, oldest first, each guard's in the order it wrote them. The
/// machine's guard owns the store and what lies outside every home, so
/// the account's journal alone never said where most of a disk's room
/// went. A machine's journal that can't be read is said on `err`, and the
/// account's still shown.
fn written(env: &Env, cut: f64, err: &mut dyn Write) -> Result<Vec<Written>, Failure> {
    let mut lines = Vec::new();
    for (whose, state) in [("account", &env.state), ("machine", &env.system_state)] {
        for name in ["journal.jsonl.1", "journal.jsonl"] {
            let path = state.join(name);
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) if whose == "machine" => {
                    writeln!(
                        err,
                        "The machine's guard's journal, {}, can't be read: {error}",
                        path.display()
                    )
                    .map_err(failed)?;
                    break;
                }
                Err(error) => {
                    return Err(Failure::new(
                        Exit::Failed,
                        format!("{} can't be read", path.display()),
                    )
                    .because(error.to_string()));
                }
            };
            for line in text.lines() {
                // A line cut short by a crash says nothing.
                let Ok(entry) = serde_json::from_str::<Value>(line) else {
                    continue;
                };
                let at = entry["at"].as_f64().unwrap_or(0.0);
                if at >= cut {
                    lines.push(Written {
                        at,
                        whose,
                        line: line.to_owned(),
                    });
                }
            }
        }
    }
    // A stable sort, so what one guard wrote at the same moment keeps its
    // order, as a reclaim's beginning comes before its action.
    lines.sort_by(|one, other| one.at.total_cmp(&other.at));
    Ok(lines)
}

/// What a journal line says, after when, whose and in what band.
fn said(entry: &Value) -> String {
    match entry["event"].as_str() {
        Some("action") => {
            let mut said = format!(
                "{:<9} {:<10} {:>10}  {}",
                entry["outcome"].as_str().unwrap_or("?"),
                entry["kind"].as_str().unwrap_or("?"),
                size(entry["size"].as_u64().unwrap_or(0)),
                entry["path"].as_str().unwrap_or("?"),
            );
            if let Some(reason) = entry["reason"].as_str() {
                // Writing to a String can't fail.
                _ = write!(said, " ({reason})");
            }
            said
        }
        Some("pass") => format!(
            "pass {}, freed {}{}",
            entry["end"].as_str().unwrap_or("?"),
            size(entry["freed"].as_u64().unwrap_or(0)),
            // A journal from before passes were timed says nothing.
            entry["seconds"]
                .as_f64()
                .map_or_else(String::new, |seconds| format!(" in {}", span(seconds))),
        ),
        _ => entry.to_string(),
    }
}

/// The journals, oldest first: as text, every entry but a reclaim's
/// beginning; as JSON, every entry as the guard wrote it, with whose guard
/// wrote it, all in one answer or one `entry` line each under `jsonl`.
pub(super) fn journal(
    env: &Env,
    since: Option<Span>,
    context: &Context,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> Result<(), Failure> {
    let now = wall();
    let cut = since.map_or(f64::NEG_INFINITY, |since| now - since.0.as_secs_f64());
    let mut entries = Vec::new();
    for Written { at, whose, line } in written(env, cut, err)? {
        let Ok(mut entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        match context.format() {
            Format::Jsonl | Format::Json => {
                // Each says whose guard wrote it, as the text does.
                if let Some(fields) = entry.as_object_mut() {
                    fields.insert("guard".to_owned(), Value::from(whose));
                }
                if context.format() == &Format::Jsonl {
                    context.line_to(out, "entry", &entry).map_err(failed)?;
                } else {
                    entries.push(entry);
                }
            }
            Format::Text | Format::Own(_) => {
                // What a reclaim began with is for a guard restarting
                // after a crash; the action after it says what happened.
                if entry["event"] == "begin" {
                    continue;
                }
                let when = format!("{} ago", span(now - at));
                let band = entry["band"].as_str().unwrap_or("?");
                writeln!(out, "{when:>12}  {whose}  {band:<8}  {}", said(&entry))
                    .map_err(failed)?;
            }
        }
    }
    if context.format() == &Format::Json {
        let answer = json!({"schema": "disk-guard.journal/1", "entries": entries});
        context.answer_to(out, &answer).map_err(failed)?;
    }
    Ok(())
}

/// The places a command that reads alone works from: no status to write.
fn reading(env: &Env) -> Places {
    Places {
        state: env.state.clone(),
        status: None,
        processes: env.processes.clone(),
        system: Some(env.system_socket.clone()),
        running: running(),
    }
}

/// A band as it's typed, from how the guard keeps it.
fn band_word(band: Band) -> Value {
    serde_json::to_value(band).unwrap_or(Value::Null)
}

pub(super) fn plan(
    env: &Env,
    config: Option<&Path>,
    (band, free): (Option<Band>, Option<Size>),
    fresh: bool,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let config = read_config(config.unwrap_or(&env.config))?;
    let json = context.format().is_json();
    let mut guard = Guard::new(config, &reading(env)).map_err(failed)?;
    let tick = crate::sys::tick().map_err(failed)?;
    if !json && (fresh || crate::surveyed::load(&env.state).is_none()) {
        // Said first, since the walk takes minutes on a full machine and
        // a plan that says nothing meanwhile looks hung.
        writeln!(
            out,
            "Surveying every source, which takes minutes on a full disk."
        )
        .map_err(failed)?;
        out.flush().map_err(failed)?;
    }
    let (surveyed, previews) = guard.preview(&tick, band, free, fresh).map_err(failed)?;
    let (unread, why) = guard.unread();
    if json {
        let filesystems: Vec<Value> = previews
            .iter()
            .map(|preview| {
                let taken: Vec<Value> = preview
                    .taken
                    .iter()
                    .map(|candidate| {
                        json!({
                            "path": said_path(&candidate.path),
                            "kind": candidate.class.kind(),
                            "size_bytes": candidate.size,
                            "idle_seconds": candidate.idle,
                        })
                    })
                    .collect();
                json!({
                    "path": said_path(&preview.path),
                    "band": band_word(preview.band),
                    "free_bytes": preview.free,
                    "target_bytes": preview.target,
                    "end": preview.end,
                    "taken": taken,
                })
            })
            .collect();
        let answer = json!({
            "schema": "disk-guard.plan/1",
            "surveyed_seconds_ago": surveyed.map(|at| tick.wall - at),
            "filesystems": filesystems,
            "unread_processes": unread,
            "unread_because": why,
        });
        return context.answer_to(out, &answer).map_err(failed);
    }
    if let Some(at) = surveyed {
        writeln!(
            out,
            "From the survey the daemon's last pass made, {} ago; --fresh surveys again.",
            span(tick.wall - at)
        )
        .map_err(failed)?;
    }
    for preview in previews {
        writeln!(
            out,
            "{}: {}, {} free, target {}; the pass would end {}",
            preview.path.display(),
            band_word(preview.band).as_str().unwrap_or("?"),
            size(preview.free),
            size(preview.target),
            preview.end,
        )
        .map_err(failed)?;
        for candidate in &preview.taken {
            writeln!(
                out,
                "  {:<10} {:>10}  idle {:>7}  {}",
                candidate.class.kind(),
                size(candidate.size),
                span(candidate.idle),
                candidate.path.display(),
            )
            .map_err(failed)?;
        }
    }
    if unread > 0 {
        let why = why.map_or_else(String::new, |why| {
            format!(" and the system daemon couldn't be asked about them ({why})")
        });
        writeln!(
            out,
            "{unread} processes couldn't be read{why}, so a tree one of them might use isn't \
             shown."
        )
        .map_err(failed)?;
    }
    writeln!(
        out,
        "Nothing was taken. Whether each is in use is judged when a pass tries it."
    )
    .map_err(failed)?;
    Ok(())
}

/// How a path is used now, as words and as facts: what it is, and the
/// process that uses it, if one does.
fn used(used: crate::activity::Use, unheard: Option<&str>) -> (String, &'static str, Option<u32>) {
    use crate::activity::Use;
    match used {
        Use::Free => ("by nothing".to_owned(), "free", None),
        Use::Busy(pid) => (
            format!("busy: process {pid} works in it"),
            "busy",
            Some(pid),
        ),
        Use::Occupied(pid) => (
            format!("process {pid} sits in its tree"),
            "occupied",
            Some(pid),
        ),
        Use::Unknown(pid) => {
            let words = match unheard {
                Some(why) => format!(
                    "maybe: process {pid} can't be read, and the system daemon couldn't be \
                     asked ({why})"
                ),
                None => format!("maybe: process {pid} can't be read"),
            };
            (words, "unknown", Some(pid))
        }
    }
}

/// Says why `path` stays or would go: the candidate a source offers there
/// or holding it, how it's in use now, and what each band would do.
pub(super) fn explain(
    env: &Env,
    path: &Path,
    config: Option<&Path>,
    context: &Context,
    out: &mut dyn Write,
) -> Result<(), Failure> {
    let config = read_config(config.unwrap_or(&env.config))?;
    let mut guard = Guard::new(config, &reading(env)).map_err(failed)?;
    let tick = crate::sys::tick().map_err(failed)?;
    let explained = guard.explain(&tick, path).map_err(failed)?;
    let (_, unheard) = guard.unread();
    let json = context.format().is_json();
    let answer = |fields: Value| {
        let mut answer = json!({"schema": "disk-guard.explain/1", "path": said_path(path)});
        if let (Some(answer), Some(fields)) = (answer.as_object_mut(), fields.as_object()) {
            answer.extend(fields.clone());
        }
        answer
    };
    let (candidate, use_of) = match explained {
        Explained::Offered { candidate, used } => (candidate, used),
        Explained::Kept {
            path: kept,
            used: use_of,
        } => {
            let (words, state, process) = used(use_of, unheard.as_deref());
            if json {
                let fields = json!({
                    "offered": true, "kept_while_used": true,
                    "in_use": state, "process": process,
                    "candidate": said_path(&kept),
                });
                return context.answer_to(out, &answer(fields)).map_err(failed);
            }
            writeln!(out, "{}: kept while it's used", kept.display()).map_err(failed)?;
            writeln!(out, "  in use: {words}").map_err(failed)?;
            return Ok(());
        }
        Explained::Nothing => {
            if json {
                let fields = json!({"offered": false});
                return context.answer_to(out, &answer(fields)).map_err(failed);
            }
            writeln!(
                out,
                "No source offers {} or anything holding it, so the guard leaves it be.",
                path.display()
            )
            .map_err(failed)?;
            return Ok(());
        }
    };
    let (words, state, process) = used(use_of, unheard.as_deref());
    let (_, windows) = crate::plan::windows(&candidate);
    if json {
        // How long idle each band waits before taking it: null for never,
        // nothing for at once.
        let bands: serde_json::Map<String, Value> = BANDS
            .iter()
            .zip(windows)
            .map(|(band, window)| ((*band).to_owned(), json!(window.map(|w| w.max(0.0)))))
            .collect();
        let fields = json!({
            "offered": true, "kept_while_used": false,
            "candidate": said_path(&candidate.path),
            "kind": candidate.class.kind(),
            "size_bytes": candidate.size,
            "idle_seconds": candidate.idle,
            "in_use": state, "process": process,
            "idle_seconds_before_taken": bands,
        });
        return context.answer_to(out, &answer(fields)).map_err(failed);
    }
    writeln!(
        out,
        "{}: {}, {}, idle {}",
        candidate.path.display(),
        candidate.class.kind(),
        size(candidate.size),
        span(candidate.idle)
    )
    .map_err(failed)?;
    writeln!(out, "  in use: {words}").map_err(failed)?;
    for (band, window) in BANDS.iter().zip(windows) {
        let when = match window {
            None => "never".to_owned(),
            Some(window) if window <= 0.0 => "at once".to_owned(),
            Some(window) => format!("after {} idle", span(window)),
        };
        writeln!(out, "  {band}: {when}").map_err(failed)?;
    }
    Ok(())
}

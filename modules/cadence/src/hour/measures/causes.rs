//! The causes seen again and again: each class of cause behind a finding
//! counted by the changes it was found on in the last week, and one found
//! on three or more filed with its owner as one fix, rather than fixed
//! change by change. On 26 November Joel asked that the work "tackle
//! things at root causes ... detect patterns of issues and just nip them
//! in the bud", and that the loops doing it be built in.
//!
//! Cases come from two places. Cells send the loop's keeper a line for
//! each cause they settle, in the shape "cause: <class> | change:
//! <repo>#<N>[, #M] | run: <ids> | severity: <s> | state: <...>", and the
//! harness writes one to its review folder's `causes.jsonl` after each
//! council round it collects, as `{cause, change, run, severity}`. The
//! mesh lets its messages go after about a week, so the lines read from
//! it are kept in cadence's folder, and each hour opens only the messages
//! written since the last. One class goes by several names, so each is
//! folded to one: written plain, then by the aliases below.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Class, DAY, Measure, count, lines};
use crate::mesh::{Mesh, Message, stamped_at};
use crate::write_whole;

/// The measure's key, from one hour to the next.
pub const KEY: &str = "causes-recurring";

/// How far back a case counts.
const WITHIN: i64 = 7 * DAY;

/// How many changes one class must be found on in that week to be one
/// fix's worth rather than each change's own.
const AT_LEAST: usize = 3;

/// Who keeps the loop, hears the cells' cause lines, and answers for a
/// class no owner below takes.
const KEEPER: &str = "1JQPPC";

/// How long a case read from the mesh is kept, past the week it counts
/// for, so one read late still counts its whole week.
const KEPT_FOR: i64 = 8 * DAY;

/// How far before the last reading the next opens messages again, in ms,
/// for one written as the last was read, or after a clock stepped back.
const SLACK_MS: i64 = 3_600_000;

/// The shape of what a reading keeps: one of another shape reads the
/// mesh's week again.
const VERSION: u32 = 1;

/// Other names a class goes by, each written plain, and the one it is
/// counted under, until the harness's shared list of classes names them.
const ALIASES: &[(&str, &str)] = &[(
    "a fix with no test that fails when it is undone",
    "test passes with the fix undone",
)];

/// Who owns fixing each class, by its counted name. A class none owns
/// goes to the loop's keeper, who names one here.
const OWNERS: &[(&str, &str)] = &[];

/// Short names a repository goes by, and the name it is counted under.
const REPOS: &[(&str, &str)] = &[("core", "core.nix"), ("harness", "agentic-harness.pi")];

/// One class of cause found on one change.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Case {
    /// The class, folded to the name it is counted under.
    pub class: String,
    /// The change, as `<repo>#<number>`.
    pub change: String,
    /// When it was said, in seconds.
    pub at: i64,
}

/// What a reading keeps for the next: the cases read from the mesh, and
/// when, in ms, it read it through.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
struct Kept {
    v: u32,
    through_ms: i64,
    cases: Vec<Case>,
}

/// The measure at `now`, in seconds, from the mesh and the review folder
/// under `state_home`'s harness folder, keeping what it read of the mesh in
/// `folder` when `keeps`, as the hour that files does. A look by hand keeps
/// nothing, and reads the week since the last kept reading. With no mesh
/// folder to read it is past its target, rather than reading no causes as
/// none recurring.
#[must_use]
pub fn read(state_home: &Path, folder: &Path, now: i64, keeps: bool) -> Measure {
    let root = state_home.join("pi/agentic-harness.pi");
    let review = fs::read_to_string(root.join("review/causes.jsonl")).unwrap_or_default();
    let mesh = Mesh { root };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return Measure {
            now: 1.0,
            behind: vec![format!(
                "unread: no mesh messages folder here, so no cause line could be read · {KEEPER}"
            )],
            ..recurring(&[], now)
        };
    }
    let kept_at = folder.join("causes.json");
    let kept: Kept = fs::read_to_string(&kept_at)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|kept: &Kept| kept.v == VERSION)
        .unwrap_or_default();
    let from_ms = (kept.through_ms - SLACK_MS).max((now - WITHIN) * 1000);
    let mut cases = kept.cases;
    cases.extend(said(&mesh.sent_since(from_ms)));
    cases.sort();
    cases.dedup();
    cases.retain(|case| case.at > now - KEPT_FOR);
    if keeps {
        let next = Kept {
            v: VERSION,
            through_ms: now * 1000,
            cases: cases.clone(),
        };
        // Unkept, the next hour reads the same messages again, so a write
        // that fails loses nothing but time.
        if let Ok(text) = serde_json::to_string(&next) {
            let _ = write_whole(&kept_at, &text);
        }
    }
    cases.extend(reviewed(&review));
    recurring(&cases, now)
}

/// The classes in `cases` found on `AT_LEAST` changes or more in the week
/// before `now`, most changes first, each with its owner after ` · `, so
/// each is filed with its own.
#[must_use]
pub fn recurring(cases: &[Case], now: i64) -> Measure {
    let mut changes: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for case in cases.iter().filter(|c| c.at > now - WITHIN && c.at <= now) {
        changes
            .entry(case.class.as_str())
            .or_default()
            .insert(case.change.as_str());
    }
    let mut over: Vec<(&str, BTreeSet<&str>)> = changes
        .into_iter()
        .filter(|(_, on)| on.len() >= AT_LEAST)
        .collect();
    over.sort_by(|(a, on_a), (b, on_b)| on_b.len().cmp(&on_a.len()).then(a.cmp(b)));
    let behind: Vec<String> = over
        .iter()
        .map(|(class, on)| {
            let on: Vec<&str> = on.iter().copied().collect();
            format!(
                "{class}: on {} changes in a week, {} · {}",
                on.len(),
                on.join(", "),
                owner_of(class)
            )
        })
        .collect();
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: format!("classes of cause found on {AT_LEAST} or more changes in the last week"),
        now: count(behind.len()),
        target: Some(0.0),
        owner: KEEPER.to_owned(),
        behind,
    }
}

/// The class a line of the measure names, as its drag is keyed: the words
/// before its first colon.
#[must_use]
pub fn class_of(line: &str) -> &str {
    line.split_once(':').map_or(line, |(class, _)| class)
}

/// Who owns fixing `class`, or the keeper where none does.
fn owner_of(class: &str) -> &'static str {
    OWNERS
        .iter()
        .find(|(named, _)| *named == class)
        .map_or(KEEPER, |&(_, owner)| owner)
}

/// The cases the cells' cause lines in `sent` say, those sent to the
/// keeper, each dated when its message was.
fn said(sent: &[Message]) -> Vec<Case> {
    let heard = format!("@{KEEPER}");
    sent.iter()
        .filter(|m| m.to.iter().any(|to| *to == heard || to == KEEPER))
        .filter_map(|m| Some((stamped_at(&m.id)? / 1000, m)))
        .flat_map(|(at, m)| m.text.lines().flat_map(move |line| cause_line(line, at)))
        .collect()
}

/// The cases one line says, when it is a cause line: one for each change
/// it names, a change naming only its number taking the repository of the
/// one before it.
fn cause_line(line: &str, at: i64) -> Vec<Case> {
    let line = line.trim().trim_start_matches(['-', '*']).trim_start();
    let fields: BTreeMap<String, &str> = line
        .split('|')
        .filter_map(|field| field.split_once(':'))
        .map(|(name, value)| (name.trim().to_lowercase(), value.trim()))
        .collect();
    let (Some(cause), Some(change)) = (fields.get("cause"), fields.get("change")) else {
        return Vec::new();
    };
    if !line.to_lowercase().starts_with("cause:") {
        return Vec::new();
    }
    let class = folded(cause);
    if class.is_empty() {
        return Vec::new();
    }
    changes(change)
        .into_iter()
        .map(|change| Case {
            class: class.clone(),
            change,
            at,
        })
        .collect()
}

/// The changes `named` names, as `<repo>#<number>`: "core.nix#6310, #6311"
/// names two, the second in the first's repository.
fn changes(named: &str) -> Vec<String> {
    let mut repo = String::new();
    named
        .split([',', ' ', ';'])
        .filter_map(|word| {
            let (in_repo, number) = word.trim().split_once('#')?;
            let number: u64 = number.trim_end_matches(['.', ')']).parse().ok()?;
            if !in_repo.is_empty() {
                repo = repo_named(in_repo);
            }
            (!repo.is_empty()).then(|| format!("{repo}#{number}"))
        })
        .collect()
}

/// A repository as it is counted: the last part of its path, lower case,
/// and its full name for a short one.
fn repo_named(said: &str) -> String {
    let last = said.rsplit('/').next().unwrap_or(said).to_lowercase();
    REPOS
        .iter()
        .find(|(short, _)| *short == last)
        .map_or(last, |(_, full)| (*full).to_owned())
}

/// A class written plain, lower case, quotes, a closing stop and extra
/// spaces gone, then by the name its alias counts it under.
fn folded(class: &str) -> String {
    let plain = class
        .trim()
        .trim_matches(['"', '\'', '`'])
        .trim_end_matches('.')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == plain)
        .map_or(plain, |(_, counted)| (*counted).to_owned())
}

/// The cases the harness's `causes.jsonl` holds, each dated when its round
/// ran, by the time its run's id opens with, or its own `at` where it has
/// one.
fn reviewed(text: &str) -> Vec<Case> {
    lines(text)
        .filter_map(|line: Value| {
            let class = folded(line["cause"].as_str()?);
            let at = super::seconds(&line["at"]).or_else(|| run_at(&line["run"]))?;
            let changes = changes(line["change"].as_str()?);
            (!class.is_empty()).then(|| {
                changes
                    .into_iter()
                    .map(|change| Case {
                        class: class.clone(),
                        change,
                        at,
                    })
                    .collect::<Vec<_>>()
            })
        })
        .flatten()
        .collect()
}

/// When the round `run` names ran, in seconds, from its id, which opens
/// with its time in UTC, as `council-20261124T142609723-000001`; the first
/// of several ids where it names a list.
fn run_at(run: &Value) -> Option<i64> {
    let id = match run {
        Value::String(id) => id.as_str(),
        Value::Array(ids) => ids.first()?.as_str()?,
        _ => return None,
    };
    let stamp = id.split('-').nth(1)?;
    let (date, clock) = stamp.split_once('T')?;
    let time = format!(
        "{}-{}-{}T{}:{}:{}Z",
        date.get(0..4)?,
        date.get(4..6)?,
        date.get(6..8)?,
        clock.get(0..2)?,
        clock.get(2..4)?,
        clock.get(4..6)?
    );
    super::seconds(&Value::String(time))
}

#[cfg(test)]
mod tests;

//! The dates at risk before they come: each waits on a change's merge, and
//! main has since changed files the change touches, so the change may be
//! refused or need its checks again before it can merge. On 25 November
//! four of a zone's changes were refused twice for one such file, and the
//! dates they waited on were said to stand.

use std::collections::BTreeMap;

use serde_json::Value;

use super::{Class, DAY, Measure, count, first_owner, lines, parked, seconds, short};
use crate::tracking::{self, Tracking};

/// The key of Joel's dates at risk; the list's own add their kind to it.
pub const KEY: &str = "dates-at-risk";

/// The files a change touches that main has changed since the two parted,
/// or why they couldn't be read.
pub type Shared = Result<Vec<String>, String>;

/// How many of the shared files a line names before it counts the rest.
const NAMED: usize = 3;

/// A date at risk: when it's due, the line naming it and its owner.
type Risk = (i64, String, String);

/// The dates still to come of open asks of the kind `which` in the asks
/// list `text` at `now`, each waiting on a change's merge that shares
/// files with what main has changed since. A `core.nix` change is asked
/// of `core` by its number, a `world` one of `world` by its piece's
/// branch, since that zone's gate writes no number of its own; the other
/// repositories' changes aren't judged here. A piece waits on a
/// merge while its `pr` names one and it has neither landed nor closed;
/// an ask with no pieces is its own. An ask [`parked`] on a ruling, put
/// down, closed or confirmed waits on nothing. Each ask is read from its
/// latest line, as the list writes an ask whole each time. A change whose
/// files couldn't be read isn't counted, and the measure says how many,
/// so a read that fails never passes for none at risk.
#[must_use]
pub fn at_risk(
    text: &str,
    now: i64,
    which: Tracking,
    core: &dyn Fn(u32) -> Shared,
    world: &dyn Fn(&str) -> Shared,
) -> Measure {
    let mut latest: BTreeMap<[String; 2], Value> = BTreeMap::new();
    for ask in lines(text) {
        latest.insert([ask["at"].to_string(), ask["words"].to_string()], ask);
    }
    let mut risks: Vec<Risk> = Vec::new();
    let mut unread = 0;
    for ask in latest.values().filter(|ask| open(ask, which)) {
        for piece in waiting(ask) {
            let Some(due) = seconds(&piece["expected"]).filter(|&due| due > now) else {
                continue;
            };
            let pr = piece["pr"].as_str().unwrap_or_default();
            for (repo, number) in changes(pr) {
                let shared = match repo {
                    Repo::Core => core(number),
                    Repo::World => piece["branch"]
                        .as_str()
                        .map_or_else(|| Err("its piece names no branch".to_owned()), world),
                    Repo::Harness | Repo::Mycelium => continue,
                };
                match shared {
                    Ok(files) if files.is_empty() => {}
                    Ok(files) => risks.push(risk(ask, piece, (due, now), &files)),
                    Err(why) => {
                        eprintln!("cadence: at risk: {pr} unread: {why}");
                        unread += 1;
                    }
                }
            }
        }
    }
    risks.sort_by_key(|(due, _, _)| *due);
    let says = "dates waiting on a merge whose files main has changed since";
    Measure {
        key: which.key(KEY),
        class: Class::Slip,
        says: if unread == 0 {
            says.to_owned()
        } else {
            format!("{says} ({unread} unread)")
        },
        now: count(risks.len()),
        target: Some(0.0),
        owner: risks.first().map(|(_, _, o)| o.clone()).unwrap_or_default(),
        behind: risks.into_iter().map(|(_, line, _)| line).collect(),
    }
}

/// Where the zone's merge gate keeps what it refused and what drifted,
/// under the state folder.
pub const REFUSALS: &str = "home-merge-gate/refusals.jsonl";

/// How old the zone gate's latest line for a branch may be and still say
/// what main has changed beside it, as its timer writes one each hour.
const DRIFT_LASTS: i64 = 2 * 3600;

/// The files `branch` shares with main by the latest line for it in the
/// zone gate's `refusals` at `now`, its hourly drift or a refusal,
/// whichever came last: each line holds the branch, when it was `at` and
/// the `overlap`. No record, no line for the branch, or none in the last
/// two hours is unread, never none shared.
pub fn drifted(refusals: Option<&str>, branch: &str, now: i64) -> Shared {
    let text = refusals.ok_or_else(|| "the zone's merge gate keeps no record yet".to_owned())?;
    let line = lines(text)
        .filter(|line| line["branch"].as_str() == Some(branch))
        .max_by_key(|line| seconds(&line["at"]))
        .ok_or_else(|| format!("the zone's gate wrote no line for {branch}"))?;
    let at = seconds(&line["at"]).unwrap_or_default();
    if at < now - DRIFT_LASTS {
        let hours = (now - at) / 3600;
        return Err(format!("its latest line for {branch} is {hours} h old"));
    }
    let overlap = line["overlap"].as_array().into_iter().flatten();
    Ok(overlap
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect())
}

/// Whether `ask` is of the kind `which` and still waits on anything: not
/// [`parked`] on a ruling, put down, closed or confirmed.
fn open(ask: &Value, which: Tracking) -> bool {
    tracking::of(ask) == Some(which)
        && parked::ruling(ask).is_none()
        && unset(&ask["closed"])
        && unset(&ask["confirmed"])
}

/// Whether a field is missing, null or empty.
fn unset(field: &Value) -> bool {
    field.is_null() || field.as_str() == Some("")
}

/// The parts of `ask` that wait on a merge: each piece naming a change
/// that has neither landed nor closed, or the ask itself where it has no
/// pieces.
fn waiting(ask: &Value) -> Vec<&Value> {
    let pieces: Vec<&Value> = ask["pieces"].as_array().into_iter().flatten().collect();
    let parts = if pieces.is_empty() { vec![ask] } else { pieces };
    parts
        .into_iter()
        .filter(|part| part["pr"].is_string() && unset(&part["landed"]))
        .filter(|part| unset(&part["closed"]))
        .collect()
}

/// The line for `piece` of `ask`, due at `due` as read at `now`, whose
/// change shares `files` with main.
fn risk(ask: &Value, piece: &Value, (due, now): (i64, i64), files: &[String]) -> Risk {
    let what = piece["what"].as_str().or_else(|| ask["words"].as_str());
    let owner = piece["owner"].as_str().or_else(|| ask["owner"].as_str());
    let owner = first_owner(owner.unwrap_or_default());
    let mut named = files[..files.len().min(NAMED)].join(", ");
    if files.len() > NAMED {
        named = format!("{named} and {} more", files.len() - NAMED);
    }
    let line = format!(
        "{}, due {}, main changed {named} since it parted · {owner}",
        short(what.unwrap_or_default()),
        due_in(due - now)
    );
    (due, line, owner)
}

/// How long until a date `left` seconds away, as the card says it.
fn due_in(left: i64) -> String {
    match left {
        ..60 => "in under a minute".to_owned(),
        60..3600 => format!("in {} min", left / 60),
        3600..DAY => format!("in {} h", left / 3600),
        _ if left < 2 * DAY => "in a day".to_owned(),
        _ => format!("in {} days", left / DAY),
    }
}

/// A repository an ask's `pr` names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Repo {
    Core,
    World,
    Harness,
    Mycelium,
}

impl Repo {
    /// The repository a name in a `pr` value means, by its last part, as
    /// its keeper writes it short, whole or with its owner.
    fn named(name: &str) -> Option<Self> {
        let last = name.rsplit('/').next().unwrap_or_default();
        match last.to_ascii_lowercase().as_str() {
            "core.nix" | "core" => Some(Self::Core),
            "world" => Some(Self::World),
            "agentic-harness.pi" | "harness" => Some(Self::Harness),
            "mycelium" => Some(Self::Mycelium),
            _ => None,
        }
    }
}

/// Each change an ask's `pr` names, in the forms its keeper has written:
/// `repo#N`, with the repository short, whole or under its owner; a
/// change's address; or several after one another, where a bare `#N`
/// is in the repository named before it. A number with no repository
/// says no change that can be found, so it names none rather than one
/// guessed.
pub fn changes(pr: &str) -> Vec<(Repo, u32)> {
    let mut named = Vec::new();
    let mut last = None;
    for part in pr.split(',').map(str::trim) {
        let (repo, number) = match part.split_once("/pull/") {
            Some((repo, number)) => (Repo::named(repo), number),
            None => match part.rsplit_once('#') {
                Some(("", number)) => (last, number),
                Some((repo, number)) => (Repo::named(repo), number),
                None => (None, ""),
            },
        };
        let number = number.trim_end_matches('/').parse::<u32>().ok();
        if let (Some(repo), Some(number)) = (repo, number) {
            named.push((repo, number));
            last = Some(repo);
        }
    }
    named
}

#[cfg(test)]
mod tests;

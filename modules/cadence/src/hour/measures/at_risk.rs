//! The dates at risk before they come: each waits on a change's merge, and
//! main has since changed files the change touches, so the change may be
//! refused or need its checks again before it can merge. On 25 November
//! four of a zone's changes were refused twice for one such file, and the
//! dates they waited on were said to stand.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::{Class, DAY, Measure, asks, count, first_owner, lines, parked, seconds, short};
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
/// files with what main has changed since, each date once however many
/// changes it waits on. A `core.nix` change is asked of `core` by its
/// number, a `world` one of `world` by its piece's branch, since that
/// zone's gate writes no number of its own; the other repositories'
/// changes aren't judged here. A piece waits on a merge while its `pr`
/// names one and it has neither landed nor closed; an ask with no pieces
/// is its own. An ask [`parked`] on a ruling, put down, closed or
/// confirmed waits on nothing. Each ask is read from its latest line, as
/// the list writes an ask whole each time. A date whose changes couldn't
/// be read, or whose `pr` names none that can be found, isn't counted,
/// and the measure says how many, and the first and why, so a measure
/// blind for hours shows what to mend on the card itself; with none at
/// risk beside them, it goes unjudged, so a read that fails never passes
/// for none at risk.
#[must_use]
pub fn at_risk(
    text: &str,
    now: i64,
    which: Tracking,
    core: &dyn Fn(u32) -> Shared,
    world: &dyn Fn(&str) -> Shared,
) -> Measure {
    let mut latest: BTreeMap<[String; 2], Value> = BTreeMap::new();
    for ask in asks(text) {
        latest.insert([ask["at"].to_string(), ask["words"].to_string()], ask);
    }
    let mut risks: Vec<Risk> = Vec::new();
    let mut unread = 0;
    let mut first_unread = None;
    for ask in latest.values().filter(|ask| open(ask, which)) {
        for piece in waiting(ask) {
            let Some(due) = seconds(&piece["expected"]).filter(|&due| due > now) else {
                continue;
            };
            match shared(piece, core, world) {
                Ok(files) if files.is_empty() => {}
                Ok(files) => risks.push(risk(ask, piece, (due, now), &files)),
                Err(why) => {
                    // The asks list and the gate's record are other
                    // programs' text, so a newline in them can't write a
                    // journal or card line that reads as cadence's own.
                    let why = why.replace(char::is_control, "?");
                    eprintln!("cadence: at risk: {why}");
                    first_unread.get_or_insert_with(|| {
                        let what = piece["what"].as_str().or_else(|| ask["words"].as_str());
                        let what = what.unwrap_or_default().replace(char::is_control, "?");
                        format!("{}: {}", short(&what), short(&why))
                    });
                    unread += 1;
                }
            }
        }
    }
    risks.sort_by_key(|(due, _, _)| *due);
    let says = "dates waiting on a merge whose files main has changed since";
    Measure {
        key: which.key(KEY),
        class: Class::Slip,
        says: match first_unread {
            Some(first) => format!("{says} ({unread} unread; the first, {first})"),
            None => says.to_owned(),
        },
        now: count(risks.len()),
        target: (unread == 0 || !risks.is_empty()).then_some(0.0),
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

/// How far ahead of the hour a gate line's time may be, as two clocks on
/// one machine may differ, before it's taken for one never going stale.
const AHEAD: i64 = 5 * 60;

/// The files `branch` shares with main by the latest line for it in the
/// zone gate's `refusals` at `now`, its hourly drift or a refusal,
/// whichever came last: each line holds the branch, when it was `at` and
/// the `overlap`. The gate adds its lines in the order it writes them, so
/// the last for the branch is its latest, whatever its stamp says: an
/// earlier line dated ahead hides none written after it, and a stamp
/// older than one before it is no reason to read an older line. No
/// record, no line for the branch, a latest line dated ahead or none in
/// the last two hours, or one naming no overlap is unread, never none
/// shared.
pub fn drifted(refusals: Option<&str>, branch: &str, now: i64) -> Shared {
    let text = refusals.ok_or_else(|| "the zone's merge gate keeps no record yet".to_owned())?;
    let theirs: Vec<Value> = lines(text)
        .filter(|line| line["branch"].as_str() == Some(branch))
        .collect();
    let line = theirs
        .last()
        .ok_or_else(|| format!("the zone's gate wrote no line for {branch}"))?;
    let at = seconds(&line["at"]).unwrap_or_default();
    if at > now + AHEAD {
        return Err(format!("its latest line for {branch} is dated ahead"));
    }
    if at < now - DRIFT_LASTS {
        let hours = (now - at) / 3600;
        return Err(format!("its latest line for {branch} is {hours} h old"));
    }
    let Some(overlap) = line["overlap"].as_array() else {
        return Err(format!("its latest line for {branch} names no overlap"));
    };
    Ok(overlap
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect())
}

/// Whether `ask` is of the kind `which` and still waits on anything: not
/// [`parked`] on a ruling, put down, closed, confirmed or landed, as the
/// slips, holds and parked measures read a landed ask too, whatever its
/// pieces still say.
fn open(ask: &Value, which: Tracking) -> bool {
    tracking::of(ask) == Some(which)
        && parked::ruling(ask).is_none()
        && unset(&ask["closed"])
        && unset(&ask["confirmed"])
        && unset(&ask["landed"])
}

/// Whether a field is missing, null or empty.
fn unset(field: &Value) -> bool {
    field.is_null() || field.as_str() == Some("")
}

/// The files the changes `piece`'s `pr` names share with main, sorted
/// and each once, asked of `core` by number and of `world` once by the
/// piece's branch. Why not, naming the change, when one couldn't be read
/// or the `pr` names no change that can be found.
fn shared(piece: &Value, core: &dyn Fn(u32) -> Shared, world: &dyn Fn(&str) -> Shared) -> Shared {
    let pr = piece["pr"].as_str().unwrap_or_default();
    let named = changes(pr);
    if named.is_empty() {
        return Err(format!("{pr} names no change that can be found"));
    }
    // One part naming nothing leaves the piece unread, as it would alone,
    // rather than judged on the parts that could be read.
    if let Some(part) = unfound(pr) {
        return Err(format!("{part} in {pr} names no change that can be found"));
    }
    let mut files = BTreeSet::new();
    let numbers = named.iter().filter(|(repo, _)| *repo == Repo::Core);
    for &(_, number) in numbers {
        let read = core(number).map_err(|why| format!("core.nix#{number} unread: {why}"))?;
        files.extend(read);
    }
    if named.iter().any(|(repo, _)| *repo == Repo::World) {
        let Some(branch) = piece["branch"].as_str() else {
            return Err(format!("{pr} unread: its piece names no branch"));
        };
        files.extend(world(branch).map_err(|why| format!("{branch} unread: {why}"))?);
    }
    Ok(files.into_iter().collect())
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
    /// The repository a name in a `pr` value means, as its keeper writes
    /// it short, whole, under its owner or in its address; one under
    /// another owner is another repository.
    fn named(name: &str) -> Option<Self> {
        let name = name.to_ascii_lowercase();
        let name = name
            .trim_start_matches("https://")
            .trim_start_matches("github.com/");
        let (owner, repo) = match name.split_once('/') {
            Some((owner, repo)) => (Some(owner), repo),
            None => (None, name),
        };
        let under = |of: &str| owner.is_none_or(|owner| owner == of);
        match repo {
            "core.nix" | "core" if under("jitsusama") => Some(Self::Core),
            "world" if under("shop") => Some(Self::World),
            "agentic-harness.pi" | "harness" if under("jitsusama") => Some(Self::Harness),
            "mycelium" => Some(Self::Mycelium),
            _ => None,
        }
    }
}

/// Each change an ask's `pr` names, in the forms its keeper has written:
/// `repo#N`, with the repository short, whole or under its owner; a
/// change's address; or several after one another, where a bare `#N`
/// is in the repository named just before it. A number with no
/// repository, or one it doesn't know, says no change that can be found,
/// so it names none rather than one guessed, and each change is named
/// once.
pub fn changes(pr: &str) -> Vec<(Repo, u32)> {
    let mut named = Vec::new();
    for change in parts(pr).into_iter().filter_map(|(_, change)| change) {
        if !named.contains(&change) {
            named.push(change);
        }
    }
    named
}

/// The first part of `pr` that names no change that can be found, as
/// [`changes`] reads it.
pub(super) fn unfound(pr: &str) -> Option<&str> {
    parts(pr)
        .into_iter()
        .find_map(|(part, change)| change.is_none().then_some(part))
}

/// Each part of `pr`, empty ones left out, with the change it names when
/// it names one that can be found, as [`changes`] says.
fn parts(pr: &str) -> Vec<(&str, Option<(Repo, u32)>)> {
    let mut last = None;
    pr.split(',')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(|part| {
            let (repo, number) = match part.split_once("/pull/") {
                Some((repo, number)) => (Repo::named(repo), number),
                None => match part.rsplit_once('#') {
                    Some(("", number)) => (last, number),
                    Some((repo, number)) => (Repo::named(repo), number),
                    None => (None, ""),
                },
            };
            last = repo;
            let number = number.split('/').next().and_then(|n| n.parse::<u32>().ok());
            (part, repo.zip(number))
        })
        .collect()
}

#[cfg(test)]
mod tests;

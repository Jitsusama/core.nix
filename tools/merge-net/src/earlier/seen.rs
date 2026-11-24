//! Whether one of our review rounds read a head before it merges. From
//! Tuesday 17 November 18:00 to Thursday, 240 of 429 reviewed pull
//! requests merged at a head no council or judge had read: a fix pushed
//! after the round went to main unread.
//!
//! The harness keeps each change's rounds in its review state, one file a
//! change, each round naming the commit it was asked of as its witness.
//! A round speaks for a head only when it's a council or a stack round,
//! whose reviewers read the code (a judge reads their findings), they read
//! that commit (no `unpinned`), it was collected (neither `open` nor
//! `closed`), and one of them answered. The harness's own merge reads its
//! records the same way (`readSomething` in its review store). A round
//! over a range of commits counts only where its base is on main: the
//! commit it records as `from`. A recorded `from` that's no full commit
//! id is no base, so the round counts for no head; only a round that
//! records none, from before rounds did, falls back to the base the
//! range is named by, and a mirrored one does so only when that name is
//! a commit id.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// Where the harness keeps its review rounds: `MERGE_NET_REVIEW_RUNS`, or
/// its state folder's `review/runs`.
pub fn rounds_folder() -> PathBuf {
    if let Some(folder) = std::env::var_os("MERGE_NET_REVIEW_RUNS").filter(|f| !f.is_empty()) {
        return PathBuf::from(folder);
    }
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|f| !f.is_empty())
        .map_or_else(
            || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(".local/state"),
            PathBuf::from,
        );
    state.join("pi/agentic-harness.pi/review/runs")
}

/// Where other machines' review rounds are mirrored, one folder (or a link
/// to one) a node, beside the rounds folder: `review/runs-from`, as the
/// harness's own review gate reads them. Rounds now run on another machine
/// while this one's memory holds councils back, and a mirror copies its
/// rounds here rather than merging them into ours.
pub fn mirrors_folder() -> PathBuf {
    mirrors_beside(&rounds_folder())
}

/// The mirrors folder beside the rounds folder `rounds`.
fn mirrors_beside(rounds: &Path) -> PathBuf {
    rounds.with_file_name("runs-from")
}

/// The rounds mirrored from other machines: what they read, the nodes whose
/// rounds were read, and why any node's weren't.
#[derive(Debug, Default)]
pub struct Mirrored {
    pub rounds: HashSet<Read>,
    pub asked: Vec<String>,
    pub trouble: Vec<String>,
}

/// Every round mirrored under `root`, node by node in name order. A root
/// not made yet mirrors nothing, as on a machine no mirror fills. A node
/// that can't be read, or whose rounds can't be, is said so and left out
/// rather than refusing every head: the rounds here still count. A stray
/// file beside the nodes is left out, and so is a hidden name, as a sync
/// tool's own folder.
///
/// A range's base is kept only when it names a commit: one spelled as a
/// ref was resolved in the other machine's clone, where a branch of that
/// name can be another commit than here, so it stands for no head.
pub fn mirrored(root: &Path) -> Mirrored {
    let mut mirrored = Mirrored::default();
    let nodes = match fs::read_dir(root) {
        Ok(nodes) => nodes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return mirrored,
        Err(error) => {
            mirrored.trouble.push(format!(
                "the rounds mirrored under {} can't be listed ({error})",
                root.display()
            ));
            return mirrored;
        }
    };
    let mut names: Vec<String> = nodes
        .flatten()
        .map(|node| node.file_name().to_string_lossy().into_owned())
        .filter(|name| !name.starts_with('.'))
        .collect();
    names.sort();
    for name in names {
        let node = root.join(&name);
        // A link to a folder is a node too, so its target is asked after.
        match fs::metadata(&node) {
            Ok(found) if found.is_dir() => match rounds_in(&node) {
                Ok(rounds) => {
                    let rounds: HashSet<Read> = rounds.into_iter().map(as_read_here).collect();
                    if rounds.is_empty() && node.join("runs").is_dir() {
                        mirrored.trouble.push(format!(
                            "the mirror from {name} holds a runs folder and no rounds, as if \
                             mirrored one folder too high"
                        ));
                        continue;
                    }
                    mirrored.rounds.extend(rounds);
                    mirrored.asked.push(name);
                }
                Err(Unreadable::Unlisted(error)) => mirrored
                    .trouble
                    .push(format!("the mirror from {name} can't be listed ({error})")),
                Err(Unreadable::Unshaped(newest)) => mirrored.trouble.push(format!(
                    "the newest {newest} round files mirrored from {name} hold no round of the \
                     shape merge-net reads, so that machine's harness may write another shape"
                )),
            },
            Ok(_) => {}
            Err(error) => mirrored
                .trouble
                .push(format!("the mirror from {name} can't be read ({error})")),
        }
    }
    mirrored
}

/// A mirrored round as it can stand here: a range's base spelled as a ref
/// becomes no base, which is on no main.
fn as_read_here(read: Read) -> Read {
    let from = read.from.map(|base| {
        if is_commit_id(&base) {
            base
        } else {
            String::new()
        }
    });
    Read { from, ..read }
}

/// `why`, a head unread, with what the mirrors said: which nodes' rounds
/// were asked as well, that a round run there counts once its mirror has
/// it, and any that couldn't be read.
pub fn unread_with_mirrors(why: String, mirrored: &Mirrored) -> String {
    use std::fmt::Write;
    let mut said = why;
    // Writing to a String can't fail.
    if !mirrored.asked.is_empty() {
        let _ = write!(
            said,
            "; the rounds mirrored from {} were asked too, and a round run there counts \
             here once its mirror holds it",
            mirrored.asked.join(", ")
        );
    }
    for trouble in &mirrored.trouble {
        let _ = write!(said, "; {trouble}");
    }
    said
}

/// A commit a round read, and where it read from when the round was over
/// a range of commits rather than a change: such a round read only what
/// lies between, so its commit counts as read in full only where that
/// base is on main.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Read {
    pub commit: String,
    pub from: Option<String>,
}

/// How many of the newest round files the record shape is judged by: the
/// harness rewrites a change's file at each round, so the newest show the
/// shape it writes now, whatever older files hold.
const SHAPE_JUDGED_BY: usize = 5;

/// Every commit a round kept under `folder` read. A folder that can't be
/// read is said so, since a council asked at the head wouldn't help where
/// the net can't see the rounds at all, as on a machine where the harness
/// keeps its review state elsewhere; one not made yet holds no round, as
/// before the first is asked. So are round files whose newest rounds
/// aren't the shape merge-net reads, which is what a change to the
/// harness's records would look like, rather than refusing every head as
/// unread.
pub fn witnessed(folder: &Path) -> Result<HashSet<Read>, String> {
    rounds_in(folder).map_err(|why| match why {
        Unreadable::Unlisted(error) => format!(
            "the review rounds at {} can't be read ({error}), so no head can be shown \
             read; the harness keeps them in its state folder's review/runs",
            folder.display()
        ),
        Unreadable::Unshaped(newest) => format!(
            "the newest {newest} review round files at {} hold no round of the shape \
             merge-net reads, so the harness's record shape may have changed: \
             earlier/seen.rs must follow it",
            folder.display()
        ),
    })
}

/// Why a folder's rounds weren't read, said by whoever asked: this
/// machine's folder and a mirror call for different advice.
enum Unreadable {
    /// The folder can't be listed.
    Unlisted(std::io::Error),
    /// The newest this many round files are none of the shape read here.
    Unshaped(usize),
}

/// Every commit a round kept under `folder` read, as `witnessed` says.
fn rounds_in(folder: &Path) -> Result<HashSet<Read>, Unreadable> {
    let files = match fs::read_dir(folder) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(HashSet::new()),
        Err(error) => return Err(Unreadable::Unlisted(error)),
    };
    let mut texts: Vec<(std::time::SystemTime, String)> = files
        .flatten()
        .filter(|file| file.path().extension().is_some_and(|e| e == "json"))
        .filter_map(|file| {
            let written = file.metadata().and_then(|m| m.modified()).ok()?;
            Some((written, fs::read_to_string(file.path()).ok()?))
        })
        .collect();
    texts.sort_by_key(|text| std::cmp::Reverse(text.0));
    let newest = &texts[..texts.len().min(SHAPE_JUDGED_BY)];
    if !newest.is_empty() && !newest.iter().any(|(_, text)| shaped(text)) {
        return Err(Unreadable::Unshaped(newest.len()));
    }
    Ok(texts.iter().flat_map(|(_, text)| read_in(text)).collect())
}

/// Whether the newest round in the change's file `text` is the shape
/// merge-net reads: a kind, when it started, and the commits it was asked
/// of, whether or not it has been collected.
fn shaped(text: &str) -> bool {
    let Ok(change) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    change["runs"]
        .as_array()
        .and_then(|rounds| rounds.last())
        .is_some_and(|round| {
            round["round"].is_string()
                && round["startedAt"].is_string()
                && (round["witness"].is_string() || round["witnesses"].is_array())
        })
}

/// The commits the rounds of one change's file `text` read in full, each
/// with the base it read from when the change is a range of commits: the
/// commit the round itself records as `from`, or no base when what it
/// records is no commit id. Only a round recording no `from` at all falls
/// back to the base the range is named by, which a ref may spell and so
/// means nothing on another machine (`as_read_here` blanks it there).
fn read_in(text: &str) -> Vec<Read> {
    let Ok(change) = serde_json::from_str::<Value>(text) else {
        return Vec::new();
    };
    let Some(rounds) = change["runs"].as_array() else {
        return Vec::new();
    };
    let named = (change["change"]["provider"].as_str() == Some("local-range"))
        .then(|| change["change"]["id"].as_str())
        .map(|range| {
            range
                .and_then(|range| range.split_once(".."))
                .map_or_else(String::new, |(base, _)| base.to_owned())
        });
    rounds
        .iter()
        .filter(|round| speaks(round))
        .flat_map(|round| {
            // A recorded base that's no commit, null and other kinds
            // included, is no base, rather than the range's name, which may
            // be another commit than the round read. Only a round that
            // records none, from before rounds did, falls back to the name.
            let from = named.as_ref().map(|named| match round.get("from") {
                None => named.clone(),
                Some(from) => from
                    .as_str()
                    .filter(|from| is_commit_id(from))
                    .map_or_else(String::new, str::to_owned),
            });
            round["witness"]
                .as_str()
                .into_iter()
                .chain(
                    round["witnesses"]
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                )
                .filter(|witness| is_commit_id(witness))
                .map(|witness| Read {
                    commit: witness.to_owned(),
                    from: from.clone(),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether `witness` is a full commit id, which is all a round names, so
/// a symbolic name such as a branch's, which git would read as whatever
/// it points at now, never stands for a read.
fn is_commit_id(witness: &str) -> bool {
    matches!(witness.len(), 40 | 64)
        && witness
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// Whether `round` is a council or stack round that was collected, whose
/// reviewers read the commit it names, and one of whom answered: didn't
/// fail, and either finished, leaving its answer's file, or filed a
/// finding before it was stopped. One stopped at its wall said what it
/// had only in what it filed, and one cancelled (`"stopped": {"limit":
/// "cancelled"}`, with no failure) said nothing, so a round whose
/// reviewers were all cancelled reads no head, as cadence counts rounds
/// too. An answer is a sign it left, never the lack of a sign it didn't:
/// of the 38,136 outcomes on this machine on 23 November, each since 15
/// November with neither a failure nor a stop named its `answerPath` or
/// filed a finding, so an outcome naming neither, or written in a shape
/// this doesn't know, reads nothing.
fn speaks(round: &Value) -> bool {
    let flag = |name: &str| round[name].as_bool().unwrap_or(false);
    let filed = |outcome: &Value| {
        outcome["findingIds"]
            .as_array()
            .is_some_and(|ids| !ids.is_empty())
    };
    let answered = round["outcomes"].as_array().is_some_and(|outcomes| {
        outcomes.iter().any(|outcome| {
            let finished = outcome.get("stopped").is_none() && outcome["answerPath"].is_string();
            outcome.get("failure").is_none() && (finished || filed(outcome))
        })
    });
    matches!(round["round"].as_str(), Some("council" | "stack"))
        && !flag("open")
        && !flag("closed")
        && round.get("unpinned").is_none()
        && round["startedAt"].is_string()
        && answered
}

/// Why `commit`, which no round of ours read, may not merge.
pub fn unread(short: &str) -> String {
    format!(
        "no review round of ours read {short}, nor an earlier commit of the same change \
         (a squash or a reworded subject is told as another): ask a council (review_ask) at \
         this head, and ask ready again once it's collected"
    )
}

/// Why `commit`, which only rounds over a range of commits read, may not
/// merge: each read from a base that's on no main, or that it recorded as
/// no commit (`read_in` above), so none stands for the commits under it.
/// The bases are named, as `bases` gives them, since git failing to check
/// one (a repo it can't read, a spawn that failed) reads the same as a base
/// off main, and the name is how a reader tells the two apart.
pub fn read_from_no_base(short: &str, bases: &[String]) -> String {
    let named = if bases.is_empty() {
        "none recorded".to_owned()
    } else {
        bases.join(", ")
    };
    format!(
        "a review round read {short} only over a range from a base that isn't a commit on \
         main, or that git couldn't check (read from: {named}), so it doesn't stand for the \
         commits under it: ask a council (review_ask) on the change, or on a range from \
         main, and ask ready again once it's collected"
    )
}

#[cfg(test)]
mod tests;

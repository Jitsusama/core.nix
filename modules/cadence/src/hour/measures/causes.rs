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
//! this machine wrote since the last, by its own clock, since a sender's
//! may run fast or slow and a message relayed late was sent before it was
//! written here. A line its sender withdrew is dropped. One class goes by
//! many wordings, so each is folded to one by the patterns in `folds`;
//! the wording is kept beside its class, so a pattern changed folds every
//! kept case anew.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

mod folds;

use folds::{MAIN_MOVES, PIN_LAGS, TEST_PASSES_UNDONE, TIMING, UNSENT, fold};

use super::misses::symbiosis;
use super::{Class, DAY, Measure, count, lines};
use crate::mesh::{Message, stamped_at};
use crate::write_whole;

/// The measure's key, from one hour to the next.
pub const KEY: &str = "causes-recurring";

/// How far back a case counts, in seconds.
pub const WITHIN: i64 = 7 * DAY;

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
const VERSION: u32 = 2;

/// The most letters an unfolded class keeps of its sender's words, so no
/// line sent makes a key or an ask of any length.
const UNFOLDED_AT_MOST: usize = 100;

/// Who owns fixing each class, by its counted name and the repository its
/// change is in, or any repository where that is empty, as the keeper
/// named them on 26 November. A class whose owner differs by repository is
/// split by it, so each half goes to its own; one none owns goes to the
/// keeper, who names one here.
const OWNERS: &[(&str, &str, &str)] = &[
    (UNSENT, "", "80ECO2"),
    (TEST_PASSES_UNDONE, "core.nix", "80ECO2"),
    (TEST_PASSES_UNDONE, "agentic-harness.pi", "85PKB2"),
    (PIN_LAGS, "", "16V6F4"),
    (folds::GNU_ONLY, "", "1OGSE7"),
    (TIMING, "", "A210E4CA"),
    (MAIN_MOVES, "", "80ECO2"),
];

/// Short names a repository goes by, and the name it is counted under.
const REPOS: &[(&str, &str)] = &[
    ("core", "core.nix"),
    ("harness", "agentic-harness.pi"),
    ("agentic-harness", "agentic-harness.pi"),
];

/// One class of cause found on one change.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Case {
    /// The class, folded to the name it is counted under.
    pub class: String,
    /// The class as it was said, written plain, which is folded again
    /// whenever the case is read back.
    #[serde(default)]
    pub said: String,
    /// The change, as `<repo>#<number>`.
    pub change: String,
    /// When it was said, in seconds.
    pub at: i64,
    /// The message that said it, so a line its sender withdraws later is
    /// dropped; none for a review record.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub said_in: Option<String>,
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
/// folder, or one that can't be listed, it is past its target, rather
/// than reading no causes as none recurring.
#[must_use]
pub fn read(state_home: &Path, folder: &Path, now: i64, keeps: bool) -> Measure {
    let root = state_home.join("pi/agentic-harness.pi");
    let messages = root.join("mesh-workflow/messages");
    let unread = |why: &str| Measure {
        now: 1.0,
        behind: vec![format!(
            "unread: {why}, so no cause line could be read · {KEEPER}"
        )],
        ..recurring(&[], now)
    };
    if !messages.is_dir() {
        return unread("no mesh messages folder here");
    }
    // No record yet is no review causes; one that can't be read is unread,
    // or the classes only it holds up would close and be asked again. A
    // byte that isn't UTF-8, as a read inside an append can meet, costs
    // only its own line.
    let review_at = root.join("review/causes.jsonl");
    let review = match fs::read(&review_at) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            return unread(&format!(
                "{} couldn't be read ({error})",
                review_at.display()
            ));
        }
    };
    let kept_at = folder.join("causes.json");
    let kept: Kept = fs::read_to_string(&kept_at)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|kept: &Kept| kept.v == VERSION)
        .unwrap_or_default();
    let now_ms = now * 1000;
    // A mark past now says this machine's clock stepped back since it was
    // kept, and a file written in the step would sit behind it, so the
    // week is read anew.
    let mark = if kept.through_ms > now_ms {
        0
    } else {
        kept.through_ms
    };
    let since_ms = (now - WITHIN) * 1000;
    let from_ms = (mark - SLACK_MS).max(since_ms);
    let Ok((listed, _)) = symbiosis::listed(&messages, from_ms, since_ms) else {
        return unread("the mesh's messages can't be listed");
    };
    let heard: Vec<(Message, i64)> = listed
        .into_iter()
        .filter_map(|(path, name, written)| {
            let message: Message = serde_json::from_slice(&fs::read(&path).ok()?).ok()?;
            genuine(&message, &name).then_some((message, written))
        })
        .collect();
    // Only its sender withdraws a line, so another's word leaves it
    // counted.
    let withdrawn: BTreeSet<&str> = heard
        .iter()
        .filter_map(|(m, _)| {
            m.supersedes
                .as_deref()
                .filter(|id| sender_of(id) == Some(m.from.as_str()))
        })
        .collect();
    let mut cases: Vec<Case> = kept
        .cases
        .into_iter()
        .map(|case| Case {
            class: class_named(&case.said),
            ..case
        })
        .collect();
    cases.extend(said(&heard));
    cases.retain(|case| {
        case.said_in
            .as_deref()
            .is_none_or(|id| !withdrawn.contains(id))
    });
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
/// each is filed with its own. A class whose owner differs by repository
/// has a line for each repository it was found in, named after the class,
/// since the class recurs though one half of it may not.
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
        .flat_map(|(class, on)| lines_of(class, on))
        .collect();
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: format!("classes of cause found on {AT_LEAST} or more changes in the last week"),
        now: count(over.len()),
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

/// The lines one class over its bar gives, found on the changes `on`: one
/// naming them all, or, where its owner differs by repository, one for
/// each repository, naming that one's changes and its owner.
fn lines_of(class: &str, on: &BTreeSet<&str>) -> Vec<String> {
    let split = OWNERS
        .iter()
        .any(|&(named, repo, _)| named == class && !repo.is_empty());
    if !split {
        let all: Vec<&str> = on.iter().copied().collect();
        return vec![format!(
            "{class}: on {} changes in a week, {} · {}",
            all.len(),
            all.join(", "),
            owner_of(class, "")
        )];
    }
    let mut by_repo: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &change in on {
        let repo = change.split_once('#').map_or(change, |(repo, _)| repo);
        by_repo.entry(repo).or_default().push(change);
    }
    by_repo
        .into_iter()
        .map(|(repo, changes)| {
            format!(
                "{class} in {repo}: on {} of its {} changes in a week, {} · {}",
                changes.len(),
                on.len(),
                changes.join(", "),
                owner_of(class, repo)
            )
        })
        .collect()
}

/// Who owns fixing `class` in `repo`: its owner there, else its owner in
/// any repository, else the keeper.
fn owner_of(class: &str, repo: &str) -> &'static str {
    let owned_in = |within: &str| {
        OWNERS
            .iter()
            .find(|&&(named, named_repo, _)| named == class && named_repo == within)
            .map(|&(_, _, owner)| owner)
    };
    owned_in(repo).or_else(|| owned_in("")).unwrap_or(KEEPER)
}

/// Whether `message` is the one its file `name` names, sent by whom its
/// id says, so no file can stand in for another message or sender.
fn genuine(message: &Message, name: &str) -> bool {
    name.strip_suffix(".json") == Some(message.id.as_str())
        && sender_of(&message.id) == Some(message.from.as_str())
}

/// Who sent the message `id` names, as `<ms>-<from>-<pid>-<short id>`.
fn sender_of(id: &str) -> Option<&str> {
    id.split('-').nth(1)
}

/// The cases the cells' cause lines in `heard` say, those sent to the
/// keeper, each with when this machine wrote it, in ms, dated when its
/// message was sent, or written where its stamp is later, and naming it.
fn said(heard: &[(Message, i64)]) -> Vec<Case> {
    let to_keeper = format!("@{KEEPER}");
    heard
        .iter()
        .filter(|(m, _)| m.to.iter().any(|to| *to == to_keeper || to == KEEPER))
        .filter_map(|(m, written)| Some((stamped_at(&m.id)?.min(*written) / 1000, m)))
        .flat_map(|(at, m)| {
            m.text.lines().flat_map(move |line| {
                cause_line(line, at).into_iter().map(|case| Case {
                    said_in: Some(m.id.clone()),
                    ..case
                })
            })
        })
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
    let said = plain(cause);
    if said.is_empty() {
        return Vec::new();
    }
    let class = class_named(&said);
    changes(change)
        .into_iter()
        .map(|change| Case {
            class: class.clone(),
            said: said.clone(),
            change,
            at,
            said_in: None,
        })
        .collect()
}

/// The changes `named` names, as `<repo>#<number>`: "core.nix#6310, #6311"
/// names two, the second in the first's repository, and "core #6310" one,
/// its repository written apart from its number.
fn changes(named: &str) -> Vec<String> {
    let words: Vec<&str> = named
        .split([',', ' ', ';'])
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect();
    let mut repo = String::new();
    let mut found = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let Some((in_repo, number)) = word.split_once('#') else {
            let apart = words.get(at + 1).is_some_and(|next| next.starts_with('#'));
            if apart && is_repo(word) {
                repo = repo_named(word);
            }
            continue;
        };
        let Ok(number) = number.trim_end_matches(['.', ')']).parse::<u64>() else {
            continue;
        };
        if !in_repo.is_empty() {
            repo = repo_named(in_repo);
        }
        if !repo.is_empty() {
            found.push(format!("{repo}#{number}"));
        }
    }
    found
}

/// Whether a word written alone is a repository's name: one with an owner
/// before it, or one `REPOS` knows, never a word such as "and".
fn is_repo(word: &str) -> bool {
    let last = word.rsplit('/').next().unwrap_or(word).to_lowercase();
    word.contains('/')
        || REPOS
            .iter()
            .any(|&(short, full)| short == last || full == last)
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

/// The class `said`, written plain, is counted under: the one its words
/// fold into, or its own first `UNFOLDED_AT_MOST` letters, marked.
fn class_named(said: &str) -> String {
    let class = fold(said);
    match class.strip_prefix(folds::UNFOLDED) {
        Some(words) => format!(
            "{}{}",
            folds::UNFOLDED,
            words
                .chars()
                .take(UNFOLDED_AT_MOST)
                .collect::<String>()
                .trim_end()
        ),
        None => class,
    }
}

/// A class written plain: lower case, quotes, a closing stop and extra
/// spaces gone, a curled apostrophe straight and a colon a comma, since a
/// colon ends the class on the measure's line. A double quote or backtick
/// within is an apostrophe, so a sender's words can't close the quotes
/// the ask puts them in.
fn plain(class: &str) -> String {
    class
        .trim()
        .trim_matches(['"', '\'', '`'])
        .trim_end_matches('.')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
        .replace(['\u{2019}', '"', '`'], "'")
        .replace(':', ",")
}

/// `word` quoted for a POSIX shell: in single quotes, each of its own
/// closing them, written escaped and opening them again, so whatever a
/// sender wrote stays one word, and nothing in it runs, when pasted.
#[must_use]
pub fn shell_quoted(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

/// The cases the harness's `causes.jsonl` holds, each dated when its round
/// ran, by the time its run's id opens with, or its own `at` where it has
/// one.
fn reviewed(text: &str) -> Vec<Case> {
    lines(text)
        .filter_map(|line: Value| {
            let said = plain(line["cause"].as_str()?);
            let at = super::seconds(&line["at"]).or_else(|| run_at(&line["run"]))?;
            let changes = changes(line["change"].as_str()?);
            (!said.is_empty()).then(|| {
                let class = class_named(&said);
                changes
                    .into_iter()
                    .map(|change| Case {
                        class: class.clone(),
                        said: said.clone(),
                        change,
                        at,
                        said_in: None,
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

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

/// The most letters a case keeps of the cause as said, which it is folded
/// from again on every read. The longest a cell sent by 27 November ran
/// about 250; past this a line's words make the kept file grow, not its
/// fold surer.
const SAID_AT_MOST: usize = 500;

/// The most changes one line or record counts, so one naming hundreds
/// can't keep a file of them, nor stand for a pattern alone.
const CHANGES_PER_LINE: usize = 10;

/// The most unfolded classes one message counts, so a message of many
/// junk wordings can't put each over the bar; the folded classes are few
/// by their making.
const UNFOLDED_PER_MESSAGE: usize = 5;

/// The most classes an hour names, and so asks of their owners; the rest
/// are counted and said in one line to the keeper, and asked once fewer
/// are over the bar.
const NAMED_AT_MOST: usize = 10;

/// The most letters a kept withdrawal's id may have, well past a real
/// one's stamp, sender, process and hex, so none makes the kept file grow
/// without end.
const WITHDRAWAL_AT_MOST: usize = 128;

/// The most letters a repository's name may have and be counted, well
/// past any real one, so a change named can't make a key of any length.
const REPO_AT_MOST: usize = 64;

/// The words that join one change's number to the next within a line,
/// keeping the repository named before them.
const JOINING: &[&str] = &["and", "or", "also", "plus", "then", "&"];

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
    /// The messages whose lines their senders withdrew, so one written
    /// here after the reading that heard its withdrawal still stays out.
    withdrawn: BTreeSet<String>,
}

/// The harness's review causes at `at`, or why they couldn't be read. No
/// record yet is no review causes; one that can't be read is unread, or
/// the classes only it holds up would close and be asked again. A byte
/// that isn't UTF-8, as a read inside an append can meet, costs only its
/// own line.
fn review_record(at: &Path) -> Result<String, String> {
    match fs::read(at) {
        Ok(bytes) => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(error) => Err(format!("{} couldn't be read ({error})", at.display())),
    }
}

/// The reading kept at `at`, or why it couldn't be read. One that can't be
/// read or doesn't parse is unread, as symbiosis's is: read as none, the
/// cases only it holds, past the mesh's week, would be lost and their asks
/// closed. None yet, or one of another version, reads the week anew.
fn kept_reading(at: &Path) -> Result<Kept, String> {
    let json = match fs::read(at) {
        Ok(json) => json,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Kept::default()),
        Err(error) => {
            return Err(format!(
                "the reading kept in {} can't be read ({error})",
                at.display()
            ));
        }
    };
    match serde_json::from_slice::<Kept>(&json) {
        Ok(kept) if kept.v == VERSION => Ok(kept),
        Ok(kept) => {
            eprintln!(
                "cadence: {} is of version {}, so the mesh's week is read anew",
                at.display(),
                kept.v
            );
            Ok(Kept::default())
        }
        // It says what moving it aside would cost, as symbiosis's does, so
        // whoever mends it knows the cases only it holds go with it.
        Err(error) => Err(format!(
            "the reading kept in {} doesn't parse ({error}); moved aside, the week would be \
             read anew, forgetting what only it held",
            at.display()
        )),
    }
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
            "{UNREAD}: {why}, so no cause line could be read · {KEEPER}"
        )],
        ..recurring(&[], now)
    };
    if !messages.is_dir() {
        return unread("no mesh messages folder here");
    }
    let review = match review_record(&root.join("review/causes.jsonl")) {
        Ok(review) => review,
        Err(why) => return unread(&why),
    };
    let kept_at = folder.join("causes.json");
    let kept = match kept_reading(&kept_at) {
        Ok(kept) => kept,
        Err(why) => return unread(&why),
    };
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
        .filter_map(|(path, name, written)| Some((message_in(&path, &name)?, written)))
        .collect();
    let withdrawn = withdrawals(kept.withdrawn, &heard);
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
    // Unkept, the next hour reads the same messages again, at the cost of
    // a week's files, and a case the mesh lets go meanwhile is lost; so
    // the card says so, though it names no class to ask of anyone.
    let unkept = keeps
        .then(|| {
            let next = Kept {
                v: VERSION,
                through_ms: now * 1000,
                cases: cases.clone(),
                withdrawn: kept_withdrawals(&withdrawn, now),
            };
            serde_json::to_string(&next)
                .map_err(|error| error.to_string())
                .and_then(|text| write_whole(&kept_at, &text).map_err(|error| error.to_string()))
                .err()
        })
        .flatten();
    if let Some(why) = &unkept {
        eprintln!(
            "cadence: keeping the causes read in {}: {why}",
            kept_at.display()
        );
    }
    cases.extend(reviewed(&review));
    let mut m = recurring(&cases, now);
    // Past its target, as an unread hour is, and first, so the card shows
    // it whatever classes are over the bar.
    if let Some(why) = unkept {
        m.now = m.now.max(1.0);
        m.behind.insert(
            0,
            format!(
                "{UNKEPT}: what this hour read couldn't be kept ({why}), so the next reads the \
                 week again · {KEEPER}"
            ),
        );
    }
    m
}

/// What a measure's line opens with when it names no class: the mesh
/// couldn't be read, what was read couldn't be kept, or more classes were
/// over the bar than an hour names.
pub const UNREAD: &str = "unread";
pub const UNKEPT: &str = "unkept";
pub const MORE: &str = "more";

/// Whether a line of the measure names no class, only that it couldn't
/// read or keep, or how many more it didn't name, so it asks nobody for a
/// fix.
#[must_use]
pub fn names_no_class(line: &str) -> bool {
    [UNREAD, UNKEPT, MORE].contains(&class_of(line))
}

/// The classes in `cases` found on `AT_LEAST` changes or more in the week
/// before `now`, the vetted before the unfolded and most changes first,
/// each with its owner after ` · `, so
/// each is filed with its own. A class whose owner differs by repository
/// has a line for each repository with an owner of its own that it was
/// found in over the bar, named after the class, and one for the rest,
/// since the class recurs though one part of it may not.
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
    // The vetted classes go before the unfolded, which any cell's wording
    // can make, so a burst of those can't push one with an owner out of
    // the named; then most changes first.
    let unfolded = |class: &str| class.starts_with(folds::UNFOLDED);
    over.sort_by(|(a, on_a), (b, on_b)| {
        unfolded(a)
            .cmp(&unfolded(b))
            .then(on_b.len().cmp(&on_a.len()))
            .then(a.cmp(b))
    });
    // The count of those left unnamed goes first, so the card shows it
    // above the classes it names. While it stands, no ask open for a class
    // it leaves unnamed closes, since this hour can't say that one is back.
    let mut behind: Vec<String> = over
        .len()
        .checked_sub(NAMED_AT_MOST)
        .filter(|&more| more > 0)
        .map(|more| {
            format!(
                "{MORE}: {more} more classes are on {AT_LEAST} or more changes, unnamed and \
                 unasked until fewer are over the bar · {KEEPER}"
            )
        })
        .into_iter()
        .collect();
    behind.extend(
        over.iter()
            .take(NAMED_AT_MOST)
            .flat_map(|(class, on)| lines_of(class, on)),
    );
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
/// each repository found on `AT_LEAST` changes of its own that `OWNERS`
/// names the class's owner in, naming those and its owner, and one for
/// the rest. A repository under the bar isn't asked apart: on 27
/// November world was asked on 1 of 55 changes. Nor is one no owner is
/// named in, since its line would go to the class's owner as the rest's
/// does, and a sender naming many such repositories would make a line
/// and an ask for each with no cap. Their changes still count in the
/// rest's line, so three repositories on two each are asked of as the
/// class is, not of nobody.
fn lines_of(class: &str, on: &BTreeSet<&str>) -> Vec<String> {
    let all: Vec<&str> = on.iter().copied().collect();
    let owned_in = |repo: &str| {
        !repo.is_empty()
            && OWNERS
                .iter()
                .any(|&(named, named_repo, _)| named == class && named_repo == repo)
    };
    let mut by_repo: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for &change in on {
        let repo = change.split_once('#').map_or(change, |(repo, _)| repo);
        by_repo.entry(repo).or_default().push(change);
    }
    let (apart, rest): (Vec<_>, Vec<_>) = by_repo
        .into_iter()
        .partition(|(repo, changes)| owned_in(repo) && changes.len() >= AT_LEAST);
    if apart.is_empty() {
        return vec![format!(
            "{class}: on {} changes in a week, {} · {}",
            all.len(),
            all.join(", "),
            owner_of(class, "")
        )];
    }
    let mut lines: Vec<String> = apart
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
        .collect();
    let rest: Vec<&str> = rest.into_iter().flat_map(|(_, changes)| changes).collect();
    if !rest.is_empty() {
        lines.push(format!(
            "{class} in other repositories: on {} of its {} changes in a week, {} · {}",
            rest.len(),
            on.len(),
            rest.join(", "),
            owner_of(class, "")
        ));
    }
    lines
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
/// id says, so no file can stand in for another message or sender. It
/// checks that a file agrees with itself, not who wrote it: every writer
/// to the mesh is the same local user, so a forger who writes the id and
/// sender alike passes, until the mesh signs what it carries.
fn genuine(message: &Message, name: &str) -> bool {
    name.strip_suffix(".json") == Some(message.id.as_str())
        && sender_of(&message.id) == Some(message.from.as_str())
}

/// The lines withdrawn: those `kept` and those `heard` withdraw, each
/// applied whatever its stamp, since a sender's clock may run fast and
/// its line counts all the same. Only a message from its line's own
/// sender withdraws it, so another's word leaves it counted. That a
/// message is from whom it says rests on the mesh, which signs nothing
/// yet, so a forged withdrawal is kept as a real one is; clearing
/// `withdrawn` in the kept file lets both go.
fn withdrawals(mut kept: BTreeSet<String>, heard: &[(Message, i64)]) -> BTreeSet<String> {
    kept.extend(heard.iter().filter_map(|(m, _)| {
        m.supersedes
            .clone()
            .filter(|id| sender_of(id) == Some(m.from.as_str()))
    }));
    kept
}

/// Of those `withdrawn`, the ones kept for later hours, as of `now` in
/// seconds: each as long as the line it withdraws could be, since that
/// may be written here later. One stamped over an hour past now, or
/// running long, would only grow the kept file every hour, so it is let
/// go; it was applied to this hour's lines all the same.
fn kept_withdrawals(withdrawn: &BTreeSet<String>, now: i64) -> BTreeSet<String> {
    withdrawn
        .iter()
        .filter(|id| {
            id.len() <= WITHDRAWAL_AT_MOST
                && stamped_at(id)
                    .is_some_and(|ms| ms > (now - KEPT_FOR) * 1000 && ms <= now * 1000 + SLACK_MS)
        })
        .cloned()
        .collect()
}

/// The message the mesh's file at `path`, named `name`, holds, when it is
/// genuine. One that can't be read, or doesn't parse, is said on stderr
/// and left out: the next hour reads it again within `SLACK_MS`, so one
/// caught as it was written still counts, and one that stays torn is seen.
fn message_in(path: &Path, name: &str) -> Option<Message> {
    let parsed = symbiosis::opened_at_most(path)
        .map_err(|error| error.to_string())
        .and_then(|json| {
            serde_json::from_slice::<Message>(&json).map_err(|error| error.to_string())
        });
    match parsed {
        Ok(message) => genuine(&message, name).then_some(message),
        Err(why) => {
            eprintln!(
                "cadence: the mesh's {} was left out of the causes read ({why})",
                path.display()
            );
            None
        }
    }
}

/// Who sent the message `id` names, as `<ms>-<from>-<pid>-<short id>`.
fn sender_of(id: &str) -> Option<&str> {
    id.split('-').nth(1)
}

/// The cases the cells' cause lines in `heard` say, those sent to the
/// keeper, each with when this machine wrote it, in ms, dated when its
/// message was sent, or written where its stamp is later, and naming it.
/// One message counts at most `UNFOLDED_PER_MESSAGE` unfolded classes.
fn said(heard: &[(Message, i64)]) -> Vec<Case> {
    let to_keeper = format!("@{KEEPER}");
    heard
        .iter()
        .filter(|(m, _)| m.to.iter().any(|to| *to == to_keeper || to == KEEPER))
        .filter_map(|(m, written)| Some((stamped_at(&m.id)?.min(*written) / 1000, m)))
        .flat_map(|(at, m)| {
            let mut unfolded = BTreeSet::new();
            m.text
                .lines()
                .flat_map(|line| cause_line(line, at))
                .filter(|case| {
                    !case.class.starts_with(folds::UNFOLDED)
                        || unfolded.contains(&case.class)
                        || (unfolded.len() < UNFOLDED_PER_MESSAGE
                            && unfolded.insert(case.class.clone()))
                })
                .map(|case| Case {
                    said_in: Some(m.id.clone()),
                    ..case
                })
                .collect::<Vec<_>>()
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
/// its repository written apart from its number. A word before a bare
/// number that is neither a repository known nor a joining word names a
/// repository unknown, so the number is left out rather than counted in
/// the repository before it, as is one in a repository no host could
/// name.
fn changes(named: &str) -> Vec<String> {
    let words: Vec<&str> = named
        .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
        .map(str::trim)
        .filter(|word| !word.is_empty())
        .collect();
    let mut repo = String::new();
    let mut found = Vec::new();
    for (at, word) in words.iter().enumerate() {
        let Some((in_repo, number)) = word.split_once('#') else {
            let apart = words.get(at + 1).is_some_and(|next| next.starts_with('#'));
            if apart && is_repo(word) {
                repo = repo_named(word).unwrap_or_default();
            } else if apart && !JOINING.contains(&word.to_lowercase().as_str()) {
                repo.clear();
            }
            continue;
        };
        let Ok(number) = number.trim_end_matches(['.', ')']).parse::<u64>() else {
            continue;
        };
        if !in_repo.is_empty() {
            repo = repo_named(in_repo).unwrap_or_default();
        }
        if !repo.is_empty() {
            found.push(format!("{repo}#{number}"));
            if found.len() == CHANGES_PER_LINE {
                break;
            }
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
/// and its full name for a short one; none for a name no host would take,
/// since a repository's name goes into the class split by it, and so into
/// its key and the ask its owner pastes from.
fn repo_named(said: &str) -> Option<String> {
    let last = said.rsplit('/').next().unwrap_or(said).to_lowercase();
    let named = !last.is_empty()
        && last.len() <= REPO_AT_MOST
        && last
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    named.then(|| {
        REPOS
            .iter()
            .find(|(short, _)| *short == last)
            .map_or(last, |(_, full)| (*full).to_owned())
    })
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
/// the ask puts them in. Only its first `SAID_AT_MOST` letters are kept.
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
        .chars()
        .take(SAID_AT_MOST)
        .collect::<String>()
        .trim_end()
        .to_owned()
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

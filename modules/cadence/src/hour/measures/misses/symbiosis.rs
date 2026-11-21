//! The work-system findings posted to the mesh's `#v2-symbiosis` group
//! that have no fix running here, by owner. On 20 November Joel asked that
//! the cells of his work system keep removing every pain on it and on the
//! mycelial network, and the orchestrator ruled that each owner's open
//! findings there count against it, fixed only once the fix runs where it
//! hurt, never at its merge. A finding closes as a miss does, on the
//! changes every landing note in its thread names once they run here, and
//! one open past a day is filed with its owner, whatever else leads the
//! hour. Cadence reads only this machine, so a finding that hurt another
//! closes on its fix running here, until the node says what each machine
//! runs; one closed only by a merge whose running cadence can't see is
//! held, said beside the count rather than filed.
//!
//! The mesh doesn't authenticate senders: every cell writes its folder as
//! the same user. So the count trusts what is posted as the mesh does, and
//! guards against slips rather than a cell meaning harm: a file must be
//! the message it names, sent by whom its id says; a landing note closes
//! nothing until the changes it names run here; and a later note adds to
//! the fix rather than replacing it, unless it says "instead".

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::super::{Class, DAY, Measure, count};
use super::running::RUN_HERE;
use super::{BLIND, Checkouts, HELD, Judged, UNRUN, UNSEEN, named, open_misses};
use crate::write_whole;

/// The measure's key, from one hour to the next.
pub const KEY: &str = "symbiosis-findings";

/// The mesh group the work system's findings go to.
const GROUP: &str = "#v2-symbiosis";

/// How far back messages are read: about as long as the mesh keeps them.
const WITHIN: i64 = 7 * DAY;

/// How long a finding and what answers it are kept, well past `WITHIN`, so
/// one left open doesn't leave the count as the mesh lets it go, as if
/// fixed; one open four weeks is past what an hour's count can move.
const KEPT_FOR: i64 = 28 * DAY;

/// How long a finding may stay open before it drags its owner.
const PAST: i64 = DAY;

/// How far before the latest write the last reading saw the next opens
/// files again, in ms, for one written as the last reading listed.
const SLACK_MS: i64 = 60_000;

/// How long after it was written a file that doesn't parse may still be
/// one being written, in ms. Past that it is no message, said and left
/// out, so one broken file can't leave the count unjudged, and each hour
/// opening the week since it, until it ages out.
const TORN_FOR_MS: i64 = 5 * 60_000;

/// How much of a message's text is kept, in bytes: the count reads only
/// its opening, and a reading opening a week holds every message at once.
const OPENING: usize = 512;

/// The shape of what a reading keeps: one of another shape is read again.
const VERSION: u32 = 2;

/// What the measure says.
const SAYS: &str = "work-system findings in #v2-symbiosis open past a day, with no fix running \
                    here";

/// The word a landing note opens with.
const LANDED: &[&str] = &["landed"];

/// The words a withdrawal opens with.
const WITHDRAWN: &[&str] = &["withdrawn", "withdrawing", "withdraw", "withdrew"];

/// A message as far as the count reads one.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Posted {
    id: String,
    #[serde(default)]
    from: String,
    #[serde(default)]
    to: Vec<String>,
    #[serde(default)]
    intent: String,
    #[serde(default)]
    re: Option<String>,
    #[serde(default)]
    supersedes: Option<String>,
    #[serde(default)]
    text: String,
    /// When this machine wrote its file, in ms, by its own clock, never
    /// past the reading's now: what ages and orders a message, since a
    /// sender's clock may run hours fast or slow.
    #[serde(default)]
    written: i64,
}

/// What a reading keeps for the next: the messages that went to the group
/// or answer one that did, every address seen sending, and, in ms, the
/// latest time this machine wrote a message file read, so the next opens
/// only files written since rather than a week's thousands. The harness
/// writes each message whole and renames it into place at once, and moves
/// it into its day's folder only by a rename, which keeps that time; so a
/// sender's clock, or a message relayed late, hides nothing.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Kept {
    v: u32,
    through_ms: i64,
    posted: Vec<Posted>,
    known: BTreeSet<String>,
}

/// A finding counted, its owner, the first line of each landing note
/// counted as its fix, and when the earliest of those came, in ms.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    posted: Posted,
    owner: String,
    landed: Vec<String>,
    landed_at: Option<i64>,
}

/// The findings read for the hour, before their fixes are judged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Findings {
    found: Vec<Finding>,
    text: String,
    unread: Option<String>,
    orchestrator: String,
}

/// The findings in `#v2-symbiosis` as of `now`, in seconds, read under the
/// harness's state `root` and kept in `folder`. A finding naming no owner
/// counts against `orchestrator`, which assigns it; a look by hand names
/// none and keeps no reading. `None` where no orchestrator is told and no
/// reading was kept: nobody would be asked, and a first reading opens a
/// week of the mesh.
#[must_use]
pub fn findings(
    root: &Path,
    folder: &Path,
    now: i64,
    orchestrator: Option<&str>,
) -> Option<Findings> {
    let kept_at = folder.join("symbiosis.json");
    let keeps = orchestrator.is_some();
    if !keeps && !kept_at.exists() {
        return None;
    }
    let orchestrator = orchestrator.unwrap_or_default().to_owned();
    let messages = root.join("mesh-workflow/messages");
    Some(match read(&messages, now, &kept_at, keeps) {
        Ok(read) => {
            let found = open_findings(&read.posted, &orchestrator, &read.known);
            Findings {
                text: as_misses(&found),
                found,
                unread: (read.unread > 0)
                    .then(|| format!("{} of the mesh's messages unreadable", read.unread)),
                orchestrator,
            }
        }
        Err(why) => Findings {
            found: Vec::new(),
            text: String::new(),
            unread: Some(why),
            orchestrator,
        },
    })
}

impl Findings {
    /// The findings as lines of the misses' ledger, judged beside them.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The measure, once the findings' fixes are `judged` in `repos`: how
    /// many are open past a day at `now`, against none, each filed with its
    /// own owner. One the mesh couldn't be read for judges none. One held
    /// only because cadence can't see its fix run, or couldn't read a main,
    /// is the hour's own blindness, so it is said beside the count rather
    /// than counted or put to its owner, as a miss held so is.
    pub(super) fn measured(&self, repos: &Checkouts, judged: &Judged<'_>, now: i64) -> Measure {
        let mut measure = Measure {
            key: KEY.to_owned(),
            class: Class::Cells,
            says: SAYS.to_owned(),
            now: 0.0,
            target: None,
            owner: self.orchestrator.clone(),
            behind: Vec::new(),
        };
        if let Some(why) = &self.unread {
            measure.says = format!("{SAYS} ({why}, so none judged)");
            return measure;
        }
        let all = open_misses(&self.text, repos, judged);
        let mut lines: Vec<&str> = all.behind.iter().map(String::as_str).collect();
        let mut owners: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        let mut unseen = 0;
        for finding in &self.found {
            let named = named_as(&finding.posted);
            let opens = format!("{named} · {}", finding.owner);
            // Each open line is one finding's, taken once, its note after
            // its owner opening with a comma, so neither a finding named
            // alike nor an owner's longer name can take another's.
            let at = lines.iter().position(|line| {
                line.strip_prefix(opens.as_str())
                    .is_some_and(|note| note.is_empty() || note.starts_with(','))
            });
            let Some(at) = at else {
                unseen += usize::from(merged_unseen(finding, repos));
                continue;
            };
            let line: &str = lines.remove(at);
            let note = &line[opens.len()..];
            if blind(note) {
                unseen += 1;
                continue;
            }
            let old = finding.posted.written / 1000 <= now - PAST;
            let counts = owners.entry(finding.owner.as_str()).or_default();
            counts.0 += 1;
            if old {
                counts.1 += 1;
                // Each line ends with its owner, whom its drag is filed with.
                measure
                    .behind
                    .push(format!("{named}{note} · {}", finding.owner));
            }
        }
        if unseen > 0 {
            measure.says = format!("{SAYS}; {unseen} more held where cadence can't see it run");
        }
        if !owners.is_empty() {
            let each: Vec<String> = owners
                .iter()
                .map(|(owner, (open, old))| format!("{owner} {open} ({old} past a day)"))
                .collect();
            measure.says = format!("{}; open: {}", measure.says, each.join(", "));
        }
        measure.now = count(measure.behind.len());
        measure.target = Some(0.0);
        measure
    }
}

/// Whether a line's `note` holds its finding only because the hour
/// couldn't look: a main unread, a measure unread, or what runs here
/// unseen.
fn blind(note: &str) -> bool {
    note.contains("cadence couldn't read")
        || note.split_once(HELD).is_some_and(|(_, why)| {
            BLIND.iter().any(|b| why.ends_with(b))
                || why
                    .split_once(UNRUN)
                    .is_some_and(|(_, unrun)| unrun.starts_with(UNSEEN))
        })
}

/// Whether `finding`, closed, was closed by a change in a repository
/// whose running revision cadence doesn't read, so only by its merge.
fn merged_unseen(finding: &Finding, repos: &Checkouts) -> bool {
    finding
        .landed
        .iter()
        .flat_map(|line| named(line, repos))
        .any(|change| !RUN_HERE.contains(&change.repo.as_str()))
}

/// A reading of the mesh: the messages kept, the addresses seen sending,
/// and how many files written since the last couldn't be read.
struct Read {
    posted: Vec<Posted>,
    known: BTreeSet<String>,
    unread: usize,
}

/// A message file listed: where it is, its name, and when this machine
/// wrote it, in ms.
type Listed = (PathBuf, String, i64);

/// The messages under `messages` of the last `WITHIN` before `now` that
/// went to the group or answer or withdraw one that did, oldest first,
/// opening only the files written since the reading kept at `kept_at`,
/// which it `keeps` for the next. A folder, an entry or a file's time that
/// can't be read is an error, since reading none would read as none open.
fn read(messages: &Path, now: i64, kept_at: &Path, keeps: bool) -> Result<Read, String> {
    let kept: Kept = std::fs::read(kept_at)
        .ok()
        .and_then(|json| serde_json::from_slice(&json).ok())
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
    let from_ms = (now_ms - WITHIN * 1000).max(mark - SLACK_MS);
    let (listed, mut known) = listed(messages, from_ms, now_ms - WITHIN * 1000)
        .map_err(|error| format!("the mesh's messages can't be listed: {error}"))?;
    known.extend(kept.known);
    let opened = opened(listed, now_ms, mark);
    let posted = kept_beside(kept.posted, opened.posted, now_ms);
    known.extend(posted.iter().map(|one| one.from.clone()));
    if keeps {
        let kept = Kept {
            v: VERSION,
            through_ms: opened.through_ms,
            posted: posted.clone(),
            known: known.clone(),
        };
        // A failure to keep only costs the next reading its opens.
        if let Err(error) = serde_json::to_string(&kept)
            .map_err(std::io::Error::from)
            .and_then(|json| write_whole(kept_at, &json))
        {
            eprintln!(
                "cadence: keeping #v2-symbiosis in {}: {error}",
                kept_at.display()
            );
        }
    }
    Ok(Read {
        posted,
        known,
        unread: opened.unread,
    })
}

/// What a reading opened: the genuine messages, oldest first, how many
/// files couldn't be read, and the mark the next reading starts from.
struct Opened {
    posted: Vec<Posted>,
    unread: usize,
    through_ms: i64,
}

/// The messages the files `listed` hold as of `now_ms`, after a reading
/// that got as far as `mark`. A file that can't be read, or doesn't parse
/// while it may still be being written, is counted and holds the next
/// reading back to it; one moved since it was listed holds it back
/// uncounted, since the next lists it in its day's folder; one that
/// doesn't parse long after it was written is no message, said and left
/// out.
fn opened(listed: Vec<Listed>, now_ms: i64, mark: i64) -> Opened {
    let mut through_ms = mark;
    let mut held: Option<i64> = None;
    let mut unread = 0;
    let mut posted = Vec::new();
    for (path, name, written) in listed {
        let at = written.min(now_ms);
        through_ms = through_ms.max(at);
        let parsed = match std::fs::read(&path) {
            Ok(json) => serde_json::from_slice::<Posted>(&json)
                .map_err(|error| (error.to_string(), now_ms - written >= TORN_FOR_MS)),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                held = Some(held.map_or(at, |held| held.min(at)));
                continue;
            }
            Err(error) => Err((error.to_string(), false)),
        };
        match parsed {
            Ok(mut one) if genuine(&one, &name) => {
                one.written = at;
                cut_to_opening(&mut one.text);
                posted.push(one);
            }
            Ok(_) => eprintln!("cadence: {name} isn't the message it names, so it's left out"),
            Err((why, true)) => {
                eprintln!(
                    "cadence: {} is no message ({why}), so it's left out",
                    path.display()
                );
            }
            Err((why, false)) => {
                eprintln!("cadence: can't read {} yet: {why}", path.display());
                unread += 1;
                held = Some(held.map_or(at, |held| held.min(at)));
            }
        }
    }
    if let Some(at) = held {
        through_ms = through_ms.min(at);
    }
    posted.sort_by(by_written);
    Opened {
        posted,
        unread,
        through_ms,
    }
}

/// Cuts `text` to its opening, at most `OPENING` bytes.
fn cut_to_opening(text: &mut String) {
    if text.len() > OPENING {
        let end = (0..=OPENING)
            .rev()
            .find(|&at| text.is_char_boundary(at))
            .unwrap_or_default();
        text.truncate(end);
    }
}

/// Messages in the order this machine wrote them, then by id.
fn by_written(one: &Posted, other: &Posted) -> Ordering {
    one.written
        .cmp(&other.written)
        .then_with(|| one.id.cmp(&other.id))
}

/// Each message file under `messages` written at `from_ms` or later, by
/// this machine's clock, in the folder itself and in each folder for a
/// day the harness files messages into, from the day before `since_ms`
/// on, since one relayed late may be filed under the day it was sent;
/// and the address of every sender a file listed is named for. Files
/// being written, named from a dot, are left for the next. A day's folder
/// or a file the harness took away as it was listed is passed over: it
/// moves only what is old, read already, or expired.
fn listed(
    messages: &Path,
    from_ms: i64,
    since_ms: i64,
) -> std::io::Result<(Vec<Listed>, BTreeSet<String>)> {
    let earliest = jiff::Timestamp::from_millisecond(since_ms - DAY * 1000)
        .map(|at| at.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let dated = |name: &str| name.len() == 10 && name.as_bytes()[4] == b'-';
    let mut folders = vec![messages.to_path_buf()];
    for entry in std::fs::read_dir(messages)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if dated(&name) && name >= earliest && entry.file_type()?.is_dir() {
            folders.push(entry.path());
        }
    }
    let mut listed = Vec::new();
    let mut known = BTreeSet::new();
    for folder in folders {
        let entries = match std::fs::read_dir(&folder) {
            Err(error) if error.kind() == ErrorKind::NotFound && folder != messages => continue,
            entries => entries.map_err(|error| in_folder(&folder, &error))?,
        };
        for entry in entries {
            let entry = entry.map_err(|error| in_folder(&folder, &error))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let json = Path::new(&name)
                .extension()
                .is_some_and(|ext| ext == "json");
            if name.starts_with('.') || !json {
                continue;
            }
            if let Some(from) = name.split('-').nth(1).filter(|from| is_address(from)) {
                known.insert(from.to_owned());
            }
            let written = match entry.metadata().and_then(|meta| meta.modified()) {
                Ok(at) => at
                    .duration_since(UNIX_EPOCH)
                    .ok()
                    .and_then(|at| i64::try_from(at.as_millis()).ok())
                    .unwrap_or_default(),
                Err(error) if error.kind() == ErrorKind::NotFound => continue,
                Err(error) => return Err(in_folder(&folder, &error)),
            };
            if written >= from_ms {
                listed.push((entry.path(), name, written));
            }
        }
    }
    Ok((listed, known))
}

/// `error`, saying the `folder` it came from.
fn in_folder(folder: &Path, error: &std::io::Error) -> std::io::Error {
    std::io::Error::new(error.kind(), format!("{}: {error}", folder.display()))
}

/// Whether `one` is the message its file `name` names, sent by whom its id
/// says, so no file can stand in for another message or another sender.
fn genuine(one: &Posted, name: &str) -> bool {
    name.strip_suffix(".json") == Some(one.id.as_str())
        && one.id.split('-').nth(1) == Some(one.from.as_str())
}

/// What was `kept`, with what was `opened` since, in the order this
/// machine wrote them, that went to the group or answers or withdraws a
/// message kept; each once, none written past `KEPT_FOR` before `now_ms`,
/// and only a finding or what follows or withdraws it past `WITHIN`, so a
/// finding left open keeps counting and one withdrawn stays withdrawn.
fn kept_beside(kept: Vec<Posted>, opened: Vec<Posted>, now_ms: i64) -> Vec<Posted> {
    let mut posted: BTreeMap<String, Posted> =
        kept.into_iter().map(|one| (one.id.clone(), one)).collect();
    for one in opened {
        let answers = |id: &Option<String>| id.as_ref().is_some_and(|id| posted.contains_key(id));
        if one.to.iter().any(|to| to == GROUP) || answers(&one.re) || answers(&one.supersedes) {
            posted.insert(one.id.clone(), one);
        }
    }
    let threaded: BTreeSet<String> = posted
        .values()
        .filter(|one| lineage(&posted, one).any(|id| posted.get(id).is_some_and(is_finding)))
        .map(|one| one.id.clone())
        .collect();
    let mut posted: Vec<Posted> = posted
        .into_values()
        .filter(|one| {
            one.written >= now_ms - KEPT_FOR * 1000
                && (one.written >= now_ms - WITHIN * 1000 || threaded.contains(&one.id))
        })
        .collect();
    posted.sort_by(by_written);
    posted
}

/// `one`'s own id, the messages it answers, and the one it supersedes with
/// those that one answers, as far as `posted` holds them.
fn lineage<'a>(
    posted: &'a BTreeMap<String, Posted>,
    one: &'a Posted,
) -> impl Iterator<Item = &'a str> + 'a {
    let superseded = one.supersedes.as_deref().and_then(|id| posted.get(id));
    std::iter::once(one.id.as_str())
        .chain(ancestors(posted, one))
        .chain(
            superseded
                .into_iter()
                .flat_map(move |it| std::iter::once(it.id.as_str()).chain(ancestors(posted, it))),
        )
}

/// The messages `one` answers, nearest first, as far as `posted` holds
/// them, never looping.
fn ancestors<'a>(
    posted: &'a BTreeMap<String, Posted>,
    one: &'a Posted,
) -> impl Iterator<Item = &'a str> + 'a {
    let mut seen = BTreeSet::new();
    std::iter::successors(one.re.as_deref(), move |id| {
        posted.get(*id).and_then(|parent| parent.re.as_deref())
    })
    .take_while(move |id| seen.insert(*id))
}

/// Whether `one` is a finding sent to the group.
fn is_finding(one: &Posted) -> bool {
    one.intent == "finding" && one.to.iter().any(|to| to == GROUP)
}

/// The findings `posted` holds that are neither withdrawn nor added to
/// another, oldest first, each with the landing notes in its thread. A
/// finding answering another adds to it while that one is open with no
/// landing note yet, and counts on its own otherwise, so one saying a
/// fixed finding is back counts again. A landing note counts for the
/// finding it answers and every one counted since that follows it up, so
/// a note on the first closes a regression said beneath it too. A finding
/// is withdrawn by its sender superseding it, or answering it with a
/// withdrawal.
fn open_findings(posted: &[Posted], orchestrator: &str, known: &BTreeSet<String>) -> Vec<Finding> {
    let by_id: BTreeMap<String, Posted> = posted
        .iter()
        .map(|one| (one.id.clone(), one.clone()))
        .collect();
    let withdrawn = |finding: &Posted| {
        posted.iter().any(|one| {
            one.from == finding.from
                && (one.supersedes.as_deref() == Some(finding.id.as_str())
                    || (one.re.as_deref() == Some(finding.id.as_str())
                        && opens_with(one, WITHDRAWN)))
        })
    };
    // A withdrawal is no finding, whatever it was sent as.
    let takes_back = |one: &Posted| {
        let answered = one.re.as_ref().and_then(|re| by_id.get(re));
        answered.is_some_and(|it| it.from == one.from) && opens_with(one, WITHDRAWN)
    };
    let mut found: Vec<Finding> = Vec::new();
    let mut counted_as: BTreeMap<&str, usize> = BTreeMap::new();
    for one in posted {
        let thread = ancestors(&by_id, one).find_map(|id| counted_as.get(id).copied());
        if is_finding(one) && !withdrawn(one) && !takes_back(one) {
            if let Some(at) = thread.filter(|&at| found[at].landed.is_empty()) {
                counted_as.insert(&one.id, at);
            } else {
                counted_as.insert(&one.id, found.len());
                found.push(Finding {
                    posted: one.clone(),
                    owner: owner(one, orchestrator, known),
                    landed: Vec::new(),
                    landed_at: None,
                });
            }
        } else if let Some(at) = thread.filter(|_| opens_with(one, LANDED)) {
            let line = one.text.lines().next().unwrap_or_default();
            let root = found[at].posted.id.clone();
            let following: Vec<usize> = (at..found.len())
                .filter(|&index| {
                    index == at || ancestors(&by_id, &found[index].posted).any(|id| id == root)
                })
                .collect();
            for index in following {
                land(&mut found[index], line, one.written);
            }
        }
    }
    found
}

/// Counts the landing note opening with `line`, written at `at`, as part
/// of `finding`'s fix. One saying it lands "instead" replaces those before
/// it; any other adds to them, so a fix landed in parts closes only once
/// every part runs.
fn land(finding: &mut Finding, line: &str, at: i64) {
    let instead = line
        .split(|c: char| !c.is_ascii_alphabetic())
        .any(|word| word.eq_ignore_ascii_case("instead"));
    if instead {
        finding.landed.clear();
        finding.landed_at = None;
    }
    finding.landed.push(line.to_owned());
    finding.landed_at.get_or_insert(at);
}

/// Whether `one`'s opening, after its sender's address, is one of `words`,
/// in any case, as a whole word in a first sentence that asks nothing:
/// "IKQ63G: withdrawn", "Landed core #6000.", never "Landed? No" or
/// "Landed core #6000? It still fails".
fn opens_with(one: &Posted, words: &[&str]) -> bool {
    let opening = one.text.trim_start();
    let opening = opening.strip_prefix(one.from.as_str()).unwrap_or(opening);
    let opening = opening.trim_start_matches([':', ',', ' ']);
    if asks(opening) {
        return false;
    }
    words.iter().any(|word| {
        opening
            .get(..word.len())
            .is_some_and(|start| start.eq_ignore_ascii_case(word))
            && !opening[word.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '?')
    })
}

/// Whether `text`'s first sentence, ended by a stop, a bang or a question
/// mark before a space or the end, or by its first line's end, is a
/// question; a stop inside a name, as in `core.nix#6000`, ends nothing.
fn asks(text: &str) -> bool {
    let line = text.lines().next().unwrap_or_default();
    let mut marks = line.char_indices().peekable();
    while let Some((_, mark)) = marks.next() {
        let ends = marks.peek().is_none_or(|(_, next)| next.is_whitespace());
        if ends && matches!(mark, '.' | '!' | '?') {
            return mark == '?';
        }
    }
    false
}

/// Who a finding counts against: an address seen on the mesh after "for"
/// in its opening, before a colon, as in "IKQ63G finding for A2VLJN and
/// WCJQSG:"; else the first address it went to beside the group but its
/// sender's; else `orchestrator`.
fn owner(finding: &Posted, orchestrator: &str, known: &BTreeSet<String>) -> String {
    let opening = finding.text.lines().next().unwrap_or_default();
    let named = opening.split_once(':').and_then(|(head, _)| {
        let mut words = head.split_whitespace();
        words.find(|word| word.eq_ignore_ascii_case("for"))?;
        // The address ends where a word does: "OSPHT4's", "A2VLJN/WCJQSG".
        let word = words
            .next()?
            .split(|c: char| !c.is_ascii_alphanumeric())
            .next()?;
        Some(word.to_owned())
    });
    named
        .filter(|word| is_address(word) && known.contains(word))
        .or_else(|| {
            finding
                .to
                .iter()
                .filter_map(|to| to.strip_prefix('@'))
                .find(|to| *to != finding.from && is_address(to))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| orchestrator.to_owned())
}

/// Whether `word` is shaped as a mesh address: six to eight capitals and
/// digits, with a capital among them.
fn is_address(word: &str) -> bool {
    (6..=8).contains(&word.len())
        && word
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
        && word.bytes().any(|b| b.is_ascii_uppercase())
}

/// `found` as lines of the misses' ledger, so their landing notes are
/// judged as a miss's check is: what each says, its owner, its landing
/// notes together as its check, and when the earliest of them came, or
/// it did where none has, so each main is read back from no further than
/// a fix it names could have merged.
fn as_misses(found: &[Finding]) -> String {
    found
        .iter()
        .map(|finding| {
            let at = finding.landed_at.unwrap_or(finding.posted.written);
            let at = jiff::Timestamp::from_millisecond(at)
                .map(|at| at.to_string())
                .ok();
            serde_json::json!({
                "what": named_as(&finding.posted),
                "owner": finding.owner,
                "check": (!finding.landed.is_empty()).then(|| finding.landed.join("; ")),
                "at": at,
            })
            .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// How a finding is named on the card, on one line and in words of its
/// own: its id's short end, its sender and its opening words, with nothing
/// a message carries able to break the line or stand for its owner.
fn named_as(posted: &Posted) -> String {
    let short: String = posted
        .id
        .rsplit('-')
        .next()
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .take(8)
        .collect();
    let from = if is_address(&posted.from) {
        posted.from.as_str()
    } else {
        "?"
    };
    let words: Vec<&str> = posted.text.split_whitespace().take(14).collect();
    let words = words.join(" ").replace('"', "'").replace('·', "-");
    format!("{short} from {from}: {words}")
}

#[cfg(test)]
mod tests;

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
//! nothing until the changes it names run here; a note that doubts
//! itself, or was cut short, lands nothing; and a later note adds to the
//! fix rather than replacing it, unless it says "instead".
//!
//! Long because reading the mesh as it moves underneath, and the rules
//! by which a thread's findings, follow-ups and landing notes count, are
//! one account that reads plainest in one place; its cases are in
//! `symbiosis/tests.rs`.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::io::{ErrorKind, Read as _};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

use super::super::{Class, DAY, Measure, count};
use super::running::RUN_HERE;
use super::{Checkouts, HELD, Judged, UNRUN, UNSEEN, named, open_misses};
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

/// How much of a first line is kept past `OPENING`, in bytes, so a landing
/// note naming many changes is read whole; one longer is marked cut.
const FIRST_LINE: usize = 4096;

/// How much of a message file is read, in bytes; the mesh's are a few
/// thousand, and one past this is no message.
const FILE_AT_MOST: u64 = 1 << 20;

/// How old a kept reading may be for a look by hand to read the mesh past
/// it: one older says no hour keeps it here, and a look would open a week
/// of the mesh each time.
const KEPT_FRESH: i64 = 2 * 3_600;

/// How far back a finding's landing notes have each main walked, at most:
/// a change merged before is one an earlier hour found and kept.
const WALKED_BACK: i64 = 2 * DAY;

/// The shape of what a reading keeps: one of another shape is read again.
const VERSION: u32 = 2;

/// What the measure says.
const SAYS: &str = "work-system findings in #v2-symbiosis open past a day, with no fix running \
                    here";

/// The word a landing note opens with.
const LANDED: &[&str] = &["landed"];

/// The words a withdrawal opens with.
const WITHDRAWN: &[&str] = &["withdrawn", "withdrawing", "withdraw", "withdrew"];

/// Words that make a landing note doubt itself, as in "Landed core #6000
/// but it still fails": one holding any lands nothing, which errs toward
/// keeping its finding open.
const DOUBTS: &[&str] = &[
    "still", "but", "though", "although", "however", "yet", "broken", "fails", "failing", "failed",
];

/// What a finding's line says when a landing note names no change cadence
/// reads, so only cadence's blindness holds it.
const UNNAMED: &str = ", though cadence can't read a change its landing notes name";

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
    /// Whether its first line was cut short, so it can't be read whole as
    /// a landing note.
    #[serde(default)]
    cut: bool,
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
/// counted as its fix, when the earliest of those came, in ms, the first
/// finding of its thread, and the one it follows up after that one's
/// landing note, if it does.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    posted: Posted,
    owner: String,
    landed: Vec<String>,
    landed_at: Option<i64>,
    root: usize,
    follows: Option<usize>,
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
/// reading was kept within `KEPT_FRESH`: nobody would be asked, and the
/// reading would open a week of the mesh.
#[must_use]
pub fn findings(
    root: &Path,
    folder: &Path,
    now: i64,
    orchestrator: Option<&str>,
) -> Option<Findings> {
    let kept_at = folder.join("symbiosis.json");
    let keeps = orchestrator.is_some();
    let fresh = || {
        std::fs::metadata(&kept_at)
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|at| at.duration_since(UNIX_EPOCH).ok())
            .and_then(|at| i64::try_from(at.as_secs()).ok())
            .is_some_and(|at| at >= now - KEPT_FRESH)
    };
    if !keeps && !fresh() {
        return None;
    }
    // A look names no orchestrator, so one naming no owner is said as the
    // orchestrator's rather than as nobody's.
    let orchestrator = orchestrator.unwrap_or("the orchestrator").to_owned();
    let messages = root.join("mesh-workflow/messages");
    Some(match read(&messages, now, &kept_at, keeps) {
        Ok(read) => {
            let found = open_findings(&read.posted, &orchestrator, &read.known);
            Findings {
                text: as_misses(&found, now),
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
    /// only because cadence can't see its fix run, couldn't read a main, or
    /// can't read a change a landing note names, is the hour's own
    /// blindness, so it is said beside the count, and kept on the card
    /// past a day, never counted or put to its owner, as a miss held so
    /// is; kept there, its owner's drag stays open while the hour can't
    /// look, rather than closing and being filed again. A follow-up said
    /// after a landing note counts only once the finding it follows has
    /// closed: until that fix runs, it adds to it.
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
        let mut open = vec![false; self.found.len()];
        for (index, finding) in self.found.iter().enumerate() {
            let named = named_as(&finding.posted);
            let opens = format!("{named} · {}", finding.owner);
            // Each open line is one finding's, taken once, its note after
            // its owner opening with a comma, so neither a finding named
            // alike nor an owner's longer name can take another's.
            let at = lines.iter().position(|line| {
                line.strip_prefix(opens.as_str())
                    .is_some_and(|note| note.is_empty() || note.starts_with(','))
            });
            let line = at.map(|at| lines.remove(at));
            let unnamed = finding
                .landed
                .iter()
                .any(|note| named(note, repos).is_empty());
            open[index] = line.is_some() || unnamed;
            if finding.follows.is_some_and(|it| open[it]) {
                continue;
            }
            let note = if unnamed {
                UNNAMED
            } else if let Some(line) = line {
                &line[opens.len()..]
            } else {
                unseen += usize::from(merged_unseen(finding, repos));
                continue;
            };
            let old = finding.posted.written / 1000 <= now - PAST;
            // Each line ends with its owner, whom its drag is filed with.
            let said = format!("{named}{note} · {}", finding.owner);
            if held_unseen(note) {
                unseen += 1;
                if old {
                    measure.behind.push(said);
                }
                continue;
            }
            let counts = owners.entry(finding.owner.as_str()).or_default();
            counts.0 += 1;
            if old {
                counts.1 += 1;
                measure.behind.push(said);
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
        measure.now = count(
            measure
                .behind
                .iter()
                .filter(|line| !held_unseen(line))
                .count(),
        );
        measure.target = Some(0.0);
        measure
    }
}

/// Whether a finding's line holds it only because the hour couldn't look:
/// a main unread, a change a landing note names unread, or what runs here
/// unseen. Such a line stays on the card, never counted or put to its
/// owner.
#[must_use]
pub fn held_unseen(line: &str) -> bool {
    line.contains("cadence couldn't read")
        || line.contains(UNNAMED)
        || line.split_once(HELD).is_some_and(|(_, why)| {
            why.split_once(UNRUN)
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
/// which it `keeps` for the next. A folder, an entry, a file's time or a
/// kept reading that can't be read is an error, since reading none would
/// read as none open; a kept reading of another shape is said, and the
/// week read anew.
fn read(messages: &Path, now: i64, kept_at: &Path, keeps: bool) -> Result<Read, String> {
    let kept: Kept = match std::fs::read(kept_at) {
        Ok(json) => match serde_json::from_slice::<Kept>(&json) {
            Ok(kept) if kept.v == VERSION => kept,
            _ => {
                eprintln!(
                    "cadence: {} is of another shape, so the week is read anew",
                    kept_at.display()
                );
                Kept::default()
            }
        },
        Err(error) if error.kind() == ErrorKind::NotFound => Kept::default(),
        Err(error) => {
            return Err(format!(
                "the reading kept in {} can't be read: {error}",
                kept_at.display()
            ));
        }
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
/// uncounted, since the next lists it in its day's folder, unless it is
/// near the week's end, which the harness expires; one that doesn't parse
/// long after it was written is no message, said and left out. A file is
/// read only as far as `FILE_AT_MOST`.
fn opened(listed: Vec<Listed>, now_ms: i64, mark: i64) -> Opened {
    let mut through_ms = mark;
    let mut held: Option<i64> = None;
    let mut unread = 0;
    let mut posted = Vec::new();
    for (path, name, written) in listed {
        let at = written.min(now_ms);
        through_ms = through_ms.max(at);
        let read = std::fs::File::open(&path).and_then(|file| {
            let mut json = Vec::new();
            file.take(FILE_AT_MOST).read_to_end(&mut json).map(|_| json)
        });
        let parsed = match read {
            Ok(json) => serde_json::from_slice::<Posted>(&json)
                .map_err(|error| (error.to_string(), now_ms - written >= TORN_FOR_MS)),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                // Held back to one at the week's end, a full reading racing
                // the harness's expiry would leave the next to read it all.
                if at >= now_ms - (WITHIN - DAY) * 1000 {
                    held = Some(held.map_or(at, |held| held.min(at)));
                }
                continue;
            }
            Err(error) => Err((error.to_string(), false)),
        };
        match parsed {
            Ok(mut one) if genuine(&one, &name) => {
                one.written = at;
                one.cut = cut_to_opening(&mut one.text);
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

/// Cuts `text` to its opening, `OPENING` bytes or its whole first line if
/// longer, up to `FIRST_LINE`. Whether that cut its first line short.
fn cut_to_opening(text: &mut String) -> bool {
    let first = text.find('\n').unwrap_or(text.len());
    let keep = first.clamp(OPENING, FIRST_LINE);
    if text.len() <= keep {
        return false;
    }
    let end = (0..=keep)
        .rev()
        .find(|&at| text.is_char_boundary(at))
        .unwrap_or_default();
    text.truncate(end);
    end < first
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
/// being written, named from a dot, are left for the next, and only plain
/// files are listed, never a link or a pipe. A day's folder or a file the
/// harness took away as it was listed is passed over: it moves only what
/// is old, read already, or expired, and into a day's folder, which is
/// gathered only once the folder itself is listed, so one moved meanwhile
/// is listed there.
fn listed(
    messages: &Path,
    from_ms: i64,
    since_ms: i64,
) -> std::io::Result<(Vec<Listed>, BTreeSet<String>)> {
    let earliest = jiff::Timestamp::from_millisecond(since_ms - DAY * 1000)
        .map(|at| at.strftime("%Y-%m-%d").to_string())
        .unwrap_or_default();
    let dated = |name: &str| name.len() == 10 && name.as_bytes()[4] == b'-';
    let mut listed = Vec::new();
    let mut known = BTreeSet::new();
    list_in(messages, messages, from_ms, &mut listed, &mut known)?;
    let mut folders = Vec::new();
    for entry in std::fs::read_dir(messages)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if dated(&name) && name >= earliest && entry.file_type()?.is_dir() {
            folders.push(entry.path());
        }
    }
    for folder in folders {
        list_in(messages, &folder, from_ms, &mut listed, &mut known)?;
    }
    Ok((listed, known))
}

/// Adds to `listed` each message file in `folder`, under `messages`,
/// written at `from_ms` or later, and to `known` each sender one is named
/// for, as `listed` says.
fn list_in(
    messages: &Path,
    folder: &Path,
    from_ms: i64,
    listed: &mut Vec<Listed>,
    known: &mut BTreeSet<String>,
) -> std::io::Result<()> {
    let entries = match std::fs::read_dir(folder) {
        Err(error) if error.kind() == ErrorKind::NotFound && folder != messages => return Ok(()),
        entries => entries.map_err(|error| in_folder(folder, &error))?,
    };
    for entry in entries {
        let entry = entry.map_err(|error| in_folder(folder, &error))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        let json = Path::new(&name)
            .extension()
            .is_some_and(|ext| ext == "json");
        if name.starts_with('.') || !json {
            continue;
        }
        match entry.file_type() {
            Ok(kind) if kind.is_file() => {}
            Ok(_) => continue,
            Err(error) if error.kind() == ErrorKind::NotFound => continue,
            Err(error) => return Err(in_folder(folder, &error)),
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
            Err(error) => return Err(in_folder(folder, &error)),
        };
        if written >= from_ms {
            listed.push((entry.path(), name, written));
        }
    }
    Ok(())
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
/// message kept, or may land or withdraw one; each once, aged from when
/// it was first written here, none written past `KEPT_FOR` before
/// `now_ms`, and only a finding or what follows or withdraws it past
/// `WITHIN`, so a finding left open keeps counting and one withdrawn
/// stays withdrawn. One sent off the group that may land or withdraw is
/// kept whether or not what it answers is yet, so one written here before
/// the finding it answers still counts once that comes.
fn kept_beside(kept: Vec<Posted>, opened: Vec<Posted>, now_ms: i64) -> Vec<Posted> {
    let mut posted: BTreeMap<String, Posted> =
        kept.into_iter().map(|one| (one.id.clone(), one)).collect();
    for one in opened {
        let answers = |id: &Option<String>| id.as_ref().is_some_and(|id| posted.contains_key(id));
        let settles = one.supersedes.is_some()
            || (one.re.is_some() && (opens_with(&one, LANDED) || opens_with(&one, WITHDRAWN)));
        if one.to.iter().any(|to| to == GROUP)
            || answers(&one.re)
            || answers(&one.supersedes)
            || settles
        {
            // A file dated ahead is opened again each hour as written then,
            // so it keeps the time it was first written here.
            let written = posted
                .get(&one.id)
                .map_or(one.written, |was| was.written.min(one.written));
            posted.insert(one.id.clone(), Posted { written, ..one });
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
/// another, oldest first, each with the landing notes in its thread, read
/// in the order written but never before what each answers. A finding
/// answering another adds to the latest counted in its thread while that
/// one has no landing note yet, and counts on its own otherwise, owned as
/// the thread's first unless it names its owner, following that latest
/// one, so one saying a fixed finding is back counts again once that fix
/// runs. A landing note, sent as whatever, counts for the finding it
/// answers and every one counted in its thread since, so a note on the
/// first closes a regression said beneath it too. A finding is withdrawn
/// by its sender superseding it, or answering it with a withdrawal.
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
    // A message written here before what it answers, as one relayed late
    // may be, is read just after it.
    let mut ordered: Vec<&Posted> = posted.iter().collect();
    ordered.sort_by_cached_key(|one| {
        let above: Vec<i64> = ancestors(&by_id, one)
            .filter_map(|id| by_id.get(id))
            .map(|it| it.written)
            .collect();
        let after = above.iter().copied().fold(one.written, i64::max);
        (after, above.len(), one.written, one.id.clone())
    });
    let mut found: Vec<Finding> = Vec::new();
    let mut counted_as: BTreeMap<&str, usize> = BTreeMap::new();
    for one in ordered {
        let thread = ancestors(&by_id, one).find_map(|id| counted_as.get(id).copied());
        let lands = thread.is_some() && opens_with(one, LANDED) && whole(one);
        if is_finding(one) && !lands && !withdrawn(one) && !takes_back(one) {
            let Some(at) = thread else {
                counted_as.insert(&one.id, found.len());
                found.push(Finding {
                    posted: one.clone(),
                    owner: owner(one, known).unwrap_or_else(|| orchestrator.to_owned()),
                    landed: Vec::new(),
                    landed_at: None,
                    root: found.len(),
                    follows: None,
                });
                continue;
            };
            let root = found[at].root;
            let latest = (root..found.len())
                .rev()
                .find(|&index| found[index].root == root)
                .unwrap_or(root);
            if found[latest].landed.is_empty() {
                counted_as.insert(&one.id, latest);
            } else {
                counted_as.insert(&one.id, found.len());
                let owner = owner(one, known).unwrap_or_else(|| found[root].owner.clone());
                found.push(Finding {
                    posted: one.clone(),
                    owner,
                    landed: Vec::new(),
                    landed_at: None,
                    root,
                    follows: Some(latest),
                });
            }
        } else if let Some(at) = thread.filter(|_| lands) {
            let line = one.text.lines().next().unwrap_or_default();
            let root = found[at].root;
            for it in found.iter_mut().skip(at).filter(|it| it.root == root) {
                land(it, line, one.written);
            }
        }
    }
    found
}

/// Whether `one`'s first line can be read whole as a landing note: not
/// cut short, and not doubting itself in any of `DOUBTS`.
fn whole(one: &Posted) -> bool {
    let line = one.text.lines().next().unwrap_or_default();
    !one.cut
        && !line
            .split(|c: char| !c.is_ascii_alphabetic())
            .any(|word| DOUBTS.iter().any(|doubt| word.eq_ignore_ascii_case(doubt)))
}

/// Counts the landing note opening with `line`, written at `at`, as part
/// of `finding`'s fix. One saying it lands "instead" replaces those before
/// it, unless it says "not" or "rather" too, as in "not instead of
/// #6020"; any other adds to them, so a fix landed in parts closes only
/// once every part runs.
fn land(finding: &mut Finding, line: &str, at: i64) {
    let words: Vec<&str> = line.split(|c: char| !c.is_ascii_alphabetic()).collect();
    let says = |word: &str| words.iter().any(|it| it.eq_ignore_ascii_case(word));
    let instead = says("instead") && !says("not") && !says("rather");
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
/// sender's; else none it names.
fn owner(finding: &Posted, known: &BTreeSet<String>) -> Option<String> {
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
/// a fix it names could have merged, and never past `WALKED_BACK` before
/// `now`, since an earlier hour kept what merged before.
fn as_misses(found: &[Finding], now: i64) -> String {
    found
        .iter()
        .map(|finding| {
            let at = finding
                .landed_at
                .unwrap_or(finding.posted.written)
                .max((now - WALKED_BACK) * 1000);
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

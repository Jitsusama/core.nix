//! The work-system findings posted to the mesh's `#v2-symbiosis` group
//! that have no fix running here, by owner. On 20 November Joel asked that
//! the cells of his work system keep removing every pain on it and on the
//! mycelial network, and the orchestrator ruled that each owner's open
//! findings there count against it, fixed only once the fix runs where it
//! hurt, never at its merge. A finding closes as a miss does, on the
//! changes its latest landing note names once they run here, and one open
//! past a day is filed with its owner. Cadence reads only this machine, so
//! a finding that hurt another closes on its fix running here, until the
//! node says what each machine runs.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::super::{Class, DAY, Measure, count};
use super::{Checkouts, Judged, open_misses};
use crate::{mesh, write_whole};

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

/// How far ahead of now a message may be stamped and still be read, in
/// ms: a clock a little fast is a sender's, a day ahead is no message.
const AHEAD_MS: i64 = 3_600_000;

/// The shape of what a reading keeps: one of another shape is read again.
const VERSION: u32 = 1;

/// What the measure says.
const SAYS: &str = "work-system findings in #v2-symbiosis open past a day, with no fix running \
                    here";

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
}

/// What a reading keeps for the next: the messages that went to the group
/// or answer one that did, every address seen sending, and, in ms, the
/// latest time this machine wrote a message file read, so the next opens
/// only files written since rather than a week's thousands. The write time
/// is this machine's, so a sender's clock running ahead, or a message
/// relayed late, hides nothing.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Kept {
    v: u32,
    through_ms: i64,
    posted: Vec<Posted>,
    known: BTreeSet<String>,
}

/// A finding counted, its owner, and the first line of the latest landing
/// note answering it or what followed it up.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    posted: Posted,
    owner: String,
    landed: Option<String>,
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
    /// own owner. One the mesh couldn't be read for judges none.
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
        let mut owners: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
        for finding in &self.found {
            let named = named_as(&finding.posted);
            let opens = format!("{named} · {}", finding.owner);
            let Some(line) = all.behind.iter().find(|line| line.starts_with(&opens)) else {
                continue;
            };
            let old = stamp(&finding.posted.id) / 1000 <= now - PAST;
            let counts = owners.entry(finding.owner.as_str()).or_default();
            counts.0 += 1;
            if old {
                counts.1 += 1;
                // Each line ends with its owner, whom its drag is filed with.
                let note = &line[opens.len()..];
                measure
                    .behind
                    .push(format!("{named}{note} · {}", finding.owner));
            }
        }
        if !owners.is_empty() {
            let each: Vec<String> = owners
                .iter()
                .map(|(owner, (open, old))| format!("{owner} {open} ({old} past a day)"))
                .collect();
            measure.says = format!("{SAYS}; open: {}", each.join(", "));
        }
        measure.now = count(measure.behind.len());
        measure.target = Some(0.0);
        measure
    }
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
/// which it `keeps` for the next. A file that can't be read is opened
/// again each hour until it can, and says so; a folder that can't be
/// listed is an error, since reading none would read as none open.
fn read(messages: &Path, now: i64, kept_at: &Path, keeps: bool) -> Result<Read, String> {
    let kept: Kept = std::fs::read(kept_at)
        .ok()
        .and_then(|json| serde_json::from_slice(&json).ok())
        .filter(|kept: &Kept| kept.v == VERSION)
        .unwrap_or_default();
    let now_ms = now * 1000;
    let from_ms = (now_ms - WITHIN * 1000).max(kept.through_ms - SLACK_MS);
    let (listed, mut known) = listed(messages, from_ms, now_ms - WITHIN * 1000)
        .map_err(|error| format!("the mesh's messages can't be listed: {error}"))?;
    known.extend(kept.known);
    let mut through_ms = kept.through_ms;
    let mut unread = 0;
    let mut first_unread: Option<i64> = None;
    let mut opened = Vec::new();
    for (path, name, written) in listed {
        let one = std::fs::read(&path)
            .ok()
            .and_then(|json| serde_json::from_slice::<Posted>(&json).ok());
        match one {
            Some(one) if genuine(&one, &name) => opened.push(one),
            Some(_) => eprintln!("cadence: {name} isn't the message it names, so it's left out"),
            None => {
                unread += 1;
                first_unread = Some(first_unread.map_or(written, |at| at.min(written)));
            }
        }
        through_ms = through_ms.max(written.min(now_ms));
    }
    // The next reading starts again at the first file it couldn't read.
    if let Some(at) = first_unread {
        through_ms = through_ms.min(at);
    }
    opened.sort_by_key(|one| stamp(&one.id));
    let posted = kept_beside(kept.posted, opened, now_ms);
    known.extend(posted.iter().map(|one| one.from.clone()));
    if keeps {
        let kept = Kept {
            v: VERSION,
            through_ms,
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
        unread,
    })
}

/// Each message file under `messages` written at `from_ms` or later, by
/// this machine's clock, in the folder itself and in each folder for a
/// day the harness files messages into, from the day before `since_ms`
/// on, since one relayed late may be filed under the day it was sent;
/// and the address of every sender a file listed is named for. Files
/// being written, named from a dot, are left for the next.
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
    for entry in std::fs::read_dir(messages)?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) && dated(&name) && name >= earliest {
            folders.push(entry.path());
        }
    }
    let mut listed = Vec::new();
    let mut known = BTreeSet::new();
    for folder in folders {
        for entry in std::fs::read_dir(&folder).into_iter().flatten().flatten() {
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
            let written = entry
                .metadata()
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|at| at.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|at| i64::try_from(at.as_millis()).ok());
            if let Some(written) = written.filter(|&written| written >= from_ms) {
                listed.push((entry.path(), name, written));
            }
        }
    }
    Ok((listed, known))
}

/// Whether `one` is the message its file `name` names, sent by whom its id
/// says, so no file can stand in for another message or another sender.
fn genuine(one: &Posted, name: &str) -> bool {
    name.strip_suffix(".json") == Some(one.id.as_str())
        && one.id.split('-').nth(1) == Some(one.from.as_str())
}

/// When a message was sent, in ms, by its id; 0 where it can't say.
fn stamp(id: &str) -> i64 {
    mesh::stamped_at(id).unwrap_or_default()
}

/// What was `kept`, with what was `opened` since, oldest first, that went
/// to the group or answers or withdraws a message kept; each once, none
/// stamped ahead of `now_ms`, none past `KEPT_FOR`, and only a finding or
/// what follows it past `WITHIN`, so a finding left open keeps counting.
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
        .filter(|one| {
            std::iter::once(one.id.as_str())
                .chain(ancestors(&posted, one))
                .any(|id| posted.get(id).is_some_and(is_finding))
        })
        .map(|one| one.id.clone())
        .collect();
    let mut posted: Vec<Posted> = posted
        .into_values()
        .filter(|one| {
            let at = stamp(&one.id);
            at <= now_ms + AHEAD_MS
                && at >= now_ms - KEPT_FOR * 1000
                && (at >= now_ms - WITHIN * 1000 || threaded.contains(&one.id))
        })
        .collect();
    posted.sort_by_key(|one| stamp(&one.id));
    posted
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
/// another, oldest first, each with the latest landing note in its thread.
/// A finding answering another adds to it while that one is open with no
/// landing note yet, and counts on its own otherwise, so one saying a
/// fixed finding is back counts again. A finding is withdrawn by its
/// sender superseding it, or answering it with a withdrawal.
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
                        && opens_with(one, "withdr")))
        })
    };
    let mut found: Vec<Finding> = Vec::new();
    let mut counted_as: BTreeMap<&str, usize> = BTreeMap::new();
    for one in posted {
        let thread = ancestors(&by_id, one).find_map(|id| counted_as.get(id).copied());
        if is_finding(one) && !withdrawn(one) {
            if let Some(at) = thread.filter(|&at| found[at].landed.is_none()) {
                counted_as.insert(&one.id, at);
            } else {
                counted_as.insert(&one.id, found.len());
                found.push(Finding {
                    posted: one.clone(),
                    owner: owner(one, orchestrator, known),
                    landed: None,
                });
            }
        } else if let Some(at) = thread.filter(|_| opens_with(one, "landed")) {
            found[at].landed = one.text.lines().next().map(str::to_owned);
        }
    }
    found
}

/// Whether `one`'s opening, after its sender's address, starts with
/// `word`, in any case: "IKQ63G: withdrawn", "Landed core #6000".
fn opens_with(one: &Posted, word: &str) -> bool {
    let opening = one.text.trim_start();
    let opening = opening.strip_prefix(one.from.as_str()).unwrap_or(opening);
    opening
        .trim_start_matches([':', ',', ' '])
        .get(..word.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(word))
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
        let word = words.next()?;
        let word = word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        Some(word.strip_suffix("'s").unwrap_or(word).to_owned())
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
/// judged as a miss's check is: what each says, its owner, its latest
/// landing note as its check, and when it was sent.
fn as_misses(found: &[Finding]) -> String {
    found
        .iter()
        .map(|finding| {
            let at = jiff::Timestamp::from_millisecond(stamp(&finding.posted.id))
                .map(|at| at.to_string())
                .ok();
            serde_json::json!({
                "what": named_as(&finding.posted),
                "owner": finding.owner,
                "check": finding.landed,
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

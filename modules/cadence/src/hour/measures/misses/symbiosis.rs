//! The work-system findings posted to the mesh's `#v2-symbiosis` group
//! that have no fix running where they hurt, by owner. On 20 November Joel
//! asked that the cells of his work system keep removing every pain on it
//! and on the mycelial network, and the orchestrator ruled that each
//! owner's open findings there count against it, fixed only once the fix
//! runs on the machine it hurt, never at its merge. A finding closes as a
//! miss does, on the changes its landing notes name once they run here,
//! and one open past a day drags its owner.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::{Class, DAY, Measure, count};
use super::{Checkouts, Judged, open_misses};
use crate::{mesh, write_whole};

/// The measure's key, from one hour to the next.
pub const KEY: &str = "symbiosis-findings";

/// The mesh group the work system's findings go to.
const GROUP: &str = "#v2-symbiosis";

/// How far back findings are read: about as long as the mesh keeps its
/// messages, so one older is gone from both.
const WITHIN: i64 = 7 * DAY;

/// How long a finding may stay open before it drags its owner.
const PAST: i64 = DAY;

/// How far behind the newest message read the next reading opens files
/// again, for a message written a moment after one named later than it.
const SLACK_MS: i64 = 60_000;

/// What the measure says.
const SAYS: &str = "work-system findings in #v2-symbiosis open past a day, with no fix running \
                    where they hurt";

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
/// or answer one that did, and the newest file's time read, in ms, so the
/// next opens only the files written since rather than a week's thousands.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
struct Kept {
    through_ms: i64,
    posted: Vec<Posted>,
}

/// A finding still open: its message, the owner it counts against, and the
/// text of every landing note answering it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Finding {
    posted: Posted,
    owner: String,
    landed: String,
}

/// The findings in `#v2-symbiosis` with no fix running here, as of `now`,
/// in seconds, read under the mesh's `root` and kept in `folder`, their
/// fixes judged as the misses' checks are, from `checkouts` and
/// `switches`. A finding naming no owner counts against `orchestrator`,
/// which assigns it.
#[must_use]
pub fn symbiosis(
    root: &Path,
    folder: &Path,
    checkouts: &[String],
    switches: &str,
    now: i64,
    orchestrator: &str,
) -> Measure {
    let posted = read(root, now, &folder.join("symbiosis.json"));
    let findings = open_findings(&posted, orchestrator);
    let text = as_misses(&findings);
    super::judged(&text, folder, checkouts, switches, |text, repos, judged| {
        measured(&findings, text, repos, judged, now)
    })
    .unwrap_or_else(|error| {
        eprintln!("cadence: judging #v2-symbiosis's fixes: {error}");
        Measure {
            key: KEY.to_owned(),
            class: Class::Functional,
            says: format!("{SAYS} (their fixes unjudged)"),
            now: 0.0,
            target: None,
            owner: orchestrator.to_owned(),
            behind: Vec::new(),
        }
    })
}

/// The messages of the last `WITHIN` before `now` that went to the group
/// or answer or withdraw one that did, oldest first, opening only the
/// files written since the reading kept at `kept`.
fn read(root: &Path, now: i64, kept_at: &Path) -> Vec<Posted> {
    let kept: Kept = std::fs::read(kept_at)
        .ok()
        .and_then(|json| serde_json::from_slice(&json).ok())
        .unwrap_or_default();
    let since_ms = (now - WITHIN) * 1000;
    let from_ms = since_ms.max(kept.through_ms - SLACK_MS);
    let mut names: Vec<(i64, String)> = std::fs::read_dir(root.join("mesh-workflow/messages"))
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter_map(|name| Some((mesh::stamped_at(&name)?, name)))
        .filter(|(ms, _)| *ms >= from_ms)
        .collect();
    names.sort();
    let folder = root.join("mesh-workflow/messages");
    let opened = names.iter().filter_map(|(_, name)| {
        serde_json::from_slice::<Posted>(&std::fs::read(folder.join(name)).ok()?).ok()
    });
    let through_ms = names.last().map_or(kept.through_ms, |(ms, _)| *ms);
    let posted = kept_beside(kept.posted, opened, since_ms);
    let json = serde_json::to_string(&Kept {
        through_ms,
        posted: posted.clone(),
    });
    // A failure to keep only costs the next reading its opens.
    if let Err(error) = json
        .map_err(std::io::Error::from)
        .and_then(|json| write_whole(kept_at, &json))
    {
        eprintln!(
            "cadence: keeping #v2-symbiosis in {}: {error}",
            kept_at.display()
        );
    }
    posted
}

/// What was `kept`, with what was `opened` since that went to the group or
/// answers or withdraws a message that did, each once, oldest first, and
/// none sent before `since_ms`. A message is read after those it answers,
/// since each is named by when it was sent.
fn kept_beside(
    kept: Vec<Posted>,
    opened: impl Iterator<Item = Posted>,
    since_ms: i64,
) -> Vec<Posted> {
    let mut posted: BTreeMap<String, Posted> =
        kept.into_iter().map(|one| (one.id.clone(), one)).collect();
    for one in opened {
        let answers = |id: &Option<String>| id.as_ref().is_some_and(|id| posted.contains_key(id));
        if one.to.iter().any(|to| to == GROUP) || answers(&one.re) || answers(&one.supersedes) {
            posted.insert(one.id.clone(), one);
        }
    }
    let mut posted: Vec<Posted> = posted
        .into_values()
        .filter(|one| mesh::stamped_at(&one.id).is_some_and(|ms| ms >= since_ms))
        .collect();
    posted.sort_by_key(|one| mesh::stamped_at(&one.id));
    posted
}

/// The findings `posted` holds that aren't withdrawn, oldest first. A
/// finding answering another in the group adds to it rather than counting
/// apart, and one is withdrawn by a message superseding it, or by its
/// sender answering it with a withdrawal.
fn open_findings(posted: &[Posted], orchestrator: &str) -> Vec<Finding> {
    let to_group: BTreeSet<&str> = posted
        .iter()
        .filter(|one| one.to.iter().any(|to| to == GROUP))
        .map(|one| one.id.as_str())
        .collect();
    posted
        .iter()
        .filter(|one| one.intent == "finding" && to_group.contains(one.id.as_str()))
        .filter(|one| one.re.as_deref().is_none_or(|re| !to_group.contains(re)))
        .filter(|finding| {
            !posted.iter().any(|one| {
                one.supersedes.as_deref() == Some(finding.id.as_str())
                    || (one.re.as_deref() == Some(finding.id.as_str())
                        && one.from == finding.from
                        && one.text.to_lowercase().contains("withdr"))
            })
        })
        .map(|finding| Finding {
            posted: finding.clone(),
            owner: owner(finding, orchestrator),
            landed: posted
                .iter()
                .filter(|one| one.re.as_deref() == Some(finding.id.as_str()))
                .filter(|one| one.text.contains("landed"))
                .map(|one| one.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
        })
        .collect()
}

/// Who a finding counts against: the address after "for" in its opening,
/// before its first colon, as in "IKQ63G finding for A2VLJN and WCJQSG:";
/// else the first address it went to beside the group but its sender;
/// else `orchestrator`.
fn owner(finding: &Posted, orchestrator: &str) -> String {
    let opening = finding.text.lines().next().unwrap_or_default();
    let opening = opening.split(':').next().unwrap_or_default();
    let mut words = opening.split_whitespace();
    let named = words.find(|word| *word == "for").and_then(|_| words.next());
    let named = named.map(|word| {
        let word = word.trim_end_matches(|c: char| !c.is_ascii_alphanumeric());
        word.strip_suffix("'s").unwrap_or(word)
    });
    named
        .filter(|word| is_address(word))
        .map(str::to_owned)
        .or_else(|| {
            finding
                .to
                .iter()
                .filter_map(|to| to.strip_prefix('@'))
                .find(|to| *to != finding.from)
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

/// `findings` as lines of the misses' ledger, so their landing notes are
/// judged as a miss's check is: what each says, its owner, the notes as
/// its check, and when it was sent.
fn as_misses(findings: &[Finding]) -> String {
    findings
        .iter()
        .map(|finding| {
            let at = mesh::stamped_at(&finding.posted.id)
                .and_then(|ms| jiff::Timestamp::from_millisecond(ms).ok())
                .map(|at| at.to_string());
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

/// How a finding is named on the card: its id's short end, its sender and
/// its opening words, on one line.
fn named_as(posted: &Posted) -> String {
    let short = posted.id.rsplit('-').next().unwrap_or_default();
    let words: Vec<&str> = posted.text.split_whitespace().take(14).collect();
    format!(
        "{short} from {}: {}",
        posted.from,
        words.join(" ").replace('"', "'")
    )
}

/// The measure from the open `findings`, shaped as misses in `text`: how
/// many are open past a day at `now`, against none, moved first by the
/// owner of the oldest, with each owner's open count and each finding
/// behind it.
fn measured(
    findings: &[Finding],
    text: &str,
    repos: &Checkouts,
    judged: &Judged<'_>,
    now: i64,
) -> Measure {
    let all = open_misses(text, repos, judged);
    let old: Vec<&str> = text
        .lines()
        .zip(findings)
        .filter(|(_, finding)| {
            mesh::stamped_at(&finding.posted.id).is_some_and(|ms| ms / 1000 <= now - PAST)
        })
        .map(|(line, _)| line)
        .collect();
    let past = open_misses(&old.join("\n"), repos, judged);
    // Each line opens with its finding's name, as `named_as` gives it.
    let lists = |behind: &[String], finding: &Finding| {
        let named = format!("{} · ", named_as(&finding.posted));
        behind.iter().any(|line| line.starts_with(&named))
    };
    let mut owners: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for finding in findings
        .iter()
        .filter(|finding| lists(&all.behind, finding))
    {
        let counts = owners.entry(finding.owner.as_str()).or_default();
        counts.0 += 1;
        counts.1 += usize::from(lists(&past.behind, finding));
    }
    let mut behind: Vec<String> = owners
        .iter()
        .map(|(owner, (open, old))| format!("{owner}: {open} open, {old} past a day"))
        .collect();
    behind.extend(all.behind.iter().cloned());
    Measure {
        key: KEY.to_owned(),
        class: Class::Functional,
        says: SAYS.to_owned(),
        now: count(past.behind.len()),
        target: Some(0.0),
        owner: if past.owner.is_empty() {
            all.owner
        } else {
            past.owner
        },
        behind,
    }
}

#[cfg(test)]
mod tests;

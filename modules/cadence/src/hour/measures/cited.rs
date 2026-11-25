//! Lines to Joel that name whose word they carry. On 25 November the
//! orchestrator gave him a time a fix would go live that nobody running
//! the switch had given, and put decisions to him that the cell concerned
//! should have put. So each line it gives him with a live or switch time,
//! each thing it asks of him, and each item the morning note puts to him
//! must cite a message from the owner of what it says, or it counts here.
//!
//! The orchestrator cites a reply's line in `cited.jsonl` before giving
//! it, naming the owner, the message's short id and a few of the line's
//! words; the morning note's items end in a comment naming the owner and
//! the message. A message cited counts only when that owner sent it.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Class, DAY, Measure, count, lines, seconds, short};
use crate::mesh::Mesh;

/// The key the count of lines citing no owner takes.
pub const KEY: &str = "lines-uncited";

/// Who writes the lines to Joel, and answers for those citing nobody.
const WRITER: &str = "WCJQSG";

/// Who runs the switches, whose word every switch time carries.
const SWITCHES: &str = "16V6F4";

/// Who answers for the measure when cadence can't read it.
const KEEPER: &str = "1JQPPC";

/// How long the hour looks back over what the writer said.
const SAID_SINCE: i64 = 3600;

/// How long before a citation the message it cites may have been sent.
const CITES_LAST: i64 = 2 * DAY;

/// What the measure counts, as the card says it.
const SAYS: &str = "lines to Joel in the last hour, and items the morning note puts to him, \
                    citing no owner's word";

/// The words that make a line with a clock time in it a live or switch
/// time, the last ones naming a switch.
const TIMED: [&str; 5] = ["live", "rides", "switch", "switched", "switches"];

/// The openings of a sentence asking Joel to do something.
const ASKING: [&str; 4] = ["please ", "could you ", "can you ", "would you "];

/// One message the writer gave Joel, and when, in seconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub at: i64,
    pub text: String,
}

/// One line of `cited.jsonl`: when it was written, whose word it cites,
/// the message's short id and words of the line it backs.
struct Cite {
    at: i64,
    owner: String,
    msgid: String,
    item: String,
}

/// The measure, from the writer's session log for the hour before `now`,
/// the mission's `cited.jsonl` and today's morning note, named for
/// `today`, with the mesh under `state_home` saying who sent each message
/// cited. Whatever can't be read leaves it unjudged, saying why, rather
/// than reading no lines as none uncited.
#[must_use]
pub fn read(state_home: &Path, now: i64, today: &str) -> Measure {
    agent_folder()
        .ok_or_else(|| "no home here to find pi's logs under".to_owned())
        .and_then(|agent| gathered(state_home, &agent.join("sessions"), now, today))
        .unwrap_or_else(|why| Measure {
            says: format!("{SAYS} (unread: {why})"),
            target: None,
            owner: KEEPER.to_owned(),
            ..uncited(&[], "", "", &BTreeMap::new())
        })
}

/// The measure as [`read`] gathers it, with pi's session logs under
/// `logs`, or why it can't be read.
fn gathered(state_home: &Path, logs: &Path, now: i64, today: &str) -> Result<Measure, String> {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return Err("no mesh messages folder here".to_owned());
    }
    let session = mesh
        .presence(WRITER)
        .map(|presence| presence.session)
        .ok_or_else(|| format!("no session here goes by {WRITER}"))?;
    let log =
        log_of(logs, &session).ok_or_else(|| format!("no log of {WRITER}'s session {session}"))?;
    let said = said_in(&log, now - SAID_SINCE, now)
        .map_err(|err| format!("{WRITER}'s log can't be read: {err}"))?;
    let mission = state_home.join("mission");
    let cited = absent_as_none(&mission.join("cited.jsonl"))?;
    let note = absent_as_none(&mission.join(format!("morning-{today}.md")))?;
    let ids = cited_ids(&cited, &note);
    let senders = mesh.senders((now - CITES_LAST) * 1000, &ids);
    Ok(uncited(&said, &note, &cited, &senders))
}

/// The file at `path`, or nothing where none is written yet, which cites
/// nothing; one there that can't be read is said why.
fn absent_as_none(path: &Path) -> Result<String, String> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(String::new()),
        Err(err) => Err(format!("{} can't be read: {err}", path.display())),
    }
}

/// Where pi keeps its sessions' logs, as the pis here run.
fn agent_folder() -> Option<PathBuf> {
    env::var_os("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".pi/agent")))
}

/// The log of `session` under `logs`, a folder for each working directory
/// holding a log for each session named `<started>_<session>.jsonl`.
fn log_of(logs: &Path, session: &str) -> Option<PathBuf> {
    let named = format!("_{session}.jsonl");
    fs::read_dir(logs)
        .ok()?
        .filter_map(Result::ok)
        .filter_map(|folder| fs::read_dir(folder.path()).ok())
        .flatten()
        .filter_map(Result::ok)
        .map(|log| log.path())
        .find(|log| log.to_string_lossy().ends_with(&named))
}

/// What the writer said in the session log at `log` after `from` and by
/// `to`, each assistant message's text. Only lines naming the assistant
/// are parsed, since the log holds far more than what it said.
fn said_in(log: &Path, from: i64, to: i64) -> io::Result<Vec<Said>> {
    let mut said = Vec::new();
    for line in BufReader::new(fs::File::open(log)?).lines() {
        let line = line?;
        if !line.contains("\"assistant\"") {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        let at = seconds(&entry["timestamp"]).unwrap_or_default();
        if entry["type"] != "message" || entry["message"]["role"] != "assistant" {
            continue;
        }
        if at <= from || at > to {
            continue;
        }
        let text: Vec<&str> = entry["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect();
        said.push(Said {
            at,
            text: text.join("\n"),
        });
    }
    Ok(said)
}

/// The short ids of every message cited in `cited` and the morning note
/// `note`, so the mesh opens only those.
#[must_use]
pub fn cited_ids(cited: &str, note: &str) -> BTreeSet<String> {
    let from_cites = cites(cited).into_iter().map(|cite| cite.msgid);
    let from_note = for_you(note)
        .into_iter()
        .filter_map(|item| owner_comment(&item).map(|(_, id)| id.to_owned()));
    from_cites.chain(from_note).collect()
}

/// Each line in `said` that gives a live or switch time, or asks Joel to
/// do something, with no citation before it backing it, and each item
/// the morning note `note` puts to him with no owner's message named,
/// by the citations in `cited` and who sent each message by its short
/// id, `senders`. A citation backs a line only from an owner other than
/// the writer whose message it names, and a switch time only from the
/// cell running the switches.
#[must_use]
pub fn uncited(
    said: &[Said],
    note: &str,
    cited: &str,
    senders: &BTreeMap<String, String>,
) -> Measure {
    let backing: Vec<Cite> = cites(cited)
        .into_iter()
        .filter(|cite| backed(cite, senders))
        .collect();
    let mut behind = Vec::new();
    for message in said {
        let before: Vec<&Cite> = backing.iter().filter(|c| c.at <= message.at).collect();
        behind.extend(timed_uncited(&message.text, &before));
        behind.extend(asks_uncited(&message.text, &before));
    }
    for item in for_you(note) {
        let named = owner_comment(&item).is_some_and(|(owner, id)| {
            !owner.eq_ignore_ascii_case(WRITER)
                && senders
                    .get(id)
                    .is_some_and(|from| from.eq_ignore_ascii_case(owner))
        });
        if !named {
            let words = item.split("<!--").next().unwrap_or_default();
            behind.push(format!(
                "for you this morning: {}, naming no owner's message · {WRITER}",
                shown(words)
            ));
        }
    }
    Measure {
        key: KEY.to_owned(),
        class: Class::Cells,
        says: SAYS.to_owned(),
        now: count(behind.len()),
        target: Some(0.0),
        owner: WRITER.to_owned(),
        behind,
    }
}

/// The citations in `cited`, each a line with its time, owner, message
/// and words; a line missing any of them cites nothing.
fn cites(cited: &str) -> Vec<Cite> {
    lines(cited)
        .filter_map(|line| {
            Some(Cite {
                at: seconds(&line["ts"])?,
                owner: line["owner"].as_str()?.to_owned(),
                msgid: line["msgid"].as_str()?.to_owned(),
                item: line["item"].as_str()?.to_lowercase(),
            })
        })
        .collect()
}

/// Whether `cite` names a message its owner sent, by `senders`, from an
/// owner other than the writer.
fn backed(cite: &Cite, senders: &BTreeMap<String, String>) -> bool {
    !cite.owner.eq_ignore_ascii_case(WRITER)
        && senders
            .get(&cite.msgid)
            .is_some_and(|from| from.eq_ignore_ascii_case(&cite.owner))
}

/// Each sentence of `text` giving a live or switch time that none of
/// `cites` backs: one naming the same clock time, and for a switch, from
/// the cell running the switches.
fn timed_uncited(text: &str, cites: &[&Cite]) -> Vec<String> {
    let mut uncited = Vec::new();
    for sentence in sentences(text) {
        let clocks = clock_times(sentence);
        let said = words(sentence);
        if clocks.is_empty() || !said.iter().any(|w| TIMED.contains(&w.as_str())) {
            continue;
        }
        let switch = said.iter().any(|w| w.starts_with("switch"));
        let backed = cites.iter().any(|cite| {
            clocks
                .iter()
                .any(|clock| cite.item.contains(clock.as_str()))
                && (!switch || cite.owner.eq_ignore_ascii_case(SWITCHES))
        });
        if !backed {
            let whose = if switch { SWITCHES } else { "its owner" };
            uncited.push(format!(
                "{}, a time with no word cited from {whose} · {WRITER}",
                shown(sentence)
            ));
        }
    }
    uncited
}

/// Each thing `text` asks Joel to do that none of `cites` backs by words
/// all found in it: the points listed under a line saying he's needed or
/// that the work is blocked, and each sentence opening with a request.
fn asks_uncited(text: &str, cites: &[&Cite]) -> Vec<String> {
    let mut asks: Vec<&str> = Vec::new();
    let mut under = false;
    for line in text.lines().map(str::trim) {
        if line.to_lowercase().contains("need you") || line.starts_with("Blocked:") {
            under = true;
        } else if under && bullet(line).is_some() {
            asks.extend(bullet(line));
        } else {
            under = under && line.is_empty();
            asks.extend(sentences(line).into_iter().filter(|s| {
                let s = s.to_lowercase();
                ASKING.iter().any(|opening| s.starts_with(opening))
            }));
        }
    }
    asks.into_iter()
        .filter(|ask| {
            let said = words(ask);
            !cites.iter().any(|cite| {
                let named = words(&cite.item);
                !named.is_empty() && named.iter().all(|w| said.contains(w))
            })
        })
        .map(|ask| format!("{}, an ask citing no owner's word · {WRITER}", shown(ask)))
        .collect()
}

/// The items the morning note `note` lists under its line putting things
/// to Joel, each with the lines that carry it on, until a line that is
/// neither.
fn for_you(note: &str) -> Vec<String> {
    let mut items: Vec<String> = Vec::new();
    let mut listing = false;
    for line in note.lines() {
        if !listing {
            listing = line.contains("For you");
            continue;
        }
        if let Some(item) = bullet(line.trim_end()) {
            items.push(item.to_owned());
        } else if line.starts_with(char::is_whitespace) && !line.trim().is_empty() {
            if let Some(last) = items.last_mut() {
                last.push(' ');
                last.push_str(line.trim());
            }
        } else if !line.trim().is_empty() {
            break;
        }
    }
    items
}

/// The owner and message an item's closing comment names, as
/// `<!-- owner: CELL id -->`.
fn owner_comment(item: &str) -> Option<(&str, &str)> {
    let comment = item.trim_end().strip_suffix("-->")?;
    let named = comment.rsplit_once("<!--")?.1.trim();
    let mut parts = named.strip_prefix("owner:")?.split_whitespace();
    let found = (parts.next()?, parts.next()?);
    parts.next().is_none().then_some(found)
}

/// A list item's words, opening `- ` or `* `, if `line` is one.
fn bullet(line: &str) -> Option<&str> {
    line.strip_prefix("- ").or_else(|| line.strip_prefix("* "))
}

/// A sentence or item as the card shows it: its opening words, with no
/// stop after them.
fn shown(said: &str) -> String {
    short(said.trim().trim_end_matches(['.', '?', '!']))
}

/// The sentences of `text`, by its lines and the stops ending them, a
/// stop ending one only before a space or the line's end, so a name such
/// as `core.nix` stays whole.
fn sentences(text: &str) -> Vec<&str> {
    let mut found = Vec::new();
    for line in text.lines() {
        let mut start = 0;
        let mut chars = line.char_indices().peekable();
        while let Some((at, c)) = chars.next() {
            let next_is_space = chars.peek().is_none_or(|(_, next)| next.is_whitespace());
            if matches!(c, '.' | '?' | '!') && next_is_space {
                found.push(line[start..=at].trim());
                start = at + 1;
            }
        }
        found.push(line[start..].trim());
    }
    found.retain(|sentence| !sentence.is_empty());
    found
}

/// The words of `text`, lower case, of letters and digits.
fn words(text: &str) -> BTreeSet<String> {
    text.split(|c: char| !c.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .collect()
}

/// Each clock time in `text`, as `9:05` or `10:06`, leaving out the
/// seconds of one given to the second.
fn clock_times(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let digit = |at: usize| bytes.get(at).is_some_and(u8::is_ascii_digit);
    let mut found = Vec::new();
    for (at, _) in text.match_indices(':') {
        if at == 0 || !digit(at - 1) || !digit(at + 1) || !digit(at + 2) || digit(at + 3) {
            continue;
        }
        let start = if at >= 2 && digit(at - 2) {
            at - 2
        } else {
            at - 1
        };
        let before = start.checked_sub(1).and_then(|b| bytes.get(b));
        if before.is_some_and(|b| b.is_ascii_digit() || *b == b':') {
            continue;
        }
        found.push(text[start..at + 3].to_owned());
    }
    found
}

#[cfg(test)]
mod tests;

//! Lines to Joel that name whose word they carry. On 25 November the
//! orchestrator gave him a time a fix would go live that nobody running
//! the switch had given, and put decisions to him that the cell concerned
//! should have put. So each line it gives him with a live or switch time,
//! each thing it asks of him, and each item the morning note puts to him
//! must cite a message from another cell, or it counts here.
//!
//! The orchestrator cites a reply's line in `cited.jsonl` before giving
//! it, naming the owner, the message's short id and a few of the line's
//! words; the morning note's items end in a comment naming the owner and
//! the message. A message cited counts only when that owner sent it, and
//! a time only when that message says it. Which cell owns a piece of work
//! is the orchestrator's to say, so only a switch time is held to one
//! cell: the one running the switches.

use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{Class, DAY, Measure, count, seconds, short};
use crate::mesh::{Mesh, Sent};

/// The key the count of lines citing no owner takes.
pub const KEY: &str = "lines-uncited";

/// Who writes the lines to Joel when the hour names no orchestrator.
const WRITER: &str = "WCJQSG";

/// Who runs the switches, whose word every switch time carries.
const SWITCHES: &str = "16V6F4";

/// Who answers for the measure when cadence can't read it.
const KEEPER: &str = "1JQPPC";

/// How long the hour looks back over what the writer said. A line said
/// can't be taken back, so it counts for a day rather than leaving the
/// card the next hour as if something had been fixed.
const SAID_SINCE: i64 = DAY;

/// How long before now a message cited may have been sent and still back
/// a line.
const CITES_LAST: i64 = 2 * DAY;

/// Under this many bytes, the stretch of a log left to search for where
/// the day starts is read through instead.
const SEARCH_LEAST: u64 = 1 << 16;

/// How many lines past a point in a log are tried for a time.
const PROBED: usize = 8;

/// What the measure counts, as the card says it.
const SAYS: &str = "lines to Joel in the last day, and items the morning note puts to him, \
                    citing no other cell's word";

/// The words that make a line with a clock time in it a live or switch
/// time; those naming a switch open `switch`.
const TIMED: [&str; 6] = [
    "live",
    "rides",
    "switch",
    "switched",
    "switches",
    "switching",
];

/// What a line says when the points listed under it need Joel.
const NEEDED: [&str; 5] = [
    "need you",
    "needs you",
    "need joel",
    "needs joel",
    "needed from you",
];

/// The openings of a sentence asking Joel to do something or to decide.
const ASKING: [&str; 12] = [
    "please ",
    "could you ",
    "can you ",
    "would you ",
    "will you ",
    "should i ",
    "shall i ",
    "do you want",
    "want me to",
    "which do you",
    "ok to ",
    "okay to ",
];

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

/// The measure, from the logs of every session the hour's orchestrator
/// has gone by over the day before `now`, the mission's `cited.jsonl`
/// and today's morning note, named for `today`, with the mesh under
/// `state_home` saying who sent each message cited and what it said.
/// Whatever can't be read leaves it unjudged, saying why, rather than
/// reading no lines as none uncited; a line of `cited.jsonl` that can't
/// be read backs nothing instead, and the card says so.
#[must_use]
pub fn read(state_home: &Path, now: i64, today: &str, orchestrator: Option<&str>) -> Measure {
    let writer = orchestrator.unwrap_or(WRITER);
    agent_folder()
        .ok_or_else(|| "no home here to find pi's logs under".to_owned())
        .and_then(|agent| gathered(state_home, &agent.join("sessions"), now, today, writer))
        .unwrap_or_else(|why| unread(&why))
}

/// The measure unjudged, saying `why` it can't be read.
#[must_use]
pub fn unread(why: &str) -> Measure {
    Measure {
        says: format!("{SAYS} (unread: {why})"),
        target: None,
        owner: KEEPER.to_owned(),
        ..judged(Vec::new(), KEEPER)
    }
}

/// The measure as [`read`] gathers it for `writer`, with pi's session
/// logs under `logs`, or why it can't be read.
fn gathered(
    state_home: &Path,
    logs: &Path,
    now: i64,
    today: &str,
    writer: &str,
) -> Result<Measure, String> {
    let mesh = Mesh {
        root: state_home.join("pi/agentic-harness.pi"),
    };
    if !mesh.root.join("mesh-workflow/messages").is_dir() {
        return Err("no mesh messages folder here".to_owned());
    }
    let newest = mesh
        .presence(writer)
        .map(|presence| presence.session)
        .ok_or_else(|| format!("no session here goes by {writer}"))?;
    let sessions: BTreeSet<String> = mesh
        .all()
        .into_iter()
        .filter(|p| p.addresses.iter().any(|a| a == writer))
        .map(|p| p.session)
        .filter(|session| !session.is_empty())
        .collect();
    let found = logs_of(logs, &sessions);
    if !found.iter().any(|(session, _)| *session == newest) {
        return Err(format!("no log of {writer}'s session {newest}"));
    }
    let mut said = Vec::new();
    for (_, log) in &found {
        let read = said_in(log, now - SAID_SINCE, now)
            .map_err(|err| format!("{writer}'s log {} can't be read: {err}", log.display()))?;
        said.extend(read);
    }
    said.sort_by_key(|s| s.at);
    let mission = state_home.join("mission");
    let cited = absent_as_none(&mission.join("cited.jsonl"))?;
    let note = absent_as_none(&mission.join(format!("morning-{today}.md")))?;
    let ids = cited_ids(&cited, &note)?;
    let senders = mesh.senders((now - CITES_LAST) * 1000, &ids)?;
    uncited(&said, &note, &cited, &senders, writer)
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

/// Every log under `logs` of one of `sessions`, with its session, from a
/// folder for each working directory holding a log for each session it
/// ran, named `<started>_<session>.jsonl`; a session resumed elsewhere
/// has a log in each.
fn logs_of(logs: &Path, sessions: &BTreeSet<String>) -> Vec<(String, PathBuf)> {
    fs::read_dir(logs)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|folder| fs::read_dir(folder.path()).ok())
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|log| {
            let name = log.file_name().to_str()?.to_owned();
            let session = name.strip_suffix(".jsonl")?.rsplit_once('_')?.1;
            sessions
                .contains(session)
                .then(|| (session.to_owned(), log.path()))
        })
        .collect()
}

/// What the writer said in the session log at `log` after `from` and by
/// `to`, each assistant message's text. Only lines naming the assistant
/// are parsed, since the log holds far more than what it said, and the
/// reading starts near `from`. An assistant entry with no time to read,
/// or a line holding the assistant's words that isn't whole before the
/// log's last, leaves the log unread, unless the next line with a time
/// is from by `from`: entries are in time order, so the torn one was
/// too, and a log torn by a crash and then resumed is unread for a day
/// at most, not for good.
fn said_in(log: &Path, from: i64, to: i64) -> io::Result<Vec<Said>> {
    let mut file = File::open(log)?;
    let start = start_of(&mut file, from)?;
    file.seek(SeekFrom::Start(start))?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    if start > 0 {
        reader.read_until(b'\n', &mut line)?;
    }
    let mut said = Vec::new();
    // A torn line, and whether lines with no time to read came after it.
    let (mut torn, mut after) = (false, false);
    loop {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            break;
        }
        let text = String::from_utf8_lossy(&line);
        if torn {
            let next = serde_json::from_str::<Value>(&text).ok();
            match next.and_then(|entry| seconds(&entry["timestamp"])) {
                Some(at) if at <= from => (torn, after) = (false, false),
                Some(_) => {
                    return Err(io::Error::other("a line of the assistant's isn't whole"));
                }
                None => {
                    after |= !text.trim().is_empty();
                    continue;
                }
            }
        }
        if !text.contains("\"assistant\"") {
            continue;
        }
        let Ok(entry) = serde_json::from_str::<Value>(&text) else {
            torn = true;
            continue;
        };
        if entry["type"] != "message" || entry["message"]["role"] != "assistant" {
            continue;
        }
        let at = seconds(&entry["timestamp"])
            .ok_or_else(|| io::Error::other("an assistant entry has no time to read"))?;
        if at <= from || at > to {
            continue;
        }
        let words: Vec<&str> = entry["message"]["content"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|block| block["type"] == "text")
            .filter_map(|block| block["text"].as_str())
            .collect();
        said.push(Said {
            at,
            text: words.join("\n"),
        });
    }
    // Only the log's last line may still be being written; a torn one
    // with more after it, none of which dates it, is torn within the day.
    if torn && after {
        return Err(io::Error::other("a line of the assistant's isn't whole"));
    }
    Ok(said)
}

/// Where to start reading the log at `file` for what was said after
/// `from`: a point whose next whole line is an entry at or before it, or
/// the log's start, found by halving, as a log's entries are in time
/// order and a session's log grows for weeks. A point with no time near
/// it is searched before, so nothing after `from` is passed over.
fn start_of(file: &mut File, from: i64) -> io::Result<u64> {
    let (mut low, mut high) = (0, file.metadata()?.len());
    while high - low > SEARCH_LEAST {
        let mid = low + (high - low) / 2;
        if time_after(file, mid)?.is_some_and(|at| at <= from) {
            low = mid;
        } else {
            high = mid;
        }
    }
    Ok(low)
}

/// The time of the first whole line after `offset` in `file` that has
/// one, among the next few.
fn time_after(file: &mut File, offset: u64) -> io::Result<Option<i64>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(&mut *file);
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line)?;
    for _ in 0..PROBED {
        line.clear();
        if reader.read_until(b'\n', &mut line)? == 0 {
            return Ok(None);
        }
        let at = serde_json::from_slice::<Value>(&line)
            .ok()
            .and_then(|entry| seconds(&entry["timestamp"]));
        if at.is_some() {
            return Ok(at);
        }
    }
    Ok(None)
}

/// The short ids of every message cited in `cited` and the morning note
/// `note`, so the mesh opens only those, or why the note can't be read.
pub fn cited_ids(cited: &str, note: &str) -> Result<BTreeSet<String>, String> {
    let from_cites = cites(cited).0.into_iter().map(|cite| cite.msgid);
    let from_note = for_you(note)?
        .into_iter()
        .filter_map(|item| owner_comment(&item).map(|(_, id)| id.to_owned()));
    Ok(from_cites.chain(from_note).collect())
}

/// Each line in `said` that gives a live or switch time, or asks Joel to
/// do something, with no citation before it backing it, and each item
/// the morning note `note` puts to him with no owner's message named,
/// by the citations in `cited` and the messages cited by short id,
/// `senders`. A citation backs a line only from a cell other than
/// `writer` whose message it names, a time only when that message says
/// it, and a switch time only from the cell running the switches. A
/// sentence both giving a time and asking counts once.
pub fn uncited(
    said: &[Said],
    note: &str,
    cited: &str,
    senders: &BTreeMap<String, Sent>,
    writer: &str,
) -> Result<Measure, String> {
    let (all, unreadable) = cites(cited);
    let backing: Vec<Cite> = all
        .into_iter()
        .filter(|cite| backed(cite, senders, writer))
        .collect();
    let mut behind = Vec::new();
    for message in said {
        let before: Vec<&Cite> = backing.iter().filter(|c| c.at <= message.at).collect();
        let timed = timed_uncited(&message.text, &before, senders);
        let counted: Vec<&str> = timed.iter().map(|(sentence, _)| *sentence).collect();
        let asked = asks_uncited(&message.text, &before, &counted);
        behind.extend(
            timed
                .into_iter()
                .map(|(_, line)| format!("{line} · {writer}")),
        );
        behind.extend(asked.into_iter().map(|line| format!("{line} · {writer}")));
    }
    behind.extend(noted_uncited(note, senders, writer)?);
    let mut measure = judged(behind, writer);
    if let Some(first) = unreadable.first() {
        let count = unreadable.len();
        let lines = if count == 1 { "line" } else { "lines" };
        measure.says = format!(
            "{SAYS} ({count} {lines} of cited.jsonl can't be read, backing nothing; the first, \
             line {first})"
        );
    }
    Ok(measure)
}

/// The measure judged, with `behind` its lines, `writer` answering.
fn judged(behind: Vec<String>, writer: &str) -> Measure {
    Measure {
        key: KEY.to_owned(),
        class: Class::Cells,
        says: SAYS.to_owned(),
        now: count(behind.len()),
        target: Some(0.0),
        owner: writer.to_owned(),
        behind,
    }
}

/// The citations in `cited`, each a line with its time, owner, message
/// and words, and the number of each line that can't be read. One that
/// can't is left out, backing nothing, rather than leaving every hour
/// after it unjudged: the file only grows, so one bad line would
/// otherwise take the measure off the card for good.
fn cites(cited: &str) -> (Vec<Cite>, Vec<usize>) {
    let read = |line: &Value| {
        Some(Cite {
            at: seconds(&line["ts"])?,
            owner: line["owner"].as_str()?.to_owned(),
            msgid: line["msgid"].as_str()?.to_owned(),
            item: line["item"].as_str()?.to_lowercase(),
        })
    };
    let (mut all, mut unreadable) = (Vec::new(), Vec::new());
    let numbered = cited.lines().zip(1..);
    for (line, number) in numbered.filter(|(line, _)| !line.trim().is_empty()) {
        match serde_json::from_str::<Value>(line)
            .ok()
            .as_ref()
            .and_then(read)
        {
            Some(cite) => all.push(cite),
            None => unreadable.push(number),
        }
    }
    (all, unreadable)
}

/// Whether `cite` names a message its owner sent, by `senders`, from an
/// owner other than `writer`.
fn backed(cite: &Cite, senders: &BTreeMap<String, Sent>, writer: &str) -> bool {
    !cite.owner.eq_ignore_ascii_case(writer)
        && senders
            .get(&cite.msgid)
            .is_some_and(|sent| sent.from.eq_ignore_ascii_case(&cite.owner))
}

/// Each sentence of `text` giving a live or switch time with a clock
/// time no citation in `cites` backs, with its line: one naming the same
/// clock time whose message, by `senders`, says it too, and for a
/// switch, from the cell running the switches.
fn timed_uncited<'a>(
    text: &'a str,
    cites: &[&Cite],
    senders: &BTreeMap<String, Sent>,
) -> Vec<(&'a str, String)> {
    let mut uncited = Vec::new();
    for sentence in sentences(text) {
        let clocks = clock_times(sentence);
        let said = words(sentence);
        if clocks.is_empty() || !said.iter().any(|w| TIMED.contains(&w.as_str())) {
            continue;
        }
        let switch = said.iter().any(|w| w.starts_with("switch"));
        let gives = |cite: &&Cite, clock: &u32| {
            let told = senders.get(&cite.msgid).map(|sent| sent.text.as_str());
            clock_times(&cite.item).contains(clock)
                && told.is_some_and(|told| clock_times(told).contains(clock))
                && (!switch || cite.owner.eq_ignore_ascii_case(SWITCHES))
        };
        if !clocks
            .iter()
            .all(|clock| cites.iter().any(|c| gives(c, clock)))
        {
            let whose = if switch { SWITCHES } else { "its owner" };
            let line = format!(
                "{}, a time with no word cited from {whose}",
                shown(sentence)
            );
            uncited.push((sentence, line));
        }
    }
    uncited
}

/// Each thing `text` asks Joel to do that none of `cites` backs by two
/// or more words all found in it, leaving out what `counted` already
/// holds: the points listed under a line saying he's needed or that the
/// work is blocked, and each sentence opening with a request or a
/// question for him to decide.
fn asks_uncited(text: &str, cites: &[&Cite], counted: &[&str]) -> Vec<String> {
    let asking = |sentence: &&str| {
        let opening = plain(sentence).to_lowercase();
        ASKING.iter().any(|ask| opening.starts_with(ask))
    };
    let mut asks: Vec<&str> = Vec::new();
    let mut under = false;
    for line in text.lines() {
        if heads(line) {
            under = true;
        } else if let Some(listed) = point(line) {
            if under {
                asks.push(listed);
            } else {
                asks.extend(sentences(listed).into_iter().filter(asking));
            }
        } else {
            under = under && line.trim().is_empty();
            asks.extend(sentences(line).into_iter().filter(asking));
        }
    }
    asks.into_iter()
        .filter(|ask| !counted.iter().any(|sentence| ask.contains(*sentence)))
        .filter(|ask| {
            let said = words(ask);
            !cites.iter().any(|cite| {
                let named = words(&cite.item);
                named.len() >= 2 && named.iter().all(|w| said.contains(w))
            })
        })
        .map(|ask| format!("{}, an ask citing no owner's word", shown(ask)))
        .collect()
}

/// Whether `line` says the points listed under it need Joel, or that the
/// work is blocked.
fn heads(line: &str) -> bool {
    let said = plain(line).to_lowercase();
    let needed = NEEDED.iter().any(|need| said.contains(need))
        && !said.contains("n't need")
        && !said.contains("not need");
    needed || said.starts_with("blocked:")
}

/// Each item the morning note `note` puts to Joel with no message from a
/// cell other than `writer` named, by `senders`, or why the note can't be
/// read.
fn noted_uncited(
    note: &str,
    senders: &BTreeMap<String, Sent>,
    writer: &str,
) -> Result<Vec<String>, String> {
    let mut uncited = Vec::new();
    for item in for_you(note)? {
        let named = owner_comment(&item).is_some_and(|(owner, id)| {
            !owner.eq_ignore_ascii_case(writer)
                && senders
                    .get(id)
                    .is_some_and(|sent| sent.from.eq_ignore_ascii_case(owner))
        });
        if !named {
            let words = item.split("<!--").next().unwrap_or_default();
            uncited.push(format!(
                "for you this morning: {}, naming no owner's message · {writer}",
                shown(words)
            ));
        }
    }
    Ok(uncited)
}

/// The items the morning note `note` lists under its line opening "For
/// you", each with the lines indented past it that carry it on, its own
/// points among them, until a line that is neither; or why it can't be
/// read, when a note is written with no such line, so a note in another
/// shape is never read as putting nothing to Joel.
fn for_you(note: &str) -> Result<Vec<String>, String> {
    let mut lines = note.lines();
    let opens = |line: &str| {
        plain(line)
            .to_lowercase()
            .strip_prefix("for you")
            .is_some_and(|rest| !rest.starts_with(char::is_alphanumeric))
    };
    if lines.by_ref().find(|line| opens(line)).is_none() {
        return if note.trim().is_empty() {
            Ok(Vec::new())
        } else {
            Err("the morning note has no line opening \"For you\"".to_owned())
        };
    }
    let mut items: Vec<String> = Vec::new();
    let mut first: Option<usize> = None;
    let mut led = false;
    for line in lines {
        if line.trim().is_empty() {
            continue;
        }
        let depth = line.len() - line.trim_start().len();
        let carried = first.is_some_and(|first| depth > first);
        if !carried && let Some(item) = point(line) {
            first.get_or_insert(depth);
            items.push(item.to_owned());
        } else if carried && let Some(last) = items.last_mut() {
            last.push(' ');
            last.push_str(line.trim());
        } else if first.is_none() && !led && !heading(line) {
            // One line of words leading into the list, as "Two calls
            // today:"; a second is the next part's, as "Overnight:".
            led = true;
        } else {
            break;
        }
    }
    if items.is_empty() && led {
        return Err("the morning note's \"For you\" holds words but no list".to_owned());
    }
    Ok(items)
}

/// Whether `line` opens the note's next part: a heading, or a line opening
/// in bold, as "**For you:**" does.
fn heading(line: &str) -> bool {
    let line = line.trim_start();
    line.starts_with('#') || line.starts_with("**") || line.starts_with("__")
}

/// The owner and message the last comment in an item names, as
/// `<!-- owner: CELL id -->`.
fn owner_comment(item: &str) -> Option<(&str, &str)> {
    item.rmatch_indices("<!--").find_map(|(at, _)| {
        let (named, _) = item[at + 4..].split_once("-->")?;
        let mut parts = named.trim().strip_prefix("owner:")?.split_whitespace();
        let found = (parts.next()?, parts.next()?);
        parts.next().is_none().then_some(found)
    })
}

/// A list item's words, if `line` is one: opening `- `, `* ` or `+ `, or
/// a number and `. ` or `) `.
fn point(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let marked = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .or_else(|| line.strip_prefix("+ "));
    if marked.is_some() {
        return marked;
    }
    let rest = line.trim_start_matches(|c: char| c.is_ascii_digit());
    if rest.len() == line.len() {
        return None;
    }
    rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))
}

/// `text` without the marks a heading or bold puts around it.
fn plain(text: &str) -> &str {
    text.trim()
        .trim_start_matches(['#', ' '])
        .trim_matches(['*', '_'])
        .trim()
}

/// A sentence or item as the card shows it: its opening words, with no
/// bold and no stop after them.
fn shown(said: &str) -> String {
    short(plain(said).trim_end_matches(['.', '?', '!', ':']))
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

/// Each clock time in `text`, as `9:05`, `10:06` or `22:45`, read as
/// its minutes past twelve on a 12-hour dial and leaving out the seconds
/// of one given to the second: lines to Joel say "10:45" where the owner
/// said "22:45", and both are the one time.
fn clock_times(text: &str) -> Vec<u32> {
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
        let (hour, minute) = (&text[start..at], &text[at + 1..at + 3]);
        if let (Ok(hour), Ok(minute)) = (hour.parse::<u32>(), minute.parse::<u32>()) {
            found.push(hour % 12 * 60 + minute);
        }
    }
    found
}

#[cfg(test)]
mod tests;

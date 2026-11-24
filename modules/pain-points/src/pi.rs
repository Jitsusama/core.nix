//! Joel's messages to pi, read from pi's own session logs: each message he
//! typed, the calls that answered it until one handed the turn back, and
//! how long its tools ran, so lost.rs can say what each go lost him and
//! pi's moments rank beside the launcher's.
//!
//! What he typed is read here and in words.rs alone, in memory, to tell
//! whether a message repeats his last one in the same session, reads as a
//! correction or asks where things stand. The text is never written,
//! logged or sent, and nothing kept counts its length or its words: the
//! day file and the reading state hold times, flags and counts of
//! messages. Tool calls, their results and what the agents wrote are never
//! decoded: a tool result gives only the time and tool name at its line's
//! head, and an assistant entry's content is skipped unread. Each line is
//! dropped as soon as it's read.

mod lost;

use crate::words::{ALIKE, alike, asking_status, automated, correcting, nudging};
pub use lost::painful;
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

/// A message much like his last in the same session, within this long,
/// says the last didn't do it.
pub const SAID_AGAIN_MS: i64 = 300_000;
/// Tools that wait on Joel himself, whose time is his, not a wait on pi.
const HIS: [&str; 1] = ["ask"];
/// How far into a line its role is looked for. pi writes an entry's type,
/// ids and time before its message's role, about 130 bytes in.
const HEAD: usize = 400;
/// How soon after a compaction's ask the call it cuts ends. pi's own cut
/// lands a tenth of a second after the ask; five seconds leaves room for
/// a loaded machine, and an abort later than that is his.
const ASK_CUTS_MS: i64 = 5_000;

/// How one call to the model ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stop {
    Done,
    Tool,
    Error,
    Aborted,
    /// Cut short by pi itself, to compact the context: an abort he
    /// didn't ask for, which hands the turn back all the same.
    Compacted,
}

/// One call to the model: when it began, in milliseconds since 1970, how
/// long it streamed, and how it ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub start: i64,
    pub ms: i64,
    pub stop: Stop,
    /// How long from its start the model took to its first token, where
    /// the harness logged it.
    #[serde(default)]
    pub first: Option<i64>,
}

/// A message Joel typed, and the calls that answered it before the next
/// message reached the session.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each is a separate reading of his words, any of them true together"
)]
pub struct Turn {
    /// Where its line begins in the log, so the next read can find the
    /// message again to compare the one after it with.
    pub offset: u64,
    /// When he pressed Enter.
    pub enter: i64,
    /// When it reached the session, later than `enter` when it waited
    /// behind a turn already running.
    pub delivered: i64,
    pub calls: Vec<Call>,
    /// Much like his last message in the session, soon after it.
    pub again: bool,
    pub correction: bool,
    pub status: bool,
    /// Only set the session going again, so he waits on its first reply
    /// alone.
    #[serde(default)]
    pub nudge: bool,
    /// How long its tools ran, leaving out those that wait on Joel.
    #[serde(default)]
    pub tools_ms: i64,
    /// How long tools that wait on Joel, a question put to him, ran.
    #[serde(default)]
    pub his_ms: i64,
    /// Where the next tool's time is counted from: the later of the last
    /// call's end and the last tool's.
    #[serde(default)]
    pub tool_mark: i64,
}

/// Where his turn stands after one of its calls stopped on an error.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Errored {
    /// No call of his turn is left on an error.
    #[default]
    No,
    /// The last call stopped on an error, with no word yet that pi tries
    /// again.
    Untried,
    /// A peer's or the harness's message came after that error, so the
    /// call after it, if pi never tries again, answers them, not him.
    Crossed,
}

impl Errored {
    /// Where a call of his turn that stopped so leaves it and, after an
    /// error, the entry pi's retry edits out of its context, none when it
    /// has no id for one to name.
    fn after(stop: Stop, line: &[u8]) -> (Self, Option<String>) {
        if stop == Stop::Error {
            let id = head_str(line, "id").filter(|id| !id.is_empty());
            (Self::Untried, id.map(str::to_owned))
        } else {
            (Self::No, None)
        }
    }
}

/// What's been read of one session's log: how far, whether the last
/// message in it was his, and his turns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub end: u64,
    pub open: bool,
    pub turns: Vec<Turn>,
    /// The harness's timing of the call to come, as when it began and its
    /// first token, which the harness logs just before the call itself.
    #[serde(default)]
    pub next: Option<(i64, i64)>,
    /// When a compaction was asked for, until it lands or settles, his
    /// next message opens a turn, or the next call of his turn is read.
    /// pi's compaction aborts the call it cuts, so an abort ending soon
    /// after the ask is pi's.
    #[serde(default)]
    pub compacting: Option<i64>,
    #[serde(default)]
    pub errored: Errored,
    /// The entry of the call that errored, which pi's retry edits out.
    #[serde(default)]
    pub error: Option<String>,
}

impl Log {
    /// Forget turns begun before `since`, keeping the last, whose message
    /// the next one is compared with.
    pub fn prune(&mut self, since: i64) {
        let last = self.turns.pop();
        self.turns.retain(|t| t.enter >= since);
        self.turns.extend(last);
    }
}

/// Reads what's been added to a session's log since `log` was last
/// brought up to date, adding his turns to it. A line not yet ended is
/// left for the next read; a log shorter than what was read is read anew.
pub fn follow<F: Read + Seek>(file: &mut F, log: &mut Log) -> io::Result<()> {
    let length = file.seek(SeekFrom::End(0))?;
    if length < log.end {
        *log = Log::default();
    }
    // The message the next one is compared with, read again from where
    // its line begins rather than kept between reads.
    let mut last = None;
    if let Some(turn) = log.turns.last() {
        file.seek(SeekFrom::Start(turn.offset))?;
        let mut line = Vec::new();
        BufReader::new(&mut *file).read_until(b'\n', &mut line)?;
        last = user(&line).and_then(|u| u.text);
    }
    file.seek(SeekFrom::Start(log.end))?;
    let mut reader = BufReader::new(file);
    let mut line = Vec::new();
    loop {
        line.clear();
        let read = reader.read_until(b'\n', &mut line)?;
        if read == 0 || line.last() != Some(&b'\n') {
            return Ok(());
        }
        let offset = log.end;
        log.end += u64::try_from(read).unwrap_or(u64::MAX);
        if begins_with_role(&line, "user") {
            let Some(message) = user(&line) else {
                continue;
            };
            let Some(text) = message.text else {
                log.open = false;
                continue;
            };
            let again = match (&last, log.turns.last()) {
                (Some(was), Some(turn)) => {
                    message.enter - turn.enter <= SAID_AGAIN_MS && alike(was, &text) >= ALIKE
                }
                _ => false,
            };
            log.turns.push(Turn {
                offset,
                enter: message.enter,
                delivered: message.delivered,
                calls: Vec::new(),
                again,
                correction: correcting(&text),
                status: asking_status(&text),
                nudge: nudging(&text),
                ..Turn::default()
            });
            log.open = true;
            log.errored = Errored::No;
            // A compaction asked before he typed can't cut a call he set
            // off, whether or not it ever lands.
            log.compacting = None;
            last = Some(text);
        } else if log.errored != Errored::No && head_has(&line, r#""type":"custom_message""#) {
            // pi tries an errored call again by first editing the error out
            // of its context, so a peer's or the harness's message before
            // that edit can still land inside his retry, and one after it
            // does. Only one with no retry after it sets off a call that
            // answers them, not him (hear/said.rs keeps the same rule).
            log.errored = Errored::Crossed;
        } else if log.errored != Errored::No && head_has(&line, r#""type":"context_edit""#) {
            if retried(&line, log.error.as_deref()) {
                log.errored = Errored::No;
            }
        } else if log.errored == Errored::Crossed && begins_with_role(&line, "assistant") {
            (log.errored, log.open) = (Errored::No, false);
        } else if head_has(&line, r#""customType":"harness-call""#) {
            log.next = harness_call(&line);
        } else if head_has(&line, r#""customType":"compaction-asked""#) {
            log.compacting = asked_at(&line);
        } else if head_has(&line, r#""type":"compaction""#)
            || head_has(&line, r#""customType":"compaction-settled""#)
        {
            log.compacting = None;
        } else if log.open
            && begins_with_role(&line, "assistant")
            && let (Some(turn), Some(mut call)) = (log.turns.last_mut(), call(&line))
        {
            call.first = log
                .next
                .take()
                .filter(|(of, _)| *of == call.start)
                .map(|(_, first)| first);
            if cut_for(log.compacting.take(), &call) {
                call.stop = Stop::Compacted;
            }
            // A call that answers or is stopped hands the turn back: what
            // runs after it was set off by something else, a mesh message
            // or a job's result, not by him.
            log.open = matches!(call.stop, Stop::Tool | Stop::Error);
            (log.errored, log.error) = Errored::after(call.stop, &line);
            turn.tool_mark = turn.tool_mark.max(call.start + call.ms);
            turn.calls.push(call);
        } else if log.open
            && begins_with_role(&line, "toolResult")
            && let (Some(turn), Some(at)) = (
                log.turns.last_mut(),
                head_str(&line, "timestamp").and_then(iso_ms),
            )
        {
            let took = (at - turn.tool_mark).max(0);
            if head_str(&line, "toolName").is_some_and(|name| HIS.contains(&name)) {
                turn.his_ms += took;
            } else {
                turn.tools_ms += took;
            }
            turn.tool_mark = turn.tool_mark.max(at);
        }
    }
}

/// A user entry: when he pressed Enter and when it reached the session,
/// with its text where he typed it, and none where something else did.
struct User {
    enter: i64,
    delivered: i64,
    text: Option<String>,
}

#[derive(Deserialize)]
struct UserLine {
    #[serde(rename = "type")]
    kind: String,
    timestamp: String,
    message: UserMessage,
}

#[derive(Deserialize)]
struct UserMessage {
    role: String,
    timestamp: i64,
    content: Content,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Content {
    Text(String),
    Blocks(Vec<Block>),
}

/// A block of a user message; an image's data is passed over unread.
#[derive(Deserialize)]
struct Block {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: Option<String>,
}

fn user(line: &[u8]) -> Option<User> {
    let entry: UserLine = serde_json::from_slice(line).ok()?;
    if entry.kind != "message" || entry.message.role != "user" {
        return None;
    }
    let text = match entry.message.content {
        Content::Text(text) => text,
        Content::Blocks(blocks) => blocks
            .into_iter()
            .filter(|b| b.kind == "text")
            .filter_map(|b| b.text)
            .collect::<Vec<_>>()
            .join(" "),
    };
    Some(User {
        enter: entry.message.timestamp,
        delivered: iso_ms(&entry.timestamp)?,
        text: Some(text).filter(|t| !automated(t)),
    })
}

/// An assistant entry's times and how it ended. Its content has no field
/// here, so serde passes over it without decoding.
#[derive(Deserialize)]
struct AssistantLine {
    message: AssistantMessage,
}

#[derive(Deserialize)]
struct AssistantMessage {
    role: String,
    timestamp: i64,
    #[serde(rename = "durationMs", default)]
    duration_ms: Option<i64>,
    #[serde(rename = "stopReason", default)]
    stop_reason: Option<String>,
}

fn call(line: &[u8]) -> Option<Call> {
    let entry: AssistantLine = serde_json::from_slice(line).ok()?;
    if entry.message.role != "assistant" {
        return None;
    }
    let stop = match entry.message.stop_reason.as_deref() {
        Some("toolUse") => Stop::Tool,
        Some("error") => Stop::Error,
        Some("aborted") => Stop::Aborted,
        _ => Stop::Done,
    };
    Some(Call {
        start: entry.message.timestamp,
        ms: entry.message.duration_ms.unwrap_or(0).max(0),
        stop,
        first: None,
    })
}

/// The harness's timing of one call: numbers alone, so it's parsed whole.
#[derive(Deserialize)]
struct HarnessLine {
    data: HarnessCall,
}

#[derive(Deserialize)]
struct HarnessCall {
    of: i64,
    #[serde(rename = "firstTokenMs", default)]
    first_token_ms: Option<i64>,
}

fn harness_call(line: &[u8]) -> Option<(i64, i64)> {
    let entry: HarnessLine = serde_json::from_slice(line).ok()?;
    Some((entry.data.of, entry.data.first_token_ms?))
}

/// Whether a call is the one pi cut for a compaction asked at `asked`: an
/// abort ending soon after the ask.
fn cut_for(asked: Option<i64>, call: &Call) -> bool {
    asked.is_some_and(|at| call.stop == Stop::Aborted && call.start + call.ms - at <= ASK_CUTS_MS)
}

/// A context edit: the entry it changes, and whether it puts anything in
/// its place, read without decoding what.
#[derive(Deserialize)]
struct EditLine {
    #[serde(rename = "targetId", default)]
    target: Option<String>,
    #[serde(default)]
    replacement: Option<serde::de::IgnoredAny>,
}

/// Whether a context edit is pi's retry of the call that errored: one that
/// drops that entry from its context with nothing in its place. Any other
/// edit, such as an extension's, isn't one (hear/said.rs keeps the same
/// rule).
fn retried(line: &[u8], error: Option<&str>) -> bool {
    serde_json::from_slice::<EditLine>(line).is_ok_and(|edit| {
        edit.replacement.is_none() && (error.is_none() || edit.target.as_deref() == error)
    })
}

/// A compaction's ask, as the harness writes it: numbers alone, so it's
/// parsed whole.
#[derive(Deserialize)]
struct AskLine {
    data: Ask,
}

#[derive(Deserialize)]
struct Ask {
    at: i64,
}

fn asked_at(line: &[u8]) -> Option<i64> {
    let entry: AskLine = serde_json::from_slice(line).ok()?;
    Some(entry.data.at)
}

/// Whether a line is a message in this role, from the head of the line
/// alone, so tool results, most of a log's bytes, are never parsed.
fn begins_with_role(line: &[u8], role: &str) -> bool {
    head_has(line, &format!(r#""role":"{role}""#))
}

fn head_has(line: &[u8], needle: &str) -> bool {
    let head = &line[..line.len().min(HEAD)];
    head.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// A string field from the head of a line, its first in the line, read
/// without parsing the rest: a tool result's time and its tool's name
/// come before its content.
fn head_str<'a>(line: &'a [u8], key: &str) -> Option<&'a str> {
    let head = &line[..line.len().min(HEAD)];
    let needle = format!(r#""{key}":""#);
    let from = head
        .windows(needle.len())
        .position(|w| w == needle.as_bytes())?
        + needle.len();
    let to = from + head[from..].iter().position(|&b| b == b'"')?;
    std::str::from_utf8(&head[from..to]).ok()
}

/// An ISO 8601 time as pi writes it ("2026-11-10T19:16:00.282Z"), in
/// milliseconds since 1970.
pub fn iso_ms(stamp: &str) -> Option<i64> {
    let field = |from: usize, to: usize| stamp.get(from..to)?.parse::<i64>().ok();
    let (y, m, d) = (field(0, 4)?, field(5, 7)?, field(8, 10)?);
    let (hh, mm, ss) = (field(11, 13)?, field(14, 16)?, field(17, 19)?);
    let millis = match stamp.get(19..20) {
        Some(".") => {
            let digits: String = stamp[20..]
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let padded = format!("{digits:0<3}");
            padded.get(..3)?.parse::<i64>().ok()?
        }
        _ => 0,
    };
    // Days since 1970 from a civil date, by Howard Hinnant's algorithm.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + hh) * 60 + mm) * 60_000 + ss * 1000 + millis)
}

#[cfg(test)]
mod tests;

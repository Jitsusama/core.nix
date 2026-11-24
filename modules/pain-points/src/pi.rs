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
    /// error, the entry pi's retry edits out of its context.
    fn after(stop: Stop, line: &[u8]) -> (Self, Option<String>) {
        if stop == Stop::Error {
            (Self::Untried, head_str(line, "id").map(str::to_owned))
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

/// A context edit: the entry it changes, and what it puts in its place.
#[derive(Deserialize)]
struct EditLine {
    #[serde(rename = "targetId", default)]
    target: Option<String>,
    #[serde(default)]
    replacement: Option<serde_json::Value>,
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
mod tests {
    use super::*;
    use crate::pain::Signs;
    use std::io::Cursor;

    /// 2026-11-11 16:00:00 UTC.
    pub(super) const AT: i64 = 1_794_412_800_000;

    fn iso(at: i64) -> String {
        let secs = at.div_euclid(1000);
        let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        assert_eq!(days, 20_768, "these tests stay on 11 November 2026");
        format!(
            "2026-11-11T{:02}:{:02}:{:02}.{:03}Z",
            rest / 3600,
            rest / 60 % 60,
            rest % 60,
            at.rem_euclid(1000)
        )
    }

    fn typed(enter: i64, delivered: i64, text: &str) -> String {
        format!(
            r#"{{"type":"message","id":"a","parentId":"b","timestamp":"{}","message":{{"role":"user","content":[{{"type":"text","text":{}}}],"timestamp":{enter}}}}}"#,
            iso(delivered),
            serde_json::to_string(text).expect("a string serializes")
        ) + "\n"
    }

    fn reply(start: i64, ms: i64, stop: &str) -> String {
        format!(
            r#"{{"type":"message","id":"c","parentId":"a","timestamp":"{}","message":{{"role":"assistant","content":[{{"type":"text","text":"an answer"}}],"api":"x","timestamp":{start},"durationMs":{ms},"stopReason":"{stop}"}}}}"#,
            iso(start + ms)
        ) + "\n"
    }

    fn tool_result(at: i64, text: &str) -> String {
        format!(
            r#"{{"type":"message","id":"d","parentId":"c","timestamp":"{}","message":{{"role":"toolResult","content":[{{"type":"text","text":{}}}],"timestamp":{at}}}}}"#,
            iso(at),
            serde_json::to_string(text).expect("a string serializes")
        ) + "\n"
    }

    fn tool_ended(at: i64, name: &str) -> String {
        format!(
            r#"{{"type":"message","id":"d","parentId":"c","timestamp":"{}","message":{{"role":"toolResult","toolCallId":"t","toolName":"{name}","content":[{{"type":"text","text":"done"}}],"timestamp":{at}}}}}"#,
            iso(at)
        ) + "\n"
    }

    fn harness_call(of: i64, first_token: i64) -> String {
        format!(
            r#"{{"type":"custom","customType":"harness-call","data":{{"v":1,"of":{of},"turnAt":"{}","preparedMs":5,"firstTokenMs":{first_token},"durationMs":9000,"stopReason":"stop"}},"id":"h","parentId":"a","timestamp":"{}"}}"#,
            iso(of),
            iso(of + 9000)
        ) + "\n"
    }

    fn compaction_asked(at: i64) -> String {
        format!(
            r#"{{"type":"custom","customType":"compaction-asked","data":{{"at":{at},"resume":true}},"id":"k","parentId":"d","timestamp":"{}"}}"#,
            iso(at)
        ) + "\n"
    }

    /// As the harness writes it, read from a session's log on 20 November
    /// with its ids changed.
    fn compaction_settled(at: i64) -> String {
        format!(
            r#"{{"type":"custom","customType":"compaction-settled","data":{{"at":{at},"resumed":false,"by":"asker"}},"id":"n","parentId":"k","timestamp":"{}"}}"#,
            iso(at)
        ) + "\n"
    }

    fn compaction(at: i64) -> String {
        format!(
            r#"{{"type":"compaction","id":"m","parentId":"k","timestamp":"{}","summary":"what was said"}}"#,
            iso(at)
        ) + "\n"
    }

    fn mesh(at: i64) -> String {
        format!(
            r#"{{"type":"custom_message","customType":"mesh","content":"Mesh message from a peer","timestamp":"{}"}}"#,
            iso(at)
        ) + "\n"
    }

    fn read(text: &str) -> Log {
        let mut log = Log::default();
        follow(&mut Cursor::new(text.as_bytes().to_vec()), &mut log).expect("a cursor reads");
        log
    }

    pub(super) fn turn(enter: i64, calls: &[(i64, i64, Stop)]) -> Turn {
        Turn {
            offset: 0,
            enter,
            delivered: enter,
            calls: calls
                .iter()
                .map(|&(start, ms, stop)| Call {
                    start,
                    ms,
                    stop,
                    first: None,
                })
                .collect(),
            ..Turn::default()
        }
    }

    #[test]
    fn what_runs_after_his_turn_is_handed_back_is_not_his() {
        let text = typed(AT, AT, "hello")
            + &reply(AT + 100, 1000, "stop")
            + &mesh(AT + 5000)
            + &reply(AT + 5100, 2000, "error")
            + &reply(AT + 8000, 500, "aborted");
        let log = read(&text);
        assert_eq!(
            log.turns[0].calls.len(),
            1,
            "calls a mesh message set off after his reply aren't his turn's"
        );
        assert!(!log.open, "the answer handed the turn back");
    }

    #[test]
    fn a_peers_message_after_an_error_takes_the_turn_unless_pi_tries_again() {
        let retry =
            || r#"{"type":"context_edit","targetId":"f","replacement":null}"#.to_owned() + "\n";
        // The call that errored is entry "f", which pi's retry names.
        let failed = reply(AT + 100, 1000, "error").replacen(r#""id":"c""#, r#""id":"f""#, 1);
        let errored = || typed(AT, AT, "hello") + &failed;
        // An extension may edit another entry, or put text in its place.
        let other = r#"{"type":"context_edit","targetId":"c","replacement":null}"#.to_owned()
            + "\n"
            + r#"{"type":"context_edit","targetId":"f","replacement":"x"}"#
            + "\n";
        let answered = || reply(AT + 5100, 2000, "stop");
        for (text, stops, what) in [
            (
                errored() + &mesh(AT + 5000) + &answered(),
                vec![Stop::Error],
                "a peer's turn after an error pi never tried again",
            ),
            (
                errored() + &mesh(AT + 5000) + &other + &answered(),
                vec![Stop::Error],
                "a peer's turn after edits that aren't pi's retry",
            ),
            (
                errored() + &mesh(AT + 5000) + &retry() + &answered(),
                vec![Stop::Error, Stop::Done],
                "a peer's message before pi's retry",
            ),
            (
                errored() + &retry() + &mesh(AT + 5000) + &answered(),
                vec![Stop::Error, Stop::Done],
                "a peer's message during pi's retry",
            ),
            (
                typed(AT, AT, "hello")
                    + &reply(AT + 100, 1000, "toolUse")
                    + &mesh(AT + 5000)
                    + &answered(),
                vec![Stop::Tool, Stop::Done],
                "a peer's message mid-turn with no error",
            ),
        ] {
            let log = read(&text);
            let got: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
            assert_eq!(got, stops, "{what} was read wrongly");
            assert!(!log.open, "{what}: the turn was handed back at its end");
        }
    }

    #[test]
    fn an_abort_pi_makes_to_compact_is_not_his_stop() {
        // As a session's log read on 23 November: a tool's result, the
        // harness asking to compact, the call it cut ending at once, the
        // compaction, and the run resumed by an automated message.
        // Its first reply comes 2.5 s after Enter, so the go is named for
        // that wait and its stopped count can be read.
        let text = typed(AT, AT, "check the gate")
            + &reply(AT + 2500, 18_000, "toolUse")
            + &tool_ended(AT + 20_800, "edit")
            + &compaction_asked(AT + 20_860)
            + &reply(AT + 20_975, 0, "aborted")
            + &compaction(AT + 21_600)
            + &typed(
                AT + 22_200,
                AT + 22_300,
                "[Automated message, no human input: the context was compacted]",
            )
            + &reply(AT + 22_400, 9000, "toolUse");
        let log = read(&text);
        let stops: Vec<Stop> = log.turns[0].calls.iter().map(|c| c.stop).collect();
        assert_eq!(stops, [Stop::Tool, Stop::Compacted]);
        let named = painful(&[&log.turns], AT - 1, AT + 1);
        assert_eq!(named.len(), 1, "the go is named for its wait");
        let Signs::Pi(signs) = &named[0].signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!(
            (signs.stopped, named[0].part),
            (0, "pi"),
            "pi cut its own call, he stopped nothing"
        );
    }

    #[test]
    fn an_ask_ends_where_it_lands_settles_or_he_types() {
        // Each abort below is his: the ask before it was spent first, or
        // came too long before it ended.
        let asked = || {
            typed(AT, AT, "check the gate")
                + &reply(AT + 500, 1000, "toolUse")
                + &compaction_asked(AT + 2000)
        };
        let stop = || reply(AT + 3000, 1000, "aborted");
        for (text, what) in [
            (
                asked() + &compaction(AT + 2500) + &stop(),
                "an ask that landed between his calls",
            ),
            (
                asked() + &compaction_settled(AT + 2500) + &stop(),
                "an ask settled with no compaction",
            ),
            (
                typed(AT, AT, "check the gate")
                    + &reply(AT + 500, 1000, "stop")
                    + &compaction_asked(AT + 2000)
                    + &typed(AT + 2500, AT + 2500, "now the launcher")
                    + &stop(),
                "an ask never settled, before he typed again",
            ),
            (
                asked() + &reply(AT + 3000, 9000, "aborted"),
                "an abort ending long after the ask",
            ),
        ] {
            let log = read(&text);
            let last = log.turns.last().and_then(|t| t.calls.last());
            assert_eq!(last.map(|c| c.stop), Some(Stop::Aborted), "{what}");
            assert_eq!(log.compacting, None, "{what}: no ask is left");
        }
    }

    #[test]
    fn a_tool_holding_the_session_past_a_minute_is_pis_own_and_the_model_is_not() {
        let text = typed(AT, AT, "run the tests")
            + &reply(AT + 500, 10_000, "toolUse")
            + &tool_ended(AT + 100_500, "bash")
            + &reply(AT + 101_000, 5000, "stop");
        let log = read(&text);
        assert_eq!(log.turns[0].tools_ms, 90_000, "the tool ran 90 s");
        let p = &painful(&[&log.turns], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part),
            (30.5, "waiting", "pi"),
            "of 106 s, the model's 15 aren't lost, nor the tool's first minute"
        );
        let Signs::Pi(signs) = &p.signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!(
            (
                signs.model_ms,
                signs.provider_ms,
                signs.pi_ms,
                signs.tools_ms
            ),
            (15_000, 0, 91_000, 90_000)
        );
    }

    #[test]
    fn a_question_put_to_him_is_not_a_wait() {
        let text = typed(AT, AT, "plan it")
            + &reply(AT + 500, 10_000, "toolUse")
            + &tool_ended(AT + 40_500, "ask")
            + &reply(AT + 41_000, 5000, "stop");
        let log = read(&text);
        assert!(
            painful(&[&log.turns], AT - 1, AT + 1).is_empty(),
            "the 30 s were his own, answering"
        );
    }

    #[test]
    fn the_first_token_comes_from_the_harness_entry_before_its_call() {
        let text =
            typed(AT, AT, "hello") + &harness_call(AT + 500, 4000) + &reply(AT + 500, 9000, "stop");
        let log = read(&text);
        assert_eq!(log.turns[0].calls[0].first, Some(4000));
        let named = painful(&[&log.turns], AT - 1, AT + 1);
        assert!(named.is_empty(), "the model thinking is no fault");
    }

    #[test]
    fn a_typed_message_and_its_replies_make_a_turn() {
        let text = typed(AT, AT + 4000, "make the launcher faster")
            + &reply(AT + 4100, 3000, "toolUse")
            + &tool_result(AT + 7200, "ok")
            + &reply(AT + 7300, 2000, "stop");
        let log = read(&text);
        assert_eq!(log.turns.len(), 1, "one message, one turn");
        let turn = &log.turns[0];
        assert_eq!(
            (turn.offset, turn.enter, turn.delivered),
            (0, AT, AT + 4000)
        );
        assert_eq!(
            turn.calls,
            vec![
                Call {
                    start: AT + 4100,
                    ms: 3000,
                    stop: Stop::Tool,
                    first: None,
                },
                Call {
                    start: AT + 7300,
                    ms: 2000,
                    stop: Stop::Done,
                    first: None,
                },
            ]
        );
        assert_eq!(log.end, u64::try_from(text.len()).expect("a length fits"));
    }

    #[test]
    fn messages_he_did_not_type_are_not_his_turns() {
        let text = typed(AT, AT, "hello")
            + &reply(AT + 100, 1000, "stop")
            + &typed(
                AT + 5000,
                AT + 5000,
                "[Automated message, no human input: carry on]",
            )
            + &reply(AT + 5100, 9000, "stop")
            + &typed(
                AT + 20_000,
                AT + 20_000,
                "[Joel is asking from outside this window] hi",
            )
            + &mesh(AT + 30_000);
        let log = read(&text);
        assert_eq!(log.turns.len(), 1, "only the typed message is his");
        assert_eq!(
            log.turns[0].calls.len(),
            1,
            "a reply to an automated message isn't his turn's"
        );
        assert!(!log.open, "the last message in the log wasn't his");
    }

    #[test]
    fn a_read_picks_up_where_the_last_one_stopped() {
        let first = typed(AT, AT, "fix the launcher") + &reply(AT + 100, 1000, "toolUse");
        let rest =
            reply(AT + 2000, 1000, "stop") + &typed(AT + 60_000, AT + 60_000, "fix the launcher!");
        let whole = read(&(first.clone() + &rest));
        let mut log = Log::default();
        let unended = first.clone() + &rest[..10];
        follow(&mut Cursor::new(unended.into_bytes()), &mut log).expect("a cursor reads");
        assert_eq!(
            log.end,
            u64::try_from(first.len()).expect("a length fits"),
            "a line not yet ended waits for the next read"
        );
        follow(&mut Cursor::new((first + &rest).into_bytes()), &mut log).expect("a cursor reads");
        assert_eq!(log, whole, "two reads make what one would");
        assert!(
            log.turns[1].again,
            "the second is the first said again, across reads"
        );
    }

    #[test]
    fn a_log_shorter_than_what_was_read_is_read_anew() {
        let mut log = read(&(typed(AT, AT, "one") + &typed(AT + 1000, AT + 1000, "two")));
        let shorter = typed(AT + 5000, AT + 5000, "three");
        follow(&mut Cursor::new(shorter.into_bytes()), &mut log).expect("a cursor reads");
        assert_eq!(log.turns.len(), 1, "what was read before is gone");
        assert_eq!(log.turns[0].enter, AT + 5000);
    }

    #[test]
    fn a_message_said_again_soon_is_a_second_go() {
        let log = read(
            &(typed(AT, AT, "why is the launcher slow to open")
                + &typed(
                    AT + 120_000,
                    AT + 120_000,
                    "why is the launcher so slow to open?",
                )
                + &typed(
                    AT + 900_000,
                    AT + 900_000,
                    "why is the launcher so slow to open?",
                )
                + &typed(AT + 960_000, AT + 960_000, "now look at the backlight")),
        );
        let again: Vec<bool> = log.turns.iter().map(|t| t.again).collect();
        assert_eq!(again, [false, true, false, false]);
    }

    #[test]
    fn what_he_typed_and_what_agents_said_is_never_kept() {
        let text = typed(AT, AT + 20_000, "the token is sk-FAKE-7QEY1S-0001 ok")
            + &reply(AT + 21_000, 1000, "stop")
            + &tool_result(AT + 22_000, "sk-FAKE-7QEY1S-0002")
            + &typed(
                AT + 30_000,
                AT + 30_000,
                "the token is sk-FAKE-7QEY1S-0001 ok!",
            );
        let log = read(&text);
        assert_eq!(log.turns.len(), 2, "both messages were read");
        let kept = serde_json::to_string(&log).expect("the state serializes")
            + &serde_json::to_string(&painful(&[&log.turns], AT - 1, AT + 1))
                .expect("the day serializes");
        assert!(
            !kept.contains("FAKE"),
            "nothing typed or answered is kept: {kept}"
        );
    }

    #[test]
    fn iso_times_read_as_pi_writes_them() {
        assert_eq!(iso_ms("2026-11-10T19:16:00.282Z"), Some(1_794_338_160_282));
        assert_eq!(iso_ms("2026-11-10T19:16:00Z"), Some(1_794_338_160_000));
        assert_eq!(iso_ms("2026-11-10T19:16:00.2Z"), Some(1_794_338_160_200));
        assert_eq!(iso_ms("not a time"), None);
    }

    #[test]
    fn pruning_keeps_the_last_turn_to_compare_with() {
        let mut log = Log {
            end: 0,
            open: true,
            turns: vec![turn(AT - 10, &[]), turn(AT - 5, &[])],
            next: None,
            compacting: None,
            errored: Errored::No,
            error: None,
        };
        log.prune(AT);
        assert_eq!(log.turns.len(), 1, "the older is gone");
        assert_eq!(log.turns[0].enter, AT - 5);
    }
}

//! Joel's messages to pi, from pi's own session logs: how long each waited
//! for its reply to begin, which a provider's error held up, which turns
//! he stopped, and which he had to say again, so pi's moments rank beside
//! the launcher's.
//!
//! What he typed is read here and in words.rs alone, in memory, to tell
//! whether a message repeats his last one in the same session, reads as a
//! correction or asks where things stand. The text is never written, logged or sent, and
//! nothing kept counts its length or its words: the day file and the
//! reading state hold times, flags and counts of messages. Tool calls,
//! their results and what the agents wrote are never decoded: their lines
//! are passed over before parsing, and an assistant entry's content is
//! skipped unread. Each line is dropped as soon as it's read.

use crate::pain::{Painful, PiSigns, Signs, tenths};
use crate::words::{ALIKE, alike, asking_status, automated, correcting};
use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

/// A reply that hasn't begun a second after he pressed Enter has broken
/// his flow; that second is the spec, and every one past it is lost.
pub const REPLY_MS: i64 = 1_000;
/// A message much like his last in the same session, within this long,
/// says the last didn't do it.
pub const SAID_AGAIN_MS: i64 = 300_000;
/// A go in pi lost him this at most: past two minutes he was most likely
/// doing something else meanwhile, as with a search he left in the
/// launcher, so a reply that hung for ten loses him two.
pub const MOST_MS: i64 = 120_000;
/// How far into a line its role is looked for. pi writes an entry's type,
/// ids and time before its message's role, about 130 bytes in.
const HEAD: usize = 400;

/// How one call to the model ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stop {
    Done,
    Tool,
    Error,
    Aborted,
}

/// One call to the model: when it began, in milliseconds since 1970, how
/// long it streamed, and how it ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Call {
    pub start: i64,
    pub ms: i64,
    pub stop: Stop,
}

/// A message Joel typed, and the calls that answered it before the next
/// message reached the session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

impl Turn {
    /// When its last call ended, or when it was typed if none answered.
    pub fn end(&self) -> i64 {
        self.calls
            .iter()
            .map(|c| c.start + c.ms)
            .max()
            .unwrap_or(self.enter)
    }
}

/// What's been read of one session's log: how far, whether the last
/// message in it was his, and his turns.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Log {
    pub end: u64,
    pub open: bool,
    pub turns: Vec<Turn>,
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
            });
            log.open = true;
            last = Some(text);
        } else if log.open
            && begins_with_role(&line, "assistant")
            && let (Some(turn), Some(call)) = (log.turns.last_mut(), call(&line))
        {
            turn.calls.push(call);
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
    })
}

/// Whether a line is a message in this role, from the head of the line
/// alone, so tool results, most of a log's bytes, are never parsed.
fn begins_with_role(line: &[u8], role: &str) -> bool {
    let needle = format!(r#""role":"{role}""#);
    let head = &line[..line.len().min(HEAD)];
    head.windows(needle.len()).any(|w| w == needle.as_bytes())
}

/// The day's turns that lost him at least a second, as the day file says
/// them: each a turn, or turns in a row where each said the last again.
/// `logs` are each session's turns, in order; only goes begun in
/// [since, until) count.
pub fn painful(logs: &[&[Turn]], since: i64, until: i64) -> Vec<Painful> {
    let mut named = Vec::new();
    for turns in logs {
        let mut goes: Vec<Vec<&Turn>> = Vec::new();
        for turn in *turns {
            match goes.last_mut() {
                Some(go) if turn.again => go.push(turn),
                _ => goes.push(vec![turn]),
            }
        }
        named.extend(
            goes.iter()
                .filter(|go| (since..until).contains(&go[0].enter))
                .map(|go| interaction(go))
                .filter(|p| p.lost_seconds >= 1.0),
        );
    }
    named
}

/// One go at something in a session, as the day file says it.
fn interaction(go: &[&Turn]) -> Painful {
    let (first, last) = (go[0], go[go.len() - 1]);
    let mut signs = PiSigns::default();
    let ms = |ms: i64| crate::pain::whole(crate::pain::float(ms));
    for turn in go {
        let waited = turn.calls.first().map_or(0, |c| c.start - turn.enter);
        signs.waited_ms = signs.waited_ms.max(ms(waited));
        signs.queued_ms = signs.queued_ms.max(ms(turn.delivered - turn.enter));
        let errors = turn.calls.iter().filter(|c| c.stop == Stop::Error).count();
        signs.errors += u32::try_from(errors).unwrap_or(u32::MAX);
        signs.stopped += u32::from(turn.calls.iter().any(|c| c.stop == Stop::Aborted));
        signs.corrections += u32::from(turn.correction);
        signs.status += u32::from(turn.status);
    }
    // Each go but the last didn't do it, so its time up to the next went
    // for nothing; the last loses what it waited on.
    let again: i64 = go.windows(2).map(|w| w[1].enter - w[0].enter).sum();
    let calls = &last.calls;
    let reply = calls
        .first()
        .map_or(0, |c| (c.start - last.enter - REPLY_MS).max(0));
    let error: i64 = calls
        .iter()
        .enumerate()
        .filter(|(_, c)| c.stop == Stop::Error)
        .map(|(n, c)| calls.get(n + 1).map_or(c.start + c.ms, |next| next.start) - c.start)
        .sum();
    let stopped = calls
        .iter()
        .find(|c| c.stop == Stop::Aborted)
        .map_or(0, |c| (c.start + c.ms - last.enter - reply - error).max(0));
    let part = [
        (reply, "reply"),
        (error, "error"),
        (stopped, "stopped"),
        (again, "again"),
    ]
    .into_iter()
    .fold(
        (i64::MIN, "reply"),
        |best, it| if it.0 > best.0 { it } else { best },
    )
    .1;
    let slowed = match part {
        _ if go.len() > 1 => "tried again",
        "stopped" => "stopped it",
        "error" => "an error",
        _ => "waiting",
    };
    let float = crate::pain::float;
    Painful {
        at: first.enter / 1000,
        place: "pi",
        trying: "a session".to_string(),
        seconds: tenths(float(last.end() - first.enter)),
        usual_seconds: tenths(float(REPLY_MS)),
        lost_seconds: tenths(float((again + reply + error + stopped).min(MOST_MS))),
        rank: 0,
        slowed,
        part,
        openings: go.len(),
        signs: Signs::Pi(signs),
    }
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
    use std::io::Cursor;

    /// 2026-11-11 16:00:00 UTC.
    const AT: i64 = 1_794_412_800_000;

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

    fn turn(enter: i64, calls: &[(i64, i64, Stop)]) -> Turn {
        Turn {
            offset: 0,
            enter,
            delivered: enter,
            calls: calls
                .iter()
                .map(|&(start, ms, stop)| Call { start, ms, stop })
                .collect(),
            again: false,
            correction: false,
            status: false,
        }
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
                    stop: Stop::Tool
                },
                Call {
                    start: AT + 7300,
                    ms: 2000,
                    stop: Stop::Done
                },
            ]
        );
        assert_eq!(turn.end(), AT + 9300);
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
    fn waiting_past_a_second_for_the_reply_is_lost() {
        let turns = [turn(AT, &[(AT + 21_000, 4000, Stop::Done)])];
        let named = painful(&[&turns], AT - 1, AT + 1);
        assert_eq!(named.len(), 1, "a 21 s wait is named");
        let p = &named[0];
        assert_eq!((p.place, p.trying.as_str()), ("pi", "a session"));
        assert_eq!(
            (p.lost_seconds, p.seconds, p.usual_seconds),
            (20.0, 25.0, 1.0)
        );
        assert_eq!((p.slowed, p.part, p.openings), ("waiting", "reply", 1));
        let quick = [turn(AT, &[(AT + 900, 30_000, Stop::Done)])];
        assert!(
            painful(&[&quick], AT - 1, AT + 1).is_empty(),
            "a reply that began within a second lost him nothing, however long it ran"
        );
    }

    #[test]
    fn an_error_loses_the_time_until_the_next_try() {
        let turns = [turn(
            AT,
            &[
                (AT + 500, 2000, Stop::Error),
                (AT + 12_500, 3000, Stop::Done),
            ],
        )];
        let p = &painful(&[&turns], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part),
            (12.0, "an error", "error")
        );
        let Signs::Pi(signs) = &p.signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!(signs.errors, 1);
        assert_eq!(
            crate::pain::float(MOST_MS),
            crate::pain::GAVE_UP_MOST_MS,
            "pi and the launcher hold a go to the same two minutes"
        );
        let hung = [turn(AT, &[(AT + 500, 600_000, Stop::Error)])];
        assert_eq!(
            painful(&[&hung], AT - 1, AT + 1)[0].lost_seconds,
            120.0,
            "past two minutes he was doing something else"
        );
    }

    #[test]
    fn a_stopped_turn_loses_what_it_ran_at_most_two_minutes() {
        let short = [turn(AT, &[(AT + 500, 30_000, Stop::Aborted)])];
        let p = &painful(&[&short], AT - 1, AT + 1)[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part),
            (30.5, "stopped it", "stopped")
        );
        let long = [turn(AT, &[(AT + 500, 600_000, Stop::Aborted)])];
        assert_eq!(painful(&[&long], AT - 1, AT + 1)[0].lost_seconds, 120.0);
    }

    #[test]
    fn saying_it_again_loses_the_first_go() {
        let mut second = turn(AT + 90_000, &[(AT + 90_500, 5000, Stop::Done)]);
        second.again = true;
        let turns = [turn(AT, &[(AT + 6000, 5000, Stop::Done)]), second];
        let named = painful(&[&turns], AT - 1, AT + 1);
        assert_eq!(named.len(), 1, "the two are one go");
        let p = &named[0];
        assert_eq!(
            (p.lost_seconds, p.slowed, p.part, p.openings),
            (90.0, "tried again", "again", 2)
        );
        assert_eq!(p.seconds, 95.5);
    }

    #[test]
    fn corrections_and_status_questions_are_signs_not_lost_time() {
        let mut asked = turn(AT, &[(AT + 3000, 1000, Stop::Done)]);
        asked.status = true;
        asked.correction = true;
        let p = &painful(&[&[asked]], AT - 1, AT + 1)[0];
        assert_eq!(p.lost_seconds, 2.0, "only its wait is lost");
        let Signs::Pi(signs) = &p.signs else {
            panic!("a pi moment carries pi's signs");
        };
        assert_eq!((signs.corrections, signs.status), (1, 1));
    }

    #[test]
    fn only_goes_begun_in_the_day_count() {
        let turns = [turn(AT, &[(AT + 21_000, 4000, Stop::Done)])];
        assert!(
            painful(&[&turns], AT + 1, AT + 2).is_empty(),
            "before the day"
        );
        assert!(painful(&[&turns], AT - 2, AT).is_empty(), "after it");
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
        };
        log.prune(AT);
        assert_eq!(log.turns.len(), 1, "the older is gone");
        assert_eq!(log.turns[0].enter, AT - 5);
    }
}

//! Asking each part's owner for its answer: its command run with a clock,
//! all at once, so the slowest owner bounds the wait rather than their sum.
//! An owner not on this machine, one that fails, and one that runs past
//! the clock each leave an answer saying why, never a hang.

use std::io::{self, Read};
use std::iter::Peekable;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::Value;
use system::parts::Answer;

/// How long an owner may take. Each answers from what it holds in well
/// under a second; `repos` reads every checkout's git files, about two
/// seconds over a thousand trees, so this leaves it room on a busy machine.
const CLOCK: Duration = Duration::from_secs(10);
/// How often a running owner is looked at.
const LOOK: Duration = Duration::from_millis(20);
/// How much of an owner's complaint a failure says: enough for its first
/// line's reason, short enough for a row's side.
const SAID_OF_ITS_COMPLAINT: usize = 160;
/// How much of an owner's complaint is kept: far more than its first line
/// needs, so a chatty owner's log costs the daemon no more than this.
const COMPLAINT_KEPT: u64 = 4096;
/// How long an owner's streams may stay open once it has ended. It has
/// written all it will by then, so one still open is held by a helper it
/// left, and that's given only a moment, past the clock if need be, so an
/// owner ending right at it still has its answer read.
const STREAMS_AFTER_IT_ENDS: Duration = Duration::from_secs(1);

/// Each command's answer, in the order asked.
pub fn all(commands: &[&[&str]]) -> Vec<Answer> {
    let asked: Vec<_> = commands
        .iter()
        .map(|words| {
            let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            thread::spawn(move || one(&words, CLOCK))
        })
        .collect();
    asked
        .into_iter()
        .map(|asked| {
            asked
                .join()
                .unwrap_or_else(|_| Err("its reader stopped on a bug".to_owned()))
        })
        .collect()
}

/// The answer of the command `words`, or why there is none, within
/// `clock` and a moment more for its streams to close, so a helper it left
/// holding one never holds the ask.
fn one(words: &[String], clock: Duration) -> Answer {
    let said = words.join(" ");
    let mut child = Command::new(&words[0])
        .args(&words[1..])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => format!("{} isn't on this machine", words[0]),
            _ => format!("{said} couldn't start: {error}"),
        })?;
    let deadline = Instant::now() + clock;
    let late = || format!("{said} gave no answer in {} s", clock.as_secs());
    let (Some(out), Some(complaint)) = (child.stdout.take(), child.stderr.take()) else {
        end(&mut child);
        return Err(format!("{said}'s answer couldn't be read"));
    };
    // Both read as it writes, so neither fills its pipe and stalls the
    // owner while it's waited on; its complaint so a failure can say why.
    let reading = reader(out, u64::MAX);
    let complaining = reader(complaint, COMPLAINT_KEPT);
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if Instant::now() > deadline {
            // Its answer would come too late to be of use; it's ended so
            // nothing of it outlives the command.
            end(&mut child);
            return Err(late());
        }
        thread::sleep(LOOK);
    };
    let until = Instant::now() + STREAMS_AFTER_IT_ENDS;
    if !status.success() {
        // What it wrote of its complaint is said, held open or not.
        let complaint = gathered(&complaining, until);
        return Err(failed(&said, status.code(), &complaint.bytes));
    }
    let answer = gathered(&reading, until);
    match answer.end {
        Some(Ok(())) => first(&said, &answer.bytes),
        Some(Err(error)) => Err(format!("{said}'s answer couldn't be read: {error}")),
        // It ended well, but something it left holds its answer open, so
        // what it wrote may not be the whole of it.
        None => Err(format!("{said} left its answer open after it ended")),
    }
}

/// Ends a child that won't be waited for, so nothing of it outlives the
/// ask.
fn end(child: &mut Child) {
    // Either may fail only for a child already gone, which is the end
    // wanted.
    let _ = child.kill();
    let _ = child.wait();
}

/// What a reader heard of a stream: some of what it wrote, or its end.
enum Heard {
    Bytes(Vec<u8>),
    End(io::Result<()>),
}

/// What `stream` writes, sent as it's read, up to `kept` bytes and the
/// rest drained unread, then its end.
fn reader(stream: impl Read + Send + 'static, kept: u64) -> Receiver<Heard> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut stream = stream;
        let mut room = kept;
        let mut buffer = [0; 8192];
        let end = loop {
            match stream.read(&mut buffer) {
                Ok(0) => break Ok(()),
                Ok(read) => {
                    let keep = read.min(usize::try_from(room).unwrap_or(usize::MAX));
                    room = room.saturating_sub(u64::try_from(keep).unwrap_or(u64::MAX));
                    if keep > 0 {
                        // The ask may have stopped waiting, which leaves no
                        // one to tell; the stream is still drained.
                        let _ = send.send(Heard::Bytes(buffer[..keep].to_vec()));
                    }
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) => break Err(error),
            }
        };
        // As above, no one may be left to tell.
        let _ = send.send(Heard::End(end));
    });
    receive
}

/// What a reader heard of its stream by `until`.
struct Gathered {
    bytes: Vec<u8>,
    /// How the stream ended, or none while it was still open then, as it
    /// is while a helper the owner left holds it.
    end: Option<io::Result<()>>,
}

/// All a reader has heard by `until`.
fn gathered(receive: &Receiver<Heard>, until: Instant) -> Gathered {
    let mut bytes = Vec::new();
    loop {
        match receive.recv_timeout(until.saturating_duration_since(Instant::now())) {
            Ok(Heard::Bytes(more)) => bytes.extend(more),
            Ok(Heard::End(end)) => {
                return Gathered {
                    bytes,
                    end: Some(end),
                };
            }
            Err(_) => return Gathered { bytes, end: None },
        }
    }
}

/// Why an owner that ended badly gave no answer: how it ended, and the
/// first line of its complaint, when it made one, which is where an owner
/// says what it needed. It's said plain, as the line is shown on rows and
/// carried to the realm's other machines.
fn failed(said: &str, code: Option<i32>, complaint: &[u8]) -> String {
    let ended = match code {
        Some(code) => format!("{said} ended with {code}"),
        None => format!("{said} was stopped"),
    };
    let complaint = String::from_utf8_lossy(complaint);
    let line = complaint
        .lines()
        .map(|line| plain(&reason(line)))
        .find(|line| !line.is_empty());
    match line {
        Some(line) => {
            let line: String = line.chars().take(SAID_OF_ITS_COMPLAINT).collect();
            format!("{ended}: {line}")
        }
        None => ended,
    }
}

/// The reason a complaint's `line` gives: a kit command asked for JSON
/// complains in an `error/1` document, which is said as its own words are,
/// what failed and why; any other line is its own reason.
fn reason(line: &str) -> String {
    let Ok(Value::Object(error)) = serde_json::from_str::<Value>(line) else {
        return line.to_owned();
    };
    if error.get("schema").and_then(Value::as_str) != Some("error/1") {
        return line.to_owned();
    }
    let what = error
        .get("what")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match error.get("why").and_then(Value::as_str) {
        Some(why) => format!("{what}: {why}"),
        None => what.to_owned(),
    }
}

/// `line` as plain words: a terminal's escape sequences gone whole, as a
/// coloured or linked complaint writes them, and with them every control
/// character and the format characters that turn, hide or break text, so a
/// row reads as it's written.
fn plain(line: &str) -> String {
    let mut kept = String::new();
    let mut chars = line.chars().peekable();
    while let Some(each) = chars.next() {
        match each {
            '\u{1b}' => escape(&mut chars),
            // The one-character forms of a control sequence and a string,
            // as characters; written as raw bytes, they were never UTF-8
            // and reach here as a replacement character, which is harmless.
            '\u{9b}' => control_sequence(&mut chars),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => string(&mut chars),
            each if hides(each) || each.is_control() => {}
            each => kept.push(each),
        }
    }
    kept.trim().to_owned()
}

/// Passes over an escape sequence's rest, its escape already taken: a
/// control sequence such as a colour, a string such as a title or a link,
/// or one that runs through intermediates to its final character, such as
/// a character set chosen; any other takes the one character after it.
fn escape(chars: &mut Peekable<std::str::Chars>) {
    match chars.next() {
        Some('[') => control_sequence(chars),
        Some(']' | 'P' | 'X' | '^' | '_') => string(chars),
        Some(' '..='/') => {
            chars.find(|end| ('0'..='~').contains(end));
        }
        _ => {}
    }
}

/// Passes over a control sequence's rest, to its final character.
fn control_sequence(chars: &mut Peekable<std::str::Chars>) {
    chars.find(|end| ('@'..='~').contains(end));
}

/// Passes over a string's rest, to the bell or string terminator ending it.
fn string(chars: &mut Peekable<std::str::Chars>) {
    while let Some(each) = chars.next() {
        match each {
            '\u{7}' | '\u{9c}' => return,
            '\u{1b}' => {
                chars.next_if_eq(&'\\');
                return;
            }
            _ => {}
        }
    }
}

/// Whether `each` is a format character that turns, hides or breaks text:
/// a soft hyphen, direction marks and overrides, joiners and invisible
/// operators, a line or paragraph separator, annotation marks, and tags.
fn hides(each: char) -> bool {
    matches!(
        each,
        '\u{ad}'
            | '\u{61c}'
            | '\u{180e}'
            | '\u{200b}'..='\u{200f}'
            | '\u{2028}'..='\u{202e}'
            | '\u{2060}'..='\u{206f}'
            | '\u{feff}'
            | '\u{fff9}'..='\u{fffb}'
            | '\u{e0000}'..='\u{e007f}'
    )
}

/// The document an owner's answer opens with. An owner may say more on
/// the lines after it, as `mycelium machines` says the builders it sends
/// to, and none of that is what its part is judged by.
fn first(said: &str, bytes: &[u8]) -> Answer {
    serde_json::Deserializer::from_slice(bytes)
        .into_iter::<Value>()
        .next()
        .and_then(Result::ok)
        .ok_or_else(|| format!("{said} said something not JSON"))
}

#[cfg(test)]
mod tests {
    use super::{COMPLAINT_KEPT, failed, first, gathered, one, reader};
    use serde_json::json;
    use std::time::{Duration, Instant};

    /// A clock short enough for a case, long enough for a shell to start
    /// on a busy machine.
    const SHORT: Duration = Duration::from_secs(3);

    fn shell(script: &str) -> Vec<String> {
        vec!["sh".to_owned(), "-c".to_owned(), script.to_owned()]
    }

    #[test]
    fn an_owner_asked_says_its_complaint_when_it_fails() {
        let said = one(&shell("echo 'say which realm' >&2; exit 1"), SHORT);
        assert_eq!(
            said,
            Err(
                "sh -c echo 'say which realm' >&2; exit 1 ended with 1: say which realm".to_owned()
            ),
            "the complaint is read from the owner itself"
        );
    }

    #[test]
    fn a_chatty_complaint_never_stalls_an_answer() {
        // Far past a pipe's room, so a complaint not read as it's written
        // would stall the owner until the clock.
        let chatty = "i=0; while [ $i -lt 4000 ]; do echo 'a line of the log, again' >&2; \
                      i=$((i+1)); done; echo '{\"ok\": 1}'";
        assert_eq!(
            one(&shell(chatty), SHORT),
            Ok(json!({ "ok": 1 })),
            "an owner writing much to its complaint still answers"
        );
    }

    #[test]
    fn a_helper_left_holding_a_stream_never_holds_the_ask() {
        // A helper left holding a stream costs a moment once its owner has
        // ended, well inside the clock; only an owner still running costs
        // the clock itself.
        let held_complaint = "echo 'say which realm' >&2; sleep 30 >/dev/null & exit 1";
        let held_answer = "sleep 30 2>/dev/null & echo '{}'";
        let complaint_held_by_a_good_answer = "sleep 30 >/dev/null & echo '{}'";
        let slow = "exec sleep 30";
        for (script, why, within) in [
            (
                held_complaint,
                Err(format!(
                    "sh -c {held_complaint} ended with 1: say which realm"
                )),
                SHORT,
            ),
            (
                held_answer,
                Err(format!(
                    "sh -c {held_answer} left its answer open after it ended"
                )),
                SHORT,
            ),
            (complaint_held_by_a_good_answer, Ok(json!({})), SHORT),
            (
                slow,
                Err(format!("sh -c {slow} gave no answer in 3 s")),
                SHORT + Duration::from_secs(2),
            ),
        ] {
            let started = Instant::now();
            let said = one(&shell(script), SHORT);
            assert!(
                started.elapsed() < within,
                "{script} held the ask {:?}, past {within:?}",
                started.elapsed()
            );
            assert_eq!(said, why, "{script} is said for what held it");
        }
    }

    #[test]
    fn a_complaint_is_kept_to_its_room() {
        let long = vec![b'x'; 10_000];
        let receive = reader(std::io::Cursor::new(long), COMPLAINT_KEPT);
        let kept = gathered(&receive, Instant::now() + SHORT);
        assert_eq!(
            kept.bytes.len(),
            4096,
            "a long complaint costs no more than its room"
        );
        // A reason written past the room is never read, which shows an
        // owner's own complaint is kept to it.
        let past = "{ head -c 5000 /dev/zero | tr '\\0' '\\n'; echo 'past its room'; } >&2; exit 1";
        assert_eq!(
            one(&shell(past), SHORT),
            Err(format!("sh -c {past} ended with 1")),
            "an owner's complaint is read no further than its room"
        );
    }

    #[test]
    fn an_owner_that_ends_badly_says_its_complaints_first_line() {
        let complaint = b"\nmycelium: say which realm, with --realm or MYCELIUM_REALM\nmore\n";
        assert_eq!(
            failed("mycelium machines", Some(1), complaint),
            "mycelium machines ended with 1: mycelium: say which realm, with --realm or \
             MYCELIUM_REALM",
            "what it needed is said, not only that it failed"
        );
        assert_eq!(
            failed("repos --json", Some(2), b"  \n"),
            "repos --json ended with 2",
            "an owner that says nothing is said as it ended"
        );
        assert_eq!(
            failed("net --json", None, b""),
            "net --json was stopped",
            "one stopped by a signal is said stopped"
        );
        let long = "é".repeat(500);
        assert_eq!(
            failed("net --json", Some(1), long.as_bytes()),
            format!("net --json ended with 1: {}", "é".repeat(160)),
            "a long complaint is cut to a row's room, by characters"
        );
        assert_eq!(
            failed(
                "net --json",
                Some(1),
                b"\x1b[1m\x1b[31merror:\x1b[0m no route\x07"
            ),
            "net --json ended with 1: error: no route",
            "a coloured complaint is said plain"
        );
        assert_eq!(
            failed(
                "net --json",
                Some(1),
                "\x07\n\u{202e}gone\u{200b}".as_bytes()
            ),
            "net --json ended with 1: gone",
            "a line of only control characters is passed over, and text that turns or hides goes"
        );
        assert_eq!(
            failed(
                "net --json",
                Some(1),
                b"\x1b]8;;https://x\x1b\\link\x1b]8;;\x1b\\ broke\x1b(B\x1b[m"
            ),
            "net --json ended with 1: link broke",
            "a linked complaint keeps its words, not the link's address"
        );
        assert_eq!(
            failed(
                "net --json",
                Some(1),
                "\x1b]0;a title\x07said\u{e0041}\u{2028}\u{ad}".as_bytes()
            ),
            "net --json ended with 1: said",
            "a title set, tags and separators go"
        );
        assert_eq!(
            failed(
                "net --json",
                Some(1),
                "\u{9b}31mred\u{9d}0;title\u{9c}".as_bytes()
            ),
            "net --json ended with 1: red",
            "a control sequence and a string in their one-character forms go"
        );
        let error = concat!(
            r#"{"exit":69,"kind":"unavailable","schema":"error/1","try":null,"#,
            r#""what":"the checkouts couldn't be read","why":"no such folder"}"#
        );
        assert_eq!(
            failed("repos --json", Some(69), error.as_bytes()),
            "repos --json ended with 69: the checkouts couldn't be read: no such folder",
            "a kit command's JSON complaint is said as its words"
        );
        assert_eq!(
            failed("repos --json", Some(1), br#"{"why": "not a kit error"}"#),
            r#"repos --json ended with 1: {"why": "not a kit error"}"#,
            "other JSON is said as written"
        );
    }

    #[test]
    fn an_answer_followed_by_another_line_is_read_as_its_first() {
        let said = b"{\"Ok\":{\"Machines\":[]}}\n{\"Ok\":{\"Builders\":[]}}\n";
        assert_eq!(
            first("mycelium machines", said),
            Ok(json!({ "Ok": { "Machines": [] } })),
            "mycelium machines writes its builders on a line after its machines"
        );
    }

    #[test]
    fn an_answer_that_opens_with_no_document_is_not_json() {
        assert_eq!(
            first("net --json", b"warning: stale\n{}"),
            Err("net --json said something not JSON".to_owned()),
            "words before the document aren't read past"
        );
    }
}

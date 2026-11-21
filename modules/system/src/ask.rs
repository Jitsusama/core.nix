//! Asking each part's owner for its answer: its command run with a clock,
//! all at once, so the slowest owner bounds the wait rather than their sum.
//! An owner not on this machine, one that fails, and one that runs past
//! the clock each leave an answer saying why, never a hang.

use std::io::{self, Read};
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
/// `clock`: the command's own run and its streams' closing both count, as
/// a helper it left holding one would otherwise hold the ask.
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
    if !status.success() {
        // A complaint not had in time leaves the failure said as it ended.
        let complaint = taken(&complaining, deadline).and_then(Result::ok);
        return Err(failed(&said, status.code(), &complaint.unwrap_or_default()));
    }
    let bytes = taken(&reading, deadline)
        .ok_or_else(|| {
            // It ended well, but something it left holds its answer open,
            // so what it wrote may not be the whole of it.
            format!("{said} left its answer open past {} s", clock.as_secs())
        })?
        .map_err(|error| error.to_string())?;
    first(&said, &bytes)
}

/// Ends a child that won't be waited for, so nothing of it outlives the
/// ask.
fn end(child: &mut Child) {
    // Either may fail only for a child already gone, which is the end
    // wanted.
    let _ = child.kill();
    let _ = child.wait();
}

/// What `stream` writes, up to `kept` bytes and the rest drained unread,
/// sent once it closes.
fn reader(stream: impl Read + Send + 'static, kept: u64) -> Receiver<io::Result<Vec<u8>>> {
    let (send, receive) = mpsc::channel();
    thread::spawn(move || {
        let mut stream = stream;
        let mut bytes = Vec::new();
        let read = (&mut stream)
            .take(kept)
            .read_to_end(&mut bytes)
            .and_then(|_| io::copy(&mut stream, &mut io::sink()))
            .map(|_| bytes);
        // The ask may have stopped waiting, which leaves no one to tell.
        let _ = send.send(read);
    });
    receive
}

/// What a reader sent by `deadline`, or none when its stream was still open
/// then, as it is while a helper the owner left holds it.
fn taken<T>(receive: &Receiver<T>, deadline: Instant) -> Option<T> {
    receive
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .ok()
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
    let line = complaint.lines().map(plain).find(|line| !line.is_empty());
    match line {
        Some(line) => {
            let line: String = line.chars().take(SAID_OF_ITS_COMPLAINT).collect();
            format!("{ended}: {line}")
        }
        None => ended,
    }
}

/// `line` as plain words: a terminal's escape sequences gone whole, as a
/// coloured complaint writes them, and with them every control character
/// and every one that turns or hides text, so a row reads as it's written.
fn plain(line: &str) -> String {
    let mut kept = String::new();
    let mut chars = line.chars();
    while let Some(each) = chars.next() {
        match each {
            // A control sequence runs to its final character, '@' to '~';
            // any other escape takes the one character after it.
            '\u{1b}' => {
                if chars.next() == Some('[') {
                    chars.by_ref().find(|end| ('@'..='~').contains(end));
                }
            }
            '\u{200b}'..='\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2060}'..='\u{2069}'
            | '\u{feff}' => {}
            each if each.is_control() => {}
            each => kept.push(each),
        }
    }
    kept.trim().to_owned()
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
    use super::{COMPLAINT_KEPT, failed, first, one, reader};
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
        let held_complaint = "sleep 30 >/dev/null & exit 1";
        let held_answer = "sleep 30 2>/dev/null & echo '{}'";
        let slow = "exec sleep 30";
        for (script, why) in [
            (
                held_complaint,
                format!("sh -c {held_complaint} ended with 1"),
            ),
            (
                held_answer,
                format!("sh -c {held_answer} left its answer open past 3 s"),
            ),
            (slow, format!("sh -c {slow} gave no answer in 3 s")),
        ] {
            let started = Instant::now();
            let said = one(&shell(script), SHORT);
            assert!(
                started.elapsed() < SHORT + Duration::from_secs(2),
                "{script} held the ask {:?}, past its clock",
                started.elapsed()
            );
            assert_eq!(said, Err(why), "{script} is said for what held it");
        }
    }

    #[test]
    fn a_complaint_is_kept_to_its_room() {
        let long = vec![b'x'; 10_000];
        let kept = reader(std::io::Cursor::new(long), COMPLAINT_KEPT).recv();
        assert_eq!(
            kept.ok().and_then(Result::ok).map(|bytes| bytes.len()),
            Some(4096),
            "a long complaint costs no more than its room"
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

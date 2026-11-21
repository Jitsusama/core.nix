//! Asking each part's owner for its answer: its command run with a clock,
//! all at once, so the slowest owner bounds the wait rather than their sum.
//! An owner not on this machine, one that fails, and one that runs past
//! the clock each leave an answer saying why, never a hang.

use std::io::Read;
use std::process::{Command, Stdio};
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

/// Each command's answer, in the order asked.
pub fn all(commands: &[&[&str]]) -> Vec<Answer> {
    let asked: Vec<_> = commands
        .iter()
        .map(|words| {
            let words: Vec<String> = words.iter().map(|word| (*word).to_owned()).collect();
            thread::spawn(move || one(&words))
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

fn one(words: &[String]) -> Answer {
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
    // Read as it writes, so a long answer never fills the pipe and stalls
    // the owner while it's waited on.
    let out = child.stdout.take().ok_or("its answer couldn't be read")?;
    let reading = thread::spawn(move || read_all(out));
    // Its complaint is read the same way, so a failure can say why.
    let complaint = child
        .stderr
        .take()
        .ok_or("its complaint couldn't be read")?;
    let complaining = thread::spawn(move || read_all(complaint));
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|error| error.to_string())? {
            break status;
        }
        if started.elapsed() > CLOCK {
            // Its answer would come too late to be of use; it's ended so
            // nothing of it outlives the command.
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!("{said} gave no answer in {} s", CLOCK.as_secs()));
        }
        thread::sleep(LOOK);
    };
    let bytes = reading
        .join()
        .map_err(|_| "its answer's reader stopped on a bug".to_owned())?
        .map_err(|error| error.to_string())?;
    if !status.success() {
        // A complaint that can't be had leaves the failure said as it ended.
        let complaint = complaining
            .join()
            .ok()
            .and_then(Result::ok)
            .unwrap_or_default();
        return Err(failed(&said, status.code(), &complaint));
    }
    first(&said, &bytes)
}

/// Everything `stream` writes, read until it closes.
fn read_all(mut stream: impl Read) -> std::io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).map(|_| bytes)
}

/// Why an owner that ended badly gave no answer: how it ended, and the
/// first line of its complaint, when it made one, which is where an owner
/// says what it needed.
fn failed(said: &str, code: Option<i32>, complaint: &[u8]) -> String {
    let ended = match code {
        Some(code) => format!("{said} ended with {code}"),
        None => format!("{said} was stopped"),
    };
    let complaint = String::from_utf8_lossy(complaint);
    let line = complaint
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty());
    match line {
        Some(line) => {
            let line: String = line.chars().take(SAID_OF_ITS_COMPLAINT).collect();
            format!("{ended}: {line}")
        }
        None => ended,
    }
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
    use super::{failed, first};
    use serde_json::json;

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
        let long = "x".repeat(500);
        assert_eq!(
            failed("net --json", Some(1), long.as_bytes()).len(),
            "net --json ended with 1: ".len() + 160,
            "a long complaint is cut to a row's room"
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

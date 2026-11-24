//! Start New Work, done as the launcher asks for it (Agents.qml): new
//! work begun from what Joel said, a link alone or words in a folder,
//! asked of the realm's node as a `Sprout` in the node's own protocol,
//! the line `mycelium sprout` sends (mycelium's PROTOCOL.md). The node
//! judges the folder, makes the quest and starts its session held in no
//! window, so the launcher runs no command for it and both ask the same
//! code. Only the shell may ask (asker.rs), since the node takes this
//! daemon for Joel.
//!
//! Only what the launcher says is sent: the words as one, a folder only
//! as a whole path, and never a `cwd`, since this daemon's own working
//! folder says nothing of where Joel is, nor the machine or cell a
//! sprout may name, which the launcher never asks for.

use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Duration;

use serde_json::{Value, json};

/// The launcher's ask for it.
pub const ASK: &str = "sprout";

/// How long the node may take to start it: it answers once the session
/// is placed, in a second or two, and the launcher waits a minute, so the
/// node's silence is said before the launcher stops waiting.
const PATIENCE: Duration = Duration::from_secs(50);

/// What a node that took the ask but said nothing in time is told as:
/// it may still start the work, so asking again could start it twice.
const SILENT: &str =
    "the node didn't answer in time; it may still start it, so look in Agents before asking again";

/// The node's `Sprout` for what the launcher said, or why it asks for
/// nothing: no words, or a folder that isn't a whole path.
///
/// # Errors
///
/// Why it isn't a sprout the node could take, in words the launcher says.
pub fn request_of(said: &Value) -> Result<Value, String> {
    let words = said["text"]
        .as_str()
        .map(str::trim)
        .filter(|words| !words.is_empty())
        .ok_or("say what the work is: a link or a few words")?;
    let mut sprout = json!({"words": [words]});
    if let Some(folder) = named(said, "folder")? {
        if !folder.starts_with('/') {
            return Err("that folder isn't a whole path".to_owned());
        }
        sprout["folder"] = json!(folder);
    }
    Ok(json!({"Sprout": sprout}))
}

/// The field of `said` named `field`, none where it's left out, null or
/// empty.
fn named<'a>(said: &'a Value, field: &str) -> Result<Option<&'a str>, String> {
    match &said[field] {
        Value::Null => Ok(None),
        Value::String(name) if name.is_empty() => Ok(None),
        Value::String(name) => Ok(Some(name)),
        _ => Err(format!("a sprout's {field} is a path")),
    }
}

/// The answer to a sprout: `sprouted` with where the node started it,
/// or an error in the node's words, with the code it refused by where it
/// gave one (`refusal`, as needs_folder), or why it wasn't asked: `asker` says
/// whether the shell asked, and `node` is the realm's, none where this
/// machine runs no node.
pub fn answer(said: &Value, asker: Result<(), String>, node: Option<&Path>) -> Value {
    let Some(node) = node else {
        return refused("this machine runs no node to start new work on");
    };
    let request = match asker.and_then(|()| request_of(said)) {
        Ok(request) => request,
        Err(why) => return refused(&why),
    };
    match asked(node, &request, PATIENCE) {
        Ok(reply) => match (&reply["Ok"]["Started"], &reply["Err"], refusal_of(&reply)) {
            (Value::Object(started), _, _) => json!({"sprouted": started}),
            (_, Value::String(why), _) => refused(why),
            (_, _, Some((code, why))) => {
                let mut answer = refused(why);
                if let Some(code) = code {
                    answer["error"]["refusal"] = json!(code);
                }
                answer
            }
            _ => refused(&format!("the node's answer couldn't be read: {reply}")),
        },
        Err(Unanswered::Unreached(why)) => refused(&format!("the node couldn't be reached: {why}")),
        Err(Unanswered::Silent) => silent(),
    }
}

/// The code and words of a refusal its judging made, as the node says
/// one, `{"Refused": {"code", "why"}}` (mycelium's PROTOCOL.md), alone or
/// under `Ok`; none where it gave no words, and no code where it gave
/// none that's a plain word of letters and underscores.
fn refusal_of(reply: &Value) -> Option<(Option<&str>, &str)> {
    let refusal = reply
        .get("Refused")
        .or_else(|| reply["Ok"].get("Refused"))?;
    let code = refusal["code"].as_str().filter(|code| {
        !code.is_empty()
            && code
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
    });
    let why = refusal["why"].as_str().filter(|why| !why.is_empty())?;
    Some((code, why))
}

/// Why the node gave no answer: it couldn't be reached, so nothing was
/// asked, or it took the ask and fell silent, so the work may yet start.
pub(crate) enum Unanswered {
    Unreached(io::Error),
    Silent,
}

fn refused(why: &str) -> Value {
    json!({"error": {"code": "unsprouted", "message": why}})
}

/// A silent node's answer, coded apart from a refusal, so the launcher
/// journals it as a start that may yet begin rather than one that didn't.
fn silent() -> Value {
    json!({"error": {"code": "unanswered", "message": SILENT}})
}

/// Asks the node at `socket` for `request`, one line of its own protocol
/// with no hello, waiting up to `patience` on each step, and gives its
/// one line back, or a line it couldn't read as null.
pub(crate) fn asked(
    socket: &Path,
    request: &Value,
    patience: Duration,
) -> Result<Value, Unanswered> {
    let mut stream = UnixStream::connect(socket).map_err(Unanswered::Unreached)?;
    stream
        .set_read_timeout(Some(patience))
        .and_then(|()| stream.set_write_timeout(Some(patience)))
        .map_err(Unanswered::Unreached)?;
    writeln!(stream, "{request}").map_err(|_| Unanswered::Silent)?;
    let mut line = String::new();
    match BufReader::new(stream).read_line(&mut line) {
        Ok(read) if read > 0 => Ok(serde_json::from_str(&line).unwrap_or(Value::Null)),
        _ => Err(Unanswered::Silent),
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::net::UnixListener;
    use std::thread;

    use super::*;

    fn ask(more: &Value) -> Value {
        let mut said = json!({"ask": ASK, "text": "  https://github.com/o/r/issues/7  "});
        if let (Some(said), Some(more)) = (said.as_object_mut(), more.as_object()) {
            said.extend(more.clone());
        }
        said
    }

    #[test]
    fn what_joel_said_is_the_sprout_the_node_takes() {
        assert_eq!(
            request_of(&ask(&json!({}))),
            Ok(json!({"Sprout": {"words": ["https://github.com/o/r/issues/7"]}})),
            "the words as one, trimmed, with no folder, no cwd"
        );
        assert_eq!(
            request_of(&ask(&json!({"folder": "/home/j/src/r", "on": "skiff",
                "from": "P9R1KI", "cwd": "/home/j/elsewhere"}))),
            Ok(
                json!({"Sprout": {"words": ["https://github.com/o/r/issues/7"],
                "folder": "/home/j/src/r"}})
            ),
            "a folder goes as said; a machine, a cell and a cwd never do"
        );
        assert_eq!(
            request_of(&ask(&json!({"folder": ""}))),
            Ok(json!({"Sprout": {"words": ["https://github.com/o/r/issues/7"]}})),
            "an empty one is none"
        );
    }

    #[test]
    fn a_sprout_of_nothing_or_nowhere_asks_nothing() {
        for (said, why) in [
            (json!({"ask": ASK, "text": "  "}), "no words"),
            (json!({"ask": ASK}), "no text"),
            (json!({"ask": ASK, "text": 7}), "text that isn't words"),
            (
                ask(&json!({"folder": "src/r"})),
                "a folder that isn't a whole path",
            ),
            (ask(&json!({"folder": 7})), "a folder that isn't a path"),
        ] {
            assert!(
                request_of(&said).is_err_and(|refused| !refused.is_empty()),
                "{why} asks nothing, and says why"
            );
        }
    }

    /// A node at a socket of its own, answering one line with `reply`,
    /// and what it was asked.
    fn node(name: &str, reply: &'static str) -> (std::path::PathBuf, thread::JoinHandle<String>) {
        let socket =
            std::env::temp_dir().join(format!("agents-sprout-{name}-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let asked = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the sprout asks the node");
            let mut answering = stream.try_clone().expect("a stream to answer on");
            let mut line = String::new();
            BufReader::new(stream).read_line(&mut line).expect("a line");
            writeln!(answering, "{reply}").expect("the sprout answered");
            line
        });
        (socket, asked)
    }

    #[test]
    fn a_sprout_is_answered_with_where_the_node_started_it() {
        let (socket, asked) = node(
            "started",
            concat!(
                r#"{"Ok": {"Started": {"machine": "osprey", "held": "h1", "#,
                r#""folder": "/home/j/src/r", "attach": ["mycelium", "join", "h1"]}}}"#,
            ),
        );
        let answer = answer(
            &ask(&json!({"folder": "/home/j/src/r"})),
            Ok(()),
            Some(&socket),
        );
        // The answer is held to first, so one that never asked fails here
        // rather than waiting on a node nothing reaches.
        assert_eq!(
            answer,
            json!({"sprouted": {"machine": "osprey", "held": "h1", "folder": "/home/j/src/r",
                "attach": ["mycelium", "join", "h1"]}}),
            "where it started, as the node says"
        );
        let line = asked.join().expect("the node stood in");
        let _ = std::fs::remove_file(&socket);
        let line: Value = serde_json::from_str(&line).expect("one line of JSON");
        assert_eq!(
            line,
            json!({"Sprout": {"words": ["https://github.com/o/r/issues/7"],
                "folder": "/home/j/src/r"}}),
            "one Sprout line, with no hello"
        );
    }

    #[test]
    fn a_sprout_the_node_refuses_says_the_nodes_words() {
        let (socket, asked) = node(
            "refused",
            r#"{"Err": "/home/j isn't a repository's folder"}"#,
        );
        let answer = answer(&ask(&json!({})), Ok(()), Some(&socket));
        assert_eq!(
            answer,
            json!({"error": {"code": "unsprouted",
                "message": "/home/j isn't a repository's folder"}}),
            "the node's own reason"
        );
        let _ = asked.join();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn a_sprout_the_node_refuses_by_code_says_its_words_and_code() {
        for (name, reply) in [
            (
                "coded",
                r#"{"Refused": {"code": "needs_folder", "why": "say which"}}"#,
            ),
            (
                "coded-ok",
                r#"{"Ok": {"Refused": {"code": "needs_folder", "why": "say which"}}}"#,
            ),
        ] {
            let (socket, asked) = node(name, reply);
            assert_eq!(
                answer(&ask(&json!({})), Ok(()), Some(&socket)),
                json!({"error": {"code": "unsprouted", "message": "say which",
                    "refusal": "needs_folder"}}),
                "{name}: the node's words, and the code it refused by"
            );
            let _ = asked.join();
            let _ = std::fs::remove_file(&socket);
        }
        for (name, reply) in [
            (
                "odd-code",
                r#"{"Refused": {"code": "Needs Folder", "why": "say which"}}"#,
            ),
            ("no-code", r#"{"Refused": {"why": "say which"}}"#),
        ] {
            let (socket, asked) = node(name, reply);
            assert_eq!(
                answer(&ask(&json!({})), Ok(()), Some(&socket)),
                json!({"error": {"code": "unsprouted", "message": "say which"}}),
                "{name}: its words alone, with no code that isn't one"
            );
            let _ = asked.join();
            let _ = std::fs::remove_file(&socket);
        }
        let (socket, asked) = node("no-words", r#"{"Refused": {"code": "needs_folder"}}"#);
        let answer = answer(&ask(&json!({})), Ok(()), Some(&socket));
        assert!(
            answer["error"]["message"]
                .as_str()
                .is_some_and(|why| why.starts_with("the node's answer couldn't be read")),
            "one with no words is unread: {answer}"
        );
        let _ = asked.join();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn a_node_that_took_the_ask_and_fell_silent_may_yet_start_it() {
        let socket =
            std::env::temp_dir().join(format!("agents-sprout-silent-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket).expect("a socket to stand in for the node");
        let held = thread::spawn(move || {
            let (stream, _) = listener.accept().expect("the sprout asks the node");
            // Held open, saying nothing, past the patience below.
            thread::sleep(Duration::from_millis(600));
            drop(stream);
        });
        let request = request_of(&ask(&json!({}))).expect("a sprout");
        let answered = asked(&socket, &request, Duration::from_millis(200));
        assert!(
            matches!(answered, Err(Unanswered::Silent)),
            "a node that took the ask and said nothing is silent, never unreached"
        );
        assert_eq!(
            silent(),
            json!({"error": {"code": "unanswered", "message": SILENT}}),
            "and is said as one that may still start it, coded apart from a refusal"
        );
        assert!(
            SILENT.contains("may still start it"),
            "it says so: {SILENT}"
        );
        let _ = held.join();
        let _ = std::fs::remove_file(&socket);
    }

    #[test]
    fn a_node_answer_that_isnt_a_sprouts_is_unread() {
        for (name, reply) in [
            ("other", r#"{"Ok": {"Other": 1}}"#),
            ("garbled", "not json"),
        ] {
            let (socket, asked) = node(name, reply);
            let answer = answer(&ask(&json!({})), Ok(()), Some(&socket));
            assert!(
                answer["error"]["message"]
                    .as_str()
                    .is_some_and(|why| why.starts_with("the node's answer couldn't be read")),
                "{name}: {answer}"
            );
            let _ = asked.join();
            let _ = std::fs::remove_file(&socket);
        }
    }

    #[test]
    fn a_sprout_not_the_shells_or_with_no_node_isnt_asked() {
        let gone = std::env::temp_dir().join("agents-sprout-no-node.sock");
        let not_shell = answer(
            &ask(&json!({})),
            Err("only the launcher starts new work here".to_owned()),
            Some(&gone),
        );
        assert_eq!(
            not_shell,
            json!({"error": {"code": "unsprouted",
                "message": "only the launcher starts new work here"}}),
            "anyone but the shell is refused before the node is asked"
        );
        let nowhere = answer(&ask(&json!({"folder": "src"})), Ok(()), Some(&gone));
        assert!(
            nowhere["error"]["message"]
                .as_str()
                .is_some_and(|why| why.contains("whole path")),
            "a sprout it can't send says why: {nowhere}"
        );
        let unreached = answer(&ask(&json!({})), Ok(()), Some(&gone));
        assert!(
            unreached["error"]["message"]
                .as_str()
                .is_some_and(|why| why.starts_with("the node couldn't be reached: ")),
            "a node that isn't there wasn't reached, so nothing started: {unreached}"
        );
        let none = answer(&ask(&json!({})), Ok(()), None);
        assert_eq!(
            none["error"]["message"],
            json!("this machine runs no node to start new work on"),
            "with no node, nothing could start it: {none}"
        );
    }
}

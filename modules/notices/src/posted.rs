//! Asking a sender's daemon whether a notice batched from a channel Joel
//! reads every message of was one of its posts, as slack-unreads answers
//! `posted` (modules/slack's posted.rs). It's asked once, on a thread of
//! its own, and an answer that doesn't come in time, or isn't a post's,
//! leaves the notice to him: staying silent on a reply he'd want costs
//! more than one knock for a post (Joel, 2026-11-12: "i get major FOMO
//! if I don't know").

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde_json::{Value, json};

use crate::addressed::Asked;

/// How long the daemon has to answer: Slack answers a channel's newest
/// messages in well under a second, and a knock later than this would
/// come too long after Slack's own.
pub const PATIENCE: Duration = Duration::from_secs(3);

/// The socket `posted` names, under the runtime folder where it isn't
/// absolute.
#[must_use]
pub fn socket(posted: &str, runtime: &Path) -> PathBuf {
    let named = Path::new(posted);
    if named.is_absolute() {
        named.to_path_buf()
    } else {
        runtime.join(named)
    }
}

/// Whether the daemon at `socket` says the notice that came `at`, in
/// milliseconds since the epoch, was a post: none when it can't be asked,
/// says nothing in `patience`, or answers with an error.
#[must_use]
pub fn post(socket: &Path, asked: &Asked, at: i64, patience: Duration) -> Option<bool> {
    let mut stream = UnixStream::connect(socket).ok()?;
    stream.set_read_timeout(Some(patience)).ok()?;
    stream.set_write_timeout(Some(patience)).ok()?;
    let ask = json!({
        "id": 1, "ask": "posted", "channel": asked.channel, "words": asked.words, "at": at,
    });
    writeln!(stream, "{ask}").ok()?;
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).ok()?;
    let said: Value = serde_json::from_str(&line).ok()?;
    said.get("post").and_then(Value::as_bool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;
    use std::thread::JoinHandle;

    fn asked() -> Asked {
        Asked {
            socket: "chat.sock".into(),
            channel: "busy".into(),
            words: "on it".into(),
        }
    }

    /// A daemon on a socket of the test's own answering the one ask with
    /// `answer`, or never once `answer` is none, and the ask it was sent.
    fn daemon(name: &str, answer: Option<&'static str>) -> (PathBuf, JoinHandle<String>) {
        let named = format!("notices-posted-{name}-{}", std::process::id());
        let folder = std::env::temp_dir().join(named);
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("a folder");
        let path = folder.join("chat.sock");
        let listener = UnixListener::bind(&path).expect("it listens");
        let heard = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("it's asked");
            let mut line = String::new();
            BufReader::new(stream.try_clone().expect("a clone"))
                .read_line(&mut line)
                .expect("the ask is read");
            match answer {
                Some(answer) => writeln!(stream, "{answer}").expect("it answers"),
                None => std::thread::sleep(Duration::from_millis(400)),
            }
            line
        });
        (path, heard)
    }

    #[test]
    fn a_post_and_a_reply_are_said_as_the_daemon_answers() {
        let (path, heard) = daemon("post", Some(r#"{"id": 1, "post": true}"#));
        assert_eq!(post(&path, &asked(), 10, PATIENCE), Some(true), "a post");
        let ask: Value = serde_json::from_str(&heard.join().expect("it ran")).expect("JSON");
        let wanted = json!({"id": 1, "ask": "posted", "channel": "busy", "words": "on it",
            "at": 10});
        assert_eq!(ask, wanted, "the ask wasn't the daemon's");
        let (path, _) = daemon("reply", Some(r#"{"id": 1, "post": false}"#));
        assert_eq!(post(&path, &asked(), 10, PATIENCE), Some(false), "a reply");
    }

    #[test]
    fn an_error_silence_or_no_daemon_says_nothing() {
        let error = r#"{"id": 1, "error": {"code": "unknown"}}"#;
        let (path, _) = daemon("error", Some(error));
        assert_eq!(post(&path, &asked(), 10, PATIENCE), None, "an error");
        let (path, _) = daemon("silent", None);
        let quick = Duration::from_millis(100);
        assert_eq!(post(&path, &asked(), 10, quick), None, "silence");
        let nowhere = std::env::temp_dir().join("notices-posted-nowhere.sock");
        assert_eq!(post(&nowhere, &asked(), 10, PATIENCE), None, "no daemon");
    }

    #[test]
    fn a_socket_not_absolute_is_the_runtime_folders() {
        let runtime = Path::new("/run/user/7");
        let under = socket("daemons/chat.sock", runtime);
        let wanted = Path::new("/run/user/7/daemons/chat.sock");
        assert_eq!(under, wanted, "under it");
        let absolute = socket("/tmp/chat.sock", runtime);
        assert_eq!(absolute, Path::new("/tmp/chat.sock"), "moved");
    }
}

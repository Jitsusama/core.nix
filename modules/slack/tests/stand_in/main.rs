//! The program against a stand-in for Slack on this machine: what it asks,
//! with which sign-in, what it tells the shell, that it finds who each
//! address is once, keeping it across a restart, and asks how they are
//! each time, that asking ahead never holds up a step-in, and that it
//! asks nobody once Slack refuses the sign-in.
//!
//! The stand-in and what more than one file of cases shares are here;
//! the cases are beside this. slack-people's are in people.rs, with the
//! Slack it asks. slack-unreads' are in three: what wants Joel and the
//! socket it's asked on (`wanting.rs`), its rows and messages
//! (`rows.rs`), and what it does without a sign-in it can use
//! (`signed_out.rs`).

mod people;
mod rows;
mod signed_out;
mod wanting;

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// The launcher's own words (tests/words.json), which the check
/// `the-daemons-test-the-launchers-words` holds to words.json, handed to
/// a case's daemon as its unit hands it words.json itself.
const WORDS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/words.json");

/// One request the stand-in heard: its first line and its headers.
#[derive(Clone, Debug)]
struct Heard {
    line: String,
    headers: Vec<String>,
    at: Instant,
}

/// A stand-in for Slack's API, answering each request's first line with
/// what `answer` gives: a 200's body, or a status line and its headers
/// when it starts `HTTP/1.1`.
struct StandIn {
    origin: String,
    heard: Arc<Mutex<Vec<Heard>>>,
}

impl StandIn {
    fn new(answer: fn(&str) -> String) -> StandIn {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let origin = format!("http://{}", listener.local_addr().unwrap());
        let heard = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heard);
        thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut reader = BufReader::new(stream.try_clone().unwrap());
                let mut line = String::new();
                // A connection shut before it asks anything, as one the
                // program opened as a test killed it, asked nothing.
                if reader.read_line(&mut line).unwrap_or(0) == 0 {
                    continue;
                }
                let mut headers = Vec::new();
                let mut length = 0;
                loop {
                    let mut header = String::new();
                    reader.read_line(&mut header).unwrap();
                    let header = header.trim_end().to_owned();
                    if header.is_empty() {
                        break;
                    }
                    if let Some(size) = header.to_lowercase().strip_prefix("content-length: ") {
                        length = size.parse().unwrap_or(0);
                    }
                    headers.push(header);
                }
                let mut body = vec![0; length];
                reader.read_exact(&mut body).unwrap();
                let line = line.trim_end().to_owned();
                let said = answer(&line);
                kept.lock().unwrap().push(Heard {
                    line,
                    headers,
                    at: Instant::now(),
                });
                let reply = if said.starts_with("HTTP/1.1 ") {
                    format!("{said}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                } else {
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                         Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                        said.len()
                    )
                };
                stream.write_all(reply.as_bytes()).unwrap();
            }
        });
        StandIn { origin, heard }
    }

    fn heard(&self) -> Vec<Heard> {
        self.heard.lock().unwrap().clone()
    }
}

/// A sign-in as the harness keeps it, in a file of the test's own.
fn credentials(test: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("slack-people-{test}-{}.json", std::process::id()));
    std::fs::write(
        &path,
        r#"{"token": {"accessToken": "xoxc-mine", "cookie": "d-cookie", "userId": "U0",
            "teamId": "T1", "teamName": "Team", "scopes": []}}"#,
    )
    .unwrap();
    path
}

/// The cache home a run with this sign-in's file keeps in.
fn cache(credentials: &std::path::Path) -> PathBuf {
    credentials.with_extension("cache")
}

/// Forgets a test's sign-in and what its runs kept.
fn forget(credentials: &std::path::Path) {
    let _ = std::fs::remove_file(credentials);
    let _ = std::fs::remove_dir_all(cache(credentials));
}

/// Slack as it answers what wants Joel: Ada's direct message with two
/// unread, a channel with one unread and a mention of him, another unread
/// with none, and three mentions in threads.
fn wants_joel(line: &str) -> String {
    let said = if line.contains("client.counts") {
        concat!(
            r#"{"ok": true, "ims": [{"id": "D1", "has_unreads": true, "mention_count": 2,"#,
            r#""last_read": "1730100000.000100"}],"#,
            r#""mpims": [], "channels": [{"id": "C2", "has_unreads": true, "mention_count": 1},"#,
            r#"{"id": "C3", "has_unreads": true, "mention_count": 0,"#,
            r#""last_read": "1730200000.000100"}],"#,
            r#""threads": {"has_unreads": true, "mention_count": 3}}"#
        )
    } else if line.contains("conversations.info?channel=C3") {
        r#"{"ok": true, "channel": {"id": "C3", "is_channel": true, "name": "random"}}"#
    } else if line.contains("conversations.info?channel=D1") {
        r#"{"ok": true, "channel": {"id": "D1", "is_im": true, "user": "U7"}}"#
    } else if line.contains("conversations.info?channel=C2") {
        r#"{"ok": true, "channel": {"id": "C2", "is_channel": true, "name": "launcher"}}"#
    } else if line.contains("users.info?user=U7") {
        r#"{"ok": true, "user": {"id": "U7", "profile": {"real_name": "Ada Lovelace"}}}"#
    } else {
        r#"{"ok": false, "error": "unknown_method"}"#
    };
    said.into()
}

/// slack-unreads, started over the stand-in with this sign-in, asking
/// every 50 ms, answering on a socket beside `run`, a path of the test's
/// own, and keeping what it asks of Joel beside it too (`waiting`).
fn unreads(stand_in: &StandIn, credentials: &std::path::Path, run: &std::path::Path) -> Child {
    unreads_from(
        Command::new(env!("CARGO_BIN_EXE_slack-unreads")),
        stand_in,
        credentials,
        run,
    )
}

/// slack-unreads as `unreads` starts it, through `command`.
fn unreads_from(
    mut command: Command,
    stand_in: &StandIn,
    credentials: &std::path::Path,
    run: &std::path::Path,
) -> Child {
    command
        .arg(credentials)
        .arg(socket(run))
        .arg(waiting(run))
        .arg(&stand_in.origin)
        .arg("50")
        // Only the sign-in a case writes is read, never one the keyring
        // of whoever runs the tests would give, which would reach the
        // stand-in and be printed by any assert of what it heard.
        .env_remove("SIGN_IN_KEYRING")
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

/// Where `run` asks Joel to sign in again.
fn waiting(run: &std::path::Path) -> PathBuf {
    run.with_extension("waiting.json")
}

/// Where `run` answers asks.
fn socket(run: &std::path::Path) -> PathBuf {
    run.with_extension("daemons").join("slack-unreads.sock")
}

/// Asks a running slack-unreads one thing over its socket, and what it
/// answered.
fn asked(run: &std::path::Path, ask: &serde_json::Value) -> serde_json::Value {
    let mut stream = std::os::unix::net::UnixStream::connect(socket(run)).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    writeln!(stream, "{ask}").unwrap();
    let mut line = String::new();
    BufReader::new(stream).read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

/// Waits until `held` holds, failing with `what` after ten seconds.
fn until(what: &str, held: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !held() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(20));
    }
}

/// What a file says, once it says anything whole.
fn kept(file: &std::path::Path) -> Option<serde_json::Value> {
    serde_json::from_str(&std::fs::read_to_string(file).ok()?).ok()
}

/// What's held of `kind`, each thing as its record says it, in the order
/// listed.
fn listed(run: &std::path::Path, kind: &str) -> Vec<serde_json::Value> {
    let held = asked(
        run,
        &serde_json::json!({"id": kind, "ask": "list", "kind": kind}),
    );
    held["records"]
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record[kind].clone())
        .collect()
}

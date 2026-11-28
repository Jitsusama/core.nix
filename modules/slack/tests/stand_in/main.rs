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
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

/// What the programs read from the environment, never taken from
/// whoever runs the tests: the keyring's sign-in, which would reach the
/// stand-in and be printed by any assert of what it heard; the
/// launcher's words, which change what is said; and systemd's watchdog
/// socket, which would be told each round. A case wanting the words or
/// the socket sets its own.
const SHUT_OUT: [&str; 3] = ["SIGN_IN_KEYRING", "LAUNCHER_WORDS", "NOTIFY_SOCKET"];

/// Takes from `command` each of `SHUT_OUT` the case hasn't set on it.
fn shut_out(command: &mut Command) -> &mut Command {
    let set: Vec<_> = command
        .get_envs()
        .filter(|(_, value)| value.is_some())
        .map(|(name, _)| name.to_owned())
        .collect();
    for name in SHUT_OUT {
        if !set.iter().any(|kept| kept == name) {
            command.env_remove(name);
        }
    }
    command
}

/// What `command` is told of each of `SHUT_OUT`: `None` where it's taken
/// away, its value where the case set one.
fn told_of(command: &Command) -> Vec<(&'static str, Option<String>)> {
    SHUT_OUT
        .into_iter()
        .map(|name| {
            let found = command.get_envs().find(|(key, _)| **key == *name);
            let Some((_, value)) = found else {
                panic!("{name} isn't taken from whoever runs the tests");
            };
            (
                name,
                value.map(|value| value.to_string_lossy().into_owned()),
            )
        })
        .collect()
}

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

/// A case's own folder, made afresh for the test and this run and open
/// to this user alone, holding all that its runs keep: its sign-in, the
/// cache beside it, its daemon's run and what that writes, and the socket
/// it listens on as systemd's watchdog. Nothing at those names can be
/// made or linked first by another run or user, and the folder goes,
/// with all of it, once the case lets go, failing or not.
struct Case(PathBuf);

impl Case {
    /// A `Case` named for the test and this run.
    fn new(test: &str) -> Case {
        let folder = std::env::temp_dir().join(format!("slack-{test}-{}", std::process::id()));
        if let Err(error) = std::fs::DirBuilder::new().mode(0o700).create(&folder) {
            panic!(
                "{} couldn't be made afresh, so the case might read what another left \
                 in it: {error}. A run killed partway leaves its folder: remove it once \
                 no run is using it.",
                folder.display()
            );
        }
        Case(folder)
    }

    /// The sign-in's file, which nothing makes until `signed_in` does.
    fn sign_in(&self) -> PathBuf {
        self.0.join("sign-in.json")
    }

    /// The sign-in's file, holding a sign-in as the harness keeps it.
    fn signed_in(&self) -> PathBuf {
        let path = self.sign_in();
        std::fs::write(
            &path,
            r#"{"token": {"accessToken": "xoxc-mine", "cookie": "d-cookie", "userId": "U0",
                "teamId": "T1", "teamName": "Team", "scopes": []}}"#,
        )
        .unwrap();
        path
    }

    /// The path the case's slack-unreads runs over (`unreads`).
    fn run(&self) -> PathBuf {
        self.0.join("run")
    }

    /// Where the case listens as systemd's watchdog would.
    fn notify(&self) -> PathBuf {
        self.0.join("notify")
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The cache home a run with this sign-in's file keeps in, beside it.
fn cache(credentials: &std::path::Path) -> PathBuf {
    credentials.with_extension("cache")
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

/// A slack-unreads a case started over `run`, stopped once the case lets
/// go of it, so a case failing partway leaves no daemon asking a stand-in
/// that's gone. What it kept beside `run` goes with the case's folder.
struct Running {
    child: Child,
    /// Where what it writes to its standard error is kept, beside `run`.
    said: PathBuf,
    /// Whether the case has stopped it.
    stopped: bool,
}

impl Running {
    /// `command` started, what it writes to its standard error kept
    /// beside `run`.
    fn started(mut command: Command, run: &std::path::Path) -> Running {
        let said = run.with_extension("stderr");
        let file = std::fs::File::create_new(&said).unwrap();
        Running {
            child: command.stderr(file).spawn().unwrap(),
            said,
            stopped: false,
        }
    }

    /// Stops it, failing the case if it had already ended on its own,
    /// saying how and what it wrote, and waits for it to end. What it kept
    /// stays until the case lets go.
    fn stop(&mut self) {
        assert!(
            !self.stopped,
            "slack-unreads was stopped twice, so the case stops what it already stopped"
        );
        self.stopped = true;
        let ended = self
            .child
            .try_wait()
            .expect("couldn't ask whether slack-unreads had ended");
        if let Some(status) = ended {
            panic!(
                "slack-unreads had ended before it was stopped ({status}), having written:\n{}",
                self.written()
            );
        }
        self.end()
            .expect("slack-unreads couldn't be stopped, or wasn't seen to end");
    }

    /// What it wrote to its standard error, or why that can't be read.
    fn written(&self) -> String {
        std::fs::read_to_string(&self.said)
            .unwrap_or_else(|error| format!("({} couldn't be read: {error})", self.said.display()))
    }

    /// Kills it and waits for it, as both stopping and letting go do.
    fn end(&mut self) -> std::io::Result<ExitStatus> {
        self.child.kill()?;
        self.child.wait()
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // Stopped already, or failing: either way, nothing more to say.
        let _ = self.end();
    }
}

#[test]
#[should_panic(expected = "slack-unreads had ended before it was stopped (exit status: 0)")]
fn a_daemon_ended_before_it_is_stopped_fails_its_case_saying_how() {
    let case = Case::new("ended-early");
    let mut ended = Running::started(Command::new("true"), &case.run());
    ended.child.wait().expect("true couldn't be waited for");
    ended.stop();
}

#[test]
#[should_panic(expected = "slack-unreads was stopped twice")]
fn a_daemon_stopped_twice_fails_its_case_without_blaming_the_daemon() {
    let case = Case::new("stopped-twice");
    let mut reading = Command::new("cat");
    reading.stdin(Stdio::piped());
    let mut running = Running::started(reading, &case.run());
    running.stop();
    running.stop();
}

/// slack-unreads, started over the stand-in with this sign-in, asking
/// every 50 ms, answering on a socket beside `run`, a path of the test's
/// own, and keeping what it asks of Joel beside it too (`waiting`).
fn unreads(stand_in: &StandIn, credentials: &std::path::Path, run: &std::path::Path) -> Running {
    unreads_from(
        Command::new(env!("CARGO_BIN_EXE_slack-unreads")),
        stand_in,
        credentials,
        run,
    )
}

/// slack-unreads as `unreads` starts it, through `command`.
fn unreads_from(
    command: Command,
    stand_in: &StandIn,
    credentials: &std::path::Path,
    run: &std::path::Path,
) -> Running {
    Running::started(unreads_command(command, stand_in, credentials, run), run)
}

/// `command` set to run slack-unreads as `unreads_from` starts it, reading
/// only what the case gives it. `unreads_from` keeps what it writes to its
/// standard error, for a case to say if it ends on its own.
fn unreads_command(
    mut command: Command,
    stand_in: &StandIn,
    credentials: &std::path::Path,
    run: &std::path::Path,
) -> Command {
    command
        .arg(credentials)
        .arg(socket(run))
        .arg(waiting(run))
        .arg(&stand_in.origin)
        .arg("50");
    shut_out(&mut command);
    command
}

#[test]
fn slack_unreads_reads_nothing_of_whoever_runs_it_but_what_the_case_sets() {
    let stand_in = StandIn::new(wants_joel);
    let run = PathBuf::from("run");
    let plain = unreads_command(Command::new("slack-unreads"), &stand_in, &run, &run);
    assert_eq!(
        told_of(&plain),
        [
            ("SIGN_IN_KEYRING", None),
            ("LAUNCHER_WORDS", None),
            ("NOTIFY_SOCKET", None)
        ],
        "a case setting nothing hands slack-unreads none of these"
    );
    let mut command = Command::new("slack-unreads");
    command.env("LAUNCHER_WORDS", WORDS);
    command.env("NOTIFY_SOCKET", "notify");
    let set = unreads_command(command, &stand_in, &run, &run);
    assert_eq!(
        told_of(&set),
        [
            ("SIGN_IN_KEYRING", None),
            ("LAUNCHER_WORDS", Some(WORDS.to_owned())),
            ("NOTIFY_SOCKET", Some("notify".to_owned()))
        ],
        "what a case sets is kept"
    );
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

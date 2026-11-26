//! The program against a stand-in for Slack on this machine: what it asks,
//! with which sign-in, what it tells the shell, that it finds who each
//! address is once, keeping it across a restart, and asks how they are
//! each time, that asking ahead never holds up a step-in, and that it
//! asks nobody once Slack refuses the sign-in.
//!
//! The stand-in and what every case shares are here; the cases are
//! beside this, a file to each of slack-people, what wants Joel, and
//! the rows the launcher is served.

mod people;
mod rows;
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

/// Runs the program over the stand-in, given these lines, and says what
/// it said back, a line each. What it keeps goes in a cache home beside
/// the sign-in's file, so a second run with the same file finds it.
fn told(stand_in: &StandIn, credentials: &PathBuf, lines: &str) -> Vec<String> {
    said_to(started(&stand_in.origin, credentials), lines)
}

/// What the program said back to these lines, a line each.
fn said_to(mut program: std::process::Child, lines: &str) -> Vec<String> {
    program
        .stdin
        .take()
        .unwrap()
        .write_all(lines.as_bytes())
        .unwrap();
    let out = program.wait_with_output().unwrap();
    String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The program, started over Slack at `origin` with this sign-in, and
/// without the launcher's words, so it says who they are alone, as each
/// case but one hears it.
fn started(origin: &str, credentials: &PathBuf) -> std::process::Child {
    program(origin, credentials).spawn().unwrap()
}

/// The program to run over Slack at `origin` with this sign-in.
fn program(origin: &str, credentials: &PathBuf) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_slack-people"));
    command
        .arg(credentials)
        .arg(origin)
        .env("XDG_CACHE_HOME", cache(credentials))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    command
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

/// Ada as the program says her, her zone with her, active.
const ADA: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"active","status":"","emoji":"","handle":""}"#
);

/// Ada asked about again, once she's set a status.
const ADA_WRITING: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"active","status":"Writing","emoji":"📝","handle":""}"#
);

/// Ada while Slack can't say whether she's active or away.
const ADA_UNSAID: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London","presence":"","status":"","emoji":"","handle":""}"#
);

/// Grace as the program says her, found by a search, as an enterprise
/// grid has it, and away.
const GRACE: &str = concat!(
    r#"{"email":"grace@example.com","id":"U2","team":"T1","name":"Grace Hopper","#,
    r#""zone":"America/New_York","presence":"away","status":"","emoji":"","handle":""}"#
);

/// Grace found ahead, before anyone asked how she is.
const GRACE_WHO: &str = concat!(
    r#"{"email":"grace@example.com","id":"U2","team":"T1","name":"Grace Hopper","#,
    r#""zone":"America/New_York","presence":"","status":"","emoji":"","handle":""}"#
);

/// Someone Slack knows nobody by, as the program says them.
fn nobody(email: &str) -> String {
    format!(r#"{{"email":"{email}","id":null,"team":"T1","name":"","zone":"","presence":"","#)
        + r#""status":"","emoji":"","handle":""}"#
}

/// Every method the program may ask, each only reading.
const READS: [&str; 4] = [
    "users.lookupByEmail?",
    "search.messages?",
    "users.info?",
    "users.getPresence?",
];

/// Whether everything the stand-in heard was a GET of one of the reads.
fn only_read(heard: &[Heard]) -> bool {
    heard.iter().all(|heard| {
        let method = heard.line.trim_start_matches("GET /api/");
        heard.line.starts_with("GET /api/") && READS.iter().any(|read| method.starts_with(read))
    })
}

/// Slack as a workspace that looks Ada up by her address, and as an
/// enterprise grid for Grace and Eve, refusing that to the sign-in: a
/// search finds each by their handle, and only Grace's profile keeps the
/// address asked about.
fn slack(line: &str) -> String {
    if line.contains("email=lin%40example.com") || line.contains("users.info?user=U4") {
        r#"{"ok": true, "user": {"id": "U4", "team_id": "T1", "name": "lin",
            "profile": {"real_name": "Lin Chen"}}}"#
            .into()
    } else if line.contains("search.messages") && line.contains("lin") {
        // Lin's direct messages with Joel, U0, newest first, and one of
        // hers elsewhere that isn't theirs.
        concat!(
            r#"{"ok": true, "messages": {"matches": ["#,
            r#"{"channel": {"id": "D4", "is_im": true, "name": "U4"}, "user": "U4","#,
            r#""text": "See you there &amp; then", "ts": "1730312345.000100","#,
            r#""permalink": "https://slack.example/archives/D4/p2"},"#,
            r#"{"channel": {"id": "C1", "is_im": false, "name": "general"}, "user": "U4","#,
            r#""text": "Hello all", "ts": "1730312320.000000"},"#,
            r#"{"channel": {"id": "D4", "is_im": true, "name": "U4"}, "user": "U0","#,
            r#""text": "Lunch, <@U4>?", "ts": "1730312300.000000"}]}}"#
        )
        .into()
    } else if line.contains("email=ada%40example.com") {
        r#"{"ok": true, "user": {"id": "U1", "team_id": "T1", "tz": "Europe/London",
            "profile": {"real_name": "Ada Lovelace"}}}"#
            .into()
    } else if line.contains("email=grace%40example.com") || line.contains("email=eve%40example.com")
    {
        r#"{"ok": false, "error": "not_allowed_token_type"}"#.into()
    } else if line.contains("search.messages") && line.contains("grace") {
        r#"{"ok": true, "messages": {"matches": [{"user": "U2", "username": "grace"}]}}"#.into()
    } else if line.contains("search.messages") && line.contains("eve") {
        r#"{"ok": true, "messages": {"matches": [{"user": "U3", "username": "eve"}]}}"#.into()
    } else if line.contains("users.info") && line.contains("user=U2") {
        r#"{"ok": true, "user": {"id": "U2", "team_id": "T1", "tz": "America/New_York",
            "profile": {"real_name": "Grace Hopper", "email": "Grace@Example.com"}}}"#
            .into()
    } else if line.contains("users.info") && line.contains("user=U1") {
        r#"{"ok": true, "user": {"id": "U1", "team_id": "T1", "tz": "Europe/London",
            "profile": {"real_name": "Ada Lovelace", "status_text": "Writing",
                "status_emoji_display_info": [{"emoji_name": "memo", "unicode": "1f4dd"}]}}}"#
            .into()
    } else if line.contains("users.getPresence") && line.contains("user=U1") {
        r#"{"ok": true, "presence": "active"}"#.into()
    } else if line.contains("users.getPresence") && line.contains("user=U2") {
        r#"{"ok": true, "presence": "away"}"#.into()
    } else if line.contains("users.info") && line.contains("user=U3") {
        r#"{"ok": true, "user": {"id": "U3", "team_id": "T1",
            "profile": {"real_name": "Another Eve", "email": "eve@elsewhere.example"}}}"#
            .into()
    } else {
        r#"{"ok": false, "error": "users_not_found"}"#.into()
    }
}

/// Slack as `slack` has it, but too busy to say who anyone is again or
/// whether they're active.
fn busy(line: &str) -> String {
    if line.contains("users.info") || line.contains("users.getPresence") {
        r#"{"ok": false, "error": "ratelimited"}"#.into()
    } else {
        slack(line)
    }
}

/// Slack as it answers what wants Joel: Ada's direct message with two
/// unread, a channel with one unread and a mention of him, another unread
/// with none, and three mentions in threads.
fn wanting(line: &str) -> String {
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

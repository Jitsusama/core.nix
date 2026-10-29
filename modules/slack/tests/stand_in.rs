//! The program against a stand-in for Slack on this machine: what it asks,
//! with which sign-in, what it tells the shell, and that it asks each
//! address once and nobody once Slack refuses the sign-in.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// One request the stand-in heard: its first line and its headers.
#[derive(Clone, Debug)]
struct Heard {
    line: String,
    headers: Vec<String>,
}

/// A stand-in for Slack's API, answering each request's first line with
/// what `answer` gives.
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
                reader.read_line(&mut line).unwrap();
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
                kept.lock().unwrap().push(Heard { line, headers });
                let reply = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                     Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                    said.len()
                );
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
/// it said back, a line each.
fn told(stand_in: &StandIn, credentials: &PathBuf, lines: &str) -> Vec<String> {
    let mut program = Command::new(env!("CARGO_BIN_EXE_slack-people"))
        .arg(credentials)
        .arg(&stand_in.origin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
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

/// Ada as the program says her, her zone with her.
const ADA: &str = concat!(
    r#"{"email":"ada@example.com","id":"U1","team":"T1","name":"Ada Lovelace","#,
    r#""zone":"Europe/London"}"#
);

fn slack(line: &str) -> String {
    if line.contains("email=ada%40example.com") {
        r#"{"ok": true, "user": {"id": "U1", "team_id": "T1", "tz": "Europe/London",
            "profile": {"real_name": "Ada Lovelace"}}}"#
            .into()
    } else {
        r#"{"ok": false, "error": "users_not_found"}"#.into()
    }
}

#[test]
fn each_address_is_asked_once_with_the_harness_sign_in_and_said_a_line() {
    let stand_in = StandIn::new(slack);
    let path = credentials("found");
    let said = told(
        &stand_in,
        &path,
        "Ada@Example.com\nnobody@example.com\nnot an address\nada@example.com\n",
    );
    assert_eq!(
        said,
        [
            ADA,
            r#"{"email":"nobody@example.com","id":null,"team":"T1","name":"","zone":""}"#,
            ADA,
        ]
    );
    let heard = stand_in.heard();
    assert_eq!(heard.len(), 2, "{heard:?}");
    assert!(
        heard[0].line.starts_with("GET /api/users.lookupByEmail?"),
        "{}",
        heard[0].line
    );
    assert!(
        heard[0]
            .headers
            .iter()
            .any(|header| header == "authorization: Bearer xoxc-mine"
                || header == "Authorization: Bearer xoxc-mine")
    );
    assert!(
        heard[0]
            .headers
            .iter()
            .any(|header| header.ends_with("d=d-cookie"))
    );
    // Only ever a look-up: nothing is posted, nothing sent.
    assert!(
        heard
            .iter()
            .all(|heard| heard.line.starts_with("GET /api/users.lookupByEmail?"))
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn a_refused_sign_in_is_said_once_and_nobody_more_is_asked_about() {
    let stand_in = StandIn::new(|_| r#"{"ok": false, "error": "invalid_auth"}"#.into());
    let path = credentials("refused");
    let said = told(&stand_in, &path, "ada@example.com\ngrace@example.com\n");
    assert_eq!(said, [r#"{"signedOut":true}"#]);
    assert_eq!(stand_in.heard().len(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn with_no_sign_in_nothing_is_said_and_slack_is_never_asked() {
    let stand_in = StandIn::new(slack);
    let missing = std::env::temp_dir().join("slack-people-no-such-file.json");
    let said = told(&stand_in, &missing, "ada@example.com\n");
    assert!(said.is_empty(), "{said:?}");
    assert!(stand_in.heard().is_empty());
}

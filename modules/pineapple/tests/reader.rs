//! pineapple-slack against a stand-in for Slack on this machine: a stop
//! said in a thread about a pull request reaches that pull request and is
//! kept as a shadow record; a tag asking to stop without the phrase is
//! written down and stops nothing; only Slack's reading methods are asked;
//! and a restart judges nothing twice.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

/// How long a case waits on the reader before it calls it stuck.
const PATIENCE: Duration = Duration::from_secs(20);

/// How often the reader searches in a case, in milliseconds.
const PACE: &str = "100";

/// The stand-in's Joel, and the colleagues who tag him.
const JOEL: &str = "U0JOEL0001";

/// A stand-in for Slack's API that keeps the first line of each request
/// and answers as Slack would to a search and to a thread's first message.
fn stand_in() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port on this machine");
    let origin = format!("http://{}", listener.local_addr().expect("its address"));
    let asked = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&asked);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut reader = BufReader::new(stream.try_clone().expect("the stream again"));
            let mut line = String::new();
            // A connection closed before it asks, as a killed reader's
            // is, asked nothing.
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                continue;
            }
            // Its headers end at an empty line; a GET has no body.
            let mut header = String::new();
            while reader.read_line(&mut header).unwrap_or(0) > 0 && !header.trim().is_empty() {
                header.clear();
            }
            let said = answer(&line, now).to_string();
            kept.lock()
                .expect("the stand-in's list")
                .push(line.trim_end().to_owned());
            let reply = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{said}",
                said.len()
            );
            // A reader killed mid-answer leaves nobody to write to.
            let _ = stream.write_all(reply.as_bytes());
        }
    });
    (origin, asked)
}

/// What Slack would answer to one request, a minute after `now` less.
fn answer(line: &str, now: u64) -> Value {
    let stop_ts = format!("{}.000100", now - 60);
    let ask_ts = format!("{}.000200", now - 30);
    let stop = json!({
        "channel": {"id": "C0ONE00001"}, "ts": stop_ts, "user": "U0OTHER001",
        "text": format!("<@{JOEL}> pineapple override"),
        "permalink": format!(
            "https://example.slack.com/archives/C0ONE00001/p{}?thread_ts=1700000000.000001&cid=C0ONE00001",
            stop_ts.replace('.', "")
        ),
    });
    let ask = json!({
        "channel": {"id": "C0ONE00001"}, "ts": ask_ts, "user": "U0OTHER002",
        "text": format!("<@{JOEL}> can you please STOP your robots?"),
        "permalink": format!(
            "https://example.slack.com/archives/C0ONE00001/p{}",
            ask_ts.replace('.', "")
        ),
    });
    if line.contains("/api/conversations.replies") {
        return json!({"ok": true, "messages": [
            {"text": "a review, please: https://code.example.com/example/shop/pull/12"}
        ]});
    }
    if line.contains("query=pineapple") {
        json!({"ok": true, "messages": {"matches": [stop]}})
    } else {
        json!({"ok": true, "messages": {"matches": [ask, stop]}})
    }
}

/// A home for one case, its sign-in in place as pi keeps it.
fn home() -> PathBuf {
    let home = std::env::temp_dir().join(format!(
        "pineapple-reader-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_nanos()
    ));
    let signed_in = home.join("config/agentic-harness/slack");
    std::fs::create_dir_all(&signed_in).expect("a home for the case");
    let account = json!({"token": {
        "accessToken": "xoxc-stand-in", "cookie": "stand-in", "teamId": "T0TEAM0001",
        "userId": JOEL,
    }});
    std::fs::write(signed_in.join("credentials.json"), account.to_string())
        .expect("the sign-in written");
    home
}

fn reader(home: &Path, origin: &str) -> Child {
    Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
        .args(["run", "--origin", origin, "--every", PACE])
        .env("HOME", home)
        .env("XDG_CONFIG_HOME", home.join("config"))
        .env("XDG_STATE_HOME", home.join("state"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("the reader starts")
}

/// The decisions written down so far.
fn decisions(home: &Path) -> Vec<Value> {
    std::fs::read_to_string(home.join("state/pineapple/decisions.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Waits until the reader has written `count` decisions, or says it's stuck.
fn until_decided(home: &Path, count: usize) {
    let began = Instant::now();
    while decisions(home).len() < count {
        assert!(
            began.elapsed() < PATIENCE,
            "the reader wrote {} decisions, not {count}",
            decisions(home).len()
        );
        // The reader searches every tenth of a second, so a short wait
        // sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn a_stop_in_a_thread_reaches_its_pull_request_in_shadow_and_is_judged_once() {
    let (origin, asked) = stand_in();
    let home = home();
    let mut first = reader(&home, &origin);
    until_decided(&home, 2);
    first.kill().expect("the reader stops");
    first.wait().expect("the reader is gone");

    let said = decisions(&home);
    let verdicts: Vec<&str> = said
        .iter()
        .filter_map(|one| one["verdict"].as_str())
        .collect();
    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "oldest first, each judged"
    );
    assert_eq!(
        said[0]["reaches"],
        json!(["pr:example/shop#12"]),
        "the thread's pull request is what stops"
    );
    assert_eq!(said[0]["shadow"], true, "it is in shadow");

    let stops: Vec<PathBuf> = std::fs::read_dir(home.join("state/pineapple/stops"))
        .expect("the stops folder")
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    assert_eq!(stops.len(), 1, "only the stop is kept as one");
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(&stops[0]).expect("the stop's record"))
            .expect("the record is JSON");
    assert_eq!(record["shadow"], true, "the record says it is in shadow");
    assert_eq!(record["said_by"], "U0OTHER001", "it names who said it");
    assert_eq!(record["resumed"], Value::Null, "nobody has resumed it");

    let methods: Vec<String> = asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter_map(|line| line.split_whitespace().nth(1))
        .filter_map(|path| path.strip_prefix("/api/"))
        .map(|path| path.split('?').next().unwrap_or_default().to_owned())
        .collect();
    assert!(
        methods
            .iter()
            .all(|method| ["search.messages", "conversations.replies"].contains(&method.as_str())),
        "only reading methods are asked: {methods:?}"
    );

    let mut again = reader(&home, &origin);
    let rounds = asked.lock().expect("what was asked").len();
    let began = Instant::now();
    while asked.lock().expect("what was asked").len() < rounds + 6 {
        assert!(
            began.elapsed() < PATIENCE,
            "the restarted reader asked nothing"
        );
        // As above, a short wait sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    again.kill().expect("the reader stops");
    again.wait().expect("the reader is gone");
    assert_eq!(decisions(&home).len(), 2, "a restart judges nothing twice");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

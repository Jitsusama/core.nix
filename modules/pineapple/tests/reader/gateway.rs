//! The reader hearing Slack through a tool gateway, against a fake one on
//! a port of this machine's own: a stop heard there is judged and kept in
//! shadow as one heard from Slack itself would be, judged once however
//! many rounds find it; the gateway is asked only to read; Slack's own API
//! is never asked, nor Joel's phone told; a gateway that can't be signed
//! in to is told in For You; and the reader refuses to go live over one.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    JOEL, PATIENCE, decisions, each, home, reader_with, reading, stand_in, the_one_reply,
    until_decided,
};

/// The thread the stop is said in, which starts with a pull request.
const THREAD: &str = "1700000000.000001";

/// A fake gateway reading Joel's Slack: who he is, a search finding one
/// stop in a thread whichever it's asked for, and that thread's first
/// message naming a pull request; with its URL and the tools it was asked
/// for, in order.
fn gateway() -> (String, Arc<Mutex<Vec<String>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a port");
    let url = format!("http://{}/mcp", listener.local_addr().expect("its address"));
    let called = Arc::new(Mutex::new(Vec::new()));
    let kept = Arc::clone(&called);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_secs();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("the stand-in gateway takes each connection");
            let mut reader = BufReader::new(stream.try_clone().expect("a stream"));
            let mut length = 0;
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).unwrap_or(0) == 0 || line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':')
                    && name.eq_ignore_ascii_case("content-length")
                {
                    length = value.trim().parse().unwrap_or(0);
                }
            }
            let mut body = vec![0; length];
            if reader.read_exact(&mut body).is_err() {
                continue;
            }
            let request: Value = serde_json::from_slice(&body).unwrap_or_default();
            let reply = match request["method"].as_str().unwrap_or_default() {
                "initialize" => Some(json!({ "protocolVersion": "x" })),
                "tools/call" => {
                    let tool = request["params"]["name"].as_str().unwrap_or_default();
                    kept.lock().expect("what was called").push(tool.to_owned());
                    Some(json!({ "content": [{ "type": "text", "text": said(tool, now) }] }))
                }
                _ => None,
            };
            let (status, body) = reply.map_or((202, String::new()), |result| {
                (
                    200,
                    json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }).to_string(),
                )
            });
            // A reader killed mid-answer leaves nobody to write to.
            let _ = write!(
                stream,
                "HTTP/1.1 {status} X\r\nContent-Type: application/json\r\n\
                 Mcp-Session-Id: one\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
        }
    });
    (url, called)
}

/// What the fake gateway's `tool` says, wrapped as the gateway wraps it,
/// a minute after a stop was said at `now`.
fn said(tool: &str, now: u64) -> String {
    let ts = format!("{}.000100", now - 60);
    let said = match tool {
        "slack_who_am_i" => json!({ "result": format!("User ID: {JOEL}\nUsername: joel\n") }),
        "slack_search_public_and_private" => json!({ "results": format!(
            "# Search Results for: \n\n## Messages (1 result)\n### Result 1 of 1\n\
             Channel: #team (ID: C0ONE00001)\n\
             From: Someone Else <someone@example.com> (ID: U0OTHER001) \n\
             Time: today\nMessage_ts: {ts}\n\
             Permalink: [link](https://example.slack.com/archives/C0ONE00001/p{}\
             ?thread_ts={THREAD}&cid=C0ONE00001)\n\
             Text: \n<@{JOEL}> pineapple override\n\n---\n",
            ts.replace('.', "")
        ) }),
        "slack_read_thread" => json!({ "messages": format!(
            "=== THREAD PARENT MESSAGE ===\nFrom: Someone (U0OTHER001)\nTime: today\n\
             Message TS: {THREAD}\n\
             a review, please: https://code.example.com/example/shop/pull/12\n\n\
             === THREAD REPLIES (0 total) ===\n"
        ) }),
        _ => json!({}),
    };
    said.to_string()
}

/// A sign-in program in `home` that prints a token, or fails when
/// `fails`, as one with no sign-in to give does.
fn sign_in(home: &Path, fails: bool) -> PathBuf {
    let program = home.join("sign-in");
    let script = if fails {
        "#!/bin/sh\nexit 1\n"
    } else {
        "#!/bin/sh\necho tok\n"
    };
    std::fs::write(&program, script).expect("a sign-in program");
    let made = Command::new("chmod")
        .arg("+x")
        .arg(&program)
        .status()
        .expect("chmod runs");
    assert!(made.success(), "the sign-in program can be run: {made}");
    program
}

#[test]
fn a_stop_heard_through_a_gateway_is_judged_once_in_shadow_and_only_read() {
    let (origin, asked) = stand_in();
    let (url, called) = gateway();
    let home = home();
    let program = sign_in(&home, false);
    let program = program.to_str().expect("a path in UTF-8");
    let mut reader = reader_with(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
    );
    until_decided(&home, 1);
    // Rounds go on finding the same stop, which none judges again.
    let rounds = called.lock().expect("what was called").len();
    let began = Instant::now();
    while called.lock().expect("what was called").len() < rounds + 6 {
        assert!(began.elapsed() < PATIENCE, "the reader stopped asking");
        // It reads every tenth of a second, so this sees each round.
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(each(&said, "verdict"), ["stop"], "the stop, judged once");
    assert_eq!(
        said[0]["reaches"],
        json!(["pr:example/shop#12"]),
        "the thread's pull request is what stops"
    );
    assert_eq!(said[0]["shadow"], true, "it is in shadow");
    assert_eq!(
        the_one_reply(&home)["post"]["thread_ts"],
        THREAD,
        "the reply it would say is kept, for the thread that asked"
    );
    let called = called.lock().expect("what was called").clone();
    assert_eq!(
        called
            .iter()
            .filter(|tool| *tool == "slack_who_am_i")
            .count(),
        1,
        "whose Slack it reads is asked once: {called:?}"
    );
    let reading = [
        "slack_who_am_i",
        "slack_search_public_and_private",
        "slack_read_thread",
    ];
    assert!(
        called.iter().all(|tool| reading.contains(&tool.as_str())),
        "only tools that post nothing are asked: {called:?}"
    );
    assert!(
        asked.lock().expect("what was asked").is_empty(),
        "Slack's own API is never asked, so no phone is told"
    );
    assert!(
        !home.join("state/pineapple/phoned.jsonl").exists(),
        "nor is a phone alert written down"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_gateway_with_no_sign_in_to_give_is_told_in_for_you() {
    let (origin, _) = stand_in();
    let (url, called) = gateway();
    let home = home();
    let program = sign_in(&home, true);
    let program = program.to_str().expect("a path in UTF-8");
    let waiting = home.join("waiting.json");
    let mut reader = reading(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
        Some(&waiting),
    );
    let began = Instant::now();
    while !waiting.exists() && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let row: Value = std::fs::read_to_string(&waiting)
        .ok()
        .and_then(|said| serde_json::from_str(&said).ok())
        .unwrap_or_default();
    assert!(
        row["items"][0]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("pineapple-slack-deaf-")),
        "Joel is told the safe word can't hear Slack: {row}"
    );
    assert!(
        called.lock().expect("what was called").is_empty(),
        "no tool is called without a sign-in"
    );
    assert!(decisions(&home).is_empty(), "and nothing is judged");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What `pineapple-slack run` says, and how it ends, given `args`.
fn ran(args: &[&str]) -> (Option<i32>, String) {
    let ran = Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
        .arg("run")
        .args(args)
        .output()
        .expect("pineapple-slack runs");
    (
        ran.status.code(),
        String::from_utf8_lossy(&ran.stderr).into_owned(),
    )
}

#[test]
fn a_reader_over_a_gateway_is_never_live_and_needs_its_sign_in() {
    let url = "http://127.0.0.1:9/mcp";
    let (code, said) = ran(&["--live", "--gateway", url, "--gateway-sign-in", "x"]);
    assert_eq!(
        code,
        Some(64),
        "live over a gateway is asked wrongly: {said}"
    );
    assert!(
        said.contains("--live isn't taken with --gateway"),
        "it says why: {said}"
    );
    for half in [["--gateway", url], ["--gateway-sign-in", "x"]] {
        let (code, said) = ran(&half);
        assert_eq!(code, Some(64), "{half:?} alone is asked wrongly: {said}");
        assert!(
            said.contains("--gateway and --gateway-sign-in go together"),
            "{half:?} alone says the two go together: {said}"
        );
    }
}

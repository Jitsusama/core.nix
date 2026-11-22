//! The reader hearing Slack through a tool gateway, against a fake one on
//! a port of this machine's own: a stop heard there is judged and kept in
//! shadow as one heard from Slack itself would be, judged once however
//! many rounds find it, in a folder of its own; the gateway is asked only
//! to read, both searches in their detailed form; Slack's own API is never
//! asked, nor Joel's phone told; a gateway that can't be signed in to
//! writes its row of what waits in its own folder; and the reader refuses
//! to go live over one.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    JOEL, PATIENCE, decisions, decisions_in, each, home, reader_with, reading, stand_in,
    the_one_reply_in, until_decided_in,
};

/// Where the reader over a gateway keeps all it writes, under its home.
const OWN: &str = "state/pineapple-gateway";

/// What the fake gateway was asked: each tool, with its arguments.
type Called = Arc<Mutex<Vec<(String, Value)>>>;

/// The tools `called` names, in order.
fn tools(called: &Called) -> Vec<String> {
    let called = called.lock().expect("what was called");
    called.iter().map(|(tool, _)| tool.clone()).collect()
}

/// The thread the stop is said in, which starts with a pull request.
const THREAD: &str = "1700000000.000001";

/// A fake gateway reading Joel's Slack: who he is, a search finding one
/// stop in a thread whichever it's asked for, and that thread's first
/// message naming a pull request; with its URL and the tools it was asked
/// for, in order, with what each was asked.
fn gateway() -> (String, Called) {
    serving(|tool, _, now| said(tool, now))
}

/// A fake gateway answering each tool, given what it was asked, as
/// `answer` says, given when a minute ago was; with its URL and what it
/// was asked, as above.
fn serving(answer: fn(&str, &Value, u64) -> String) -> (String, Called) {
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
                    let asked = request["params"]["arguments"].clone();
                    let text = answer(tool, &asked, now);
                    kept.lock()
                        .expect("what was called")
                        .push((tool.to_owned(), asked));
                    Some(json!({ "content": [{ "type": "text", "text": text }] }))
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
            "# Search Results for: \n\n## Messages (1 results)\n### Result 1 of 1\n\
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

/// A fresh alert for the phone, where either reader would find one, so a
/// reader telling the phone would leave its mark; with how many files the
/// live reader's folder then holds.
fn an_alert_waits(home: &Path) -> usize {
    let at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    let alert = json!({"at": u64::try_from(at).expect("a time in range"),
        "key": "pineapple:slack:C1:1.0", "saying": "stopped", "shadow": false,
        "title": "Someone stopped your agents", "words": "pineapple override",
        "link": "https://example.slack.com/archives/C1/p1"});
    for folder in ["state/pineapple", OWN] {
        std::fs::create_dir_all(home.join(folder)).expect("a state folder");
        std::fs::write(
            home.join(folder).join("would-tell.jsonl"),
            format!("{alert}\n"),
        )
        .expect("what the launcher's daemon would tell");
    }
    std::fs::read_dir(home.join("state/pineapple"))
        .expect("the live reader's folder")
        .count()
}

/// That nothing a reader in shadow did reached past its own folder: no
/// phone told, by Slack's own API or by the alert waiting, and nothing
/// written where the live reader keeps its own, which held `live` files.
fn nothing_left_the_shadow(home: &Path, asked: &Arc<Mutex<Vec<String>>>, live: usize) {
    assert!(
        asked.lock().expect("what was asked").is_empty(),
        "Slack's own API is never asked, so no phone is told"
    );
    for folder in ["state/pineapple", OWN] {
        assert!(
            !home.join(folder).join("phoned.jsonl").exists(),
            "nor is a phone told the alert waiting in {folder}"
        );
    }
    assert_eq!(
        std::fs::read_dir(home.join("state/pineapple"))
            .expect("the live reader's folder")
            .count(),
        live,
        "nothing is written where the live reader keeps its own"
    );
    assert!(
        decisions(home).is_empty(),
        "and no decision is written there"
    );
}

#[test]
fn a_stop_heard_through_a_gateway_is_judged_once_in_shadow_and_only_read() {
    let (origin, asked) = stand_in();
    let (url, called) = gateway();
    let home = home();
    let program = sign_in(&home, false);
    let program = program.to_str().expect("a path in UTF-8");
    let live = an_alert_waits(&home);
    let mut reader = reader_with(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
    );
    let own = home.join(OWN);
    until_decided_in(&own, 1);
    // Rounds go on finding the same stop, which none judges again.
    let rounds = tools(&called).len();
    let began = Instant::now();
    while tools(&called).len() < rounds + 6 {
        assert!(began.elapsed() < PATIENCE, "the reader stopped asking");
        // It reads every tenth of a second, so this sees each round.
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let said = decisions_in(&own);
    assert_eq!(each(&said, "verdict"), ["stop"], "the stop, judged once");
    assert_eq!(
        said[0]["reaches"],
        json!(["pr:example/shop#12"]),
        "the thread's pull request is what stops"
    );
    assert_eq!(said[0]["shadow"], true, "it is in shadow");
    assert_eq!(
        the_one_reply_in(&own)["post"]["thread_ts"],
        THREAD,
        "the reply it would say is kept, for the thread that asked"
    );
    let searched: Vec<Value> = called
        .lock()
        .expect("what was called")
        .iter()
        .filter(|(tool, _)| tool == "slack_search_public_and_private")
        .map(|(_, asked)| asked.clone())
        .take(2)
        .collect();
    let tag = format!("<@{JOEL}>");
    assert_eq!(
        each(&searched, "query"),
        ["pineapple", tag.as_str()],
        "a round searches for the safe word, then for Joel tagged"
    );
    assert!(
        searched
            .iter()
            .all(|asked| asked["response_format"] == "detailed"),
        "each search asks for the detailed form it is read in: {searched:?}"
    );
    let called = tools(&called);
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
    nothing_left_the_shadow(&home, &asked, live);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_gateway_with_no_sign_in_to_give_writes_its_row_in_its_own_folder() {
    let (origin, _) = stand_in();
    let (url, called) = gateway();
    let home = home();
    let program = sign_in(&home, true);
    let program = program.to_str().expect("a path in UTF-8");
    // The live reader's row, which this reader must never write.
    let named = home.join("named-waiting.json");
    let waiting = home.join(OWN).join("waiting.json");
    let mut reader = reading(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
        Some(&named),
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
        "its row says the safe word can't hear Slack: {row}"
    );
    assert!(
        row["items"][0]["why"]
            .as_str()
            .is_some_and(|why| why.contains("no Slack sign-in it can read")),
        "for want of a sign-in, not a refused one: {row}"
    );
    assert!(
        !named.exists(),
        "the row the environment names is the live reader's, so it's never written"
    );
    assert!(
        tools(&called).is_empty(),
        "no tool is called without a sign-in"
    );
    assert!(decisions(&home).is_empty(), "and nothing is judged");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_gateway_whose_search_cant_be_read_writes_its_row_in_its_own_folder() {
    let (origin, _) = stand_in();
    // The search for Joel's tag counts two results and gives one, as one
    // would whose wording changed under the reader; the phrase's reads.
    let (url, called) = serving(|tool, asked, now| {
        if tool == "slack_search_public_and_private" && asked["query"] != "pineapple" {
            json!({ "results": "## Messages (2 results)\n### Result 1 of 2\n\
                Channel: #team (ID: C1)\nFrom: Someone (ID: U2)\n\
                Message_ts: 1.000100\nText: \nhi\n\n---\n" })
            .to_string()
        } else {
            said(tool, now)
        }
    });
    let home = home();
    let program = sign_in(&home, false);
    let program = program.to_str().expect("a path in UTF-8");
    let named = home.join("named-waiting.json");
    let waiting = home.join(OWN).join("waiting.json");
    let mut reader = reading(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
        Some(&named),
    );
    let began = Instant::now();
    while !waiting.exists() && began.elapsed() < PATIENCE {
        // The reader goes round every tenth of a second in a case.
        thread::sleep(Duration::from_millis(20));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let row: Value = std::fs::read_to_string(&waiting)
        .ok()
        .and_then(|said| serde_json::from_str(&said).ok())
        .unwrap_or_default();
    assert!(
        row["items"][0]["why"]
            .as_str()
            .is_some_and(|why| why.contains("can't be read")),
        "answers it can't read are told, not taken for a quiet Slack: {row}"
    );
    assert!(!named.exists(), "the live reader's row is never written");
    assert!(
        tools(&called).contains(&"slack_search_public_and_private".to_owned()),
        "it did ask: {:?}",
        tools(&called)
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

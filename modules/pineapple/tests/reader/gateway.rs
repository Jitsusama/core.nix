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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    JOEL, PATIENCE, decisions, decisions_in, each, home, journal, reader_with, reading, reading_in,
    stand_in, the_one_reply_in, until_decided_in,
};

mod cut;
mod pages;
mod unread;

/// Where the reader over a gateway keeps all it writes, under its home.
const OWN: &str = "state/pineapple-gateway";

/// How an answer from the fake gateway's tool starts when the tool
/// failed, as the gateway says one Slack refused.
const FAILS: &str = "failed: ";

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
                    let (failed, text) = text
                        .strip_prefix(FAILS)
                        .map_or((false, text.as_str()), |why| (true, why));
                    Some(json!({
                        "content": [{ "type": "text", "text": text }],
                        "isError": failed,
                    }))
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
    assert!(
        decisions_in(&home.join(OWN)).is_empty(),
        "and nothing is judged"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_gateway_reader_given_a_relative_state_home_keeps_to_its_own_row() {
    let (origin, _) = stand_in();
    let (url, _) = gateway();
    let home = home();
    let program = sign_in(&home, true);
    let program = program.to_str().expect("a path in UTF-8");
    let waiting = home.join(OWN).join("waiting.json");
    let mut reader = reading_in(
        &home,
        Path::new("state"),
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
        None,
    );
    let began = Instant::now();
    while !waiting.exists() && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(
        waiting.exists(),
        "a state home named from where it runs still holds its own row"
    );
    for for_you in ["state/mission", ".local/state/mission"] {
        assert!(
            !home.join(for_you).exists(),
            "For You's own row, {for_you}, is the live reader's, so it's never written"
        );
    }
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// A search's answer counting one result and giving two, as a message
/// typing a result into its text leaves it, whatever it's asked for.
fn poisoned() -> String {
    json!({ "results": "## Messages (1 results)\n### Result 1 of 1\n\
        Channel: #team (ID: C1)\nFrom: Someone (ID: U2)\n\
        Message_ts: 1.000100\nText: \nhi\n\n---\n\n### Result 2 of 1\n\
        Channel: #team (ID: C1)\nFrom: Someone (ID: U2)\n\
        Message_ts: 1.000200\nText: \nhi\n\n---\n" })
    .to_string()
}

/// When Slack was last read whole, as a case keeps it before the reader
/// starts, in milliseconds.
const READ_WHOLE: u64 = 1_700_000_000_000;

/// What a reader over a fake gateway is waited on for: its row told,
/// `judging` messages judged and `asks` searches asked, from a judged.json
/// saying Slack was last read whole at `read_at`, if it says so.
struct Until {
    judging: usize,
    asks: usize,
    read_at: Option<u64>,
}

/// What a reader over a fake gateway came to: the row's why, the
/// decisions in its folder, when its judged.json says Slack was last read
/// whole and how many messages it keeps as judged, the searches it asked,
/// and whether it wrote the live reader's row.
struct Came {
    why: String,
    said: Vec<Value>,
    read_at: Option<u64>,
    keys: usize,
    asked: Vec<Value>,
    /// Each search asked, with how many it asked for.
    limits: Vec<(Value, Value)>,
    /// What each search was asked with, whole.
    searched: Vec<Value>,
    journal: String,
    wrote_live: bool,
    /// Its rows saying what a search read on past was never read.
    missed: Value,
    /// Each stop's record it kept, by its file's name.
    stops: Vec<Value>,
}

/// Each stop's record kept in `folder`, by its file's name.
fn kept_stops(folder: &Path) -> Vec<Value> {
    let mut named: Vec<_> = std::fs::read_dir(folder)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|it| it == "json"))
        .collect();
    named.sort();
    named
        .iter()
        .filter_map(|path| std::fs::read_to_string(path).ok())
        .filter_map(|record| serde_json::from_str(&record).ok())
        .collect()
}

fn hearing(answer: fn(&str, &Value, u64) -> String, until: &Until) -> Came {
    hearing_in(answer, until, |_| {})
}

/// What a reader over a fake gateway came to, as [`hearing`] says, from
/// its own folder as `before` leaves it.
fn hearing_in(answer: fn(&str, &Value, u64) -> String, until: &Until, before: fn(&Path)) -> Came {
    let (origin, _) = stand_in();
    let (url, called) = serving(answer);
    let home = home();
    let program = sign_in(&home, false);
    let program = program.to_str().expect("a path in UTF-8");
    let named = home.join("named-waiting.json");
    let own = home.join(OWN);
    let waiting = own.join("waiting.json");
    if let Some(read_at) = until.read_at {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs();
        std::fs::create_dir_all(&own).expect("its folder");
        let kept =
            json!({ "since": format!("{}.000000", now - 600), "keys": [], "read_at": read_at });
        std::fs::write(own.join("judged.json"), kept.to_string()).expect("what it judged");
    }
    before(&own);
    let searches = || {
        called
            .lock()
            .expect("what was called")
            .iter()
            .filter(|(tool, _)| tool == "slack_search_public_and_private")
            .map(|(_, asked)| asked["query"].clone())
            .collect::<Vec<_>>()
    };
    let mut reader = reading(
        &home,
        &origin,
        &["--gateway", &url, "--gateway-sign-in", program],
        Some(&named),
    );
    let began = Instant::now();
    // A row told may be taken away again once Slack is heard, so it's
    // waited on as having been told.
    let mut told = false;
    while !(told && decisions_in(&own).len() >= until.judging && searches().len() >= until.asks)
        && began.elapsed() < PATIENCE
    {
        told |= waiting.exists();
        // The reader goes round every tenth of a second in a case.
        thread::sleep(Duration::from_millis(20));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let read = |file: &str| -> Value {
        std::fs::read_to_string(own.join(file))
            .ok()
            .and_then(|said| serde_json::from_str(&said).ok())
            .unwrap_or_default()
    };
    // Its own statement, as a lock taken in the one below would be held to
    // its end.
    let asks: Vec<Value> = called
        .lock()
        .expect("what was called")
        .iter()
        .filter(|(tool, _)| tool == "slack_search_public_and_private")
        .map(|(_, asked)| asked.clone())
        .collect();
    let came = Came {
        why: read("waiting.json")["items"][0]["why"]
            .as_str()
            .unwrap_or_default()
            .to_owned(),
        said: decisions_in(&own),
        read_at: read("judged.json")["read_at"].as_u64(),
        keys: read("judged.json")["keys"].as_array().map_or(0, Vec::len),
        asked: searches(),
        limits: asks
            .iter()
            .map(|asked| (asked["query"].clone(), asked["limit"].clone()))
            .collect(),
        searched: asks,
        journal: journal(&home),
        wrote_live: named.exists(),
        missed: read("missed.json"),
        stops: kept_stops(&own.join("stops")),
    };
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    came
}

#[test]
fn a_tag_search_that_cant_be_read_is_told_while_the_phrase_is_judged() {
    let came = hearing(
        |tool, asked, now| {
            if tool == "slack_search_public_and_private" && asked["query"] != "pineapple" {
                poisoned()
            } else {
                said(tool, now)
            }
        },
        &Until {
            judging: 1,
            asks: 0,
            read_at: Some(READ_WHOLE),
        },
    );
    assert!(
        came.why
            .contains("search of Slack for your tag can't be read"),
        "its row names the search it can't read: {}",
        came.why
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["stop"],
        "the stop the phrase's search found is judged, once"
    );
    assert_eq!(
        (came.read_at, came.keys),
        (Some(READ_WHOLE), 1),
        "Slack wasn't read whole, so its last whole read stays, beside the stop judged"
    );
    assert!(!came.wrote_live, "the live reader's row is never written");
}

#[test]
fn a_phrase_search_that_cant_be_read_is_told_while_the_tag_is_judged() {
    let came = hearing(
        |tool, asked, now| {
            if tool == "slack_search_public_and_private" && asked["query"] == "pineapple" {
                poisoned()
            } else {
                said(tool, now)
            }
        },
        &Until {
            judging: 1,
            asks: 0,
            read_at: None,
        },
    );
    assert!(
        came.asked.contains(&json!(format!("<@{JOEL}>"))),
        "the tag's search is asked though the phrase's couldn't be read: {:?}",
        came.asked
    );
    assert!(
        came.why
            .contains("search of Slack for pineapple can't be read"),
        "its row names the search it can't read: {}",
        came.why
    );
    assert_eq!(
        each(&came.said, "verdict"),
        ["stop"],
        "the stop the tag's search found is judged, once"
    );
    assert_eq!(
        (came.read_at, came.keys),
        (None, 1),
        "Slack was never read whole, so none says it was"
    );
}

#[test]
fn a_search_slack_refuses_stops_the_round_though_the_other_reads() {
    let came = hearing(
        |tool, asked, now| {
            if tool == "slack_search_public_and_private" && asked["query"] != "pineapple" {
                format!("{FAILS}Slack said: invalid_auth")
            } else {
                said(tool, now)
            }
        },
        &Until {
            judging: 0,
            asks: 0,
            read_at: None,
        },
    );
    assert!(
        came.why
            .starts_with("Slack refused the safe word's sign-in"),
        "a refusal is told as one, not as a search that can't be read: {}",
        came.why
    );
    assert!(came.said.is_empty(), "and the round judges nothing");
}

#[test]
fn a_search_that_blinks_beside_one_that_cant_be_read_isnt_told_as_both() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                poisoned()
            } else if TAGS.fetch_add(1, Ordering::SeqCst) < 2 {
                said(tool, now)
            } else {
                format!("{FAILS}Slack said: ratelimited")
            }
        },
        &Until {
            judging: 1,
            asks: 12,
            read_at: None,
        },
    );
    assert!(
        came.why
            .contains("search of Slack for pineapple can't be read"),
        "rounds whose tag search wasn't asked leave the row as it was: {}",
        came.why
    );
}

#[test]
fn a_first_page_blinking_beside_a_search_that_cant_be_read_leaves_the_row_as_it_was() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    static BLINKS: AtomicUsize = AtomicUsize::new(0);
    let came = hearing(
        |tool, asked, now| {
            if tool != "slack_search_public_and_private" {
                said(tool, now)
            } else if asked["query"] == "pineapple" {
                poisoned()
            } else if TAGS.fetch_add(1, Ordering::SeqCst) < 3 {
                // Both can't be read until the row says so, then the
                // tag's first page blinks in every round.
                poisoned()
            } else {
                if cut::opens(asked) {
                    BLINKS.fetch_add(1, Ordering::SeqCst);
                }
                format!("{FAILS}Slack said: ratelimited")
            }
        },
        // Told, then rounds enough to tell it anew.
        &Until {
            judging: 0,
            asks: 30,
            read_at: None,
        },
    );
    // A blink heard narrows the row in its second round, so four blinks
    // leave it room to.
    assert!(
        BLINKS.load(Ordering::SeqCst) >= 4,
        "four rounds blinked after the row said both: {}",
        came.journal
    );
    assert!(
        came.why
            .contains("Slack's answers to the safe word can't be read"),
        "a search whose first page blinks, waiting on no later page, isn't heard, so the \
         row still says both can't be read: {}",
        came.why
    );
}

#[test]
fn searches_that_cant_be_read_judge_nothing_and_are_told() {
    let came = hearing(
        |tool, _, now| {
            if tool == "slack_search_public_and_private" {
                poisoned()
            } else {
                said(tool, now)
            }
        },
        &Until {
            judging: 0,
            asks: 8,
            read_at: None,
        },
    );
    assert!(
        came.why
            .starts_with("Slack's answers to the safe word can't be read"),
        "answers it can't read are told, not taken for a quiet Slack: {}",
        came.why
    );
    assert!(came.said.is_empty(), "and nothing is judged");
    assert!(
        came.asked.len() >= 8 && came.why.starts_with("Slack's answers"),
        "it goes on asking both, and still tells them as both: {:?}",
        came.asked
    );
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

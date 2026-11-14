//! pineapple-slack let go live, against the same stand-in: its stops
//! stand, the one reply the rules let through is posted in the thread that
//! asked, an alert goes to Joel's own IM once, and his Slack resume still
//! ends no live stop until Slack resumes are let go live.

use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    PATIENCE, a_stop, answer, answering, decisions, each, home, joel_resumes, reader_with, resumed,
    the_one_reply, until_decided,
};

/// What Slack would answer with the reader let go live: a post taken, and
/// everything else as `answer` says.
fn live(line: &str, now: u64) -> Value {
    if line.starts_with("POST /api/chat.postMessage ") {
        return json!({"ok": true, "ts": "1800000000.000001"});
    }
    answer(line, now)
}

/// As `live`, but with nothing said in Slack for a search to find.
fn quiet_live(line: &str, now: u64) -> Value {
    if line.contains("/api/search.messages") {
        return json!({"ok": true, "messages": {"matches": []}});
    }
    live(line, now)
}

/// The forms posted to the stand-in, as Slack would read them.
fn posted(asked: &Mutex<Vec<String>>) -> Vec<Value> {
    asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter(|line| line.starts_with("POST /api/chat.postMessage "))
        .filter_map(|line| line.split_once(" HTTP/1.1 "))
        .map(|(_, form)| {
            let fields = form
                .split('&')
                .filter_map(|pair| pair.split_once('='))
                .map(|(name, value)| (decoded(name), Value::String(decoded(value))));
            Value::Object(fields.collect())
        })
        .collect()
}

/// A form's field as it was before it was encoded.
fn decoded(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let hex = bytes
            .get(at + 1..at + 3)
            .and_then(|hex| std::str::from_utf8(hex).ok())
            .and_then(|hex| u8::from_str_radix(hex, 16).ok());
        match (byte, hex) {
            (b'%', Some(byte)) => {
                out.push(byte);
                at += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                at += 1;
            }
            (byte, _) => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Waits until `path` is there, or says what never came.
fn until_there(path: &Path, what: &str) {
    let began = Instant::now();
    while !path.exists() {
        assert!(began.elapsed() < PATIENCE, "{what}");
        // The reader goes round every tenth of a second.
        thread::sleep(Duration::from_millis(50));
    }
    // Another round, to be sure what it waited for happens only once.
    thread::sleep(Duration::from_millis(300));
}

#[test]
fn let_go_live_its_stops_stand_and_one_reply_is_posted_in_the_thread_that_asked() {
    let (origin, asked) = answering(live);
    let home = home();
    let mut reading = reader_with(&home, &origin, &["--live"]);
    until_decided(&home, 4);
    until_there(
        &home.join("state/pineapple/replies.jsonl"),
        "no reply was said",
    );
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "stop", "stop-asked", "stop"],
        "judged as in shadow"
    );
    assert_eq!(said[1]["shadow"], false, "it isn't in shadow");
    let stops: Vec<Value> = std::fs::read_dir(home.join("state/pineapple/stops"))
        .expect("the stops folder")
        .filter_map(|entry| std::fs::read_to_string(entry.ok()?.path()).ok())
        .filter_map(|record| serde_json::from_str(&record).ok())
        .collect();
    assert_eq!(
        stops.iter().map(|stop| &stop["shadow"]).collect::<Vec<_>>(),
        [&json!(false); 3],
        "each stop stands for real"
    );
    assert_eq!(
        posted(&asked),
        [json!({
            "channel": "C0ONE00001",
            "thread_ts": "1700000000.000001",
            "text": "Automated reply, not Joel: his agents have stopped work on pull request \
                     example/shop#12. Only he can restart them, and he's been told.",
            "unfurl_links": "false",
            "unfurl_media": "false",
        })],
        "one reply, in Joel's words, in the thread that asked"
    );
    assert_eq!(the_one_reply(&home)["shadow"], false, "kept as said");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn let_go_live_an_alert_goes_to_joels_own_im_once() {
    let (origin, asked) = answering(quiet_live);
    let home = home();
    let at = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_millis(),
    )
    .expect("a time in range");
    let alert = json!({"at": at, "key": "pineapple:slack:C1:1.0", "saying": "stopped",
        "shadow": false, "title": "Someone stopped your agents on quest SAFEQA",
        "words": "pineapple override", "link": "https://example.slack.com/archives/C1/p1"});
    std::fs::write(
        home.join("state/pineapple/would-tell.jsonl"),
        format!("{alert}\n"),
    )
    .expect("what the launcher's daemon would tell");
    let phoned = home.join("state/pineapple/phoned.jsonl");
    let mut reading = reader_with(&home, &origin, &["--live"]);
    until_there(&phoned, "Joel's phone was never told");
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let kept = std::fs::read_to_string(&phoned).expect("what Joel's phone was told");
    let lines: Vec<Value> = kept
        .lines()
        .map(|line| serde_json::from_str(line).expect("a line is JSON"))
        .collect();
    assert_eq!(
        lines
            .iter()
            .map(|line| (&line["im"], &line["shadow"], &line["sent"]))
            .collect::<Vec<_>>(),
        [(&json!("D0SELF0001"), &json!(false), &json!(true))],
        "told once, in Joel's own IM: {kept}"
    );
    assert_eq!(
        posted(&asked),
        [json!({
            "channel": "D0SELF0001",
            "text": "pineapple: Someone stopped your agents on quest SAFEQA\n\
                     Said: \"pineapple override\"\n\
                     Where: https://example.slack.com/archives/C1/p1",
            "unfurl_links": "false",
            "unfurl_media": "false",
        })],
        "the one post is the alert's one form, in Joel's own IM"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn let_go_live_joels_slack_resume_still_ends_no_live_stop() {
    let (origin, asked) = answering(joel_resumes);
    let home = home();
    a_stop(&home, "named", "pr:example/shop#12", true, 120);
    a_stop(&home, "live", "pr:example/shop#12", false, 120);
    let mut reading = reader_with(&home, &origin, &["--live"]);
    until_decided(&home, 1);
    let began = Instant::now();
    while resumed(&home, "named").is_null() {
        assert!(began.elapsed() < PATIENCE, "Joel's resume ended nothing");
        // As above, a short wait sees it land without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        resumed(&home, "live"),
        Value::Null,
        "until Slack resumes are let go live, a live stop waits for Joel's press"
    );
    assert!(posted(&asked).is_empty(), "nobody is answered for a resume");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

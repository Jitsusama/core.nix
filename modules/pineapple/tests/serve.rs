//! pineapple serve, as the launcher asks it: a live stop and a live
//! alert are served as rows titled in the vocabulary's words, a shadow
//! stop isn't and is only written down as what it would have told, and
//! a press from anything but Joel's shell, as this test is, is refused
//! and changes nothing.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use daemon_client::{Daemon, until};
use serde_json::{Value, json};

fn state() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("pineapple-serve-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let kept = dir.join("pineapple");
    fs::create_dir_all(kept.join("stops")).expect("a scratch state folder");
    let stop = |id: &str, file: &str, shadow: bool| {
        let record = json!({"id": id, "shadow": shadow, "reaches": ["pr:example/shop#12"],
            "fell_back": false, "heard_at": 10, "by_joel": false, "said_by": "U0OTHER001",
            "where": "https://example.slack.com/archives/C1/p1", "words": "pineapple override",
            "resumed": null});
        fs::write(kept.join("stops").join(file), record.to_string()).expect("a stop is written");
    };
    stop("slack:C1:1.0", "live.json", false);
    stop("slack:C2:1.0", "shadow.json", true);
    let joke = json!({"key": "slack:C3:2.0", "thread": "slack:C3:1.0", "verdict": "waved:joke",
        "heard_at": 4_102_444_800_000_u64, "by_joel": false, "shadow": false,
        "text": "pineapple override jk", "link": "https://example.slack.com/archives/C3/p2",
        "author": "U0OTHER001", "reaches": [], "would_reach": ["everything"],
        "would_fall_back": true});
    fs::write(kept.join("decisions.jsonl"), format!("{joke}\n")).expect("a decision is written");
    let vocabulary = json!({"things": [{"id": "for-you:source:pineapple", "said": {
        "someone": "someone", "stopped": "{who} stopped agents on {reach}",
        "joke": "{who} joked", "reach:pr": "pull request {repo}#{number}"}}]});
    fs::write(dir.join("words.json"), vocabulary.to_string()).expect("words are written");
    dir
}

fn keys(answer: &Value) -> Vec<&str> {
    answer["records"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|record| record["key"].as_str())
        .collect()
}

#[test]
fn live_rows_are_served_shadow_ones_written_down_and_a_stranger_refused() {
    let dir = state();
    let socket = dir.join("pineapple.sock");
    let mut command = Command::new(env!("CARGO_BIN_EXE_pineapple"));
    command
        .args(["serve", "--socket"])
        .arg(&socket)
        .arg("--vocabulary")
        .arg(dir.join("words.json"))
        .args(["--every", "50"])
        .env("XDG_STATE_HOME", &dir);
    let daemon = Daemon::start(&mut command, socket, dir.clone());
    let mut client = daemon.client();
    let would_tell = dir.join("pineapple/would-tell.jsonl");
    until("the shadow stop was never written down", || {
        fs::read_to_string(&would_tell).is_ok_and(|text| text.contains("slack:C2:1.0"))
    });
    let listed = client.ask(&json!({"id": 1, "ask": "list", "kind": "pineapple"}));
    assert_eq!(
        keys(&listed),
        ["pineapple:slack:C1:1.0", "pineapple:joke:slack:C3:1.0"],
        "the live stop and the live joke, oldest first, and no shadow stop: {listed}"
    );
    assert_eq!(
        listed["records"][0]["title"], "someone stopped agents on pull request example/shop#12",
        "titled in the vocabulary's words"
    );
    let refused = client.ask(&json!({"id": 2, "ask": "resume", "key": "pineapple:slack:C1:1.0"}));
    let code = refused["error"]["code"].as_str().unwrap_or_default();
    assert!(
        code == "not-joel" || code == "no-peer",
        "a press from a test isn't Joel's: {refused}"
    );
    let record: Value = serde_json::from_str(
        &fs::read_to_string(dir.join("pineapple/stops/live.json")).expect("kept"),
    )
    .expect("json");
    assert!(record["resumed"].is_null(), "and the stop still stands");
    drop(daemon);
}

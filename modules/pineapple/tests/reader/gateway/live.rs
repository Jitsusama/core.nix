//! The reader over a gateway, live, in the live reader's place: a stop it
//! hears stands in the live reader's folder, where the agents' guard reads;
//! Joel's phone is told once, through the gateway's one tool that sends to
//! him alone; no reply is said and Slack's own API is never asked; and a
//! search it reads on past is said in a row beside his row in For You.

use std::path::Path;
use std::sync::atomic::AtomicUsize;
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::cut::{a_gap, nothing};
use super::{
    OWN, PATIENCE, an_alert_waits, decisions_in, each, gateway, home, journal, kept_stops, reading,
    said, serving, sign_in, stand_in, tools, until_decided_in,
};

/// Where For You's row of what waits goes, under a case's home, as the
/// unit names it.
const WAITING: &str = "state/mission/waiting/pineapple.json";

/// The reader over the gateway at `url`, live, in `home`, telling For You
/// where its unit would.
fn live(home: &Path, origin: &str, url: &str) -> crate::Running {
    let program = sign_in(home, false);
    let program = program.to_str().expect("a path in UTF-8");
    reading(
        home,
        origin,
        &["--live", "--gateway", url, "--gateway-sign-in", program],
        Some(&home.join(WAITING)),
    )
}

#[test]
fn a_reader_over_a_gateway_live_stops_agents_and_tells_joel_alone() {
    let (origin, asked) = stand_in();
    let (url, called) = gateway();
    let home = home();
    an_alert_waits(&home);
    let mut reader = live(&home, &origin, &url);
    let kept = home.join("state/pineapple");
    until_decided_in(&kept, 1);
    let began = Instant::now();
    while !kept.join("phoned.jsonl").exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "Joel's phone was never told: {}",
            journal(&home)
        );
        thread::sleep(Duration::from_millis(50));
    }
    // Rounds go on, none telling his phone again.
    let rounds = tools(&called).len();
    while tools(&called).len() < rounds + 6 {
        assert!(began.elapsed() < PATIENCE, "the reader stopped asking");
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");

    let decided = decisions_in(&kept);
    assert_eq!(each(&decided, "verdict"), ["stop"], "the stop, judged once");
    assert_eq!(decided[0]["shadow"], false, "and live");
    let stops = kept_stops(&kept.join("stops"));
    assert!(
        stops.len() == 1
            && stops[0]["shadow"] == false
            && stops[0]["reaches"] == json!(["pr:example/shop#12"]),
        "it stands where the agents' guard reads, live, on the thread's pull request: {stops:?}"
    );
    let sent: Vec<Value> = called
        .lock()
        .expect("what was called")
        .iter()
        .filter(|(tool, _)| tool == "slack_send_message_to_self")
        .map(|(_, asked)| asked.clone())
        .collect();
    assert_eq!(
        sent,
        [json!({"message": "pineapple: Someone stopped your agents\n\
            Said: \"pineapple override\"\n\
            Where: https://example.slack.com/archives/C1/p1"})],
        "the alert waiting goes to Joel himself once, in its one form, and nothing else is sent"
    );
    let phoned: Vec<Value> = std::fs::read_to_string(kept.join("phoned.jsonl"))
        .expect("what Joel's phone was told")
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    assert!(
        phoned.len() == 1 && phoned[0]["sent"] == true && phoned[0]["im"] == "self",
        "kept as told, to himself: {phoned:?}"
    );
    let called = tools(&called);
    let allowed = [
        "slack_who_am_i",
        "slack_search_public_and_private",
        "slack_read_thread",
        "slack_send_message_to_self",
    ];
    assert!(
        called.iter().all(|tool| allowed.contains(&tool.as_str())),
        "it reads, and sends only to Joel himself: {called:?}"
    );
    assert!(
        asked.lock().expect("what was asked").is_empty(),
        "Slack's own API is never asked"
    );
    assert!(
        journal(&home).contains("a reply isn't said, as the gateway sends to Joel alone"),
        "no reply is said, and the journal says why: {}",
        journal(&home)
    );
    assert!(
        decisions_in(&home.join(OWN)).is_empty(),
        "nothing is written in the shadow's own folder"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_reader_over_a_gateway_live_says_what_it_never_read_beside_joels_row() {
    static TAGS: AtomicUsize = AtomicUsize::new(0);
    let (origin, _) = stand_in();
    let (url, _) = serving(|tool, asked, now| {
        if tool != "slack_search_public_and_private" {
            said(tool, now)
        } else if asked["query"] == "pineapple" {
            nothing()
        } else {
            a_gap(&TAGS, asked, now)
        }
    });
    let home = home();
    let mut reader = live(&home, &origin, &url);
    let missed = home.join(WAITING).with_file_name("pineapple-missed.json");
    let began = Instant::now();
    while !missed.exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "no row said what was never read: {}",
            journal(&home)
        );
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let rows: Value = serde_json::from_str(
        &std::fs::read_to_string(&missed).expect("the rows of what was never read"),
    )
    .expect("JSON");
    assert!(
        rows["items"][0]["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("pineapple-slack-missed-tag-")),
        "the span the tag's search read on past is a row For You reads: {rows}"
    );
    assert!(
        !home.join(OWN).join("missed.json").exists()
            && !home.join("state/pineapple/missed.json").exists(),
        "and it's kept nowhere else"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

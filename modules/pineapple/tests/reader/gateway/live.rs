//! The reader over a gateway, live, in the live reader's place: a stop it
//! hears stands in the live reader's folder, where the agents' guard reads;
//! Joel's phone is told once, through the gateway's one tool that sends to
//! him alone, and once more with the words left at the link if it refuses
//! them; no reply is said and Slack's own API is never asked; and a search
//! it reads on past is said in a row beside his row in For You.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
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
        !kept.join("replies.jsonl").exists(),
        "and none is written down as said"
    );
    assert!(
        decisions_in(&home.join(OWN)).is_empty(),
        "nothing is written in the shadow's own folder"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// The fake gateway's answer, but with its search also finding the alert
/// the reader sent Joel, in his own messages to himself, as the gateway's
/// search does once Slack has indexed it.
fn echoing(tool: &str, _: &Value, now: u64) -> String {
    if tool != "slack_search_public_and_private" {
        return said(tool, now);
    }
    let ts = format!("{}.000200", now - 30);
    let found: Value = serde_json::from_str(&said(tool, now)).expect("the search's answer");
    let real = found["results"].as_str().expect("its results");
    let (head, first) = real.split_once("### Result 1 of 1\n").expect("one result");
    let echo = format!(
        "### Result 2 of 2\nChannel: DM (ID: D0SELF0001)\n\
         From: Joel <joel@example.com> (ID: {}) \nTime: today\nMessage_ts: {ts}\n\
         Permalink: [link](https://example.slack.com/archives/D0SELF0001/p{})\n\
         Text: \npineapple: Someone stopped your agents\n\
         Said: \"pineapple override\"\n\
         Where: https://example.slack.com/archives/C1/p1\n\n---\n",
        crate::JOEL,
        ts.replace('.', "")
    );
    json!({ "results": format!(
        "{}### Result 1 of 2\n{first}\n{echo}",
        head.replace("(1 results)", "(2 results)")
    ) })
    .to_string()
}

#[test]
fn the_alert_joels_phone_was_sent_found_by_the_search_stops_nothing_more() {
    let (origin, _) = stand_in();
    let (url, called) = serving(echoing);
    let home = home();
    an_alert_waits(&home);
    let mut reader = live(&home, &origin, &url);
    let kept = home.join("state/pineapple");
    until_decided_in(&kept, 1);
    let began = Instant::now();
    let rounds = tools(&called).len();
    while tools(&called).len() < rounds + 12 {
        assert!(began.elapsed() < PATIENCE, "the reader stopped asking");
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let decided = decisions_in(&kept);
    assert_eq!(
        each(&decided, "verdict"),
        ["stop"],
        "the stop is judged once, and the alert quoting it as Joel's is no saying: {decided:?}"
    );
    let stops = kept_stops(&kept.join("stops"));
    assert_eq!(
        stops.len(),
        1,
        "one stop stands, not one for its echo: {stops:?}"
    );
    let sent = tools(&called)
        .iter()
        .filter(|tool| *tool == "slack_send_message_to_self")
        .count();
    assert_eq!(
        sent, 1,
        "and Joel's phone is told once, never of its own alert"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// The messages sent to Joel himself, in order.
fn sent_to_self(called: &super::Called) -> Vec<String> {
    called
        .lock()
        .expect("what was called")
        .iter()
        .filter(|(tool, _)| tool == "slack_send_message_to_self")
        .filter_map(|(_, asked)| asked["message"].as_str().map(str::to_owned))
        .collect()
}

/// What Joel's phone was told, line by line.
fn phoned(home: &Path) -> Vec<Value> {
    std::fs::read_to_string(home.join("state/pineapple/phoned.jsonl"))
        .unwrap_or_default()
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Runs the reader live over `url` until it has asked the gateway `more`
/// times past its first stop, so later rounds have had their say.
fn live_for_rounds(home: &Path, url: &str, called: &super::Called, more: usize) {
    let (origin, _) = stand_in();
    let mut reader = live(home, &origin, url);
    until_decided_in(&home.join("state/pineapple"), 1);
    let began = Instant::now();
    let rounds = tools(called).len();
    while tools(called).len() < rounds + more {
        assert!(began.elapsed() < PATIENCE, "the reader stopped asking");
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
}

#[test]
fn an_alert_slacks_spent_rate_holds_back_goes_whole_the_next_round() {
    static SENDS: AtomicUsize = AtomicUsize::new(0);
    let (url, called) = serving(|tool, _, now| {
        if tool == "slack_send_message_to_self" && SENDS.fetch_add(1, Ordering::SeqCst) == 0 {
            format!("{}Slack said: ratelimited", super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    live_for_rounds(&home, &url, &called, 12);
    let sent = sent_to_self(&called);
    assert!(
        sent.len() == 2
            && sent
                .iter()
                .all(|it| it.contains("Said: \"pineapple override\"")),
        "a spent rate sent nothing, so the next round sends it whole, words and all: {sent:?}"
    );
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 1 && phoned[0]["sent"] == true,
        "and it's kept as told once, when it went: {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_the_tool_always_fails_is_tried_two_rounds_then_left() {
    let (url, called) = serving(|tool, _, now| {
        if tool == "slack_send_message_to_self" {
            format!("{}Slack said: not_allowed", super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    live_for_rounds(&home, &url, &called, 30);
    let sent = sent_to_self(&called);
    assert_eq!(
        sent.len(),
        4,
        "each round tries it with its words and once without, for two rounds: {sent:?}"
    );
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 2 && phoned.iter().all(|line| line["sent"] == "unsure"),
        "each kept as may have gone, so it's left after two: {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_naming_everyone_is_sent_with_the_mention_said_as_at() {
    let (url, called) = serving(|tool, asked, now| {
        let message = asked["message"].as_str().unwrap_or_default();
        if tool == "slack_send_message_to_self" && message.contains("@here") {
            format!("{}the message mentions everyone", super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    let waiting = home.join("state/pineapple/would-tell.jsonl");
    let planted = std::fs::read_to_string(&waiting)
        .expect("the alert waiting")
        .replace(
            "\"words\":\"pineapple override\"",
            "\"words\":\"pineapple override @here\"",
        );
    assert!(planted.contains("@here"), "the words planted: {planted}");
    std::fs::write(&waiting, planted).expect("the alert rewritten");
    live_for_rounds(&home, &url, &called, 6);
    let sent = sent_to_self(&called);
    assert!(
        sent.len() == 1 && sent[0].contains("Said: \"pineapple override (at)here\""),
        "the mention goes said as (at), so its words reach him at the first send: {sent:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_reader_over_a_gateway_live_with_no_sign_in_writes_joels_row() {
    let (origin, _) = stand_in();
    let (url, called) = gateway();
    let home = home();
    let program = sign_in(&home, true);
    let program = program.to_str().expect("a path in UTF-8");
    let mut reader = reading(
        &home,
        &origin,
        &["--live", "--gateway", &url, "--gateway-sign-in", program],
        Some(&home.join(WAITING)),
    );
    let began = Instant::now();
    while !home.join(WAITING).exists() && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(20));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let row: Value = std::fs::read_to_string(home.join(WAITING))
        .ok()
        .and_then(|said| serde_json::from_str(&said).ok())
        .unwrap_or_default();
    assert!(
        row["items"][0]["why"]
            .as_str()
            .is_some_and(|why| why.contains("no tool gateway sign-in it can read")),
        "live, its row goes where the unit names, saying the gateway's sign-in is wanting: {row}"
    );
    assert!(
        !home.join("state/pineapple/waiting.json").exists()
            && !home.join(OWN).join("waiting.json").exists(),
        "and nowhere else"
    );
    assert!(
        tools(&called).is_empty(),
        "no tool is called without a sign-in"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// What the to-self tool sends to Joel's phone when the words it was
/// given are refused.
const WORDS_NOT_TAKEN: &str = "(the words are at the link)";

#[test]
fn an_alert_the_gateway_refuses_goes_again_with_its_words_left_at_the_link() {
    let (origin, _) = stand_in();
    let (url, called) = serving(|tool, asked, now| {
        let message = asked["message"].as_str().unwrap_or_default();
        if tool == "slack_send_message_to_self" && !message.contains(WORDS_NOT_TAKEN) {
            format!("{}the message holds what can't be sent", super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    let mut reader = live(&home, &origin, &url);
    let phoned = home.join("state/pineapple/phoned.jsonl");
    let began = Instant::now();
    while !phoned.exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "Joel's phone was never told: {}",
            journal(&home)
        );
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    let sent: Vec<String> = called
        .lock()
        .expect("what was called")
        .iter()
        .filter(|(tool, _)| tool == "slack_send_message_to_self")
        .filter_map(|(_, asked)| asked["message"].as_str().map(str::to_owned))
        .collect();
    assert!(
        sent.len() == 2
            && sent[0].contains("Said: \"pineapple override\"")
            && sent[1].contains(&format!("Said: \"{WORDS_NOT_TAKEN}\""))
            && sent[1].ends_with("Where: https://example.slack.com/archives/C1/p1"),
        "refused with the words, it goes once more with them left at the link: {sent:?}"
    );
    let told = std::fs::read_to_string(&phoned).expect("what Joel's phone was told");
    assert!(
        told.lines().count() == 1 && told.contains("\"sent\":true"),
        "and it's kept as told: {told}"
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

//! The reader over a gateway, live, in the live reader's place: a stop it
//! hears stands in the live reader's folder, where the agents' guard reads;
//! Joel's phone is told once, through the gateway's one tool that sends to
//! him alone, and once more with the words left at the link if it refuses
//! them; no reply is said and Slack's own API is never asked; and a search
//! it reads on past is said in a row beside his row in For You. A second
//! reader started on the folder it keeps ends at once, beside the keyring's
//! reader too, an alert left untold for its ten minutes is said to be let
//! go, once, and searches failing round after round write Joel's row.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::cut::{a_gap, nothing, result, stop};
use super::{
    OWN, PATIENCE, an_alert_waits, decisions_in, each, gateway, home, journal, kept_stops, reading,
    said, serving, sign_in, stand_in, tools, until_decided_in,
};

/// Where For You's row of what waits goes, under a case's home, as the
/// unit names it.
const WAITING: &str = "state/mission/waiting/pineapple.json";
/// Where the alert waiting links to.
const LINK: &str = "https://example.slack.com/archives/C1/p1";

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
    assert_eq!(
        decided[0]["gate"], "held:no-replies-here",
        "no reply is said, and its decision says it's held, not that one went"
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
fn echoing(tool: &str, asked: &Value, now: u64) -> String {
    if tool != "slack_search_public_and_private" {
        return said(tool, now);
    }
    let alert = "pineapple: Someone stopped your agents\n\
        Said: \"pineapple override\"\n\
        Where: https://example.slack.com/archives/C1/p1";
    let found = [
        (now - 30, crate::JOEL, alert.to_owned()),
        (now - 60, "U0OTHER001", stop()),
    ];
    let limit = asked["limit"]
        .as_u64()
        .and_then(|limit| usize::try_from(limit).ok())
        .unwrap_or(20);
    let counted = limit.min(found.len());
    let results = (1..)
        .zip(&found[..counted])
        .map(|(at, (when, author, text))| result(at, counted, *when, author, text))
        .collect::<Vec<_>>()
        .concat();
    json!({ "results": format!(
        "# Search Results for: \n\n## Messages ({counted} results)\n{results}"
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
        if tool == "slack_send_message_to_self" && SENDS.fetch_add(1, Ordering::SeqCst) < 2 {
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
        sent.len() == 3
            && sent[1].contains(WORDS_NOT_TAKEN)
            && [&sent[0], &sent[2]]
                .iter()
                .all(|it| it.contains("Said: \"pineapple override\"")),
        "a failing is asked again without the words, and that one's spent rate sent \
         nothing, so the next round sends it whole, words and all: {sent:?}"
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
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 1 && phoned[0]["text"] == sent[0].as_str(),
        "what's kept as told is the text that went, its mention said as (at): {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_second_reader_on_a_folder_held_ends_at_once_saying_why() {
    let (origin, _) = stand_in();
    let (url, _) = gateway();
    let home = home();
    let mut first = live(&home, &origin, &url);
    until_decided_in(&home.join("state/pineapple"), 1);
    let mut second = live(&home, &origin, &url);
    let began = Instant::now();
    let ended = loop {
        if let Some(status) = second.try_wait().expect("the second reader is asked after") {
            break status;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "a second reader on the folder held goes on: {}",
            journal(&home)
        );
        thread::sleep(Duration::from_millis(20));
    };
    assert!(
        !ended.success() && journal(&home).contains("another Slack reader keeps"),
        "a second reader ends failing, saying another keeps its folder: {}",
        journal(&home)
    );
    assert!(
        first
            .try_wait()
            .expect("the first reader is asked after")
            .is_none(),
        "the first reader goes on: {}",
        journal(&home)
    );
    drop(first);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_reader_over_a_gateway_beside_the_keyrings_ends_at_once_saying_why() {
    let (origin, _) = stand_in();
    let (url, _) = gateway();
    let home = home();
    let mut first = crate::reader(&home, &origin);
    crate::until_decided(&home, 1);
    let mut second = live(&home, &origin, &url);
    let began = Instant::now();
    let ended = loop {
        if let Some(status) = second
            .try_wait()
            .expect("the gateway's reader is asked after")
        {
            break status;
        }
        assert!(
            began.elapsed() < PATIENCE,
            "a reader over the gateway beside the keyring's goes on: {}",
            journal(&home)
        );
        thread::sleep(Duration::from_millis(20));
    };
    assert!(
        !ended.success() && journal(&home).contains("another Slack reader keeps"),
        "a reader over the gateway started on the keyring reader's folder ends, saying \
         another keeps it, so the two never both tell Joel's phone: {}",
        journal(&home)
    );
    assert!(
        first
            .try_wait()
            .expect("the keyring's reader is asked after")
            .is_none(),
        "the keyring's reader goes on: {}",
        journal(&home)
    );
    drop(first);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// Ages the alert waiting in `home` into one for each of `ages`, keyed
/// and in minutes, so each is that old as the reader starts, and each
/// links to its own message, the original's with `-` and its key.
fn aged(home: &Path, ages: &[(&str, u64)]) {
    let waiting = home.join("state/pineapple/would-tell.jsonl");
    let alert: Value = serde_json::from_str(
        std::fs::read_to_string(&waiting)
            .expect("the alert waiting")
            .trim(),
    )
    .expect("the alert reads");
    let at = alert["at"].as_u64().expect("when it was written");
    let each: Vec<String> = ages
        .iter()
        .map(|(key, minutes)| {
            let mut one = alert.clone();
            one["key"] = json!(key);
            one["at"] = json!(at - minutes * 60 * 1000);
            one["link"] = json!(format!("{LINK}-{key}"));
            one.to_string()
        })
        .collect();
    std::fs::write(&waiting, each.join("\n") + "\n").expect("the alerts made old");
}

/// Marks the alert waiting in `home` keyed `key` as written in shadow.
fn in_shadow(home: &Path, key: &str) {
    let waiting = home.join("state/pineapple/would-tell.jsonl");
    let each: Vec<String> = std::fs::read_to_string(&waiting)
        .expect("the alerts waiting")
        .lines()
        .map(|line| {
            let mut one: Value = serde_json::from_str(line).expect("an alert reads");
            if one["key"] == key {
                one["shadow"] = json!(true);
            }
            one.to_string()
        })
        .collect();
    std::fs::write(&waiting, each.join("\n") + "\n").expect("the alert put in shadow");
}

/// The time `minutes` ago, in milliseconds.
fn minutes_ago(minutes: u64) -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("a time after 1970");
    u64::try_from(now.as_millis()).expect("a time in range") - minutes * 60 * 1000
}

/// The keys of what phoned.jsonl in `home` keeps as let go.
fn let_go(home: &Path) -> Vec<Value> {
    phoned(home)
        .into_iter()
        .filter(|line| line["sent"] == "given-up")
        .map(|line| line["key"].clone())
        .collect()
}

#[test]
fn an_alert_untold_for_its_ten_minutes_is_said_let_go_once() {
    let (url, called) = gateway();
    let home = home();
    an_alert_waits(&home);
    aged(
        &home,
        &[
            ("untold", 15),
            ("older", 30),
            ("unsure", 12),
            ("twice", 13),
            ("told", 14),
            ("doubled", 16),
            ("doubled", 16),
            ("shadowed", 15),
        ],
    );
    in_shadow(&home, "shadowed");
    let kept = |key: &str, sent: Value| {
        json!({
            "at": minutes_ago(3),
            "key": key,
            "saying": "stopped",
            "sent": sent,
            "shadow": false,
        })
    };
    std::fs::write(
        home.join("state/pineapple/phoned.jsonl"),
        format!(
            "{}\n{}\n{}\n{}\n",
            kept("unsure", json!("unsure")),
            kept("twice", json!("unsure")),
            kept("twice", json!("unsure")),
            kept("told", json!(true)),
        ),
    )
    .expect("sends that may have gone, and one told");
    live_for_rounds(&home, &url, &called, 12);
    assert!(
        sent_to_self(&called).is_empty(),
        "an alert past its ten minutes isn't sent"
    );
    assert_eq!(
        let_go(&home),
        [
            json!("untold"),
            json!("unsure"),
            json!("twice"),
            json!("doubled")
        ],
        "each past its ten minutes is kept as let go, once, however many rounds pass, even \
         one left after two sends that may have gone or written down twice, but one told, \
         one in shadow, and one from twenty minutes before the phone was last told \
         aren't: {:?}",
        phoned(&home)
    );
    let journal = journal(&home);
    assert!(
        journal.contains(&format!(
            "Joel's phone alert is let go, untold after ten minutes: {LINK}-untold"
        )) && journal.contains(&format!(
            "after ten minutes, its one send that may have gone unanswered: {LINK}-unsure"
        )) && journal.contains(&format!(
            "after ten minutes, each send that may have gone unanswered: {LINK}-twice"
        )),
        "and the journal says so, and which may have gone: {journal}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_live_keyring_reader_says_an_alert_let_go_too_while_signed_out() {
    let (origin, _) = stand_in();
    let home = home();
    std::fs::remove_file(home.join("config/agentic-harness/slack/credentials.json"))
        .expect("the sign-in taken away");
    an_alert_waits(&home);
    aged(&home, &[("untold", 15)]);
    let mut reader = crate::reader_with(&home, &origin, &["--live"]);
    let began = Instant::now();
    while let_go(&home).is_empty() && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert_eq!(
        let_go(&home),
        [json!("untold")],
        "a live keyring reader signed out says an alert past its ten minutes let go too: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_reader_over_a_gateway_live_with_no_sign_in_writes_joels_row() {
    let (origin, _) = stand_in();
    let (url, called) = gateway();
    let home = home();
    an_alert_waits(&home);
    aged(
        &home,
        &[("untold", 15), ("while-down", 40), ("just-before", 75)],
    );
    let told = json!({
        "at": minutes_ago(60),
        "key": "told",
        "saying": "stopped",
        "sent": "pineapple: an earlier stop",
        "shadow": false,
    });
    std::fs::write(
        home.join("state/pineapple/phoned.jsonl"),
        format!("{told}\n"),
    )
    .expect("Joel's phone last told an hour ago");
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
    assert_eq!(
        let_go(&home),
        [json!("untold"), json!("while-down"), json!("just-before")],
        "with no sign-in an alert past its ten minutes is still said let go, and one \
         written while no reader ran, or untold for its ten minutes as his phone was last \
         told: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_reader_over_a_gateway_whose_searches_always_fail_writes_joels_row() {
    let (origin, _) = stand_in();
    let (url, _) = serving(|tool, _, now| {
        if tool == "slack_search_public_and_private" {
            format!("{}Slack said: fatal_error", super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    let mut reader = live(&home, &origin, &url);
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
            .is_some_and(|why| why.contains("isn't answering its searches")),
        "a gateway whose searches fail each round, past its rounds, writes Joel's row: {row} {}",
        journal(&home)
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

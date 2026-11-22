//! What the Slack reader makes of Joel's own words and of what it would
//! tell him: his resume in shadow ends only the shadow stop it names, the
//! reader's own alert echoed in his IM is never his saying, heard says
//! when Slack was last read, the switch holds replies back while stops
//! carry on, and an alert for his phone finds his own IM and, in shadow,
//! is only written down.

use std::process::Command;
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};

use super::{
    PATIENCE, a_stop, answering, decisions, each, home, joel_hears_his_own_alert, joel_resumes,
    reader, resumed, stand_in, until_decided,
};

#[test]
fn joels_own_resume_in_shadow_ends_only_the_shadow_stop_it_names() {
    let (origin, _) = answering(joel_resumes);
    let home = home();
    // Joel's resume was said a minute ago.
    a_stop(&home, "named", "pr:example/shop#12", true, 120);
    a_stop(&home, "elsewhere", "quest:QEST-A", true, 120);
    a_stop(&home, "live", "pr:example/shop#12", false, 120);
    a_stop(&home, "later", "pr:example/shop#12", true, 30);
    let mut reading = reader(&home, &origin);
    until_decided(&home, 1);
    let began = Instant::now();
    while resumed(&home, "named").is_null() {
        assert!(began.elapsed() < PATIENCE, "Joel's resume ended nothing");
        // The decision is written just before the resume; a short wait
        // sees it land without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(each(&said, "verdict"), ["resume"], "Joel's own resume");
    assert_eq!(
        resumed(&home, "named")["by"],
        "joel",
        "the stop it names is resumed by Joel"
    );
    assert_eq!(
        resumed(&home, "elsewhere"),
        Value::Null,
        "a stop it doesn't name still stands"
    );
    assert_eq!(
        resumed(&home, "live"),
        Value::Null,
        "a reader in shadow never ends a live stop"
    );
    assert_eq!(
        resumed(&home, "later"),
        Value::Null,
        "a stop said after the resume, though found beside it, still stands"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn the_readers_own_alert_in_joels_im_is_never_judged_as_his_saying() {
    let (origin, asked) = answering(joel_hears_his_own_alert);
    let home = home();
    a_stop(&home, "standing", "everything", true, 120);
    let mut reading = reader(&home, &origin);
    let began = Instant::now();
    // Each round asks twice; three rounds give the echo every chance.
    while asked.lock().expect("what was asked").len() < 6 {
        assert!(began.elapsed() < PATIENCE, "the reader asked nothing");
        // As above, a short wait sees each round without spinning.
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        resumed(&home, "standing"),
        Value::Null,
        "the echo resumes nothing"
    );
    assert!(
        decisions(&home).is_empty(),
        "the reader's own alert is no saying: {:?}",
        decisions(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn heard_says_when_slack_was_last_read_while_nothing_is_heard() {
    let home = home();
    let heard = || {
        let said = Command::new(env!("CARGO_BIN_EXE_pineapple-slack"))
            .arg("heard")
            .env("XDG_STATE_HOME", home.join("state"))
            .output()
            .expect("heard answers");
        String::from_utf8_lossy(&said.stdout).into_owned()
    };
    assert_eq!(
        heard(),
        "nothing heard yet, and Slack hasn't been read yet\n",
        "a mark with no read_at, as before a first round"
    );
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("after the epoch")
        .as_millis();
    let judged = json!({"since": "1.000000", "keys": [], "read_at": now - 5000});
    std::fs::write(home.join("state/pineapple/judged.json"), judged.to_string())
        .expect("the mark written");
    let said = heard();
    assert!(
        said.starts_with("nothing heard yet; Slack was last read ") && said.ends_with(" s ago\n"),
        "says how long ago it read: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn the_switch_holds_replies_back_and_stops_carry_on() {
    let (origin, _) = stand_in();
    let home = home();
    std::fs::write(home.join("state/pineapple/replies-switch"), "off\n")
        .expect("the switch written");
    let mut reading = reader(&home, &origin);
    until_decided(&home, 4);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        said[1]["gate"], "held:switched-off",
        "the reply it would have sent is held back"
    );
    assert_eq!(said[1]["verdict"], "stop", "and it still stops");
    assert!(
        !home.join("state/pineapple/replies.jsonl").exists(),
        "no reply is kept as sent"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_for_joels_phone_finds_his_own_im_and_in_shadow_sends_nothing() {
    let (origin, asked) = stand_in();
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
    let stale = json!({"at": at - 3_600_000, "key": "pineapple:slack:C2:1.0",
        "saying": "stopped", "shadow": false, "title": "an hour ago", "words": "",
        "link": ""});
    std::fs::write(
        home.join("state/pineapple/would-tell.jsonl"),
        format!("{stale}\n{alert}\n"),
    )
    .expect("what the launcher's daemon would tell");
    let phoned = home.join("state/pineapple/phoned.jsonl");
    let mut reading = reader(&home, &origin);
    until_decided(&home, 4);
    let began = Instant::now();
    while !phoned.exists() {
        assert!(
            began.elapsed() < PATIENCE,
            "Joel's phone was never written down"
        );
        // The reader goes round every tenth of a second.
        thread::sleep(Duration::from_millis(50));
    }
    // Another round, to be sure the alert is told only once.
    thread::sleep(Duration::from_millis(300));
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let kept = std::fs::read_to_string(&phoned).expect("what Joel's phone was told");
    let lines: Vec<Value> = kept
        .lines()
        .map(|line| serde_json::from_str(line).expect("a line is JSON"))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "the fresh alert once, not the hour-old one: {kept}"
    );
    assert_eq!(
        (&lines[0]["im"], &lines[0]["shadow"], &lines[0]["sent"]),
        (&json!("D0SELF0001"), &json!(true), &json!(false)),
        "Joel's own IM is found, and in shadow nothing is sent"
    );
    assert_eq!(
        lines[0]["text"],
        "pineapple: Someone stopped your agents on quest SAFEQA\n\
         Said: \"pineapple override\"\n\
         Where: https://example.slack.com/archives/C1/p1",
        "in the alert's one form"
    );
    let methods = asked.lock().expect("the stand-in's list").clone();
    assert!(
        !methods.iter().any(|line| line.contains("chat.postMessage")),
        "nothing is posted in shadow: {methods:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

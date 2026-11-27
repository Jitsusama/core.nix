//! Which alerts a reader says are let go: only those written since twenty
//! minutes before phoned.jsonl's newest line of any kind, not only its
//! newest told, and none at all from a keyring reader in shadow, though it
//! reads the live reader's folder.

use std::thread;
use std::time::{Duration, Instant};

use serde_json::json;

use super::{
    PATIENCE, aged, an_alert_waits, gateway, home, journal, let_go, live_for_rounds, minutes_ago,
    stand_in,
};

#[test]
fn an_alert_is_said_let_go_from_twenty_minutes_before_the_newest_line_of_any_kind() {
    let (url, called) = gateway();
    let home = home();
    an_alert_waits(&home);
    aged(&home, &[("older", 30), ("untold", 15)]);
    let kept = |key: &str, sent: serde_json::Value, minutes: u64| {
        json!({
            "at": minutes_ago(minutes),
            "key": key,
            "saying": "stopped",
            "sent": sent,
            "shadow": false,
        })
    };
    std::fs::write(
        home.join("state/pineapple/phoned.jsonl"),
        format!(
            "{}\n{}\n",
            kept("told", json!(true), 60),
            kept("unsure", json!("unsure"), 3),
        ),
    )
    .expect("a send told an hour ago, and one that may have gone since");
    live_for_rounds(&home, &url, &called, 4);
    assert_eq!(
        let_go(&home),
        [json!("untold")],
        "the send that may have gone three minutes ago sets where letting go starts, so an \
         alert from thirty minutes ago is left as it was, though the phone was last told an \
         hour ago: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn a_keyring_reader_in_shadow_says_no_alert_let_go() {
    let (origin, _) = stand_in();
    let home = home();
    std::fs::remove_file(home.join("config/agentic-harness/slack/credentials.json"))
        .expect("the sign-in taken away");
    an_alert_waits(&home);
    aged(&home, &[("untold", 15)]);
    let mut reader = crate::reader_with(&home, &origin, &[]);
    let rounds = || journal(&home).matches("no Slack sign-in yet").count();
    let began = Instant::now();
    while rounds() < 3 && began.elapsed() < PATIENCE {
        thread::sleep(Duration::from_millis(50));
    }
    reader.kill().expect("the reader stops");
    reader.wait().expect("the reader is gone");
    assert!(
        rounds() >= 3,
        "the reader went three rounds: {}",
        journal(&home)
    );
    assert!(
        let_go(&home).is_empty() && !journal(&home).contains("phone alert is let go"),
        "a keyring reader in shadow lets no alert go, though it reads the live folder: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

//! The reader over a gateway, live, with an alert the launcher's daemon
//! wrote down twice: it's sent to Joel's phone once, and one that may
//! have gone is tried two rounds, as one written once is.

use std::path::Path;

use super::{an_alert_waits, home, live_for_rounds, phoned, said, sent_to_self, serving, tools};

/// Writes the alert waiting in `home` down a second time, as the
/// launcher's daemon may.
fn written_down_twice(home: &Path) {
    let waiting = home.join("state/pineapple/would-tell.jsonl");
    let line = std::fs::read_to_string(&waiting).expect("the alert waiting");
    std::fs::write(&waiting, line.repeat(2)).expect("the alert, written down twice");
}

#[test]
fn an_alert_written_down_twice_is_sent_once() {
    let (url, called) = serving(|tool, _, now| said(tool, now));
    let home = home();
    an_alert_waits(&home);
    written_down_twice(&home);
    live_for_rounds(&home, &url, &called, 12);
    let sent = sent_to_self(&called);
    assert_eq!(sent.len(), 1, "its two lines send it once: {sent:?}");
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 1 && phoned[0]["sent"] == true,
        "and it's kept as told once: {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_written_down_twice_that_may_have_gone_is_tried_two_rounds() {
    let (url, called) = serving(|tool, _, now| {
        if tool == "slack_send_message_to_self" {
            format!("{}Slack said: not_allowed", super::super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    written_down_twice(&home);
    live_for_rounds(&home, &url, &called, 30);
    let asked = tools(&called);
    let sends: Vec<usize> = asked
        .iter()
        .enumerate()
        .filter(|(_, tool)| *tool == "slack_send_message_to_self")
        .map(|(at, _)| at)
        .collect();
    // Each round tries it with its words and once without; a round's
    // search comes before its sends.
    assert!(
        sends.len() == 4
            && asked[sends[1]..sends[2]]
                .iter()
                .any(|tool| tool == "slack_search_public_and_private"),
        "tried in two rounds, a search between, as one written once: {asked:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

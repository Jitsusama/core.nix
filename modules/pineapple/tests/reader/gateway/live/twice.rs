//! The reader over a gateway, live, with an alert the launcher's daemon
//! wrote down twice: it's sent to Joel's phone once, and one that may
//! have gone is tried two rounds, as one written once is, and one whose
//! send met a spent rate goes the next round, not again the same one.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{an_alert_waits, home, live_for_rounds, phoned, said, sent_to_self, serving, tools};

/// Where in what the reader asked of the gateway each send to Joel's
/// phone was, with all it asked.
fn sends(called: &super::super::Called) -> (Vec<String>, Vec<usize>) {
    let asked = tools(called);
    let sends = asked
        .iter()
        .enumerate()
        .filter(|(_, tool)| *tool == "slack_send_message_to_self")
        .map(|(at, _)| at)
        .collect();
    (asked, sends)
}

/// Whether a round's search was asked between two of `asked`.
fn a_round_between(asked: &[String], from: usize, to: usize) -> bool {
    asked[from..to]
        .iter()
        .any(|tool| tool == "slack_search_public_and_private")
}

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
    let (asked, sends) = sends(&called);
    // Each round tries it with its words and once without; a round's
    // search comes before its sends.
    assert!(
        sends.len() == 4 && a_round_between(&asked, sends[1], sends[2]),
        "tried in two rounds, a search between, as one written once: {asked:?}"
    );
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 2 && phoned.iter().all(|one| one["sent"] == "unsure"),
        "and kept as unsure once a round, so it's left after two: {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn an_alert_written_down_twice_whose_send_met_a_spent_rate_goes_the_next_round() {
    static SENDS: AtomicUsize = AtomicUsize::new(0);
    let (url, called) = serving(|tool, _, now| {
        if tool == "slack_send_message_to_self" && SENDS.fetch_add(1, Ordering::SeqCst) < 2 {
            format!("{}Slack said: ratelimited", super::super::FAILS)
        } else {
            said(tool, now)
        }
    });
    let home = home();
    an_alert_waits(&home);
    written_down_twice(&home);
    live_for_rounds(&home, &url, &called, 12);
    let (asked, sends) = sends(&called);
    // Its first round's two tries met the spent rate, so its second
    // line waits for the next round rather than spending it again.
    assert!(
        sends.len() == 3 && a_round_between(&asked, sends[1], sends[2]),
        "tried twice in one round, then once the next, a search between: {asked:?}"
    );
    let phoned = phoned(&home);
    assert!(
        phoned.len() == 1 && phoned[0]["sent"] == true,
        "and kept as told once, when it went: {phoned:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

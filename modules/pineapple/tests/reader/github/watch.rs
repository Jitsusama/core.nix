//! Joel's own stop where GitHub's search hasn't indexed it yet: heard the
//! round after he says it, since the newest comments where his account
//! was last active list it at once, and read once however many rounds
//! list it after.

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, commented, github_reader, read_lately, until_asked};
use crate::{answering, decisions, each, until_decided};

/// Where only Joel spoke, on example/shop#12, a moment ago: no
/// notification, a search that hasn't indexed it, his events feed naming
/// example/shop, and its newest comments listing his stop.
fn unindexed(line: &str, now: u64) -> Value {
    let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, 5));
    if line.contains(" /notifications?") {
        return json!([]);
    }
    if line.contains(" /search/issues?") {
        return json!({"items": []});
    }
    if line.contains(&format!(" /users/{LOGIN}/events?")) {
        return json!([{"type": "IssueCommentEvent", "created_at": stamp(now - 3600),
            "repo": {"name": "example/shop"}}]);
    }
    if line.contains(" /repos/example/shop/issues/comments?") {
        let mut listed = his;
        listed["issue_url"] = json!("https://api.example.com/repos/example/shop/issues/12");
        return json!([listed]);
    }
    answer_with(line, now, &[his])
}

#[test]
fn github_joels_stop_no_search_finds_is_heard_where_he_was_last_active() {
    let (origin, _) = answering(unindexed);
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    // The thread's words are judged in the order they were said, so a
    // colleague's earlier ask to stop comes first.
    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop-asked", "stop"],
        "Joel's stop no search finds is heard from the newest comments where he was active"
    );
    assert_eq!(said[1]["by_joel"], true, "his own words");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_joels_listed_stop_is_read_once_however_often_it_is_listed() {
    let (origin, asked) = answering(unindexed);
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    let rounds = asked.lock().expect("what was asked").len();
    until_asked(&asked, rounds, 12);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let asked = asked.lock().expect("what was asked");
    let listed = asked
        .iter()
        .filter(|line| line.contains(" /repos/example/shop/issues/comments?"))
        .count();
    let read = asked
        .iter()
        .filter(|line| line.contains(" /repos/example/shop/issues/12/comments?"))
        .count();
    assert!(
        listed > 1,
        "the newest comments are listed each round: {asked:?}"
    );
    assert_eq!(
        read, 1,
        "the thread his judged stop is on isn't read again for it: {asked:?}"
    );
    drop(asked);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

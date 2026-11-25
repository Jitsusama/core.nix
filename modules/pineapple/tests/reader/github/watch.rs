//! Joel's own stop where GitHub's search hasn't indexed it yet: heard the
//! round after he says it, since the newest comments where his account
//! was last active list it at once, and read once however many rounds
//! list it after.

use std::sync::atomic::{AtomicUsize, Ordering};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, commented, github_reader, read_lately, until_asked};
use crate::{Answered, answering, answering_whole, decisions, each, journal, until_decided};

/// Where only Joel spoke, on example/shop#12, a moment ago: no
/// notification, a search that hasn't indexed it, his events feed naming
/// example/shop, and its newest comments listing his stop.
fn unindexed(line: &str, now: u64) -> Value {
    if line.contains(" /notifications?") {
        return json!([]);
    }
    listed_and_notified(line, now)
}

/// The same, but with Joel notified of example/shop#12 too, as he is once
/// anyone else speaks there.
fn listed_and_notified(line: &str, now: u64) -> Value {
    let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, 5));
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
fn github_joels_stop_listed_and_notified_is_judged_once() {
    let (origin, asked) = answering(listed_and_notified);
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    let rounds = asked.lock().expect("what was asked").len();
    until_asked(&asked, rounds, 12);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked", "stop"],
        "a thread both listed and notified is read as one, so each saying is judged once"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_list_unchanged_is_asked_with_its_tag_and_its_304_taken_as_before() {
    let listings = AtomicUsize::new(0);
    let (origin, asked) = answering_whole(move |line, now| {
        if !line.contains(" /repos/example/shop/issues/comments?") {
            return Answered::ok(unindexed(line, now));
        }
        if listings.fetch_add(1, Ordering::SeqCst) == 0 {
            Answered {
                headers: vec![("ETag", "\"t1\"".to_owned())],
                ..Answered::ok(unindexed(line, now))
            }
        } else {
            Answered {
                status: 304,
                ..Answered::ok(Value::Null)
            }
        }
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    let rounds = asked.lock().expect("what was asked").len();
    until_asked(&asked, rounds, 12);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let asked = asked.lock().expect("what was asked");
    assert!(
        asked.iter().any(
            |line| line.contains("/issues/comments?") && line.ends_with("if-none-match=\"t1\"")
        ),
        "a list is asked again with the tag its answer gave: {asked:?}"
    );
    drop(asked);
    let said = journal(&home);
    assert!(
        !said.contains("answered 304"),
        "a 304 is taken as the list kept, not said as a failing: {said}"
    );
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked", "stop"],
        "and what the first listing gave is judged once"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_list_that_fails_is_said_and_lists_no_more_that_round() {
    let (origin, asked) = answering_whole(|line, now| {
        if line.contains(" /repos/example/shop/issues/comments?") {
            return Answered {
                status: 502,
                ..Answered::ok(json!({"message": "bad gateway"}))
            };
        }
        let mut said = unindexed(line, now);
        if line.contains(&format!(" /users/{LOGIN}/events?")) {
            said = json!([
                {"created_at": stamp(now - 60), "repo": {"name": "example/shop"}},
                {"created_at": stamp(now - 120), "repo": {"name": "example/other"}},
            ]);
        }
        Answered::ok(said)
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_asked(&asked, 0, 12);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let said = journal(&home);
    assert!(
        said.contains("answered 502 for /repos/example/shop/issues/comments?")
            && said.contains("wait on its search"),
        "the failing is said, with what it costs: {said}"
    );
    let asked = asked.lock().expect("what was asked");
    assert!(
        !asked
            .iter()
            .any(|line| line.contains(" /repos/example/other/issues/comments?")),
        "the round lists no more once one list fails, so a slow site costs one wait: {asked:?}"
    );
    drop(asked);
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

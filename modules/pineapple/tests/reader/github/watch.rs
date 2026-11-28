//! Joel's own stop where GitHub's search hasn't indexed it yet: heard the
//! round after he says it, since the newest comments where his account
//! was last active list it at once, and read once however many rounds
//! list it after, even where a repository before it is refused, and
//! owed where its read fails, though nothing lists it again, and waiting
//! its turn while owed, though the watch lists it every round. A list
//! failing is listed last the round after, and the watch rests while
//! little of Joel's rate limit is left.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, commented, github_reader, read_lately, searches, until_asked};
use crate::{
    Answered, PATIENCE, answering, answering_whole, decisions, each, journal, until_decided,
};

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

/// The repositories whose newest comments were listed, in the order
/// asked.
fn listings(asked: &[String]) -> Vec<&str> {
    asked
        .iter()
        .filter_map(|line| {
            let (_, after) = line.split_once(" /repos/")?;
            let (repo, _) = after.split_once("/issues/comments?")?;
            Some(repo)
        })
        .collect()
}

#[test]
fn github_a_list_that_fails_lists_no_more_that_round_and_comes_last_the_next() {
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
    let began = Instant::now();
    while listings(&asked.lock().expect("what was asked")).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never listed three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let said = journal(&home);
    assert!(
        said.contains("answered 502 for /repos/example/shop/issues/comments?")
            && said.contains("wait on its search"),
        "the failing is said, with what it costs: {said}"
    );
    let asked = asked.lock().expect("what was asked");
    assert_eq!(
        listings(&asked)[..3],
        ["example/shop", "example/other", "example/shop"],
        "the round lists no more once one list fails, so a slow site costs one wait, and the \
         next lists the one that failed last, so it can't keep the others unlisted: {asked:?}"
    );
    drop(asked);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_the_watch_rests_while_little_of_joels_rate_limit_is_left() {
    // Only the first listing says the limit is nearly spent, until a
    // reset a few seconds on.
    let listed = AtomicUsize::new(0);
    let (origin, asked) = answering_whole(move |line, now| {
        let answered = Answered::ok(unindexed(line, now));
        if !line.contains(" /repos/example/shop/issues/comments?")
            || listed.fetch_add(1, Ordering::SeqCst) > 0
        {
            return answered;
        }
        Answered {
            headers: vec![
                ("x-ratelimit-remaining", "999".to_owned()),
                ("x-ratelimit-reset", (now + 3).to_string()),
            ],
            ..answered
        }
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while listings(&asked.lock().expect("what was asked")).len() < 2 {
        assert!(began.elapsed() < PATIENCE, "it never listed again");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let asked = asked.lock().expect("what was asked").clone();
    let at: Vec<usize> = asked
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" /repos/example/shop/issues/comments?"))
        .map(|(place, _)| place)
        .collect();
    let between = &asked[at[0]..at[1]];
    let count = |what: &str| between.iter().filter(|line| line.contains(what)).count();
    assert!(
        count(" /search/issues?") >= 3 && count(&format!(" /users/{LOGIN}/events?")) <= 1,
        "rounds go on searching while the watch rests, its events unasked till the reset: \
         {between:?}"
    );
    let said = journal(&home);
    assert!(
        said.contains("has 999 requests left this hour") && said.contains("wait on its search"),
        "the rest is said, with what it costs: {said}"
    );
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked", "stop"],
        "what the listing before the rest gave is still judged"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_watched_repository_refused_every_round_is_passed_by_for_the_rest() {
    let (origin, _) = answering_whole(|line, now| {
        if line.contains(" /repos/example/gone/issues/comments?") {
            return Answered {
                status: 404,
                ..Answered::ok(json!({"message": "Not Found"}))
            };
        }
        let mut said = unindexed(line, now);
        if line.contains(&format!(" /users/{LOGIN}/events?")) {
            said = json!([
                {"created_at": stamp(now - 60), "repo": {"name": "example/gone"}},
                {"created_at": stamp(now - 120), "repo": {"name": "example/shop"}},
            ]);
        }
        Answered::ok(said)
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked", "stop"],
        "Joel's stop no search finds is heard where he was active after one refused"
    );
    let said = journal(&home);
    assert!(
        said.contains("answered 404 for /repos/example/gone/issues/comments?")
            && said.contains("passed by"),
        "the refusal is said, and that it's passed by: {said}"
    );
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

#[test]
fn github_joels_stop_listed_once_on_a_thread_that_failed_is_owed_and_heard() {
    // His events name example/shop only the first time they're asked,
    // and that round's read of the thread fails, so only what's owed
    // brings it back.
    let (feeds, reads) = (AtomicUsize::new(0), AtomicUsize::new(0));
    let (origin, _) = answering_whole(move |line, now| {
        let feed = line.contains(&format!(" /users/{LOGIN}/events?"));
        if feed && feeds.fetch_add(1, Ordering::SeqCst) > 0 {
            return Answered::ok(json!([]));
        }
        let opening = line.contains(" /repos/example/shop/issues/12 ");
        if opening && reads.fetch_add(1, Ordering::SeqCst) == 0 {
            return Answered {
                status: 502,
                ..Answered::ok(json!({"message": "bad gateway"}))
            };
        }
        Answered::ok(unindexed(line, now))
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked", "stop"],
        "a thread only his events listed, unread that round, is owed and heard after"
    );
    let said = journal(&home);
    assert!(
        said.contains("couldn't read github:example/shop#12"),
        "its failed read is said: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_thread_failing_where_joel_spoke_lately_waits_its_turn_though_listed_each_round() {
    // Every read of example/shop#12 fails, so his stop there stays
    // unjudged, and the watch lists it in every round.
    let (origin, asked) = answering_whole(|line, now| {
        if line.contains(" /repos/example/shop/issues/12 ") {
            return Answered {
                status: 502,
                ..Answered::ok(json!({"message": "bad gateway"}))
            };
        }
        Answered::ok(unindexed(line, now))
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 7 {
        assert!(began.elapsed() < PATIENCE, "it never searched seven times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    // Only the six rounds before the seventh search, so a late stop
    // can't count an eighth round's read.
    let seen = asked.lock().expect("what was asked").clone();
    let seventh = seen
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" /search/issues?"))
        .nth(6)
        .map(|(place, _)| place)
        .expect("a seventh search");
    let read = seen[..seventh]
        .iter()
        .filter(|line| line.contains(" /repos/example/shop/issues/12 "))
        .count();
    assert_eq!(
        read, 3,
        "it's asked in rounds one, two and four as its wait doubles, not in each round the \
         watch lists it: {seen:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

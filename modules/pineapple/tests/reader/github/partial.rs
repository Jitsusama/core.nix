//! A round that can't read every pull request and issue it named: it
//! judges what it did read and reads the rest again, rather than judging
//! nothing until a round reads them all, which a flood past Joel's limit
//! or one thread that keeps failing would never let come; it reads past a
//! few that fail; and what others said to him comes before what his own
//! search found.

use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, answer_with, commented, github_reader, journal, read_lately, searches};
use crate::{Answered, PATIENCE, answering_whole, decisions, each, until_decided};

/// When every thread a search gives last changed: a minute before its
/// first answer, and the same in every round after, as nothing changes.
static CHANGED: OnceLock<String> = OnceLock::new();

/// What a search for Joel's words gives at `now`: example/shop's pull
/// requests `numbers`, in that order.
fn found(numbers: &[u64], now: u64) -> Value {
    let at = CHANGED.get_or_init(|| stamp(now - 60));
    let items: Vec<Value> = numbers
        .iter()
        .map(|number| {
            json!({"number": number, "pull_request": {}, "updated_at": at,
                "repository_url": "https://api.example.com/repos/example/shop"})
        })
        .collect();
    json!({ "items": items })
}

/// Where Joel said the safe word a minute back on example/shop#12, which
/// nobody notified him of, and his search gives `numbers`, any of whose
/// comments but #12's GitHub refuses with `status`, every time.
fn refusing(line: &str, now: u64, numbers: &[u64], status: u16) -> Answered {
    if line.contains(" /search/issues?") {
        return Answered::ok(found(numbers, now));
    }
    if line.contains(" /notifications?") {
        return Answered::ok(json!([]));
    }
    if line.contains("/comments?") && !line.contains("/12/comments?") {
        return Answered {
            status,
            headers: vec![("x-ratelimit-remaining", "0".to_owned())],
            body: json!({"message": "API rate limit exceeded"}),
        };
    }
    let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, 60));
    Answered::ok(answer_with(line, now, &[his]))
}

/// How many times `asked` holds a request for the comments of
/// example/shop#`number`.
fn read(asked: &[String], number: u64) -> usize {
    let comments = format!(" /repos/example/shop/issues/{number}/comments?");
    asked.iter().filter(|line| line.contains(&comments)).count()
}

/// Lets the reader search `count` times, then stops it, giving what it
/// asked for.
fn until_searched(asked: &Mutex<Vec<String>>, count: usize) -> Vec<String> {
    let began = Instant::now();
    while searches(asked).len() < count {
        assert!(
            began.elapsed() < PATIENCE,
            "it never searched {count} times"
        );
        thread::sleep(Duration::from_millis(50));
    }
    asked.lock().expect("what was asked").clone()
}

#[test]
fn github_a_round_that_cant_read_all_it_named_judges_what_it_read_and_reads_the_rest_again() {
    let (origin, asked) = answering_whole(|line, now| refusing(line, now, &[12, 14], 403));
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 1);
    let seen = until_searched(&asked, 3);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert!(
        each(&decisions(&home), "verdict").contains(&"stop"),
        "Joel's stop on what it read is heard, though its limit refuses another thread"
    );
    assert_eq!(
        (read(&seen, 12), read(&seen, 14) >= 2),
        (1, true),
        "what it read isn't read again, unchanged, and what it couldn't is, each round"
    );
    let journal = journal(&home);
    assert!(
        journal.contains("read 1 of the 2 it named, so the rest is read again next round"),
        "the journal says so: {journal}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_thread_that_keeps_failing_holds_back_none_after_it() {
    let (origin, asked) = answering_whole(|line, now| refusing(line, now, &[14, 12], 500));
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 1);
    let seen = until_searched(&asked, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert!(
        each(&decisions(&home), "verdict").contains(&"stop"),
        "Joel's stop after a thread that fails every round is heard"
    );
    assert!(read(&seen, 14) >= 1, "the failing thread was asked for");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_what_others_said_to_joel_is_read_before_what_his_search_found() {
    let (origin, asked) = answering_whole(|line, now| {
        if line.contains(" /search/issues?") {
            return Answered::ok(found(&[14], now));
        }
        Answered::ok(answer_with(line, now, &[]))
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let seen = until_searched(&asked, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let first = |number: u64| {
        let issue = format!(" /repos/example/shop/issues/{number}");
        seen.iter().position(|line| line.contains(&issue))
    };
    assert!(
        first(12).is_some() && first(14).is_some() && first(12) < first(14),
        "the pull request he was notified of is read before the one his search found: \
         {seen:?}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

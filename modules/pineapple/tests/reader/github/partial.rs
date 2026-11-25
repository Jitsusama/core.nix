//! A round that can't read every pull request and issue it named: it
//! judges what it did read, moves on, and owes the rest, read again each
//! round from where it was to be, so one that keeps failing holds back
//! nothing and is judged once it reads; a spent rate limit stops the round
//! at once; and one owed for hours is let go, said loudly.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, MARK, answer_with, commented, github_reader, journal, read_lately, searches};
use crate::{Answered, PATIENCE, answering_whole, decisions, each, home};

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
/// he was notified of, and his search gives `numbers`.
fn spoke(line: &str, now: u64, numbers: &[u64]) -> Answered {
    if line.contains(" /search/issues?") {
        return Answered::ok(found(numbers, now));
    }
    let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, 60));
    Answered::ok(answer_with(line, now, &[his]))
}

/// Whether `line` asks for the comments of a pull request but #12.
fn others_comments(line: &str) -> bool {
    line.contains("/comments?") && !line.contains("/12/comments?")
}

/// An answer GitHub gives for a thread it fails on.
fn failing() -> Answered {
    Answered {
        status: 500,
        headers: Vec::new(),
        body: json!({"message": "Server Error"}),
    }
}

/// How many times `asked` holds a request for the comments of
/// example/shop#`number`.
fn read(asked: &[String], number: u64) -> usize {
    let comments = format!(" /repos/example/shop/issues/{number}/comments?");
    asked.iter().filter(|line| line.contains(&comments)).count()
}

/// Lets the reader search `count` times, then stops it, giving what it
/// asked for.
fn until_searched(
    asked: &Mutex<Vec<String>>,
    reading: &mut std::process::Child,
    count: usize,
) -> Vec<String> {
    let began = Instant::now();
    while searches(asked).len() < count {
        assert!(
            began.elapsed() < PATIENCE,
            "it never searched {count} times"
        );
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    asked.lock().expect("what was asked").clone()
}

/// Each verdict of what was judged, in order of name, so a saying judged
/// twice shows.
fn verdicts(home: &std::path::Path) -> Vec<String> {
    let said = decisions(home);
    let mut verdicts: Vec<String> = each(&said, "verdict")
        .into_iter()
        .map(str::to_owned)
        .collect();
    verdicts.sort();
    verdicts
}

/// What the reader keeps of how far it read, as `home` holds it.
fn kept(home: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(home.join("state/pineapple").join(MARK)).expect("the mark");
    serde_json::from_str(&text).expect("the mark is JSON")
}

#[test]
fn github_a_thread_that_keeps_failing_is_owed_and_the_round_moves_on() {
    let (origin, asked) = answering_whole(|line, now| {
        if others_comments(line) {
            return failing();
        }
        spoke(line, now, &[14, 12])
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let seen = until_searched(&asked, &mut reading, 3);

    assert_eq!(
        verdicts(&home),
        ["stop", "stop-asked"],
        "Joel's stop after a thread that fails every round is judged, and once"
    );
    let notified: Vec<&String> = seen
        .iter()
        .filter(|line| line.contains(" /notifications?"))
        .collect();
    assert!(
        notified.len() >= 3 && notified.first() != notified.last(),
        "how far it has read moves on though a thread fails: {notified:?}"
    );
    assert!(
        read(&seen, 14) >= 3,
        "the thread it couldn't read is read again each round"
    );
    let kept = kept(&home);
    assert_eq!(
        (kept["owed"][0]["number"].as_u64(), kept.get("read_at")),
        (Some(14), None),
        "what it couldn't read is kept as owed, and the site isn't said to be read whole: {kept}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_spent_limit_stops_the_round_and_what_it_owes_is_judged_once_it_lifts() {
    let refused = Arc::new(AtomicUsize::new(0));
    let refusing = Arc::clone(&refused);
    let (origin, asked) = answering_whole(move |line, now| {
        if line.contains("/comments?") && refusing.fetch_add(1, Ordering::SeqCst) < 3 {
            return Answered {
                status: 403,
                headers: vec![("x-ratelimit-remaining", "0".to_owned())],
                body: json!({"message": "API rate limit exceeded"}),
            };
        }
        if line.contains(" /notifications?") {
            return Answered::ok(json!([]));
        }
        spoke(line, now, &[14, 15, 16, 12])
    });
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let seen = until_searched(&asked, &mut reading, 5);

    let first = seen
        .iter()
        .position(|line| line.contains(" /search/issues?"));
    let second = seen
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" /search/issues?"))
        .nth(1)
        .map(|(place, _)| place);
    let (Some(first), Some(second)) = (first, second) else {
        panic!("it searched twice: {seen:?}");
    };
    let asked_first = seen[first..second]
        .iter()
        .filter(|line| line.contains("/comments?"))
        .count();
    assert_eq!(
        asked_first, 1,
        "a spent limit stops the round at once: {seen:?}"
    );
    assert_eq!(
        verdicts(&home),
        ["stop", "stop-asked"],
        "what it owed is judged once the limit lifts, and once"
    );
    let said = journal(&home);
    assert!(
        said.contains("read 0 of the 4 it named, so the rest is read again next round"),
        "the journal says what it owes: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_one_owed_for_two_hours_is_let_go_and_said_so() {
    let (origin, asked) = answering_whole(|line, now| {
        if others_comments(line) {
            return failing();
        }
        if line.contains(" /notifications?") {
            return Answered::ok(json!([]));
        }
        spoke(line, now, &[])
    });
    let home = home();
    let last = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
        - 30;
    let state = home.join("state/pineapple");
    std::fs::create_dir_all(&state).expect("the state folder");
    let owed = json!({"repo": "example/shop", "number": 14, "pull": true,
        "from": last - 60, "read_from": last - 60, "at": last - 3 * 60 * 60});
    let judged = json!({"since": last, "keys": [], "owed": [owed]});
    std::fs::write(state.join(MARK), judged.to_string()).expect("the mark");
    let mut reading = github_reader(&home, &origin, &[]);
    let seen = until_searched(&asked, &mut reading, 3);

    assert_eq!(
        read(&seen, 14),
        1,
        "one owed past two hours is read no more: {seen:?}"
    );
    let said = journal(&home);
    assert!(
        said.contains(
            "<3>couldn't read github:example/shop#14 for two hours, so it's read again only once \
             it changes"
        ),
        "the journal says so, loudly: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

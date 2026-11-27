//! Joel's own stop where GitHub's search hasn't indexed it yet: heard the
//! round after he says it, since the newest comments where his account
//! was last active list it at once, and read once however many rounds
//! list it after, even where a repository before it is refused, and
//! owed where its read fails, though nothing lists it again, and waiting
//! its turn while owed, though the watch lists it every round. A list
//! failing is listed last the round after. The watch rests while little
//! of Joel's rate limit is left, a fifth of the limit GitHub says, on a
//! 304 too, leaving the round's other lists unlisted, until a reset an
//! hour on at most and never one already past; and as long as a refusal
//! asks it to wait.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

/// Where in `asked` each listing of `repo`'s newest comments is.
fn listed_at(asked: &[String], repo: &str) -> Vec<usize> {
    let listing = format!(" /repos/{repo}/issues/comments?");
    let at = asked
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(&listing));
    at.map(|(place, _)| place).collect()
}

/// What was asked from the `nth` listing of `repo`'s newest comments,
/// counted from 0, to the next listing of it or the end; nothing where
/// it wasn't listed that often.
fn after_listing<'a>(asked: &'a [String], repo: &str, nth: usize) -> &'a [String] {
    let at = listed_at(asked, repo);
    let Some(&from) = at.get(nth) else {
        return &[];
    };
    &asked[from..at.get(nth + 1).copied().unwrap_or(asked.len())]
}

/// How many of `asked` ask for `what`.
fn count(asked: &[String], what: &str) -> usize {
    asked.iter().filter(|line| line.contains(what)).count()
}

/// Whether `asked` is what rounds ask while the watch rests: three
/// searches or more, and the events feed once at most, as the rest ends.
fn rested(asked: &[String]) -> bool {
    count(asked, " /search/issues?") >= 3 && count(asked, &format!(" /users/{LOGIN}/events?")) <= 1
}

/// The seconds since the epoch now, as the reader reads its clock.
fn seconds() -> u64 {
    let since = SystemTime::now().duration_since(UNIX_EPOCH);
    since.expect("after the epoch").as_secs()
}

/// What GitHub says when `remaining` of Joel's limit are left, until a
/// reset `ahead` seconds from now, as the answer is given rather than as
/// the stand-in started.
fn left(remaining: &str, ahead: i64) -> Vec<(&'static str, String)> {
    let reset = seconds()
        .checked_add_signed(ahead)
        .expect("a second after the epoch");
    vec![
        ("x-ratelimit-remaining", remaining.to_owned()),
        ("x-ratelimit-reset", reset.to_string()),
    ]
}

/// An events feed naming `repos`, the latest first, a minute apart.
fn feed(repos: &[&str], now: u64) -> Value {
    let events = repos.iter().zip(1..).map(|(repo, minutes): (&&str, u64)| {
        json!({"created_at": stamp(now - 60 * minutes), "repo": {"name": repo}})
    });
    events.collect()
}

/// Answers as [`unindexed`] does, but with an events feed naming `repos`,
/// and each listing of example/shop's newest comments as `shop` makes of
/// it, given how many were listed before it.
fn watching(
    repos: &'static [&'static str],
    shop: impl Fn(usize, Answered) -> Answered + Send + 'static,
) -> impl Fn(&str, u64) -> Answered + Send + 'static {
    let listed = AtomicUsize::new(0);
    move |line: &str, now: u64| {
        if line.contains(&format!(" /users/{LOGIN}/events?")) {
            return Answered::ok(feed(repos, now));
        }
        let answered = Answered::ok(unindexed(line, now));
        if !line.contains(" /repos/example/shop/issues/comments?") {
            return answered;
        }
        shop(listed.fetch_add(1, Ordering::SeqCst), answered)
    }
}

/// What a reader asked of a stand-in answering as `answer` does, in
/// order, and what its journal said, once what it asked is `enough`.
fn asked_until(
    answer: impl Fn(&str, u64) -> Answered + Send + 'static,
    enough: impl Fn(&[String]) -> bool,
) -> (Vec<String>, String) {
    let (origin, asked) = answering_whole(answer);
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while !enough(&asked.lock().expect("what was asked")) {
        assert!(
            began.elapsed() < PATIENCE,
            "it never asked what the case waits for"
        );
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let asked = asked.lock().expect("what was asked").clone();
    let said = journal(&home);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    (asked, said)
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
            said = feed(&["example/shop", "example/other"], now);
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
    let (origin, asked) = answering_whole(watching(&["example/shop"], |nth, answered| {
        if nth > 0 {
            return answered;
        }
        Answered {
            headers: left("999", 3),
            ..answered
        }
    }));
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
    let between = after_listing(&asked, "example/shop", 0);
    assert!(
        rested(between),
        "rounds go on searching while the watch rests, its events unasked till the reset: \
         {between:?}"
    );
    let said = journal(&home);
    assert!(
        said.contains("has 999 requests left this hour, fewer than the 1000 kept")
            && said.contains("aren't listed until")
            && said.contains(", and Joel's newest words wait on the search"),
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
fn github_a_rest_begun_on_one_list_leaves_the_next_unlisted_till_its_reset() {
    let repos = &["example/shop", "example/other"];
    let (asked, _) = asked_until(
        watching(repos, |nth, answered| {
            if nth > 0 {
                return answered;
            }
            Answered {
                headers: left("999", 3),
                ..answered
            }
        }),
        |asked| !listed_at(asked, "example/other").is_empty(),
    );
    assert_eq!(
        listings(&asked)[..3],
        ["example/shop", "example/shop", "example/other"],
        "the round the rest begins in lists no more, and the next after the reset lists both: \
         {asked:?}"
    );
    let between = after_listing(&asked, "example/shop", 0);
    assert!(rested(between), "the rounds between rest: {between:?}");
}

#[test]
fn github_a_304_saying_little_is_left_rests_the_watch_too() {
    let (asked, said) = asked_until(
        watching(&["example/shop"], |nth, answered| match nth {
            0 => Answered {
                headers: vec![("ETag", "\"t1\"".to_owned())],
                ..answered
            },
            1 => Answered {
                status: 304,
                headers: left("999", 3),
                ..Answered::ok(Value::Null)
            },
            _ => Answered {
                status: 304,
                ..Answered::ok(Value::Null)
            },
        }),
        |asked| listed_at(asked, "example/shop").len() >= 3,
    );
    let between = after_listing(&asked, "example/shop", 1);
    assert!(
        rested(between),
        "a 304 saying little is left rests the watch as a 200 would: {between:?}"
    );
    assert!(
        said.contains("has 999 requests left this hour"),
        "the rest a 304 begins is said: {said}"
    );
}

#[test]
fn github_the_watch_rests_an_hour_at_most_whatever_reset_github_says() {
    let before = seconds();
    let (asked, said) = asked_until(
        watching(&["example/shop"], |nth, answered| {
            if nth > 0 {
                return answered;
            }
            Answered {
                headers: left("999", 24 * 60 * 60),
                ..answered
            }
        }),
        |asked| count(after_listing(asked, "example/shop", 0), " /search/issues?") >= 3,
    );
    let after = seconds();
    assert!(
        rested(after_listing(&asked, "example/shop", 0)),
        "a reset a day ahead rests the watch: {asked:?}"
    );
    // A couple of seconds either side, for the reader's clock read
    // between this case's two.
    let hour = 60 * 60;
    let within = (before + hour - 2..=after + hour + 2)
        .any(|until| said.contains(&format!("aren't listed until {},", stamp(until))));
    assert!(
        within,
        "a reset said a day ahead rests the watch an hour from now, not a day: {said}"
    );
}

#[test]
fn github_a_reset_already_past_rests_nothing() {
    let (_, said) = asked_until(
        watching(&["example/shop"], |nth, answered| {
            if nth > 0 {
                return answered;
            }
            Answered {
                headers: left("999", -10),
                ..answered
            }
        }),
        |asked| listed_at(asked, "example/shop").len() >= 2,
    );
    assert!(
        !said.contains("requests left"),
        "a reset already past rests nothing, so no rest is said: {said}"
    );
}

#[test]
fn github_the_watch_keeps_a_fifth_of_the_limit_github_says() {
    let (asked, said) = asked_until(
        watching(&["example/shop"], |nth, answered| {
            if nth > 0 {
                return answered;
            }
            let mut headers = left("2999", 3);
            headers.push(("x-ratelimit-limit", "15000".to_owned()));
            Answered {
                headers,
                ..answered
            }
        }),
        |asked| listed_at(asked, "example/shop").len() >= 2,
    );
    let between = after_listing(&asked, "example/shop", 0);
    assert!(
        rested(between),
        "2,999 left of 15,000 is under a fifth, so the watch rests: {between:?}"
    );
    assert!(
        said.contains("has 2999 requests left this hour, fewer than the 3000 kept"),
        "the rest names the fifth kept: {said}"
    );
}

#[test]
fn github_a_refusal_asking_to_wait_rests_the_watch_that_long() {
    let (asked, said) = asked_until(
        watching(&["example/shop"], |nth, answered| {
            if nth > 0 {
                return answered;
            }
            Answered {
                status: 429,
                headers: vec![("retry-after", "3".to_owned())],
                ..Answered::ok(json!({"message": "slow down"}))
            }
        }),
        |asked| listed_at(asked, "example/shop").len() >= 2,
    );
    let between = after_listing(&asked, "example/shop", 0);
    assert!(
        rested(between),
        "a 429 asking to wait three seconds rests the watch: {between:?}"
    );
    assert!(
        said.contains("asking to wait 3 seconds, so the newest comments aren't listed until"),
        "the rest is said, with why: {said}"
    );
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

//! Joel's own stop where GitHub's search indexed it late: heard though an
//! hour late, and past that, on a thread something changes since, told of
//! in the journal once a run, unless it was judged; a first start judges
//! none from before its look back, and a restart keeps to what the first
//! start judged and to how far back a failed search held it, which a site
//! read by what Joel opened never searches, so keeps nothing of; a search
//! giving fewer than it found holds its hour too, reading each it gave
//! once.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{
    CODE_SITE, LOGIN, MARK, answer_with, by_address, commented, github_reader, journal,
    read_lately, searches, until_asked,
};
use crate::{PATIENCE, answering, decisions, each, home, until_decided};

/// The key of Joel's comment in these cases, as the journal names it.
const HIS: &str = "github:example/shop#12:comment:5";

/// The key of Joel's comment quoting the safe word.
const QUOTED: &str = "github:example/shop#12:comment:6";

/// The key of Joel's stop from long before, as the stand-in gives it
/// however far back it's asked from, as GitHub gives a review.
const LONG_AGO: &str = "github:example/shop#12:comment:7";

/// Where only Joel spoke, on example/shop#12, `ago` seconds back: no
/// notification, and a search finding it, or where GitHub indexed it
/// `late`, only when asked from at or before then, as its search does.
fn only_joel_spoke_ago(line: &str, now: u64, ago: u64, late: bool) -> Value {
    if line.contains(" /notifications?") {
        return json!([]);
    }
    if line.contains(" /search/issues?") {
        // ISO stamps of one form sort as the times they name.
        let from = line
            .split("updated%3A%3E%3D")
            .nth(1)
            .and_then(|rest| rest.split(['&', ' ']).next())
            .map(|from| from.replace("%3A", ":"));
        if !late || from.is_some_and(|from| from <= stamp(now - ago)) {
            return json!({"items": [{"number": 12, "pull_request": {},
                "repository_url": "https://api.example.com/repos/example/shop"}]});
        }
        return json!({"items": []});
    }
    let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, ago));
    answer_with(line, now, &[his])
}

/// How many times the busy thread's stand-in has been searched.
static SEARCHED: AtomicU64 = AtomicU64::new(0);

/// Where Joel spoke ninety minutes back on example/shop#12, saying the
/// safe word as the judge reads it, though no plain phrase, and quoting
/// it, and said it three hours back, which someone changes before every
/// search, so each round reads it again.
fn busy_thread(line: &str, now: u64) -> Value {
    if line.contains(" /search/issues?") {
        let round = SEARCHED.fetch_add(1, Ordering::SeqCst);
        return json!({"items": [{"number": 12, "pull_request": {},
            "updated_at": stamp(now - 1000 + round),
            "repository_url": "https://api.example.com/repos/example/shop"}]});
    }
    if line.contains(" /notifications?") {
        return json!([]);
    }
    let ago = (now, 90 * 60);
    let his = commented(5, LOGIN, "OWNER", "Pineapple-Override", ago);
    let quoted = commented(6, LOGIN, "OWNER", "> pineapple override\n\nis it", ago);
    let long_ago = commented(7, LOGIN, "OWNER", "pineapple override", (now, 3 * 60 * 60));
    answer_with(line, now, &[his, quoted, long_ago])
}

/// When `line` asks for what was said since, as its stamp reads.
fn asked_since(line: &str) -> Option<String> {
    line.split("since=")
        .nth(1)
        .and_then(|rest| rest.split(['&', ' ']).next())
        .map(|since| since.replace("%3A", ":"))
}

/// The time now, in seconds since the epoch.
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
}

/// How many times the journal names Joel's comment as too old to judge.
fn told_of(said: &str) -> usize {
    said.lines()
        .filter(|line| line.contains(&format!("{HIS} holds Joel's pineapple override from")))
        .count()
}

/// Lets the reader run a round and more past what it has decided.
fn another_round(asked: &Mutex<Vec<String>>) {
    let rounds = asked.lock().expect("what was asked").len();
    until_asked(asked, rounds, 12);
}

#[test]
fn github_joels_stop_the_search_indexed_fifteen_minutes_late_is_heard() {
    let (origin, _) = answering(|line, now| only_joel_spoke_ago(line, now, 15 * 60, true));
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = decisions(&home);
    assert_eq!(
        each(&said, "verdict"),
        ["stop", "stop-asked"],
        "Joel's stop a search finds only from fifteen minutes back is heard"
    );
    assert_eq!(said[0]["by_joel"], true, "his own words");
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_joels_stop_older_than_the_hour_stops_nothing_and_is_told_of_once() {
    let (origin, asked) = answering(busy_thread);
    let (home, _) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 1);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(
            began.elapsed() < PATIENCE,
            "it never read the busy thread thrice"
        );
        // It asks every tenth of a second, so a short wait sees each round.
        thread::sleep(Duration::from_millis(50));
    }
    another_round(&asked);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked"],
        "Joel's stop from ninety minutes back is past what a round reaches"
    );
    let said = journal(&home);
    assert_eq!(
        told_of(&said),
        1,
        "the journal names his stop as too old once a run, read however often: {said}"
    );
    assert!(
        !said.contains(&format!("{QUOTED} holds")),
        "his words quoting the safe word, which the judge waves off, aren't named: {said}"
    );
    assert!(
        !said.contains(&format!("{LONG_AGO} holds")),
        "nor his stop from before the hour it reads, judged long since: {said}"
    );
    let said_then = stamp(now() - 90 * 60);
    let long_ago = stamp(now() - 3 * 60 * 60);
    let asked = asked.lock().expect("what was asked");
    let comments: Vec<&String> = asked
        .iter()
        .filter(|line| line.contains(" /repos/example/shop/issues/12/comments?"))
        .collect();
    assert!(
        comments.len() >= 2,
        "the busy thread is read each round it changes: {comments:?}"
    );
    assert!(
        comments
            .iter()
            .all(|line| asked_since(line)
                .is_some_and(|since| since <= said_then && since > long_ago)),
        "what a search found is read from an hour before what it reaches, so his stop \
         from then is seen, though not whole: {comments:?}"
    );
    drop(asked);
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_first_start_judges_joels_stop_from_inside_its_look_back() {
    let (origin, _) = answering(|line, now| only_joel_spoke_ago(line, now, 15 * 60, false));
    let home = home();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 2);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop", "stop-asked"],
        "a first start judges Joel's stop from fifteen minutes before it ran"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_first_start_judges_none_of_joels_words_from_before_its_look_back() {
    let (origin, asked) = answering(|line, now| only_joel_spoke_ago(line, now, 30 * 60, false));
    let home = home();
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 1);
    another_round(&asked);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    // The stand-in gives the thread to a search from any time, as one
    // changed since would be, the hard case for the floor.
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked"],
        "a first start doesn't judge Joel's stop from half an hour before it ran"
    );
    let mut again = github_reader(&home, &origin, &[]);
    another_round(&asked);
    again.kill().expect("the reader stops again");
    again.wait().expect("the reader is gone again");
    assert_eq!(
        each(&decisions(&home), "verdict"),
        ["stop-asked"],
        "restarted, it keeps to what its first start reaches: {}",
        journal(&home)
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_joels_stop_judged_already_is_never_told_of_as_stopping_nothing() {
    let (origin, asked) = answering(busy_thread);
    let (home, last) = read_lately();
    // His stop from ninety minutes back was judged when he said it.
    let judged = json!({"since": last, "keys": [HIS], "own": {"from": last, "floor": 0}});
    std::fs::write(home.join("state/pineapple").join(MARK), judged.to_string()).expect("the mark");
    let mut reading = github_reader(&home, &origin, &[]);
    until_decided(&home, 1);
    another_round(&asked);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let said = journal(&home);
    assert_eq!(
        told_of(&said),
        0,
        "a stop that stopped what it named isn't said to stop nothing: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_search_that_keeps_failing_keeps_its_hour_across_a_restart() {
    let (origin, asked) = answering(|line, now| {
        if line.contains(" /search/issues?") {
            return json!({"incomplete_results": true, "items": []});
        }
        only_joel_spoke_ago(line, now, 15 * 60, true)
    });
    let (home, last) = read_lately();
    let searched_twice = |from: usize| {
        let began = Instant::now();
        while searches(&asked).len() < from + 2 {
            assert!(began.elapsed() < PATIENCE, "it never searched twice");
            thread::sleep(Duration::from_millis(50));
        }
    };
    let mut reading = github_reader(&home, &origin, &[]);
    searched_twice(0);
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let before = searches(&asked).len();
    let said_before = journal(&home).len();
    let mut again = github_reader(&home, &origin, &[]);
    searched_twice(before);
    again.kill().expect("the reader stops again");
    again.wait().expect("the reader is gone again");

    let from = format!("updated%3A%3E%3D{}", stamp(last - 3600).replace(':', "%3A"));
    let searched = searches(&asked);
    assert!(
        searched[before].contains(&from),
        "restarted, it searches from the hour its failed search held back: {}",
        searched[before]
    );
    let warned = format!("so it's searched from {} again", stamp(last - 3600));
    let said = journal(&home);
    assert!(
        said[said_before..].contains(&warned),
        "restarted, its warning names the hour it's searched from, as the search does: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

/// Where a search for Joel's words found two threads but gave only
/// example/shop#12, never changed since, as GitHub gives at most a
/// thousand.
fn searched_short(line: &str, now: u64) -> Value {
    static CHANGED: AtomicU64 = AtomicU64::new(0);
    if line.contains(" /search/issues?") {
        let changed = CHANGED
            .compare_exchange(0, now - 1000, Ordering::SeqCst, Ordering::SeqCst)
            .map_or_else(|changed| changed, |_| now - 1000);
        return json!({"total_count": 2, "items": [{"number": 12, "pull_request": {},
            "updated_at": stamp(changed),
            "repository_url": "https://api.example.com/repos/example/shop"}]});
    }
    if line.contains(" /notifications?") {
        return json!([]);
    }
    answer_with(line, now, &[])
}

#[test]
fn github_a_search_that_gives_fewer_than_it_found_keeps_its_hour_and_reads_each_once() {
    let (origin, asked) = answering(searched_short);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never searched three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let from = format!("updated%3A%3E%3D{}", stamp(last - 3600).replace(':', "%3A"));
    let searched = searches(&asked);
    assert!(
        searched.iter().all(|it| it.contains(&from)),
        "a search cut short doesn't move on, as what it left out may hold his stop: \
         {searched:?}"
    );
    let warned = format!(
        "found 2 but gave 1, so it's searched from {} again",
        stamp(last - 3600)
    );
    let said = journal(&home);
    assert!(said.contains(&warned), "the journal says so: {said}");
    let comments = " /repos/example/shop/issues/12/comments?";
    let read = asked
        .lock()
        .expect("what was asked")
        .iter()
        .filter(|line| line.contains(comments))
        .count();
    assert_eq!(
        read, 1,
        "a thread it gave, unchanged since, is read once, so a flood isn't read every round"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_site_read_by_what_joel_opened_keeps_no_reach_of_a_search() {
    let (origin, _) = answering(by_address);
    let home = home();
    let mut reading = github_reader(&home, &origin, &CODE_SITE);
    let mark = home.join("state/pineapple/judged-code.json");
    let began = Instant::now();
    let kept: Value = loop {
        let kept = std::fs::read_to_string(&mark).ok();
        if let Some(kept) = kept.and_then(|kept| serde_json::from_str(&kept).ok()) {
            break kept;
        }
        assert!(began.elapsed() < PATIENCE, "it never kept how far it read");
        thread::sleep(Duration::from_millis(50));
    };
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    assert!(
        kept.get("own").is_none(),
        "a site that never searches for Joel's words keeps no reach, which would hold a \
         later search back to its first start: {kept}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

//! Joel's own stop where GitHub's search indexed it late: heard though an
//! hour late, and past that, on a thread something changes since, told of
//! in the journal once a run, unless it was judged; a first start judges
//! none from before its look back, and a restart keeps to what the first
//! start judged and to how far back a failed search held it, which a site
//! read by what Joel opened never searches, so keeps nothing of; a search
//! cut short is followed by one of what changed before the oldest it
//! gave, and moves on, saying what it left unread when that's cut too,
//! and from when it's searched again once the round moves on, and telling
//! Joel in one row however many rounds leave it, unless the first gives
//! none it can read, which holds.

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
use crate::{Answered, PATIENCE, answering, answering_whole, decisions, each, home, until_decided};

/// The key of Joel's comment in these cases, as the journal names it.
const HIS: &str = "github:example/shop#12:comment:5";

/// The key of Joel's comment quoting the safe word.
const QUOTED: &str = "github:example/shop#12:comment:6";

/// The key of Joel's stop from long before, as the stand-in gives it
/// however far back it's asked from, as GitHub gives a review.
const LONG_AGO: &str = "github:example/shop#12:comment:7";

/// From when a search of what changed since a time searches, as `line`
/// asks it of the stand-in.
fn searched_from(line: &str) -> Option<String> {
    // ISO stamps of one form sort as the times they name.
    line.split("updated%3A%3E%3D")
        .nth(1)
        .and_then(|rest| rest.split(['&', ' ']).next())
        .map(|from| from.replace("%3A", ":"))
}

/// Where only Joel spoke, on example/shop#12, `ago` seconds back: no
/// notification, and a search finding it, or where GitHub indexed it
/// `late`, only when asked from at or before then, as its search does.
fn only_joel_spoke_ago(line: &str, now: u64, ago: u64, late: bool) -> Value {
    if line.contains(" /notifications?") {
        return json!([]);
    }
    if line.contains(" /search/issues?") {
        if !late || searched_from(line).is_some_and(|from| from <= stamp(now - ago)) {
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

/// When the thread a cut search gives in [`cut_once`] last changed.
static CUT_ONCE: AtomicU64 = AtomicU64::new(0);

/// When the thread a cut search gives in [`cut_twice`] last changed.
static CUT_TWICE: AtomicU64 = AtomicU64::new(0);

/// When the thread a cut search gives in [`cut_then_failing`] last
/// changed.
static CUT_FAILING: AtomicU64 = AtomicU64::new(0);

/// A pull request of example/shop's, as a search gives it, last changed
/// `at`.
fn thread_at(number: u64, at: u64) -> Value {
    json!({"number": number, "pull_request": {}, "updated_at": stamp(at),
        "repository_url": "https://api.example.com/repos/example/shop"})
}

/// Where a search for Joel's words finds three threads but gives only
/// example/shop#12 and #11, newest first, changed when `changed` first
/// holds and 100 seconds before, and never since, as GitHub gives at most
/// a thousand. One of what changed before #11, which GitHub's range takes
/// in at both ends, finds `older` and gives #11 again and #13, 500
/// seconds before #12; with no `older`, it comes back incomplete.
fn searched_short(line: &str, now: u64, changed: &AtomicU64, older: Option<u64>) -> Value {
    if line.contains(" /search/issues?") {
        let changed = changed
            .compare_exchange(0, now - 1000, Ordering::SeqCst, Ordering::SeqCst)
            .map_or_else(|changed| changed, |_| now - 1000);
        let mid = thread_at(11, changed - 100);
        if !line.contains("..") {
            return json!({"total_count": 3, "items": [thread_at(12, changed), mid]});
        }
        return older.map_or_else(
            || json!({"incomplete_results": true, "total_count": 0, "items": []}),
            |all| json!({"total_count": all, "items": [mid, thread_at(13, changed - 500)]}),
        );
    }
    if line.contains(" /notifications?") {
        return json!([]);
    }
    answer_with(line, now, &[])
}

/// A search cut short whose search of what's older is whole.
fn cut_once(line: &str, now: u64) -> Value {
    searched_short(line, now, &CUT_ONCE, Some(2))
}

/// A search cut short whose search of what's older is cut short too.
fn cut_twice(line: &str, now: u64) -> Value {
    searched_short(line, now, &CUT_TWICE, Some(5))
}

/// When the thread a cut search gives in [`cut_twice_told`] last
/// changed.
static CUT_TOLD: AtomicU64 = AtomicU64::new(0);

/// A search cut short twice, as [`cut_twice`], for the row it leaves.
fn cut_twice_told(line: &str, now: u64) -> Value {
    searched_short(line, now, &CUT_TOLD, Some(5))
}

/// A search cut short whose search of what's older fails.
fn cut_then_failing(line: &str, now: u64) -> Value {
    searched_short(line, now, &CUT_FAILING, None)
}

/// When the thread a cut search gives in [`cut_twice_unread`] last
/// changed.
static CUT_UNREAD: AtomicU64 = AtomicU64::new(0);

/// When the thread a cut search gives in [`cut_then_empty`] last changed.
static CUT_EMPTY: AtomicU64 = AtomicU64::new(0);

/// When the thread a cut search gives in [`cut_twice_first`] last
/// changed.
static CUT_FIRST: AtomicU64 = AtomicU64::new(0);

/// A search cut short twice, as [`cut_twice`], for a reader's first start.
fn cut_twice_first(line: &str, now: u64) -> Value {
    searched_short(line, now, &CUT_FIRST, Some(5))
}

/// A search cut short twice, in a round that can never read #13.
fn cut_twice_unread(line: &str, now: u64) -> Answered {
    if line.contains(" /repos/example/shop/issues/13/comments?") {
        return Answered {
            status: 500,
            headers: Vec::new(),
            body: json!({"message": "Server Error"}),
        };
    }
    Answered::ok(searched_short(line, now, &CUT_UNREAD, Some(5)))
}

/// A search cut short whose search of what's older is cut too, giving no
/// thread it can read.
fn cut_then_empty(line: &str, now: u64) -> Value {
    if line.contains(" /search/issues?") && line.contains("..") {
        return json!({"total_count": 2, "items": []});
    }
    searched_short(line, now, &CUT_EMPTY, Some(2))
}

/// A search cut short that gives no thread it can read.
fn cut_unreadable(line: &str, now: u64) -> Value {
    if line.contains(" /search/issues?") {
        return json!({"total_count": 3, "items": []});
    }
    if line.contains(" /notifications?") {
        return json!([]);
    }
    answer_with(line, now, &[])
}

/// Where the round after the one that searched `searched` reaches, as its
/// first search names it, which the journal says what's left out is
/// searched again from.
fn next_reach(searched: &[String]) -> String {
    searched_from(&searched[2]).expect("the next round searches from a time")
}

/// How many times the comments of example/shop#`number` were asked for.
fn read(asked: &Mutex<Vec<String>>, number: u64) -> usize {
    let comments = format!(" /repos/example/shop/issues/{number}/comments?");
    let asked = asked.lock().expect("what was asked");
    asked.iter().filter(|line| line.contains(&comments)).count()
}

#[test]
fn github_a_search_cut_short_reads_what_changed_before_its_oldest_and_moves_on() {
    let (origin, asked) = answering(cut_once);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    // Two searches a round, so the fifth begins the third round, once the
    // second has read what it found.
    while searches(&asked).len() < 5 {
        assert!(began.elapsed() < PATIENCE, "it never searched five times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let reach = stamp(last - 3600).replace(':', "%3A");
    let oldest = stamp(CUT_ONCE.load(Ordering::SeqCst) - 100).replace(':', "%3A");
    let searched = searches(&asked);
    assert!(
        searched[0].contains(&format!("updated%3A%3E%3D{reach}"))
            && searched[1].contains(&format!("updated%3A{reach}..{oldest}")),
        "a search cut short is followed by one of what changed from its reach to the oldest \
         it gave, not its newest: {searched:?}"
    );
    assert!(
        searched[2].contains("updated%3A%3E%3D") && !searched[2].contains(&reach),
        "once what's older is whole, the round is, so the next searches from later: \
         {searched:?}"
    );
    assert_eq!(
        (read(&asked, 11), read(&asked, 12), read(&asked, 13)),
        (1, 1, 1),
        "each thread either search gave is read, and once, though both gave #11 and it's \
         unchanged since"
    );
    let said = journal(&home);
    assert!(
        !said.contains(", where the next round reaches"),
        "a search of what's older that's whole leaves nothing unread: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_search_cut_short_twice_says_what_it_left_unread_and_moves_on() {
    let (origin, asked) = answering(cut_twice);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never searched three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let reach = stamp(last - 3600);
    let oldest = stamp(CUT_TWICE.load(Ordering::SeqCst) - 500);
    let said = journal(&home);
    let searched = searches(&asked);
    let next = next_reach(&searched);
    let left = format!(
        "found 5 but gave 2, back to {oldest}, so threads it left out, changed from {reach} \
         to then, are searched again only from {next}, where the next round reaches"
    );
    assert!(
        said.contains(&left),
        "the journal names what's left unread, and from when the next round searches: {said}"
    );
    assert!(
        searched[2].contains("updated%3A%3E%3D")
            && !searched[2].contains(&reach.replace(':', "%3A")),
        "two searches a round, and the next moves on rather than hold every round back on a \
         flood that never leaves its search: {searched:?}"
    );
    assert_eq!(
        read(&asked, 13),
        1,
        "what the older search gave is read: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_search_cut_short_twice_tells_joel_in_one_row_however_many_rounds_leave_it() {
    let (origin, asked) = answering(cut_twice_told);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    // Two searches a round, so the fifth begins the third round, once two
    // have each left the oldest unread.
    while searches(&asked).len() < 5 {
        assert!(began.elapsed() < PATIENCE, "it never searched five times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");
    let shadow = home.join("state/pineapple/missed-github.json");
    let rows: Value = std::fs::read_to_string(&shadow)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default();
    let said = journal(&home);
    std::fs::remove_dir_all(&home).expect("the case's home goes");

    let (reach, oldest) = (last - 3600, CUT_TOLD.load(Ordering::SeqCst) - 500);
    let id = format!(
        "pineapple-github-missed-own-{}-{}",
        reach * 1000,
        oldest * 1000
    );
    let row = &rows["items"][0];
    assert_eq!(
        (
            rows["items"].as_array().map(Vec::len),
            &row["id"],
            &row["whole"]
        ),
        (Some(1), &json!(id), &json!(true)),
        "in shadow, in its own folder, one row spans what both rounds left out, from the \
         first's reach, as the two meet: {rows}"
    );
    let search = format!(
        "pineapple involves:{LOGIN} updated:{}..{}",
        stamp(reach),
        stamp(oldest)
    );
    assert!(
        row["steps"][0]
            .as_str()
            .is_some_and(|step| step.contains(&search))
            && row["title"] == "The Safe Word Missed Part of GitHub",
        "it names the search that finds what was left out: {rows}"
    );
    // A third round may have moved on before the reader stopped.
    assert!(
        said.matches("<5>wrote Joel's row in ").count() >= 2,
        "each round that left it out said so as it moved on, so the one row met both: {said}"
    );
}

#[test]
fn github_a_search_cut_short_twice_on_a_first_start_names_its_floor_as_the_next_reach() {
    let (origin, asked) = answering(cut_twice_first);
    // Nothing kept, so a first start: its floor, ten minutes before its
    // look back, is later than an hour before any round in this case.
    let home = home();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never searched three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let searched = searches(&asked);
    let next = next_reach(&searched);
    assert_eq!(
        searched_from(&searched[0]),
        Some(next.clone()),
        "in a first start's first hour the floor is the reach, round after round: {searched:?}"
    );
    let said = journal(&home);
    assert!(
        said.contains(&format!(
            "searched again only from {next}, where the next round reaches"
        )),
        "the journal names the floor the next round searches from: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_search_of_whats_older_that_fails_says_so_and_moves_on() {
    let (origin, asked) = answering(cut_then_failing);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never searched three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let reach = stamp(last - 3600);
    let oldest = stamp(CUT_FAILING.load(Ordering::SeqCst) - 100);
    let said = journal(&home);
    let searched = searches(&asked);
    let next = next_reach(&searched);
    let failed = format!("for what Joel said changed {reach}..{oldest} (");
    let left = format!(
        "), so threads the search before left out are searched again only from {next}, where \
         the next round reaches"
    );
    assert!(
        said.contains(&failed) && said.contains(&left),
        "the journal names the search that failed, and from when the next searches: {said}"
    );
    assert!(
        searched[2].contains("updated%3A%3E%3D")
            && !searched[2].contains(&reach.replace(':', "%3A")),
        "the next round moves on rather than hold every round back on a wide search that \
         may keep failing: {searched:?}"
    );
    assert_eq!(
        (read(&asked, 11), read(&asked, 12)),
        (1, 1),
        "what the first search gave is read: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_cut_search_in_a_round_that_cant_read_a_thread_moves_on_owing_it() {
    let (origin, asked) = answering_whole(cut_twice_unread);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    // Two searches a round, so the fifth begins the round after the second
    // read of what it owes.
    while searches(&asked).len() < 5 {
        assert!(began.elapsed() < PATIENCE, "it never searched five times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let searched = searches(&asked);
    let next = next_reach(&searched);
    assert_ne!(
        next,
        stamp(last - 3600),
        "a round that couldn't read a thread moves on, owing it: {searched:?}"
    );
    assert!(read(&asked, 13) >= 2, "the thread it owes is read again");
    let oldest = stamp(CUT_UNREAD.load(Ordering::SeqCst) - 500);
    let said = journal(&home);
    assert!(
        said.contains(&format!("found 5 but gave 2, back to {oldest},"))
            && said.contains(&format!(
                "searched again only from {next}, where the next round"
            )),
        "what the cut left out is named, from where the next round reaches: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_cut_search_of_whats_older_giving_none_it_can_read_moves_on() {
    let (origin, asked) = answering(cut_then_empty);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 3 {
        assert!(began.elapsed() < PATIENCE, "it never searched three times");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let reach = stamp(last - 3600);
    let oldest = stamp(CUT_EMPTY.load(Ordering::SeqCst) - 100);
    let searched = searches(&asked);
    let next = next_reach(&searched);
    assert_ne!(
        next, reach,
        "it moves on rather than hold every round back: {searched:?}"
    );
    let said = journal(&home);
    let left = format!(
        "found 2 but gave 0, back to {oldest}, so threads it left out, changed from {reach} \
         to then, are searched again only from {next}, where the next round reaches"
    );
    assert!(
        said.contains(&left),
        "it names what's left out, up to where the search before was cut: {said}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_search_cut_short_giving_none_it_can_read_holds_its_reach() {
    let (origin, asked) = answering(cut_unreadable);
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let began = Instant::now();
    while searches(&asked).len() < 2 {
        assert!(began.elapsed() < PATIENCE, "it never searched twice");
        thread::sleep(Duration::from_millis(50));
    }
    reading.kill().expect("the reader stops");
    reading.wait().expect("the reader is gone");

    let reach = stamp(last - 3600);
    let searched = searches(&asked);
    assert_eq!(
        searched_from(&searched[1]),
        Some(reach.clone()),
        "a search that can't say how far it got holds, rather than skip all it found: \
         {searched:?}"
    );
    let said = journal(&home);
    let held = format!(
        "found 3 but gave 0, none it could read, so it's searched from {reach} again next round"
    );
    assert!(said.contains(&held), "the journal says so: {said}");
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

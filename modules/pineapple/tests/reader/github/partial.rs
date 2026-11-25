//! A round that can't read every pull request and issue it named: it
//! judges what it did read, moves on, and owes the rest, read again from
//! where each was to be, less often while one keeps failing, merged with
//! a fresh listing by the farther back, and judged once; one GitHub says
//! is gone stays owed; a spent rate limit or three failures stop the
//! round's reads, search hits read first; and one failing in every try is
//! let go, the journal naming the span unread. How often one is asked, and
//! in what order, is owed.rs's own cases'.
//! Each case waits a round past what it counts, so stopping the reader
//! can't cut short a read it counts.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use pineapple::github::stamp;
use serde_json::{Value, json};

use super::{LOGIN, MARK, answer_with, commented, github_reader, journal, read_lately, searches};
use crate::{Answered, PATIENCE, answering_whole, decisions, each};

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

/// What a case's stand-in of GitHub answers.
#[derive(Default)]
struct Case {
    /// The pull requests Joel's search gives.
    found: &'static [u64],
    /// Those whose comments fail on every ask.
    failing: &'static [u64],
    /// Whether #12 is left out of his notifications.
    quiet: bool,
    /// When Joel said the safe word on #12, or a minute before each answer.
    said: Option<u64>,
    /// Which of the asks for #12's comments fails, counting from one.
    twelve_fails: Option<usize>,
    /// Which of them GitHub answers as gone, counting from one.
    twelve_gone: Option<usize>,
    /// How many of the first asks for comments a spent limit refuses.
    refused: usize,
}

/// A stand-in answering as `case` says, with #12's opening, a review on it
/// asking Joel to stop without the phrase, and his own stop there.
fn stand_in(case: Case) -> impl Fn(&str, u64) -> Answered + Send + 'static {
    let (comments, twelve) = (Arc::new(AtomicUsize::new(0)), Arc::new(AtomicUsize::new(0)));
    move |line, now| {
        if line.contains("/comments?") && comments.fetch_add(1, Ordering::SeqCst) < case.refused {
            return Answered {
                status: 403,
                headers: vec![("x-ratelimit-remaining", "0".to_owned())],
                body: json!({"message": "API rate limit exceeded"}),
            };
        }
        let failing = case
            .failing
            .iter()
            .any(|number| asks_comments_of(line, *number));
        let twelfth = asks_comments_of(line, 12).then(|| twelve.fetch_add(1, Ordering::SeqCst) + 1);
        if twelfth.is_some() && twelfth == case.twelve_gone {
            return Answered {
                status: 404,
                headers: Vec::new(),
                body: json!({"message": "Not Found"}),
            };
        }
        if failing || (twelfth.is_some() && twelfth == case.twelve_fails) {
            return Answered {
                status: 500,
                headers: Vec::new(),
                body: json!({"message": "Server Error"}),
            };
        }
        if line.contains(" /search/issues?") {
            return Answered::ok(found(case.found, now));
        }
        if case.quiet && line.contains(" /notifications?") {
            return Answered::ok(json!([]));
        }
        let ago = case.said.map_or(60, |at| now - at);
        let his = commented(5, LOGIN, "OWNER", "pineapple override", (now, ago));
        Answered::ok(answer_with(line, now, &[his]))
    }
}

/// Whether `line` asks for the comments of example/shop#`number`.
fn asks_comments_of(line: &str, number: u64) -> bool {
    line.contains(&format!(" /repos/example/shop/issues/{number}/comments?"))
}

/// Each time `asked` holds a request for the comments of
/// example/shop#`number`, from when it asked, as its stamp reads.
fn reads(asked: &[String], number: u64) -> Vec<String> {
    let from = |line: &String| {
        let since = line.split("since=").nth(1)?.split(['&', ' ']).next()?;
        Some(since.replace("%3A", ":"))
    };
    let of = asked.iter().filter(|line| asks_comments_of(line, number));
    of.map(|line| from(line).unwrap_or_default()).collect()
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

/// How far the reader at `home` has read, as its mark says.
fn mark(home: &std::path::Path) -> Value {
    let text = std::fs::read_to_string(home.join("state/pineapple").join(MARK)).expect("the mark");
    serde_json::from_str(&text).expect("the mark is JSON")
}

/// What the reader judged in `case` over `rounds` rounds, the verdicts
/// sorted so a saying judged twice shows, what it asked, its journal and
/// its mark, its case's home gone.
fn run(
    case: Case,
    rounds: usize,
    seeded: Option<Value>,
) -> (Vec<String>, Vec<String>, String, Value) {
    let (origin, asked) = answering_whole(stand_in(case));
    let (home, _) = read_lately();
    if let Some(seeded) = seeded {
        std::fs::write(home.join("state/pineapple").join(MARK), seeded.to_string())
            .expect("the mark");
    }
    let mut reading = github_reader(&home, &origin, &[]);
    // The round after the last counted begins with its search.
    let seen = until_searched(&asked, &mut reading, rounds + 1);
    let said = decisions(&home);
    let mut verdicts: Vec<String> = each(&said, "verdict")
        .into_iter()
        .map(str::to_owned)
        .collect();
    verdicts.sort();
    let (journal, mark) = (journal(&home), mark(&home));
    std::fs::remove_dir_all(&home).expect("the case's home goes");
    (verdicts, seen, journal, mark)
}

/// When the reader was last read, as [`read_lately`] marks it: half a
/// minute before now, to the second.
fn last() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs()
        - 30
}

#[test]
fn github_a_thread_that_keeps_failing_is_owed_asked_less_often_and_holds_back_nothing() {
    let (origin, asked) = answering_whole(stand_in(Case {
        found: &[14, 12],
        failing: &[14],
        ..Case::default()
    }));
    let (home, last) = read_lately();
    let mut reading = github_reader(&home, &origin, &[]);
    let seen = until_searched(&asked, &mut reading, 6);

    let said = decisions(&home);
    let mut verdicts = each(&said, "verdict");
    verdicts.sort_unstable();
    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "Joel's stop beside a thread that fails every round is judged, and once"
    );
    let notified: Vec<&String> = seen
        .iter()
        .filter(|line| line.contains(" /notifications?"))
        .collect();
    assert!(
        notified.first() != notified.last(),
        "how far it has read moves on though a thread fails: {notified:?}"
    );
    assert_eq!(
        reads(&seen, 14),
        [stamp(last - 7200), stamp(last - 7200), stamp(last - 7200)],
        "the thread it owes is asked in rounds one, two and four, as its wait doubles, each \
         time from where it was to be read: {seen:?}"
    );
    let kept = mark(&home);
    assert_eq!(
        (
            kept["owed"][0]["number"].as_u64(),
            kept["owed"][0]["tries"].as_u64(),
            kept.get("read_at")
        ),
        (Some(14), Some(3), None),
        "what it couldn't read is kept as owed, and the site isn't said to be read whole: {kept}"
    );
    std::fs::remove_dir_all(&home).expect("the case's home goes");
}

#[test]
fn github_a_spent_limit_stops_the_round_at_a_search_hit_and_what_it_owes_is_judged_once() {
    let case = Case {
        found: &[14, 15, 16],
        said: Some(last() - 50),
        refused: 3,
        ..Case::default()
    };
    let (verdicts, seen, said, _) = run(case, 4, None);

    let searched: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" /search/issues?"))
        .map(|(place, _)| place)
        .collect();
    let first: Vec<&String> = seen[searched[0]..searched[1]]
        .iter()
        .filter(|line| line.contains("/comments?"))
        .collect();
    assert!(
        first.len() == 1 && asks_comments_of(first[0], 14),
        "a spent limit stops the round at once, at the search's first hit, not at what \
         notified Joel: {seen:?}"
    );
    assert_eq!(
        (reads(&seen, 15).len(), reads(&seen, 16).len()),
        (1, 1),
        "what the limit kept back is read once it lifts: {seen:?}"
    );
    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "Joel's stop, which only where #12 was owed from reaches, is judged once the limit \
         lifts, and once"
    );
    assert!(
        said.contains("read 0 of the 4 it named, so the rest is read again later"),
        "the journal says what it owes: {said}"
    );
}

#[test]
fn github_a_round_stops_reading_at_its_third_failure_and_reads_what_it_kept_back_first() {
    let case = Case {
        found: &[14, 15, 16, 17, 12],
        failing: &[14, 15, 16, 17],
        ..Case::default()
    };
    let (verdicts, seen, _, _) = run(case, 2, None);

    let searched: Vec<usize> = seen
        .iter()
        .enumerate()
        .filter(|(_, line)| line.contains(" /search/issues?"))
        .map(|(place, _)| place)
        .collect();
    let first = seen[searched[0]..searched[1]]
        .iter()
        .filter(|line| line.contains("/comments?"))
        .count();
    assert_eq!(
        first, 3,
        "a round that fails three times asks no more: {seen:?}"
    );
    let second: Vec<&String> = seen[searched[1]..searched[2]]
        .iter()
        .filter(|line| line.contains("/comments?"))
        .collect();
    assert!(
        second
            .first()
            .is_some_and(|line| asks_comments_of(line, 17))
            && second.iter().any(|line| asks_comments_of(line, 12)),
        "the next round reads what it kept back before what keeps failing, the third to fail \
         counted as one that failed: {seen:?}"
    );
    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "and judges Joel's stop there, once"
    );
}

#[test]
fn github_a_thread_owed_and_notified_again_is_read_once_from_where_it_was_owed() {
    let case = Case {
        found: &[],
        said: Some(last() - 50),
        twelve_fails: Some(1),
        ..Case::default()
    };
    let (verdicts, _, _, _) = run(case, 3, None);

    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "a stop said before the next round's minute of overlap is judged, from where it was \
         owed, and once though the thread is both owed and notified"
    );
}

#[test]
fn github_a_thread_owed_that_github_says_is_gone_is_owed_still_and_read_from_where_it_was() {
    let case = Case {
        found: &[],
        said: Some(last() - 50),
        twelve_fails: Some(1),
        twelve_gone: Some(2),
        ..Case::default()
    };
    let (verdicts, _, _, _) = run(case, 3, None);

    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "a stop on one owed, then said to be gone, is judged once it reads, from where it was \
         owed"
    );
}

#[test]
fn github_a_stop_judged_before_its_thread_was_owed_is_read_again_and_not_judged_twice() {
    let last = last();
    let owed = json!({"repo": "example/shop", "number": 12, "pull": true,
        "from": last - 3000, "read_from": last - 3000, "at": last - 60});
    let judged = ["github:example/shop#12:comment:5"];
    let case = Case {
        found: &[],
        said: Some(last - 500),
        ..Case::default()
    };
    let seeded = json!({"since": last, "keys": judged, "owed": [owed]});
    let (verdicts, seen, _, _) = run(case, 1, Some(seeded));

    assert_eq!(
        reads(&seen, 12).first(),
        Some(&stamp(last - 3000)),
        "the thread is read from where it was owed, which alone reaches the stop: {seen:?}"
    );
    assert_eq!(
        verdicts,
        ["stop-asked"],
        "Joel's stop, judged before, isn't judged again"
    );
}

#[test]
fn github_an_owed_stop_judged_late_is_not_also_told_of_as_too_old() {
    let case = Case {
        found: &[12],
        quiet: true,
        said: Some(last() - 3590),
        twelve_fails: Some(1),
        ..Case::default()
    };
    let (verdicts, _, said, _) = run(case, 2, None);

    assert_eq!(
        verdicts,
        ["stop", "stop-asked"],
        "Joel's stop on a thread owed from the search's reach is judged"
    );
    assert!(
        !said.contains("holds Joel's pineapple override from"),
        "a stop it judges isn't also told of as older than the search reaches: {said}"
    );
}

#[test]
fn github_one_failing_in_every_try_is_let_go_naming_the_span_nobody_read() {
    let last = last();
    let owed = json!({"repo": "example/shop", "number": 14, "pull": true,
        "from": last - 60, "read_from": last - 60, "at": last - 3 * 60 * 60,
        "tries": 11, "skip": 0});
    let case = Case {
        found: &[],
        failing: &[14],
        quiet: true,
        ..Case::default()
    };
    let seeded = json!({"since": last, "keys": [], "owed": [owed]});
    let (_, seen, said, kept) = run(case, 2, Some(seeded));

    assert_eq!(
        (reads(&seen, 14).len(), kept.get("owed")),
        (1, None),
        "one failing in every try is owed and asked no more: {seen:?}"
    );
    let gone = format!(
        "<3>couldn't read github:example/shop#14 in 12 tries since {}, so what was said there \
         from {} on goes unjudged: look there for a stop",
        stamp(last - 3 * 60 * 60),
        stamp(last - 60)
    );
    assert!(
        said.contains(&gone),
        "the journal says so, loudly, naming the span: {said}"
    );
}

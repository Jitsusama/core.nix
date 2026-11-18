use std::cell::Cell;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc;
use std::time::Duration;

use jiff::Timestamp;
use jiff::tz::{self, TimeZone};

use super::*;

/// A shell running `script`: the build's own where it gives one, which
/// a build's sandbox has where it has no /bin/sh.
fn sh(script: &str) -> Command {
    let shell = std::env::var("STAND_IN_SHELL").unwrap_or_else(|_| "/bin/sh".to_owned());
    let mut sh = Command::new(shell);
    sh.args(["-c", script]);
    sh
}

/// A command writing `lines`, one to a line. printf given none still
/// writes an empty line, so for none it's a command that writes
/// nothing.
fn writing(lines: &[String]) -> Command {
    if lines.is_empty() {
        return sh(":");
    }
    let quoted: Vec<String> = lines.iter().map(|line| format!("'{line}'")).collect();
    sh(&format!("printf '%s\\n' {}", quoted.join(" ")))
}

fn numbered(number: u64, cursor: &str) -> String {
    format!(
        r#"{{"__CURSOR":"{cursor}","__SEQNUM_ID":"s","__SEQNUM":"{number}","__REALTIME_TIMESTAMP":"{number}"}}"#
    )
}

fn history(lines: &[String]) -> History {
    History::of(writing(lines)).expect("the shell starts")
}

#[test]
fn a_history_is_read_to_its_last_line() {
    let history = History::of(sh("printf 'a\\nb'")).expect("the shell starts");
    assert_eq!(
        history.collect::<Vec<_>>(),
        ["a\n", "b"],
        "a history lost a line, or its last"
    );
}

#[test]
fn a_history_the_replay_stops_reading_is_ended_and_waited_on() {
    // The shell says who it is, then writes far longer than the test
    // waits, and on once its pipe closes, so only a kill ends it in
    // time. A failed test leaves it to end by itself, its errors unsaid.
    // A history that only waited would hold its thread, and one that
    // didn't wait would leave it behind.
    let (dropped, done) = mpsc::channel();
    let reading = thread::spawn(move || {
        let writes = "i=0; while [ $i -lt 10000000 ]; do echo y; i=$((i+1)); done";
        let mut history = History::of(sh(&format!(
            "echo $$; exec 2>/dev/null; trap '' PIPE; {writes}"
        )))
        .expect("the shell starts");
        let pid = history.next().unwrap_or_default().trim().to_owned();
        drop(history);
        let _ = dropped.send(());
        pid
    });
    assert!(
        done.recv_timeout(Duration::from_secs(10)).is_ok(),
        "a history dropped was waited on without being ended"
    );
    let pid = reading.join().expect("the history was read");
    if cfg!(target_os = "linux") {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "a history dropped was left behind, as {pid}"
        );
    }
}

#[test]
fn the_replay_sends_the_day_in_order_then_each_follows_on_from_its_own_end() {
    let histories = vec![
        (history(&[numbered(1, "a1"), numbered(3, "a3")]), 0),
        (history(&[numbered(2, "b2")]), 1),
        (history(&[]), 2),
    ];
    let (send, heard) = mpsc::channel();
    replay(
        histories,
        |at, after| writing(&[format!("{at} after {}", after.unwrap_or("nothing"))]),
        &send,
    )
    .expect("every follower starts");
    drop(send);
    let heard: Vec<Option<String>> = heard.iter().collect();
    let day: Vec<&str> = heard
        .iter()
        .take(3)
        .flatten()
        .filter_map(|line| line.split('"').nth(3))
        .collect();
    assert_eq!(
        day,
        ["a1", "b2", "a3"],
        "the day wasn't sent first, in order"
    );
    let mut followed: Vec<&str> = heard.iter().skip(3).flatten().map(String::as_str).collect();
    followed.sort_unstable();
    assert_eq!(
        followed,
        ["0 after a3\n", "1 after b2\n", "2 after nothing\n"],
        "a follower didn't follow on from its own history's end"
    );
    assert_eq!(
        heard.iter().filter(|line| line.is_none()).count(),
        3,
        "a follower ended without a word, or twice"
    );
}

#[test]
fn the_replay_starts_no_follower_once_nothing_hears_it() {
    let (send, heard) = mpsc::channel();
    drop(heard);
    let made = Cell::new(0);
    let replayed = replay(
        vec![(history(&[numbered(1, "a1")]), ())],
        |(), _| {
            made.set(made.get() + 1);
            writing(&[])
        },
        &send,
    );
    assert!(
        replayed.is_ok() && made.get() == 0,
        "a follower was started with nothing to hear it"
    );
}

#[test]
fn a_follower_that_cant_follow_on_is_an_error_naming_it() {
    let (send, _heard) = mpsc::channel();
    let replayed = replay(
        vec![(history(&[numbered(1, "a1")]), ())],
        |(), _| {
            let mut follower = Command::new("/nonexistent/journalctl");
            follower.args(["--follow", "-k"]);
            follower
        },
        &send,
    );
    assert!(
        replayed.is_err_and(|error| error
            .to_string()
            .starts_with("/nonexistent/journalctl -k: ")),
        "a follower that couldn't start was let pass, or wasn't named"
    );
}

#[test]
fn a_history_and_its_follower_are_named_by_what_they_ask() {
    // One asked only shares a shared argument's start, and is kept.
    let asked = [
        "-p".to_owned(),
        "warning".to_owned(),
        "--output-x".to_owned(),
    ];
    let name = "journalctl -p warning --output-x".to_owned();
    assert_eq!(
        (
            named(&journal::reading(&asked, 1, 2)),
            named(&journal::following(&asked, Some("s=1"), 1)),
            named(&journal::following(&asked, None, 1)),
        ),
        (name.clone(), name.clone(), name),
        "a history or follower was named by what every one asks"
    );
}

#[test]
fn every_history_reads_from_the_days_start_to_now() {
    // 21:00 on 17 November, five hours behind UTC, where it's already
    // the 18th.
    let now = Timestamp::from_second(1_794_967_200)
        .expect("a time")
        .to_zoned(TimeZone::fixed(tz::offset(-5)));
    assert_eq!(
        day_of(&now).expect("the day has a start"),
        (1_794_891_600, 1_794_967_200),
        "the replay didn't read its own day, from its midnight to now"
    );
}

/// Every `None` `replaying` and its followers send, once all are done,
/// however the replay ends.
fn ends_of(follow_on: impl Fn(&usize, Option<&str>) -> Command) -> usize {
    let histories = vec![
        (history(&[numbered(1, "a1")]), 0),
        (history(&[numbered(2, "b2")]), 1),
    ];
    let (send, heard) = mpsc::channel();
    let _ = panic::catch_unwind(AssertUnwindSafe(|| {
        replaying(histories, follow_on, &send);
    }));
    drop(send);
    heard.iter().filter(Option::is_none).count()
}

#[test]
fn a_whole_replay_ends_once_for_each_follower_and_once_for_itself() {
    assert_eq!(
        ends_of(|_, _| writing(&[])),
        3,
        "a replay's ends didn't match what it counts"
    );
}

#[test]
fn a_replay_whose_follower_cant_follow_on_ends_every_count() {
    // The first follower ends at once, and says so as well; a real one
    // follows for good, so the replay's own three must come whole.
    let ends = ends_of(|at, _| {
        if *at == 0 {
            writing(&[])
        } else {
            Command::new("/nonexistent/journalctl")
        }
    });
    assert_eq!(
        ends, 4,
        "health would wait for good, with {ends} of 3 ends and the follower's"
    );
}

#[test]
fn a_replay_that_panics_ends_every_count() {
    assert_eq!(
        ends_of(|_, _| panic!("a stand-in replay's panic")),
        3,
        "a replay that panicked would leave health waiting for good"
    );
}

#[test]
fn a_history_that_finds_nothing_is_whole_only_if_it_asked_grep() {
    assert!(
        whole(true, Some(0), 5, false),
        "a history that succeeded wasn't whole"
    );
    assert!(
        whole(false, Some(1), 0, true),
        "a quiet day's grep that found nothing wasn't whole"
    );
    assert!(
        !whole(false, Some(1), 1, true),
        "a grep that failed after a line was taken as whole"
    );
    assert!(
        !whole(false, Some(1), 0, false),
        "a history that didn't ask grep and failed was taken as whole"
    );
    assert!(
        !whole(false, Some(2), 0, true) && !whole(false, None, 0, true),
        "a grep that failed or was killed before a line was taken as whole"
    );
}

#[test]
fn a_history_ending_in_failure_is_whole_only_as_a_grep_that_found_nothing() {
    let mut greps = sh("exit 1");
    greps.arg("--grep=x");
    let mut asked = History::of(greps).expect("the shell starts");
    assert_eq!(
        asked.next(),
        None,
        "a history that wrote nothing gave a line"
    );
    assert!(asked.ended(), "a grep that found nothing ended early");
    let mut failed = History::of(sh("exit 1")).expect("the shell starts");
    assert_eq!(
        failed.next(),
        None,
        "a history that wrote nothing gave a line"
    );
    assert!(!failed.ended(), "a history that failed was taken as whole");
    assert_eq!(
        failed.name,
        format!("{} -c exit 1", sh("").get_program().to_string_lossy()),
        "a history wasn't named by what it asked, for what health says of it"
    );
}

#[test]
fn a_history_that_cant_start_is_an_error_naming_it() {
    let mut history = Command::new("/nonexistent/journalctl");
    history.args(["--since=@1", "-k"]);
    assert!(
        History::of(history).is_err_and(|error| error
            .to_string()
            .starts_with("/nonexistent/journalctl -k: ")),
        "a history that couldn't start wasn't named"
    );
}

#[test]
fn every_history_and_follower_has_a_name_of_its_own() {
    let followed = journal::followed();
    let forms: [fn(&[String]) -> Command; 3] = [
        |asked| journal::reading(asked, 1, 2),
        |asked| journal::following(asked, Some("s=1"), 1),
        |asked| journal::following(asked, None, 1),
    ];
    for command in forms {
        let mut names: Vec<String> = followed
            .iter()
            .map(|asked| named(&command(asked)))
            .collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(
            names.len(),
            followed.len(),
            "two histories or followers share a name: {names:?}"
        );
    }
}

#[test]
fn each_source_reads_its_own_lines() {
    let journal = r#"{"__REALTIME_TIMESTAMP":"1","PRIORITY":"3","MESSAGE":"m"}"#;
    let unified = r#"{"timestamp":"2026-11-12 05:55:36.0-0500","messageType":"Error","eventType":"logEvent","processImagePath":"/usr/libexec/d","eventMessage":"m"}"#;
    assert!(
        Source::Journal.parse(journal).is_some() && Source::Journal.parse(unified).is_none(),
        "the journal reads the journal's lines alone"
    );
    assert!(
        Source::UnifiedLog.parse(unified).is_some() && Source::UnifiedLog.parse(journal).is_none(),
        "the unified log reads its own lines alone"
    );
}

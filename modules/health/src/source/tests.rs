use std::cell::{Cell, RefCell};
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::time::{Duration, Instant};

use jiff::Timestamp;
use jiff::tz::{self, TimeZone};

use super::*;

// systemd's journal, which a Mac doesn't have.
#[cfg(target_os = "linux")]
mod real_journal;

// The followers health ends as it ends.
mod followers;

/// Every saying of health's own and its mark, from every test at once.
static HEARD: Mutex<Vec<(u8, String)>> = Mutex::new(Vec::new());

/// Keep what `say_marked` says, at its `mark`.
pub(super) fn hear(words: &str, mark: u8) {
    // Only this push and the filter in `marks_of` run under the lock, and
    // neither panics, so no test's failure can poison it; were one to all
    // the same, taking it back would keep it from failing every saying
    // after it.
    HEARD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push((mark, words.to_owned()));
}

/// Check that `what`, each saying heard holding every one of `words`,
/// was said at least `times` and every time at `mark`, telling words never
/// heard, as reworded words are, from words said at another mark.
pub(crate) fn assert_marked(what: &str, words: &[&str], times: usize, mark: u8) {
    let marks = marks_of(words);
    assert!(
        marks.len() >= times,
        "{what} was heard {} times, not at least {times}; were its words changed?",
        marks.len()
    );
    assert!(
        marks.iter().all(|said| *said == mark),
        "{what} wasn't said at mark {mark} every time: {marks:?}"
    );
}

/// The mark of each saying heard so far holding every one of `words`,
/// which name the test's own, since the tests run at once.
pub(crate) fn marks_of(words: &[&str]) -> Vec<u8> {
    HEARD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .iter()
        .filter(|(_, said)| words.iter().all(|word| said.contains(word)))
        .map(|(mark, _)| *mark)
        .collect()
}

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
    let quoted: Vec<String> = lines
        .iter()
        .map(|line| format!("'{}'", line.replace('\'', r"'\''")))
        .collect();
    sh(&format!("printf '%s\\n' {}", quoted.join(" ")))
}

fn numbered(number: u64, cursor: &str) -> String {
    format!(
        r#"{{"__CURSOR":"{cursor}","__SEQNUM_ID":"s","__SEQNUM":"{number}","__REALTIME_TIMESTAMP":"{number}"}}"#
    )
}

/// Every line and end sent on `heard` until its last sender is dropped,
/// failing rather than waiting for good on one left behind, as health
/// would.
fn all_of(heard: &mpsc::Receiver<Option<String>>) -> Vec<Option<String>> {
    let mut all = Vec::new();
    loop {
        match heard.recv_timeout(Duration::from_secs(10)) {
            Ok(said) => all.push(said),
            Err(mpsc::RecvTimeoutError::Disconnected) => return all,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                panic!("a sender was left behind, so health would wait on it for good: {all:?}")
            }
        }
    }
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
        None,
        |at, after| writing(&[format!("{at} after {}", after.unwrap_or("nothing"))]),
        &send,
        &Children::default(),
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
        None,
        |(), _| {
            made.set(made.get() + 1);
            writing(&[])
        },
        &send,
        &Children::default(),
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
        None,
        |(), _| {
            let mut follower = Command::new("/nonexistent/journalctl");
            follower.args(["--follow", "-k"]);
            follower
        },
        &send,
        &Children::default(),
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
            named(&journal::reading(&asked, 1)),
            named(&journal::following(&asked, Some("s=1"), 1)),
            named(&journal::following(&asked, None, 1)),
        ),
        (name.clone(), name.clone(), name),
        "a history or follower was named by what every one asks"
    );
}

#[test]
fn every_history_reads_from_the_days_start() {
    // 21:00 on 17 November, five hours behind UTC, where it's already
    // the 18th.
    let now = Timestamp::from_second(1_794_967_200)
        .expect("a time")
        .to_zoned(TimeZone::fixed(tz::offset(-5)));
    assert_eq!(
        day_of(&now).expect("the day has a start"),
        1_794_891_600,
        "the replay didn't read its own day, from its midnight"
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
        replaying(histories, None, follow_on, &send, &Children::default());
    }));
    drop(send);
    heard.iter().filter(Option::is_none).count()
}

#[test]
fn a_whole_replay_sends_no_end_of_its_own() {
    // Each stand-in follower ends at once and sends its end; a whole
    // replay's own would end health ten minutes into every start.
    assert_eq!(
        ends_of(|_, _| writing(&[])),
        2,
        "a whole replay sent an end of its own, or a follower's went unsent"
    );
}

#[test]
fn a_replay_whose_follower_cant_follow_on_sends_every_followers_end() {
    // The first follower ends at once, and says so as well; a real one
    // follows for good, so both followers' ends must come whole from the
    // replay.
    let ends = ends_of(|at, _| {
        if *at == 0 {
            writing(&[])
        } else {
            Command::new("/nonexistent/follow-on-journalctl")
        }
    });
    assert_eq!(
        ends, 3,
        "with {ends} of 2 ends and the follower's, a follower that couldn't follow on would go \
         unheard, with no start again, until another ended"
    );
    // Said as a failure, which health ends on.
    assert_marked(
        "a follower that couldn't follow on",
        &["couldn't follow on from the day's replay: /nonexistent/follow-on-journalctl"],
        1,
        ERROR,
    );
}

#[test]
fn a_replay_that_panics_sends_every_followers_end() {
    assert_eq!(
        ends_of(|_, _| panic!("a stand-in replay's panic")),
        2,
        "a replay that panicked didn't send every follower's end, which health hears at once \
         rather than only once every sender is gone"
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
    // Said as a warning, which health runs on through.
    assert_marked(
        "a history ended early",
        &["history asking ", " -c exit 1 ended early"],
        1,
        WARNING,
    );
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
        |asked| journal::reading(asked, 1),
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
fn the_journals_last_line_is_where_journald_numbered_its_last() {
    // Where it was numbered, and what health said of it.
    let told = |newest: Command| {
        let name = named(&newest);
        let said = asking(newest).expect("the shell starts").wait_with_output();
        let notices = RefCell::new(Vec::new());
        let last = last_of(&name, said, |words| {
            notices.borrow_mut().push(words.to_owned());
        });
        (last, notices.into_inner())
    };
    let none: Vec<String> = Vec::new();
    assert_eq!(
        told(writing(&[numbered(1, "a"), numbered(7, "b")])),
        (Some(("s".to_owned(), 7)), none.clone()),
        "the last line's number wasn't the end, or was said as none"
    );
    assert_eq!(
        told(writing(&[])),
        (None, none),
        "a journal with no lines had an end, or was said as a fault"
    );
    // Its line is numbered, so only its failure keeps it from the end.
    let (last, notices) = told(sh(&format!(
        "printf '%s\\n' '{}'; exit 1",
        numbered(7, "b")
    )));
    assert!(
        last.is_none() && notices.len() == 1 && notices[0].contains("exit status: 1"),
        "a journalctl that failed gave an end, or wasn't said with how it ended: {notices:?}"
    );
    let (last, notices) = told(writing(&["not json".to_owned()]));
    assert!(
        last.is_none() && notices.len() == 1 && notices[0].contains("didn't number"),
        "a line journald didn't number gave an end, or wasn't said: {notices:?}"
    );
    let notices = RefCell::new(Vec::new());
    let last = last_of("newest", Err(io::Error::other("gone")), |words| {
        notices.borrow_mut().push(words.to_owned());
    });
    let notices = notices.into_inner();
    assert!(
        last.is_none()
            && notices.len() == 1
            && notices[0].contains("newest")
            && notices[0].contains("gone"),
        "a journalctl that couldn't be read gave an end, or wasn't said: {notices:?}"
    );
    assert!(
        last_written(Command::new("/nonexistent/journalctl"))
            .is_err_and(|error| error.to_string().starts_with("/nonexistent/journalctl: ")),
        "a journalctl that couldn't start wasn't an error naming it"
    );
    // Said as a warning, which health runs on through.
    assert!(
        last_written(sh("exit 7")).is_ok_and(|last| last.is_none()),
        "a journalctl that failed gave an end"
    );
    assert_marked(
        "a journalctl that failed giving its last line",
        &[" -c exit 7 ended, ", "may be heard out of order"],
        1,
        WARNING,
    );
}

#[test]
fn the_replay_asks_the_journals_last_line_first_off_healths_own_thread() {
    let folder = tempfile::tempdir().expect("a folder");
    let answered = folder.path().join("answered");
    // The path is the script's argument, not its words, so no quote in
    // it can break the script.
    let mut newest = sh("sleep 1; : > \"$1\"");
    newest.arg("sh").arg(&answered);
    let started = Arc::new(Mutex::new(Vec::new()));
    let reading = {
        let started = Arc::clone(&started);
        move |_: &[String]| {
            started
                .lock()
                .expect("the starts are kept")
                .push(answered.exists());
            writing(&[])
        }
    };
    let (send, heard) = mpsc::channel();
    let asked = Instant::now();
    let count = replayed_by(
        newest,
        reading,
        |_, _| writing(&[]),
        &send,
        &Children::default(),
    );
    assert!(
        asked.elapsed() < Duration::from_millis(500),
        "health's start waited on the journal's last line"
    );
    drop(send);
    assert_eq!(
        all_of(&heard).iter().filter(|said| said.is_none()).count(),
        count,
        "a follower ended without a word, or twice"
    );
    let started = started.lock().expect("the starts are kept");
    assert_eq!(
        started.len(),
        journal::followed().len(),
        "a history didn't start"
    );
    assert!(
        started.iter().all(|answered| *answered),
        "a history started before the journal's last line was answered, so could end before it"
    );
}

#[test]
fn a_replay_that_cant_start_sends_every_followers_end() {
    // Said as a failure, which health ends on, each way it can't, each
    // named for its way so the marks read are its own.
    let unstartable = || Command::new("/nonexistent/reading-journalctl");
    for (newest, history) in [
        (Command::new("/nonexistent/newest-journalctl"), None),
        (writing(&[]), Some(unstartable as fn() -> Command)),
    ] {
        let (send, heard) = mpsc::channel();
        let count = replayed_by(
            newest,
            move |_| history.map_or_else(|| writing(&[]), |start| start()),
            |_, _| writing(&[]),
            &send,
            &Children::default(),
        );
        drop(send);
        let heard = all_of(&heard);
        assert!(count > 0, "the replay said it starts no followers");
        assert_eq!(
            heard,
            vec![None; count],
            "a replay that couldn't start didn't send every follower's end, so health would rest \
             on its every sender being gone"
        );
    }
    for way in ["newest", "reading"] {
        assert_marked(
            &format!("a replay that couldn't start its {way}"),
            &[&format!(
                "the day's replay couldn't start: /nonexistent/{way}-journalctl"
            )],
            1,
            ERROR,
        );
    }
}

#[test]
fn a_replay_that_panics_as_it_starts_sends_every_followers_end() {
    let (send, heard) = mpsc::channel();
    let count = replayed_by(
        writing(&[]),
        |_| panic!("a stand-in history's panic"),
        |_, _| writing(&[]),
        &send,
        &Children::default(),
    );
    drop(send);
    let heard = all_of(&heard);
    assert!(count > 0, "the replay said it starts no followers");
    assert_eq!(
        heard,
        vec![None; count],
        "a replay that panicked as it started didn't send every follower's end, so health would \
         rest on its every sender being gone"
    );
}

#[test]
fn a_warning_or_failure_is_marked_one_only_in_the_journal() {
    assert_eq!(
        as_marked("health: w", true, WARNING),
        "<4>health: w",
        "a warning into the journal wasn't marked one"
    );
    assert_eq!(
        as_marked("health: w\nand on", true, WARNING),
        "<4>health: w\n<4>and on",
        "a warning's second line into the journal wasn't marked one"
    );
    assert_eq!(
        as_marked("health: f", true, ERROR),
        "<3>health: f",
        "a failure into the journal wasn't marked one"
    );
    assert_eq!(
        as_marked("health: n", true, NOTICE),
        "<5>health: n",
        "a notice into the journal wasn't marked one"
    );
    assert_eq!(
        as_marked("health: w", false, WARNING),
        "health: w",
        "a warning elsewhere carried the journal's mark"
    );
}

#[test]
fn only_a_macs_log_show_ends_whole_by_design() {
    let [stream, show] = unified::commands("midnight", "now")
        .try_into()
        .expect("a Mac streams and shows");
    assert!(
        !follows(&show),
        "log show's whole end would end health ten minutes into every start"
    );
    assert!(
        follows(&stream),
        "log stream, which follows for as long as health runs, could end unsaid"
    );
    for asked in journal::followed() {
        assert!(
            follows(&journal::following(&asked, None, 1)),
            "{asked:?}'s follower, which follows for as long as health runs, could end unsaid"
        );
    }
}

#[test]
fn a_follower_ending_is_said_by_name_unless_it_ends_whole_by_design() {
    // A journal's follower follows for as long as health runs; a Mac's log
    // show reads the day and ends whole by design.
    let follower = |waited| follower_ended("journalctl -k", waited, true);
    let reader = |waited| follower_ended("log show", waited, false);
    let again = |said: &str| {
        said.contains("starts again with every follower")
            && said.contains(&format!("{} minutes", follow::SETTLED.as_secs() / 60))
    };
    assert_eq!(
        reader(sh(":").status()),
        None,
        "a day's reader that ended whole by design was said for ending so"
    );
    assert!(
        follower(sh(":").status())
            .is_some_and(|said| said.contains("journalctl -k") && again(&said)),
        "a follower that ended whole wasn't said by name with health starting again, and when"
    );
    assert!(
        follower(sh("exit 3").status()).is_some_and(|said| said.contains("journalctl -k")
            && said.contains("exit status: 3")
            && again(&said)),
        "a follower ending in failure wasn't said by name and status with health starting again"
    );
    assert!(
        follower(sh("kill -9 $$").status())
            .is_some_and(|said| said.contains("journalctl -k") && said.contains("signal: 9")),
        "a follower ended by a signal wasn't said by name and signal"
    );
    assert!(
        follower(Err(io::Error::other("gone"))).is_some_and(|said| said.contains("journalctl -k")
            && said.contains("gone")
            && again(&said)),
        "a follower that couldn't be waited on wasn't said with health starting again"
    );
    assert!(
        reader(sh("exit 3").status()).is_some_and(|said| said.contains("log show")
            && said.contains("exit status: 3")
            && again(&said)),
        "a day's reader that failed wasn't said by name and status with health starting again"
    );
    assert!(
        reader(Err(io::Error::other("gone")))
            .is_some_and(|said| said.contains("log show") && said.contains("gone") && again(&said)),
        "a day's reader that couldn't be waited on wasn't said with health starting again"
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_macs_log_that_cant_start_is_an_error_naming_it() {
    // Off a Mac there's no /usr/bin/log, so its first command can't start.
    let now = Timestamp::from_second(1_763_467_200)
        .expect("a moment")
        .to_zoned(TimeZone::UTC);
    let first = named(&unified::commands("2025-11-18 00:00:00", "2025-11-18 12:00:00")[0]);
    let (send, _heard) = mpsc::channel();
    assert!(
        Source::UnifiedLog
            .start(&now, &send, &Followers::default())
            .is_err_and(|error| error.to_string().starts_with(&format!("{first}: "))),
        "a Mac's log that couldn't start wasn't named"
    );
}

#[test]
fn a_follower_that_ends_is_said_before_its_end_is_sent() {
    // What's said goes on the follower's own channel, so its order with
    // the follower's end is the order they were sent in: health may end
    // at a follower's end, so a word sent after it could go unsaid.
    for script in ["echo a", "echo a; exit 3"] {
        let (send, heard) = mpsc::channel();
        let tell = send.clone();
        spawn_saying(
            sh(script),
            send,
            true,
            move |words| {
                let _ = tell.send(Some(format!("said: {words}")));
            },
            &Children::default(),
        )
        .expect("the shell starts");
        let heard = all_of(&heard);
        let name = named(&sh(script));
        assert!(
            matches!(
                heard.as_slice(),
                [Some(line), Some(said), None]
                    if line == "a\n" && said.starts_with("said: ") && said.contains(&name)
            ),
            "{name} ended and wasn't said by name before its end was sent: {heard:?}"
        );
    }
}

#[test]
fn a_reader_that_ends_whole_by_design_sends_an_end_only_once_it_fails() {
    // A Mac's log show reads the day and ends, so its whole end mustn't
    // count as a follower's, which would end health ten minutes into every
    // start; one that fails sends one, so the day is read again.
    for (script, failed) in [("echo a", false), ("echo a; exit 3", true)] {
        let (send, heard) = mpsc::channel();
        let tell = send.clone();
        spawn_saying(
            sh(script),
            send,
            false,
            move |words| {
                let _ = tell.send(Some(format!("said: {words}")));
            },
            &Children::default(),
        )
        .expect("the shell starts");
        let heard = all_of(&heard);
        let ok = match heard.as_slice() {
            [Some(line)] => line == "a\n" && !failed,
            [Some(line), Some(said), None] => line == "a\n" && said.starts_with("said: ") && failed,
            _ => false,
        };
        assert!(
            ok,
            "a reader that ends whole by design sent an end, or its failure went unsaid or sent \
             none: {heard:?}"
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

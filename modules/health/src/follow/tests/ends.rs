//! When health's run ends: once a follower has ended and it has run long
//! enough, never while its followers are only quiet, on a start that
//! fails, and on a stop in the day's replay; and how often it ticks on a
//! busy journal.

use super::*;

#[test]
fn a_follower_that_ends_ends_health_once_it_has_run_long_enough() {
    // The end comes first and every look finds a minute gone, so health
    // has run long enough a few lines later, within a loop of two looks.
    // Until then it still takes what the other followers say.
    let (unread, ran, _) = Ran {
        ends: 1,
        after: 30,
        step: Duration::from_mins(1),
        ..Ran::default()
    }
    .ran();
    assert!(
        unread < 30,
        "health stopped taking the other followers' lines once one had ended"
    );
    assert!(
        ran >= SETTLED && ran <= SETTLED + Duration::from_mins(2),
        "health ran {ran:?} after a follower ended, so it forgot it had"
    );
}

#[test]
fn a_follower_that_ends_after_health_has_run_long_enough_ends_it_at_once() {
    // As a journalctl dying hours in: 30 lines at two looks a line, each
    // a minute on, so the end is read 61 minutes in, and health ends
    // within a few looks of it.
    let (unread, ran, _) = Ran {
        before: 30,
        ends: 1,
        held_for: SETTLED * 11,
        step: Duration::from_mins(1),
        ..Ran::default()
    }
    .ran();
    assert!(
        unread == 0 && ran <= Duration::from_mins(61 + 3),
        "health ran {ran:?}, leaving {unread} unread, after a follower ended an hour in"
    );
}

#[test]
fn health_ends_once_it_has_run_long_enough_while_its_other_followers_are_quiet() {
    // As on a journal, one follower ends and the rest hold their senders
    // and say nothing. Every look finds a half minute gone, so health
    // never waits on the quiet ones in real time, and a loop of two looks
    // can carry it a minute past. It still looks at the lid every loop.
    let (_, ran, looks) = Ran {
        ends: 1,
        held_for: SETTLED * 11,
        step: EVERY,
        ..Ran::default()
    }
    .ran();
    assert!(
        ran >= SETTLED && ran <= SETTLED + EVERY * 2 && looks >= 9,
        "health ran {ran:?}, looking {looks} times, after a follower ended while the others \
         were quiet"
    );
}

#[test]
fn quiet_followers_never_end_health() {
    // Every follower holds its sender and says nothing for half an hour,
    // health hearing only that nothing came, then they all go.
    let (_, ran, _) = Ran {
        held_for: SETTLED * 3,
        step: EVERY,
        ..Ran::default()
    }
    .ran();
    assert!(
        ran >= SETTLED * 3,
        "health ended {ran:?} in, while its followers were only quiet"
    );
}

#[test]
fn health_runs_on_while_no_follower_ends() {
    let (unread, _, _) = Ran {
        before: 30,
        step: Duration::from_mins(1),
        ..Ran::default()
    }
    .ran();
    assert_eq!(
        unread, 0,
        "health ended past ten minutes though no follower had"
    );
}

#[test]
fn health_ends_no_sooner_than_it_has_run_long_enough_however_its_followers_end() {
    // Each with no time gone as lines are read, and every sender gone
    // once they are: no follower ending, as when the replay's thread lets
    // go of its senders having sent no end; one ending; and two, as when
    // the replay can't start. The last wait is cut short to end health on
    // time, though the oversleeping waits leave it off the half minute,
    // and health still looks about every half minute as it waits, 19
    // times as the waits fall.
    for ends in [0, 1, 2] {
        let (unread, ran, looks) = Ran {
            ends,
            after: 3,
            ..Ran::default()
        }
        .ran();
        assert!(
            unread == 0 && ran >= SETTLED && ran <= SETTLED + OVERSLEPT && looks >= 18,
            "health ran {ran:?}, leaving {unread} unread and looking {looks} times as it waited, \
             after {ends} followers ended at its start"
        );
    }
}

#[test]
fn a_stop_before_the_first_tick_writes_no_day() {
    // A line a second, as the day's replay sends them, and a stop at the
    // clock's tenth look, four lines in and well within the half minute,
    // whose day so far would be written over any whole one a last run
    // kept on disk.
    let Outcome {
        result,
        unread,
        today,
        ..
    } = Ran {
        before: 30,
        step: Duration::from_secs(1),
        stops: 10,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.is_ok() && unread == 26 && today.is_none(),
        "a stop in the replay ended the run with {unread} unread, writing a day: {}",
        today.is_some()
    );
}

#[test]
fn a_stop_cuts_short_a_wait_for_a_line_from_a_sender_it_leaves() {
    // As the day's replay holds its senders past a stop while its merge
    // sends nothing, so only the run's own looks for a stop end it, in
    // real time, well within the half minute it would wait for a line.
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let (send, read) = mpsc::channel();
    let start = Instant::now();
    let result = run(
        &mut follower,
        &read,
        || (Instant::now(), wall(start.elapsed().as_secs())),
        |_| {},
        Looked::default,
        || start.elapsed() >= Duration::from_millis(100),
        &|_| {},
        &|_| {},
    );
    let took = start.elapsed();
    drop(send);
    assert!(
        result.is_ok() && took < Duration::from_secs(2),
        "a stop waited {took:?} for a line from a sender it left: {result:?}"
    );
}

#[test]
fn a_start_that_fails_is_said_at_once_and_taken_as_an_end() {
    // On a Mac, log stream may start and log show then fail, so a follower
    // still holds its sender: only the end sent here ends health.
    let (lines, read) = mpsc::channel();
    let said = RefCell::new(Vec::new());
    let say = |words: &str| said.borrow_mut().push(words.to_owned());
    let ended = ended_by(Err(io::Error::other("log show: no room")), &lines, say);
    let heard: Vec<_> = read.try_iter().collect();
    assert!(
        heard == [None]
            && ended.to_string() == "its followers couldn't start: log show: no room"
            && said
                .borrow()
                .iter()
                .any(|words| words.contains("log show: no room")
                    && words.contains("starts again with every follower")),
        "a failed start wasn't said with what follows, taken as an end, or named in the exit: \
         {heard:?}, {ended}, {said:?}"
    );
    let ended = ended_by(Ok(()), &lines, say);
    assert!(
        read.try_iter().next().is_none()
            && ended.to_string() == "a follower ended, or the day's replay failed",
        "a start that worked sent an end, or ended health saying otherwise: {ended}"
    );
}

#[test]
fn a_busy_journal_still_ticks_every_half_minute() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let (lines, read) = mpsc::channel();
    for second in 0..9 {
        lines
            .send(Some(warning(NOON + second)))
            .expect("a line sent");
    }
    lines.send(None).expect("its end sent");
    drop(lines);
    // Each look at the clock finds ten seconds gone, so the lines come
    // far faster than the half minute a quiet journal would wait.
    let start = Instant::now();
    let mut asked = 0;
    let now = || {
        asked += 1;
        (
            start + Duration::from_secs(10 * asked),
            NOON + wall(10 * asked),
        )
    };
    // Only the looks taken as the lines are read count, not those taken
    // as health waits out its start once its follower has ended.
    let waiting = Cell::new(false);
    let mut looks = 0;
    run(
        &mut follower,
        &read,
        now,
        |_| waiting.set(true),
        || {
            if !waiting.get() {
                looks += 1;
            }
            Looked::default()
        },
        || false,
        &|_| {},
        &|_| {},
    )
    .expect("the run");
    assert!(
        looks >= 3,
        "looked {looks} times over ten lines a hundred seconds apart"
    );
}

//! A spell of failed writes, as a flapping disk's, on its own and as a run
//! keeps it: what of it health says, and when, on the awake clock and the
//! wall's.

use super::*;

/// What a spell said and told, at each of `tries`: a write's error, or
/// none, so many seconds in on the awake clock and on the wall's.
fn heard(tries: &[(u64, u64, Option<&io::Error>)]) -> Vec<(u64, &'static str, String)> {
    let step = Cell::new(0);
    let heard = RefCell::new(Vec::new());
    let say = |words: &str| {
        heard
            .borrow_mut()
            .push((step.get(), "said", words.to_owned()));
    };
    let notice = |words: &str| {
        heard
            .borrow_mut()
            .push((step.get(), "told", words.to_owned()));
    };
    let first = Instant::now();
    let mut spell = Spell::default();
    for &(after, on_the_wall, error) in tries {
        step.set(after);
        spell.tried(
            error,
            first + Duration::from_secs(after),
            wall(on_the_wall),
            Path::new("days"),
            &say,
            &notice,
        );
    }
    heard.take()
}

#[test]
fn a_flapping_disk_is_said_at_most_once_in_ten_minutes() {
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let unreadable = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
    let tries = [
        // A short spell, said and told.
        (0, Some(&full)),
        (30, None),
        // A flap soon after goes unsaid, and so does the next, until ten
        // minutes have passed since the end told, when it's said with how
        // long it has lasted, its error then, not its first, the folder's
        // and no longer the disk's, and the short spell unsaid before it.
        (60, Some(&full)),
        (90, None),
        (120, Some(&full)),
        (600, Some(&unreadable)),
        (630, Some(&unreadable)),
        (690, None),
        // An unsaid flap's end doesn't hold off the next spell: ten
        // minutes from the end told, one is said at once, with it.
        (900, Some(&full)),
        (930, None),
        (1290, Some(&full)),
    ]
    .map(|(after, error)| (after, after, error));
    let heard = heard(&tries);
    let when: Vec<(u64, &str)> = heard.iter().map(|(at, kind, _)| (*at, *kind)).collect();
    assert_eq!(
        when,
        [
            (0, "said"),
            (30, "told"),
            (630, "said"),
            (690, "told"),
            (1290, "said")
        ],
        "a flapping disk was said more than once in ten minutes, or a spell at the wrong time"
    );
    let words = |at: usize| heard[at].2.as_str();
    assert!(
        words(1).ends_with("after trying for less than a minute")
            && words(2).contains(
                ": permission denied, having tried for 8 minutes, and a short spell before it \
                 went unsaid, so"
            )
            && words(2).contains("for about 52 minutes")
            && words(3).ends_with("after trying for 9 minutes")
            && words(4)
                .contains("no space left on device, and a short spell before it went unsaid, so")
            && words(4).contains("for about an hour"),
        "a spell's cause, how long it lasted, the time it has left or the short spells before \
         it went unsaid: {heard:?}"
    );
}

#[test]
fn short_spells_gone_unsaid_are_counted_in_the_next_said() {
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let mut tries = vec![(0, Some(&full)), (30, None)];
    for flap in 1..=3 {
        tries.extend([(flap * 60, Some(&full)), (flap * 60 + 30, None)]);
    }
    tries.push((630, Some(&full)));
    let tries: Vec<_> = tries
        .into_iter()
        .map(|(after, error)| (after, after, error))
        .collect();
    let heard = heard(&tries);
    assert!(
        heard.len() == 3
            && heard[2]
                .2
                .contains("device, and 3 short spells before it went unsaid, so"),
        "the short spells kept unsaid weren't counted in the next said: {heard:?}"
    );
}

#[test]
fn either_clock_ends_a_flapping_disks_quiet() {
    // The awake clock stands still while the machine sleeps, so a wake on
    // a disk still full is a minute on there and twenty on the wall; the
    // wall clock may be set back, too, an hour here. Eleven minutes awake
    // end it as well, though the wall was set back to five minutes on.
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    for (awake, on_the_wall, started_on_the_wall) in [(60, 1200, 0), (60, 0, 3600), (660, 300, 0)] {
        let heard = heard(&[
            (0, started_on_the_wall, Some(&full)),
            (30, started_on_the_wall + 30, None),
            (awake, on_the_wall, Some(&full)),
        ]);
        assert!(
            heard.len() == 3 && heard[2].1 == "said",
            "a spell {awake} s on awake and at {on_the_wall} s on the wall went unsaid: \
             {heard:?}"
        );
    }
}

#[test]
fn a_sleep_in_a_spell_kept_quiet_has_it_said() {
    // The spell starts in the quiet, and the machine sleeps twenty minutes
    // before its next try, which still fails.
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let heard = heard(&[
        (0, 0, Some(&full)),
        (30, 30, None),
        (60, 60, Some(&full)),
        (90, 1290, Some(&full)),
    ]);
    assert!(
        heard.len() == 3 && heard[2].0 == 90 && heard[2].1 == "said",
        "a spell kept quiet went unsaid after a sleep: {heard:?}"
    );
}

#[test]
fn short_spells_gone_unsaid_are_told_once_the_quiet_ends_with_none_on() {
    // No spell follows the short one kept quiet, so the tick that finds
    // the quiet over tells it, and the tick after says nothing more. That
    // keeps no new quiet, so the next flap is said as failing, as a disk
    // flapping on would otherwise only ever be told writing again.
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let heard = heard(&[
        (0, 0, Some(&full)),
        (30, 30, None),
        (60, 60, Some(&full)),
        (90, 90, None),
        (600, 600, None),
        (630, 630, None),
        (660, 660, None),
        (690, 690, Some(&full)),
        (720, 720, None),
    ]);
    let when: Vec<(u64, &str)> = heard.iter().map(|(at, kind, _)| (*at, *kind)).collect();
    assert!(
        when == [
            (0, "said"),
            (30, "told"),
            (630, "told"),
            (690, "said"),
            (720, "told")
        ] && heard[2]
            .2
            .ends_with("again, after a short spell failing that went unsaid"),
        "short spells kept quiet weren't told once the quiet ended: {heard:?}"
    );
}

#[test]
fn short_spells_gone_unsaid_are_told_as_health_ends_if_it_writes() {
    let full = io::Error::new(io::ErrorKind::StorageFull, "no space left on device");
    let ended = |written: io::Result<()>| {
        let told = RefCell::new(Vec::new());
        let notice = |words: &str| told.borrow_mut().push(words.to_owned());
        let first = Instant::now();
        let mut spell = Spell::default();
        for (after, error) in [(0, Some(&full)), (30, None), (60, Some(&full)), (90, None)] {
            let at = first + Duration::from_secs(after);
            spell.tried(error, at, wall(after), Path::new("days"), &|_| {}, &notice);
        }
        let failed = written.is_err();
        let result = spell.ending(written, Path::new("days"), &notice);
        (failed == result.is_err(), told.take())
    };
    let (handed_back, wrote) = ended(Ok(()));
    let (failure_handed_back, failed) = ended(Err(io::Error::other("full")));
    assert!(
        handed_back
            && failure_handed_back
            && wrote.len() == 2
            && wrote[1].ends_with("again, after a short spell failing that went unsaid")
            && failed.len() == 1,
        "the short spells kept quiet as health ended were lost, or told though it couldn't \
         write: {wrote:?}, {failed:?}"
    );
}

/// A run over a disk that flaps at four ticks running, from the third,
/// failing, writing, failing and writing again, so its second spell comes
/// within the quiet its first one's told end keeps, with the machine
/// asleep `slept` between the first spell's end and the second's start;
/// its follower has ended, so it ends once it has run `SETTLED`.
fn flapping(slept: Option<Duration>) -> Outcome {
    Ran {
        ends: 1,
        held_for: SETTLED * 2,
        step: EVERY,
        busy: true,
        flaps: &[3, 4, 5, 6],
        slept: slept.map(|slept| (4, slept)),
        ..Ran::default()
    }
    .outcome()
}

#[test]
fn a_run_ends_a_flapping_disks_quiet_on_the_wall_clock() {
    // Twenty minutes asleep, which only the wall clock sees, so the second
    // spell is said at once rather than kept quiet.
    let Outcome {
        result,
        said,
        noticed,
        ..
    } = flapping(Some(Duration::from_mins(20)));
    assert!(
        result.is_ok() && said.len() == 2 && noticed.len() == 2,
        "a run didn't hand its spell the wall clock, so a sleep kept a flapping disk quiet: \
         {result:?}, {said:?}, {noticed:?}"
    );
}

#[test]
fn a_run_that_ends_in_the_quiet_tells_the_short_spells_it_kept() {
    // The run ends within ten minutes of the first spell's told end, so
    // only its final write can tell the second.
    let Outcome {
        result,
        said,
        noticed,
        ..
    } = flapping(None);
    assert!(
        result.is_ok()
            && said.len() == 1
            && noticed.len() == 2
            && noticed[1].ends_with("again, after a short spell failing that went unsaid"),
        "a run ending in the quiet didn't hand its final write to the spell, losing the short \
         spell it kept: {result:?}, {said:?}, {noticed:?}"
    );
}

#[test]
fn a_stop_that_cant_write_the_day_ends_the_run_saying_so() {
    // A day never writable, as on a full disk, a busy journal so it has
    // lines to write, and a stop past three ticks, so the stop's write
    // fails as any end's would.
    let Outcome { result, .. } = Ran {
        held_for: SETTLED * 2,
        step: EVERY,
        busy: true,
        unwritable_for: usize::MAX,
        stops: 8,
        ..Ran::default()
    }
    .outcome();
    let error = result.map_err(|error| error.to_string());
    assert!(
        error
            .as_ref()
            .is_err_and(|error| error.starts_with("the day couldn't be written as health ended")),
        "a stop that couldn't write the day ended the run as if it had: {error:?}"
    );
}

#[test]
fn a_stop_ends_a_run_at_once_telling_the_short_spells_it_kept() {
    // As a restart or a reboot stops health: its followers still hold
    // their senders, and the second spell is a minute old, well within
    // the quiet, so only the stop's write can tell it. The stop comes at
    // the clock's 18th look, the ninth tick's first, 17 half minutes in,
    // and the run ends at it, with no tick more.
    let Outcome {
        result,
        ran,
        noticed,
        ..
    } = Ran {
        held_for: SETTLED * 2,
        step: EVERY,
        busy: true,
        flaps: &[3, 4, 5, 6],
        stops: 18,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.is_ok()
            && ran <= EVERY * 17
            && noticed.len() == 2
            && noticed[1].ends_with("again, after a short spell failing that went unsaid"),
        "a stop didn't end the run at once, writing the day and telling the short spell it \
         kept: {result:?}, {ran:?}, {noticed:?}"
    );
}

#[test]
fn how_long_a_spell_lasted_reads_to_the_minute() {
    let read = |seconds| spell::lasting(Duration::from_secs(seconds));
    assert_eq!(
        [read(59), read(60), read(119), read(120)],
        ["less than a minute", "a minute", "a minute", "2 minutes"],
        "how long a spell lasted read wrong"
    );
}

#[test]
fn a_spells_time_left_reads_to_the_nearest_minute() {
    let left = |seconds| spell::left(GIVES_UP.saturating_sub(Duration::from_millis(seconds)));
    assert_eq!(
        [
            left(29_999),
            left(30_000),
            left(89_999),
            left(90_000),
            left(3_569_999),
            left(3_570_000),
            left(3_600_000),
        ],
        [
            "less than a minute",
            "about a minute",
            "about a minute",
            "about 2 minutes",
            "about 59 minutes",
            "about an hour",
            "about an hour",
        ],
        "a spell's time left read wrong"
    );
}

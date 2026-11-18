//! A day health can't write, as on a full disk, and how it waits that
//! out rather than starting again every few seconds.

use super::*;

#[test]
fn a_new_days_line_after_a_failed_write_waits_for_the_next_try() {
    // Room is made just after the roll fails, so a line that wrote again
    // would start the new day; only `write`, the tick's, may.
    let into = tempfile::tempdir().expect("a folder");
    let days = into.path().join("days");
    fs::write(&days, "").expect("a file in the folder's place");
    let mut follower = Follower::new("2026-11-12", &days, TimeZone::UTC);
    follower.take(&warning(NOON)).expect("the day's own line");
    assert!(
        follower.take(&warning(NOON + DAY)).is_err(),
        "a new day began on a day that couldn't be written"
    );
    fs::remove_file(&days).expect("room made for the day");
    assert!(
        follower.take(&warning(NOON + DAY + 1)).is_err() && follower.day.date == "2026-11-12",
        "a new day's line wrote the day again rather than waiting for the next try"
    );
    follower.write().expect("the next try");
    follower
        .take(&warning(NOON + DAY + 2))
        .expect("a new day once the old was written");
    assert_eq!(
        (
            read(&days, "2026-11-12").things["people"].lines,
            follower.day.date.as_str()
        ),
        (1, "2026-11-13"),
        "the old day wasn't written whole before the new began"
    );
}

#[test]
fn a_day_that_cant_be_written_keeps_health_running_until_it_can_saying_so_once() {
    // As on a full disk under a busy journal: the followers end at ten
    // minutes, but health runs on, keeping the day, until the thirtieth
    // look finds room, and says the failure once however many lines come
    // between its tries.
    let Outcome {
        result,
        ran,
        looks,
        said,
        noticed,
        busy_sent,
        today,
        ..
    } = Ran {
        before: 1,
        held_for: SETTLED,
        step: EVERY,
        unwritable_for: 30,
        busy: true,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.is_ok() && today.is_some(),
        "a day written once there was room ended health as a fault: {result:?}"
    );
    assert!(
        looks == 30 && ran > SETTLED,
        "health ended at look {looks}, {ran:?} in, rather than at the first write that worked"
    );
    assert_eq!(
        said.len(),
        1,
        "a spell of failed writes was said other than once"
    );
    assert!(
        noticed.len() == 1
            && noticed[0].contains("/days again, after trying for ")
            && noticed[0].ends_with(" minutes"),
        "a long spell's end wasn't told once, in minutes: {noticed:?}"
    );
    assert_eq!(
        today.map(|day| day.things["people"].lines),
        Some(1 + busy_sent),
        "the day kept in memory while it couldn't be written didn't come through whole"
    );
}

#[test]
fn a_day_that_can_never_be_written_ends_health_after_an_hour_of_trying_without_spinning() {
    // As a folder health may no longer write: no room ever comes, and
    // nothing more is heard, so health waits for each try and ends once
    // it has tried for an hour, saying the day couldn't be written.
    let Outcome {
        result,
        ran,
        said,
        idle_waits,
        today,
        ..
    } = Ran {
        before: 1,
        step: Duration::from_secs(1),
        unwritable_for: usize::MAX,
        ..Ran::default()
    }
    .outcome();
    assert!(
        today.is_none()
            && result.as_ref().is_err_and(|error| {
                let words = error.to_string();
                words.contains("gave up writing the day into ")
                    && words.contains("/days after 60 minutes of trying: ")
                    && !words.ends_with(": ")
            }),
        "a day that could never be written didn't end health saying so: {result:?}"
    );
    assert!(
        ran > GIVES_UP && ran <= GIVES_UP + EVERY * 2,
        "health ran {ran:?}, not about an hour"
    );
    assert_eq!(idle_waits, 0, "health waited nothing, so it would spin");
    assert!(
        said.len() == 1 && said[0].contains("/days") && said[0].contains("60 minutes"),
        "an hour of failed writes wasn't said once, naming the folder and how long: {said:?}"
    );
}

#[test]
fn health_gives_up_an_hour_after_its_first_failed_write_not_its_start() {
    // Nothing needs writing until the twelfth look, minutes in, so a give
    // up counted from health's start would come that much sooner.
    let Outcome { result, ran, .. } = Ran {
        held_for: SETTLED * 2,
        step: EVERY,
        unwritable_for: usize::MAX,
        late: 12,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.as_ref().is_err_and(|error| {
            let words = error.to_string();
            words.contains("gave up writing the day into ")
                && words.contains("/days after 60 minutes of trying: ")
                && !words.ends_with(": ")
        }),
        "a day that could never be written didn't end health giving up: {result:?}"
    );
    assert!(
        ran >= GIVES_UP + SETTLED && ran <= GIVES_UP + SETTLED * 3,
        "health gave up {ran:?} in, not an hour from its first failed write"
    );
}

#[test]
fn a_new_days_line_whose_roll_cant_write_starts_the_spell_itself() {
    // Every line comes before any tick, so the first roll's write is the
    // spell's first failure, the second's is refused within it, and room
    // comes at the first tick: only the roll can have said it, once, and
    // health ends for the replay once it has run long enough.
    let Outcome {
        result,
        said,
        noticed,
        today,
        ..
    } = Ran {
        before: 1,
        tomorrow: 2,
        step: Duration::from_secs(7),
        unwritable_for: 1,
        ..Ran::default()
    }
    .outcome();
    assert_eq!(
        said.len(),
        1,
        "a roll that couldn't write was said other than once"
    );
    assert_eq!(
        noticed.len(),
        1,
        "the end of a spell the roll started wasn't told once: {noticed:?}"
    );
    assert!(
        today.is_some()
            && result.as_ref().is_err_and(|error| {
                let words = error.to_string();
                words.contains("untaken")
                    && words.contains("writing the day into ")
                    && words.contains("/days failed for a time: ")
            }),
        "the lost line didn't end health once the day was written: {result:?}"
    );
}

#[test]
fn a_day_written_again_once_it_can_be_is_no_fault() {
    // Room is made at the second look, and every follower ends after
    // three minutes, so health ends at ten minutes, well.
    let Outcome {
        result,
        ran,
        said,
        noticed,
        today,
        ..
    } = Ran {
        before: 1,
        held_for: Duration::from_mins(3),
        step: EVERY,
        unwritable_for: 2,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.is_ok() && today.is_some(),
        "a day written again once there was room ended health as a fault: {result:?}"
    );
    assert!(
        ran >= SETTLED && ran <= SETTLED + EVERY * 2,
        "health ran {ran:?} rather than ending at ten minutes"
    );
    assert_eq!(said.len(), 1, "the failed write was said other than once");
    assert!(
        noticed.len() == 1 && noticed[0].contains("/days again, after trying for "),
        "the day written again wasn't told once, naming the folder and how long: {noticed:?}"
    );
}

#[test]
fn a_line_lost_as_the_day_couldnt_be_written_ends_health_for_the_replay_to_hear() {
    // The next day's first line can't be taken while the day it ends
    // can't be written, so it's lost until health starts again and
    // replays it. Once there's room, the next one starts the next day, and
    // health ends at ten minutes for the replay. The first tick's write
    // starts the spell, so the line is refused in health's own words, and
    // health ends naming the disk's.
    let Outcome {
        result,
        ran,
        said,
        noticed,
        today,
        tomorrow,
        ..
    } = Ran {
        before: 1,
        tomorrow: 1,
        tomorrow_after_room: 1,
        held_for: SETTLED * 2,
        step: EVERY,
        unwritable_for: 2,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.as_ref().is_err_and(|error| {
            let words = error.to_string();
            words.contains("untaken")
                && words.contains("still unwritten, as writing it into ")
                && words.contains("/days failed for a time: ")
                && !words.ends_with(": ")
        }),
        "a lost line didn't end health naming the folder and the spell's cause: {result:?}"
    );
    assert_eq!(
        said.len(),
        1,
        "the failed roll and the tick's failed write were said other than once"
    );
    assert_eq!(
        noticed.len(),
        1,
        "the day written again was told other than once"
    );
    let lines = |day: Option<Day>| day.map(|day| day.things["people"].lines);
    assert_eq!(
        (lines(today), lines(tomorrow)),
        (Some(1), Some(1)),
        "the day kept its own line, and the next only the line taken once there was room"
    );
    assert!(
        ran >= SETTLED && ran <= SETTLED + EVERY * 2,
        "health ran {ran:?} after a line was lost"
    );
}

#[test]
fn a_spell_of_failed_writes_is_said_as_it_starts_and_told_as_it_ends_once_each() {
    let heard = RefCell::new(Vec::new());
    let say = |words: &str| heard.borrow_mut().push(format!("said: {words}"));
    let notice = |words: &str| heard.borrow_mut().push(format!("told: {words}"));
    let full = io::Error::other("no space left on device");
    // A new day's line refused within a spell, with health's own words:
    // nothing more is said until the spell ends.
    let refused = io::Error::other("the day it ends is still unwritten");
    let first = Instant::now();
    let mut spell = Spell::default();
    for (after, error) in [
        (0, Some(&full)),
        (30, Some(&refused)),
        (40, Some(&full)),
        (90, None),
    ] {
        spell.tried(
            error,
            first + Duration::from_secs(after),
            Path::new("days"),
            &say,
            &notice,
        );
    }
    let heard = heard.take();
    assert!(
        heard.len() == 2
            && heard[0].starts_with(
                "said: health: can't write the day into days: no space left on device, so it"
            )
            && heard[0].contains("for up to 60 minutes")
            && heard[1]
                == "told: health: writes the day into days again, after trying for a minute",
        "a spell's start and end weren't each said once, with its cause and how long: {heard:?}"
    );
}

#[test]
fn a_flapping_disk_is_said_at_most_once_in_ten_minutes() {
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
    let full = io::Error::other("no space left on device");
    let refused = io::Error::other("the day it ends is still unwritten");
    let first = Instant::now();
    let mut spell = Spell::default();
    for (after, error) in [
        // A short spell, said and told.
        (0, Some(&full)),
        (30, None),
        // A flap soon after goes unsaid, and so does the next, until ten
        // minutes have passed since the end told, when it's said with how
        // long it has lasted and its first error, the disk's.
        (60, Some(&full)),
        (90, None),
        (120, Some(&full)),
        (600, Some(&refused)),
        (630, Some(&refused)),
        (690, None),
        // An unsaid flap's end doesn't hold off the next spell: ten
        // minutes from the end told, one is said at once.
        (900, Some(&full)),
        (930, None),
        (1290, Some(&full)),
    ] {
        step.set(after);
        spell.tried(
            error,
            first + Duration::from_secs(after),
            Path::new("days"),
            &say,
            &notice,
        );
    }
    let heard = heard.take();
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
            && words(2).contains("no space left on device, having tried for 8 minutes, so")
            && words(2).contains("for up to 51 minutes")
            && words(3).ends_with("after trying for 9 minutes")
            && words(4).contains("no space left on device, so")
            && words(4).contains("for up to 60 minutes"),
        "a spell's cause, how long it lasted or the time it has left went unsaid: {heard:?}"
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
fn a_line_lost_within_a_spell_names_the_folder_and_the_spells_cause() {
    let full = || io::Error::other("no space left on device");
    let into = Path::new("days");
    let mut spell = Spell::default();
    spell.tried(Some(&full()), Instant::now(), into, &|_| {}, &|_| {});
    let refused = io::Error::other("the day it ends is still unwritten");
    assert_eq!(
        (
            spell.lost(&full(), into).to_string(),
            spell.lost(&refused, into).to_string()
        ),
        (
            "writing the day into days failed for a time: no space left on device".to_owned(),
            "the day it ends is still unwritten, as writing it into days failed for a time: no \
             space left on device"
                .to_owned()
        ),
        "a line lost within a spell didn't name the folder, or the spell's cause once"
    );
}

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
            && result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("gave up")),
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
        result
            .as_ref()
            .is_err_and(|error| error.to_string().contains("gave up")),
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
    assert!(
        today.is_some()
            && result
                .as_ref()
                .is_err_and(|error| error.to_string().contains("untaken")),
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
}

#[test]
fn a_line_lost_as_the_day_couldnt_be_written_ends_health_for_the_replay_to_hear() {
    // The next day's first line can't be taken while the day it ends
    // can't be written, so it's lost until health starts again and
    // replays it. Once there's room, the next one starts the next day, and
    // health ends at ten minutes for the replay.
    let Outcome {
        result,
        ran,
        said,
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
        result
            .as_ref()
            .is_err_and(|error| error.to_string().contains("untaken")),
        "a lost line didn't end health: {result:?}"
    );
    assert_eq!(
        said.len(),
        1,
        "the failed roll and the tick's failed write were said other than once"
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
fn a_spell_of_failed_writes_is_said_once() {
    let said = Cell::new(0);
    let say = |_: &str| said.set(said.get() + 1);
    let full = io::Error::other("no space left on device");
    let first = Instant::now();
    let into = Path::new("days");
    let mut failing = unwritten(Some(&full), None, first, into, &say);
    failing = unwritten(Some(&full), failing, first + EVERY, into, &say);
    assert_eq!(
        failing,
        Some(first),
        "a spell's start moved, or the write was taken as written"
    );
    failing = unwritten(None, failing, first + EVERY * 2, into, &say);
    assert!(failing.is_none(), "a write that worked was taken as failed");
    unwritten(Some(&full), failing, first + EVERY * 3, into, &say);
    assert_eq!(
        said.get(),
        2,
        "a spell of failures was said other than once, or a later spell went unsaid"
    );
}

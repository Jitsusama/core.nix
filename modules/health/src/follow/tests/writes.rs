//! A day health can't write, as on a full disk, and how it waits that
//! out rather than starting again every few seconds.

use super::*;

#[test]
fn a_new_day_starts_on_a_day_that_cant_be_written_holding_it_for_the_next_try() {
    // What a tick sees while the old day is held is the new day's, as
    // the lid seen wrong is, and the next try writes both.
    let into = tempfile::tempdir().expect("a folder");
    let days = into.path().join("days");
    fs::write(&days, "").expect("a file in the folder's place");
    let mut follower = Follower::new("2026-11-12", &days, TimeZone::UTC);
    follower.take(&warning(NOON)).expect("the day's own line");
    assert!(
        follower.take(&warning(NOON + DAY)).is_err() && follower.day.date == "2026-11-13",
        "the roll's failed write went unsaid, or the new day didn't start"
    );
    fs::remove_file(&days).expect("room made for the day");
    follower
        .take(&warning(NOON + DAY + 1))
        .expect("a line of the new day");
    let wrong = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    for keys in 1..5 {
        follower.look_at_lid(wrong(keys), NOON + DAY + 2);
    }
    follower.write().expect("the next try");
    let lines = |date| {
        read(&days, date)
            .things
            .get("people")
            .map(|thing| thing.lines)
    };
    let tomorrow = read(&days, "2026-11-13");
    assert_eq!(
        (
            lines("2026-11-12"),
            lines("2026-11-13"),
            tomorrow.things.get("lid").map(|thing| thing.errors)
        ),
        (Some(1), Some(2), Some(1)),
        "the day held, or the new day's lines and what its ticks saw, weren't written: {:?}",
        tomorrow.things
    );
}

#[test]
fn days_held_across_two_midnights_are_all_written_at_the_next_try() {
    // As when the machine sleeps past a second midnight while it can't
    // write: neither day before is dropped for the next.
    let into = tempfile::tempdir().expect("a folder");
    let days = into.path().join("days");
    fs::write(&days, "").expect("a file in the folder's place");
    let mut follower = Follower::new("2026-11-12", &days, TimeZone::UTC);
    follower.take(&warning(NOON)).expect("the day's own line");
    assert!(
        follower.take(&warning(NOON + DAY)).is_err(),
        "the roll wrote"
    );
    follower
        .take(&warning(NOON + DAY * 2))
        .expect("a roll after a failed write tries none");
    fs::remove_file(&days).expect("room made for the days");
    follower.write().expect("the next try");
    assert_eq!(
        ["2026-11-12", "2026-11-13", "2026-11-14"]
            .map(|date| written(&days, date).map(|day| day.things["people"].lines)),
        [Some(1); 3],
        "a day held across a second midnight wasn't written with its own line"
    );
}

#[test]
fn a_held_day_that_cant_be_written_keeps_neither_today_nor_itself_from_the_disk() {
    // A folder in the oldest held day's place fails its write alone; the
    // day held after it and today are written all the same, the failure
    // is said as that day's, its part taken away, and the day is written
    // once there's room for it.
    let into = tempfile::tempdir().expect("a folder");
    let days = into.path().join("days");
    fs::create_dir_all(file_of(&days, "2026-11-12")).expect("a folder in the day's place");
    let mut follower = Follower::new("2026-11-12", &days, TimeZone::UTC);
    follower.take(&warning(NOON)).expect("the day's own line");
    assert!(
        follower.take(&warning(NOON + DAY)).is_err(),
        "the roll wrote"
    );
    follower
        .take(&warning(NOON + DAY * 2))
        .expect("a roll after a failed write tries none");
    let failed = follower.write().expect_err("the oldest day was written");
    assert!(
        failed.to_string().starts_with("2026-11-12's file: ")
            && ["2026-11-13", "2026-11-14"].map(|date| written(&days, date).is_some()) == [true; 2]
            && !days.join(".2026-11-12.json.part").exists(),
        "the held day's failure wasn't said as its own, kept the rest from the disk, or left \
         its part: {failed}"
    );
    fs::remove_dir(file_of(&days, "2026-11-12")).expect("room made for the held day");
    follower.write().expect("the next try");
    assert!(
        written(&days, "2026-11-12").is_some(),
        "the held day wasn't kept for the next try"
    );
}

#[test]
fn a_look_with_nothing_new_still_writes_today() {
    // `health` calls today's day stood still when its file goes three
    // minutes unwritten, which holds only while every look rewrites it:
    // a day taken away after its last write is written again.
    let into = tempfile::tempdir().expect("a folder");
    let proc = tempfile::tempdir().expect("no processes");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC).minding(proc.path());
    follower.mind(NOON);
    follower.write().expect("the day is written");
    assert!(!follower.changed, "a written day was left changed");
    fs::remove_file(file_of(into.path(), "2026-11-12")).expect("the day taken away");
    follower.mind(NOON + 30_000_000);
    follower.write().expect("the day is written again");
    assert!(
        written(into.path(), "2026-11-12").is_some(),
        "a look that saw nothing new left the day unwritten"
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
fn health_giving_up_past_midnight_names_the_day_held_from_before() {
    // A start after giving up replays only the new day, so what the day
    // before it couldn't write is lost, and said to be.
    let Outcome { result, .. } = Ran {
        before: 1,
        tomorrow: 1,
        step: Duration::from_secs(1),
        unwritable_for: usize::MAX,
        ..Ran::default()
    }
    .outcome();
    assert!(
        result.as_ref().is_err_and(|error| {
            let words = error.to_string();
            words
                .contains("trying, losing what it couldn't write of 2026-11-12, held from before: ")
                && !words.ends_with(": ")
        }),
        "giving up past midnight didn't name the day held from before: {result:?}"
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
    // Every line comes before any tick, so the roll's write is the
    // spell's first failure, and room comes at the first tick: only the
    // roll can have said it, once, and both days are written whole.
    let Outcome {
        result,
        said,
        noticed,
        today,
        tomorrow,
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
    let lines = |day: Option<Day>| day.map(|day| day.things["people"].lines);
    assert!(
        result.is_ok() && (lines(today), lines(tomorrow)) == (Some(1), Some(2)),
        "a roll that couldn't write lost a day's lines, or ended health: {result:?}"
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
fn a_new_days_lines_while_the_day_cant_be_written_are_kept_in_the_next() {
    // The first tick's write starts the spell, so the next day's first
    // line, before there's room, tries no write of its own: the new day
    // starts all the same, holding the old, and keeps it and the line
    // after there's room. Health runs on until its followers end.
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
        result.is_ok(),
        "a day written again once there was room ended health as a fault: {result:?}"
    );
    assert_eq!(
        said.len(),
        1,
        "the tick's failed write was said other than once"
    );
    assert_eq!(
        noticed.len(),
        1,
        "the day written again was told other than once"
    );
    let lines = |day: Option<Day>| day.map(|day| day.things["people"].lines);
    assert_eq!(
        (lines(today), lines(tomorrow)),
        (Some(1), Some(2)),
        "the day kept its own line, and the next both of its own"
    );
    assert!(
        ran >= SETTLED * 2 && ran <= SETTLED * 2 + EVERY * 2,
        "health ran {ran:?} rather than until its followers ended"
    );
}

#[test]
fn a_spell_of_failed_writes_is_said_as_it_starts_and_told_as_it_ends_once_each() {
    let heard = RefCell::new(Vec::new());
    let say = |words: &str| heard.borrow_mut().push(format!("said: {words}"));
    let notice = |words: &str| heard.borrow_mut().push(format!("told: {words}"));
    let full = io::Error::other("no space left on device");
    // Another write's failure within the spell: nothing more is said
    // until the spell ends.
    let unreadable = io::Error::other("permission denied");
    let first = Instant::now();
    let mut spell = Spell::default();
    for (after, error) in [
        (0, Some(&full)),
        (30, Some(&unreadable)),
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
    let unreadable = io::Error::other("permission denied");
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
        (600, Some(&unreadable)),
        (630, Some(&unreadable)),
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
fn health_giving_up_names_the_days_held_it_loses_and_what_failed_last() {
    // The spell's first failure was a day held from before, written
    // since, so only the latest, as a final try's, names the day still
    // failing.
    let held = |dates: &[&str]| dates.iter().map(|date| Day::new(date)).collect::<Vec<_>>();
    let mut spell = Spell::default();
    for date in ["2026-11-10", "2026-11-13"] {
        let full = io::Error::other(format!("{date}'s file: no space left on device"));
        spell.tried(
            Some(&full),
            Instant::now(),
            Path::new("days"),
            &|_| {},
            &|_| {},
        );
    }
    assert_eq!(
        [
            spell::losing(&held(&[])),
            spell::losing(&held(&["2026-11-12"])),
            spell
                .given_up(
                    Path::new("days"),
                    &spell::losing(&held(&["2026-11-11", "2026-11-12", "2026-11-13"])),
                )
                .to_string(),
        ],
        [
            String::new(),
            ", losing what it couldn't write of 2026-11-12, held from before".to_owned(),
            "health gave up writing the day into days after 60 minutes of trying, losing what it \
             couldn't write of 2026-11-11, 2026-11-12 and 2026-11-13, held from before: \
             2026-11-13's file: no space left on device"
                .to_owned(),
        ],
        "health gave up without naming the days held it lost, or what failed last"
    );
}

#[test]
fn health_says_its_faults_as_failures_and_a_mended_write_as_a_notice() {
    // Health's warnings follower counts a failure as one of health's own
    // and a notice not, so the two swapped would count every mended write
    // as a fault, and none of health's. This pins what each sayer `follow`
    // hands out says at; that each goes where its name says is read there.
    FAILS("health: a failure said to pin its mark");
    NOTICES("health: a notice said to pin its mark");
    crate::source::tests::assert_marked(
        "health's own fault",
        &["a failure said to pin its mark"],
        1,
        crate::source::ERROR,
    );
    crate::source::tests::assert_marked(
        "a mended write",
        &["a notice said to pin its mark"],
        1,
        crate::source::NOTICE,
    );
}

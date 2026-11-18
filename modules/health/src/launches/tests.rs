use super::*;
use crate::day::Day;
use crate::day::tests::entry;
use crate::journal::Mark;

fn day(lines: &[&str]) -> Launches {
    let mut launches = Launches::default();
    for line in lines {
        assert!(
            launches.took(IDENTIFIER, line),
            "a windows daemon's line wasn't taken: {line}"
        );
    }
    launches
}

#[test]
fn openings_and_ends_are_read_by_program_and_other_lines_left() {
    let launches = day(&[
        "opened foot 212 ms after its process started",
        "opened google-chrome 1840 ms after its process started",
        "opened foot 190 ms after its process started",
        "closed foot; its process ended 14 ms later",
        "closed zoom; its process was still running 30 s later",
    ]);
    assert_eq!(
        launches.opened.get("foot"),
        Some(&vec![212, 190]),
        "a program's openings weren't kept by its name"
    );
    assert_eq!(
        launches.ended.get("foot"),
        Some(&vec![14]),
        "an end wasn't kept"
    );
    assert_eq!(
        launches.outlived(),
        1,
        "a process outliving the wait wasn't counted"
    );
    let mut other = Launches::default();
    assert!(
        !other.took(IDENTIFIER, "niri's event stream ended; asking again"),
        "another of the daemon's lines was taken for a timing"
    );
    assert_eq!(other, Launches::default(), "another line changed the day");
}

#[test]
fn a_daemon_saying_its_ready_is_kept_by_its_name_and_other_timings_left() {
    let mut launches = Launches::default();
    for (thing, line) in [
        ("dictation", "dictation: ready in 1065 ms"),
        ("meaning", "meaning: bge loaded on the NPU in 688 ms"),
        (
            "noise-cancelled",
            "noise-cancelled: the model loaded in 398 ms, cleaning with deepfilternet",
        ),
    ] {
        assert!(
            launches.took(thing, line),
            "a ready line wasn't taken: {line}"
        );
    }
    for (thing, line) in [
        (
            "jobs-sorter",
            "Moved timeout (process group 3, 2 processes) from kitty-1.scope to jobs: \
             6810 ms of CPU in 1040 ms.",
        ),
        (
            "call-transcripts",
            "call-transcripts: mel 33 ms, encoder 325 ms on NPU",
        ),
        ("dictation", "dictation: ready; the model is already loaded"),
    ] {
        assert!(
            !launches.took(thing, line),
            "a line that isn't a ready was taken: {line}"
        );
    }
    assert_eq!(
        launches.ready,
        BTreeMap::from([
            ("dictation".to_owned(), vec![1065]),
            ("meaning".to_owned(), vec![688]),
            ("noise-cancelled".to_owned(), vec![398]),
        ]),
        "each daemon's ready wasn't kept by its name"
    );
    assert_eq!(
        launches.said().as_deref(),
        Some(
            "3 daemons ready: a median 688 ms, the slowest 1065 ms (dictation); 1 past \
             1000 ms (33%)"
        ),
        "the day's readies were said wrongly"
    );
    assert_eq!(
        launches.by_program().first(),
        Some(&(
            "dictation".to_owned(),
            "1 ready, the slowest 1065 ms".to_owned()
        )),
        "the slowest daemon wasn't said first"
    );
}

#[test]
fn the_day_says_its_median_slowest_and_share_past_each_budget() {
    let launches = day(&[
        "opened foot 212 ms after its process started",
        "opened google-chrome 1840 ms after its process started",
        "opened foot 190 ms after its process started",
        "closed foot; its process ended 14 ms later",
        "closed zoom; its process was still running 30 s later",
    ]);
    let opened = launches.openings().expect("openings were timed");
    assert_eq!(
        (opened.count, opened.median_ms, opened.p99_ms, opened.late),
        (3, 212, 1840, 1),
        "the openings' spread was misread"
    );
    assert_eq!(
        opened.slowest,
        (1840, "google-chrome"),
        "the slowest wasn't named"
    );
    assert_eq!(
        launches.said().as_deref(),
        Some(
            "3 openings: a median 212 ms, the slowest 1840 ms (google-chrome); 1 past \
             300 ms (33%); 1 end: a median 14 ms, the slowest 14 ms (foot); none past \
             200 ms; 1 program still ran 30 s after its last window"
        ),
        "the day's line was said wrongly"
    );
    assert_eq!(
        launches.by_program(),
        [
            ("google-chrome", "1 opened, the slowest 1840 ms"),
            (
                "foot",
                "2 opened, the slowest 212 ms; 1 ended, the slowest 14 ms"
            ),
            ("zoom", "1 still running 30 s on"),
        ]
        .map(|(name, said)| (name.to_owned(), said.to_owned())),
        "each program wasn't said, the slowest first"
    );
    assert_eq!(
        Launches::default().said(),
        None,
        "a day with none said something"
    );
}

#[test]
fn a_service_saying_no_ready_line_after_its_restarts_is_named() {
    let mut day = Day::new("2026-11-13");
    let job = |unit: &str, at, job| Entry {
        at,
        since_boot: at,
        job: Some(job),
        ..entry(unit, 6, "")
    };
    for at in [1_000_000, 9_000_000] {
        day.take(&job("google.service", at, Job::Stopping));
        day.take(&job("google.service", at + 64_000, Job::Started));
    }
    day.take(&job("dictation.service", 2_000_000, Job::Stopping));
    day.take(&job("dictation.service", 2_200_000, Job::Started));
    day.take(&Entry {
        at: 3_300_000,
        since_boot: 3_300_000,
        ..entry("dictation", 6, "dictation: ready in 1065 ms")
    });
    assert_eq!(
        day.launches.unstamped(),
        BTreeMap::from([("google.service", 2)]),
        "a service with no ready line wasn't named, or one that said it was"
    );
}

#[test]
fn a_oneshot_finishing_after_a_stop_is_no_restart() {
    // systemd says a oneshot's run ended with the message a start has,
    // so home-manager's activation, which serves nothing, was being
    // timed as a service gone for the whole of its run.
    let mut day = Day::new("2026-11-13");
    let job = |at, job, message: &str| Entry {
        at,
        since_boot: at,
        job: Some(job),
        ..entry("home-manager-jitsusama.service", 6, message)
    };
    day.take(&job(
        1_000_000,
        Job::Stopping,
        "Stopping Home Manager environment for jitsusama...",
    ));
    day.take(&job(
        13_000_000,
        Job::Started,
        "Finished Home Manager environment for jitsusama.",
    ));
    assert_eq!(
        day.launches.restarted,
        BTreeMap::new(),
        "a oneshot's run was timed as a service gone"
    );
}

#[test]
fn a_restart_is_timed_from_stopping_to_its_ready_line_or_else_its_start() {
    let mut day = Day::new("2026-11-12");
    let job = |unit: &str, at, job| Entry {
        at,
        since_boot: at,
        job: Some(job),
        ..entry(unit, 6, "")
    };
    let said = |thing: &str, at, message: &str, mark| Entry {
        at,
        since_boot: at,
        mark,
        ..entry(thing, 6, message)
    };
    day.take(&job("google.service", 1_000_000, Job::Stopping));
    day.take(&job("google.service", 1_064_000, Job::Started));
    day.take(&job("dictation.service", 2_000_000, Job::Stopping));
    day.take(&job("dictation.service", 2_200_000, Job::Started));
    day.take(&said(
        "dictation",
        3_300_000,
        "dictation: ready in 1065 ms",
        None,
    ));
    day.take(&said(
        "dictation",
        3_400_000,
        "dictation: ready in 90 ms",
        None,
    ));
    day.take(&job("quickshell.service", 4_000_000, Job::Stopping));
    day.take(&job("quickshell.service", 4_100_000, Job::Started));
    day.take(&said(
        "quickshell",
        6_500_000,
        "launcher: home kept as the shell started",
        Some(Mark::Ready),
    ));
    day.take(&job("meaning.service", 5_000_000, Job::Started));
    day.take(&job("sound.service", 5_000_000, Job::Stopping));
    day.take(&job("sound.service", 605_000_000, Job::Started));
    day.take(&job("notices.service", 7_000_000, Job::Stopping));
    day.take(&job("notices.service", 7_100_000, Job::Started));
    day.take(&said(
        "notices",
        90_000_000,
        "notices: loaded in 5 ms",
        None,
    ));
    assert_eq!(
        day.launches.restarted,
        BTreeMap::from([
            ("dictation.service".to_owned(), vec![1_300]),
            ("google.service".to_owned(), vec![64]),
            ("notices.service".to_owned(), vec![100]),
            ("quickshell.service".to_owned(), vec![2_500]),
        ]),
        "a restart wasn't timed to its ready line, its start, or the shell's ready; or a \
         start alone, a stop long before or a ready long after was taken for one"
    );
    let restarts = day.launches.restarts().expect("restarts were timed");
    assert_eq!(
        (restarts.median_ms, restarts.slowest, restarts.late),
        (100, (2_500, "quickshell.service"), 0),
        "the restarts' spread was misread"
    );
    assert!(
        day.things.is_empty(),
        "a restart's lines were counted as warnings"
    );
}

#[test]
fn a_restart_is_timed_on_its_boots_clock_whatever_the_wall_clock_does() {
    // A clock synced back a second, one stepped forward within the window
    // a stop may be a restart's, and a ready line said after a step back:
    // the boot's clock, which no step moves, times each. The wall clock
    // reads as it does on a machine, far past the boot's, so a figure
    // taken from the two at once is seen.
    const WALL: i64 = 1_794_978_000_000_000;
    let mut day = Day::new("2026-11-18");
    let clocked = |thing: &str, since_boot, at, job, message: &str| Entry {
        at: WALL + at,
        since_boot,
        job,
        ..entry(thing, 6, message)
    };
    for (unit, since_boot, at, job) in [
        ("back.service", 500_000_000, 10_000_000, Job::Stopping),
        ("back.service", 500_300_000, 9_000_000, Job::Started),
        ("forward.service", 600_000_000, 20_000_000, Job::Stopping),
        ("forward.service", 600_400_000, 80_000_000, Job::Started),
        ("ready.service", 700_000_000, 30_000_000, Job::Stopping),
        ("ready.service", 700_100_000, 30_100_000, Job::Started),
        // Ten minutes on the boot's clock with the wall clock synced back
        // as long: too long to be a restart, whatever the wall clock says.
        ("late.service", 800_000_000, 40_000_000, Job::Stopping),
        ("late.service", 1_400_000_000, 40_500_000, Job::Started),
    ] {
        day.take(&clocked(unit, since_boot, at, Some(job), ""));
    }
    day.take(&clocked(
        "ready",
        700_900_000,
        29_000_000,
        None,
        "ready: ready in 5 ms",
    ));
    assert_eq!(
        day.launches.restarted,
        BTreeMap::from([
            ("back.service".to_owned(), vec![300]),
            ("forward.service".to_owned(), vec![400]),
            ("ready.service".to_owned(), vec![900]),
        ]),
        "a restart was timed by the wall clock, so a step read as too short or too long, \
         or as a ready line before its stop"
    );
    assert_eq!(
        day.launches.unstamped(),
        BTreeMap::from([("back.service", 1), ("forward.service", 1)]),
        "a ready line after a step back didn't stamp its restart"
    );
}

#[test]
fn a_restart_isnt_timed_across_a_sleep_or_out_of_order() {
    const WALL: i64 = 1_794_978_000_000_000;
    let mut day = Day::new("2026-11-18");
    let clocked = |thing: &str, since_boot, at, job, message: &str| Entry {
        at: WALL + at,
        since_boot,
        job,
        ..entry(thing, 6, message)
    };
    for (unit, since_boot, at, job) in [
        // Stopped as the lid closed and started an hour later as it
        // opened: the boot's clock stood still, the wall clock didn't.
        ("slept.service", 500_000_000, 10_000_000, Job::Stopping),
        ("slept.service", 500_400_000, 3_610_400_000, Job::Started),
        // A start read without the boot's stamp, which reads as 0.
        (
            "unstamped.service",
            600_000_000,
            3_620_000_000,
            Job::Stopping,
        ),
        ("unstamped.service", 0, 3_620_300_000, Job::Started),
        // A ready line written before its start and heard after it, as
        // one follower ran ahead of another; and one more said after its
        // start, which the early one mustn't keep from timing it.
        ("early.service", 700_000_000, 3_630_000_000, Job::Stopping),
        ("early.service", 700_200_000, 3_630_200_000, Job::Started),
        ("twice.service", 800_000_000, 3_640_000_000, Job::Stopping),
        ("twice.service", 800_200_000, 3_640_200_000, Job::Started),
        // Two restarts in a row, the second program's ready line written
        // before its start: it isn't the first restart's either.
        ("again.service", 900_000_000, 3_650_000_000, Job::Stopping),
        ("again.service", 900_100_000, 3_650_100_000, Job::Started),
        ("again.service", 900_200_000, 3_650_200_000, Job::Stopping),
        // Two restarts in a row again, the first program's ready line
        // written before the second stop and heard after it.
        (
            "lagged.service",
            1_000_000_000,
            3_660_000_000,
            Job::Stopping,
        ),
        ("lagged.service", 1_000_100_000, 3_660_100_000, Job::Started),
        (
            "lagged.service",
            1_000_400_000,
            3_660_400_000,
            Job::Stopping,
        ),
    ] {
        day.take(&clocked(unit, since_boot, at, Some(job), ""));
    }
    for (thing, since_boot) in [
        ("early", 700_100_000),
        ("twice", 800_100_000),
        ("twice", 800_500_000),
        ("lagged", 1_000_300_000),
    ] {
        day.take(&clocked(
            thing,
            since_boot,
            since_boot + 2_930_000_000,
            None,
            &format!("{thing}: ready in 5 ms"),
        ));
    }
    day.take(&clocked(
        "again",
        900_250_000,
        3_650_250_000,
        None,
        "again: ready in 5 ms",
    ));
    day.take(&clocked(
        "again.service",
        900_300_000,
        3_650_300_000,
        Some(Job::Started),
        "",
    ));
    day.take(&clocked(
        "lagged.service",
        1_000_500_000,
        3_660_500_000,
        Some(Job::Started),
        "",
    ));
    assert_eq!(
        day.launches.restarted,
        BTreeMap::from([
            ("again.service".to_owned(), vec![100, 100]),
            ("early.service".to_owned(), vec![200]),
            ("lagged.service".to_owned(), vec![300, 100]),
            ("twice.service".to_owned(), vec![500]),
        ]),
        "a stop and a start across a sleep, or before it on the boot's clock, were timed as a \
         restart, or a ready line from before its start re-timed one or kept one after from it"
    );
    assert_eq!(
        day.launches.unstamped(),
        BTreeMap::from([("again.service", 2), ("early.service", 1)]),
        "a ready line from before its start stamped a restart, its own or the one before"
    );
}

#[test]
fn a_stop_at_shutdown_and_a_start_in_the_next_boot_are_no_restart() {
    // The day is read through every boot, so a shutdown's stops come
    // before the next boot's starts, a reboot's length apart.
    let mut day = Day::new("2026-11-17");
    let job = |boot: &str, at, job| Entry {
        at,
        since_boot: at,
        job: Some(job),
        boot: boot.to_owned(),
        ..entry("google.service", 6, "")
    };
    let ready = |boot: &str, at| Entry {
        at,
        since_boot: at,
        boot: boot.to_owned(),
        ..entry("google", 6, "google: ready in 5 ms")
    };
    // A switch's restart not yet ready when the machine shuts down, and
    // ready only in the next boot: the reboot isn't its restart.
    day.take(&job("before", 1_000_000, Job::Stopping));
    day.take(&job("before", 1_200_000, Job::Started));
    day.take(&job("before", 2_000_000, Job::Stopping));
    day.take(&job("after", 41_000_000, Job::Started));
    day.take(&ready("after", 42_000_000));
    day.take(&job("after", 50_000_000, Job::Stopping));
    day.take(&job("after", 50_300_000, Job::Started));
    // The day is replayed in the order it was written (`replay.rs`), but a
    // ready heard late, from the boot before, still mustn't end this
    // boot's restart: the pairing doesn't rest on the order.
    day.take(&ready("before", 1_500_000));
    day.take(&ready("after", 50_400_000));
    assert_eq!(
        day.launches.restarted,
        BTreeMap::from([("google.service".to_owned(), vec![200, 400])]),
        "a reboot was timed as a restart, or a restart in one boot was missed"
    );
}

#[test]
fn a_pis_start_is_held_to_its_own_budget_apart_from_the_daemons() {
    let mut launches = Launches::default();
    for line in [
        "pi: ready in 889 ms, session 01a1d0f7-99f2-72a3-b390-9ab085c4dda4",
        "pi: ready in 4120 ms, session 01a1c28f-032e-7954-aad3-8ec9694a15aa",
    ] {
        assert!(launches.took(PI, line), "a pi's start wasn't taken: {line}");
    }
    assert_eq!(
        launches.readies(),
        None,
        "a pi's start was taken for a daemon's ready"
    );
    assert_eq!(
        launches.said().as_deref(),
        Some(
            "2 pis started: a median 889 ms, the slowest 4120 ms (pi); 1 past 3000 ms \
             (50%)"
        ),
        "pi's starts weren't said against their own budget"
    );
}

#[test]
fn the_day_takes_the_daemons_timings_and_counts_its_warnings_as_ever() {
    let mut day = Day::new("2026-11-12");
    day.take(&entry(
        IDENTIFIER,
        6,
        "opened foot 212 ms after its process started",
    ));
    day.take(&entry(
        IDENTIFIER,
        4,
        "niri's event stream ended; asking again",
    ));
    day.take(&entry("dictation", 6, "dictation: ready in 1065 ms"));
    assert_eq!(
        day.launches.ready.get("dictation"),
        Some(&vec![1065]),
        "the day didn't take a daemon's ready"
    );
    assert_eq!(
        day.launches.opened.get("foot"),
        Some(&vec![212]),
        "the day didn't take the daemon's opening"
    );
    assert_eq!(
        day.things.get(IDENTIFIER).map(|thing| thing.lines),
        Some(1),
        "the daemon's warning wasn't counted, or its timing was"
    );
}

#[test]
fn a_program_reloading_over_and_over_costs_health_only_the_reloads_it_keeps() {
    let mut launches = Launches::default();
    let reload = |since_boot: i64| Entry {
        at: since_boot,
        since_boot,
        origin: Origin::Systemd,
        ..entry(
            "systemd",
            5,
            "Reload requested from client PID 1520947 ('systemctl') (unit x.service)...",
        )
    };
    // A reload a minute for twenty minutes keeps only the last ten
    // minutes' of them, and a flood a few milliseconds apart only the
    // most.
    for minute in 0..20 {
        launches.heard(&reload(minute * 60_000_000));
    }
    let lately = launches.reloaded.get(&false).map_or(0, Vec::len);
    for flood in 0..1_000 {
        launches.heard(&reload(1_200_000_000 + flood * 5_000));
    }
    let flooded = launches.reloaded.get(&false).map_or(0, Vec::len);
    // Still, the flood's last reload makes a stop the tool's own when only
    // it is near enough, though one more reload after the stop is heard
    // first: the cap lets the oldest go, not the recent ones.
    launches.heard(&reload(1_225_000_000));
    let stop = Entry {
        at: 1_213_000_000,
        since_boot: 1_213_000_000,
        job: Some(Job::Stopping),
        ..entry("backend.service", 6, "")
    };
    launches.heard(&stop);
    assert_eq!(
        (
            lately,
            flooded,
            launches.stopping.contains_key("backend.service")
        ),
        (11, MOST_RELOADS, false),
        "the reloads kept grew past their bounds, or the newest was lost"
    );
}

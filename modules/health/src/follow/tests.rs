use std::cell::{Cell, RefCell};

use super::*;
use crate::journal;

/// A warning line at a moment in microseconds.
fn warning(at: i64) -> String {
    format!(
        r#"{{"__REALTIME_TIMESTAMP":"{at}","PRIORITY":"4","SYSLOG_IDENTIFIER":"people","MESSAGE":"slow"}}"#
    )
}

/// 2026-11-12 at noon, UTC, in microseconds.
const NOON: i64 = 1_794_484_800_000_000;
const DAY: i64 = 86_400_000_000;

fn read(into: &Path, date: &str) -> Day {
    let file = fs::read(file_of(into, date)).expect("the day was written");
    serde_json::from_slice(&file).expect("the day's file is a day")
}

#[test]
fn a_day_is_written_only_once_something_changed() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    follower.write().expect("nothing to write");
    assert!(
        !file_of(into.path(), "2026-11-12").exists(),
        "an untouched day isn't written"
    );
    follower.take(&warning(NOON)).expect("a line");
    follower.write().expect("the day is written");
    assert_eq!(
        read(into.path(), "2026-11-12").things["people"].lines,
        1,
        "its line counted"
    );
    assert!(
        fs::read_dir(into.path()).expect("the folder").count() == 1,
        "nothing but the day is left behind"
    );
}

#[test]
fn a_new_day_writes_the_old_and_starts_empty() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    follower.take(&warning(NOON)).expect("a line");
    follower.take(&warning(NOON + 1)).expect("a line");
    follower
        .take(&warning(NOON + DAY))
        .expect("the next day's line");
    assert_eq!(
        read(into.path(), "2026-11-12").things["people"].lines,
        2,
        "the old day whole"
    );
    follower.take(&warning(NOON)).expect("a late line");
    follower.write().expect("the new day is written");
    assert_eq!(
        read(into.path(), "2026-11-13").things["people"].lines,
        1,
        "the new day starts empty, and a late line of a day done is left out"
    );
    assert_eq!(
        read(into.path(), "2026-11-12").things["people"].lines,
        2,
        "the old day stays"
    );
}

#[test]
fn a_line_that_isnt_an_entry_changes_nothing() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    follower.take("-- No entries --").expect("words");
    follower.write().expect("nothing to write");
    assert!(
        !file_of(into.path(), "2026-11-12").exists(),
        "nothing was taken"
    );
}

#[test]
fn a_moment_is_dated_in_its_zone() {
    assert_eq!(
        date_of(NOON, &TimeZone::UTC),
        "2026-11-12",
        "noon is that day"
    );
    let west = TimeZone::fixed(jiff::tz::offset(-5));
    assert_eq!(
        date_of(NOON - 8 * 3_600_000_000, &west),
        "2026-11-11",
        "04:00 UTC is still the day before five hours west"
    );
}

#[test]
fn a_shutdown_seen_is_timed_to_its_boots_end() {
    let boots = r#"[{"index":-1,"boot_id":"old","first_entry":1,"last_entry":40000000},
            {"index":0,"boot_id":"now","first_entry":41000000,"last_entry":99000000}]"#;
    let ends = ends_of(boots);
    assert_eq!(
        ends.into_iter().collect::<Vec<_>>(),
        vec![("old".to_owned(), 40_000_000)],
        "only the boots before this one have ended"
    );
    let into = tempfile::tempdir().expect("a folder");
    let mut follower =
        Follower::new("1970-01-01", into.path(), TimeZone::UTC).knowing(ends_of(boots));
    let said = format!(
        r#"{{"__REALTIME_TIMESTAMP":"10000000","_BOOT_ID":"old","_SYSTEMD_CGROUP":"{}",
        "MESSAGE_ID":"{}","MESSAGE":"System is rebooting."}}"#,
        journal::LOGIND,
        journal::SHUTTING_DOWN
    )
    .replace('\n', "");
    follower.take(&said).expect("taken");
    assert_eq!(
        follower.day.boots["old"].shutdown(),
        Some(30_000_000),
        "from logind's line to the boot's last entry"
    );
}

/// How far a test's wait goes past what it asks, as a sleep may, which
/// leaves health's ticks off the half minute.
const OVERSLEPT: Duration = Duration::from_secs(1);

/// A run of health over followers that send `before` lines, `ends` ends
/// and `after` lines, all at once, every sender then gone unless one is
/// held, as a quiet follower holds it, until the clock passes `held_for`.
/// Each look at the clock moves it on `step`, and each wait as far as it
/// asks and `OVERSLEPT` more. While a sender is held `run` never waits,
/// so only a `step` of a half minute or more moves the clock past each
/// half minute for it to hear at once, rather than in real time.
#[derive(Default)]
struct Ran {
    before: i64,
    ends: usize,
    after: i64,
    held_for: Duration,
    step: Duration,
}

impl Ran {
    /// What `run` left unread, how long health ran, from its first look
    /// at the clock to its last, and how often it looked at the lid. A
    /// run that would never end fails at two hours, or at ten thousand
    /// looks at the clock, for one that spins with the clock still.
    fn ran(&self) -> (usize, Duration, usize) {
        assert!(
            self.held_for.is_zero() || self.step >= EVERY,
            "a held sender with a step under the half minute waits in real time"
        );
        let into = tempfile::tempdir().expect("a folder");
        let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
        let (send, read) = mpsc::channel();
        let line = |second| Some(warning(NOON + second));
        for second in 0..self.before {
            send.send(line(second)).expect("a line sent");
        }
        for _ in 0..self.ends {
            send.send(None).expect("a follower's end sent");
        }
        for second in 0..self.after {
            send.send(line(self.before + second)).expect("a line sent");
        }
        let held = RefCell::new((self.held_for > Duration::ZERO).then(|| send.clone()));
        drop(send);
        let start = Instant::now();
        let clock = Cell::new(start);
        let first = Cell::new(None);
        let looks = Cell::new(0);
        let times = Cell::new(0);
        let moved = |by: Duration| {
            times.set(times.get() + 1);
            assert!(
                times.get() < 10_000,
                "health looked at the clock ten thousand times, so it would spin for ever"
            );
            clock.set(clock.get() + by);
            let gone = clock.get().saturating_duration_since(start);
            if gone >= self.held_for {
                held.borrow_mut().take();
            }
            assert!(
                gone < SETTLED * 12,
                "health ran two hours, so it would never have ended"
            );
            first.set(first.get().or(Some(clock.get())));
            clock.get()
        };
        let now = || moved(self.step);
        let wait = |time| {
            moved(time + OVERSLEPT);
        };
        let look = || {
            looks.set(looks.get() + 1);
            Looked::default()
        };
        run(&mut follower, &read, now, wait, look).expect("the run");
        let started = first.get().expect("a look at the clock");
        (
            read.try_iter().count(),
            clock.get().saturating_duration_since(started),
            looks.get(),
        )
    }
}

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
        start + Duration::from_secs(10 * asked)
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
    )
    .expect("the run");
    assert!(
        looks >= 3,
        "looked {looks} times over ten lines a hundred seconds apart"
    );
}

#[test]
fn a_lid_gone_wrong_is_a_devices_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC);
    let used = |keys| Looked {
        shut: true,
        screen_on: true,
        keys,
    };
    for keys in 1..5 {
        follower.look_at_lid(used(keys), NOON);
    }
    let open = Looked {
        shut: false,
        ..used(5)
    };
    follower.look_at_lid(open, NOON);
    follower.look_at_lid(used(6), NOON);
    follower.look_at_lid(used(7), NOON);
    follower.write().expect("the day is written");
    let lid = &read(into.path(), "2026-11-12").things["lid"];
    assert_eq!(lid.kind, Some(Kind::Devices), "a lid is a device");
    assert_eq!(lid.errors, 2, "once a time it went wrong: {lid:?}");
    let said: Vec<&str> = lid
        .signatures
        .values()
        .map(|said| said.example.as_str())
        .collect();
    assert_eq!(said, [LID_WRONG], "said the way Joel reads it");
}

#[test]
fn a_disk_guard_that_stopped_is_a_services_fault_once() {
    let into = tempfile::tempdir().expect("a folder");
    let status = into.path().join("status.json");
    let said = |seconds: i64, free: u64| {
        fs::write(
            &status,
            format!(
                r#"{{"heartbeat": {seconds}, "filesystems": [
                        {{"path": "/", "band": "filling", "free": {free}, "size": 100}}]}}"#
            ),
        )
        .expect("a status written");
    };
    let beat = |seconds: i64| said(seconds, 10);
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).guarding(vec![status.clone()]);
    let noon = NOON / 1_000_000;
    beat(noon);
    follower.guard(NOON);
    // Quiet past the two minutes a guard may be, for two looks.
    follower.guard(NOON + 200_000_000);
    follower.guard(NOON + 230_000_000);
    beat(noon + 260);
    follower.guard(NOON + 260_000_000);
    follower.guard(NOON + 500_000_000);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    assert_eq!(
        day.things["disk-guard"].errors, 2,
        "once each time it stopped: {:?}",
        day.things["disk-guard"]
    );
    assert_eq!(
        day.disks.filesystems["/"].band, "filling",
        "the guard's word on the disks wasn't kept"
    );

    // A restart, after which the disk has more free than the day's least.
    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .guarding(vec![status.clone()])
        .resuming(day);
    said(noon + 600, 60);
    again.guard(NOON + 600_000_000);
    again.write().expect("the day is written again");
    assert_eq!(
        read(into.path(), "2026-11-12").disks.filesystems["/"].least_free,
        10,
        "a restart lost the day's least free space"
    );
}

#[test]
fn the_days_pains_are_kept_as_the_detector_ranks_them() {
    let into = tempfile::tempdir().expect("a folder");
    let pains = tempfile::tempdir().expect("the detector's folder");
    let mut follower =
        Follower::new("2026-11-12", into.path(), TimeZone::UTC).feeling(pains.path());
    follower.feel();
    follower.write().expect("nothing to write");
    assert!(
        !file_of(into.path(), "2026-11-12").exists(),
        "no pains yet is no change"
    );
    fs::write(
        pains.path().join("2026-11-12.jsonl"),
        "{\"trying\":\"sound\",\"rank\":1}\n",
    )
    .expect("the detector's day");
    follower.feel();
    follower.write().expect("the day is written");
    let feel = read(into.path(), "2026-11-12").feel;
    assert_eq!(feel.len(), 1, "the pain kept: {feel:?}");
    assert_eq!(feel[0].trying, "sound", "as the detector said it");
}

#[test]
fn memory_is_minded_and_kept_through_a_restart() {
    let into = tempfile::tempdir().expect("a folder");
    let proc = tempfile::tempdir().expect("a stand-in /proc");
    fs::write(proc.path().join("uptime"), "100.0 1.0\n").expect("uptime");
    let pi = proc.path().join("7");
    fs::create_dir_all(&pi).expect("a pi");
    fs::write(pi.join("cmdline"), "pi\0").expect("its command");
    fs::write(pi.join("status"), "VmRSS:\t 2048 kB\nVmSwap:\t 0 kB\n").expect("status");
    let fields = "S 1 1 1 0 -1 0 0 0 0 0 0 0 0 0 20 0 1 0 0 0 0";
    fs::write(pi.join("stat"), format!("7 (pi) {fields}\n")).expect("stat");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC).minding(proc.path());
    follower.mind(NOON);
    fs::remove_dir_all(&pi).expect("the pi ends");
    follower.mind(NOON + 1);
    follower.write().expect("the day is written");
    let day = read(into.path(), "2026-11-12");
    let held = &day.memory["pi"];
    assert_eq!(held.copies, 0, "none running at the last look: {held:?}");
    assert_eq!(
        (held.most_kib, held.most_at),
        (2_048, NOON),
        "the day's most"
    );

    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .minding(proc.path())
        .resuming(day);
    again.mind(NOON + 2);
    again.write().expect("the day is written again");
    assert_eq!(
        read(into.path(), "2026-11-12").memory["pi"].most_kib,
        2_048,
        "a restart keeps the day's most"
    );
}

#[test]
fn defunct_children_are_minded_and_kept_through_a_restart() {
    let into = tempfile::tempdir().expect("a folder");
    let proc = tempfile::tempdir().expect("a stand-in /proc");
    fs::write(proc.path().join("uptime"), "100.0 1.0\n").expect("uptime");
    let process = |pid: u32, parent: u32, state: char, started_by: &str| {
        let path = proc.path().join(pid.to_string());
        fs::create_dir_all(&path).expect("a process");
        fs::write(path.join("cmdline"), started_by).expect("its command");
        fs::write(
            path.join("stat"),
            format!("{pid} (x) {state} {parent} 1 1\n"),
        )
        .expect("stat");
    };
    process(7, 1, 'S', "pi\0");
    for child in 20..23 {
        process(child, 7, 'Z', "");
    }
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC).minding(proc.path());
    follower.mind(NOON);
    fs::remove_dir_all(proc.path().join("20")).expect("one is reaped");
    follower.mind(NOON + 1);
    follower.write().expect("the day is written");
    let kept: serde_json::Value =
        serde_json::from_slice(&fs::read(into.path().join("2026-11-12.json")).expect("the day"))
            .expect("as JSON");
    assert_eq!(
        kept["defunct"],
        serde_json::json!({"most": 3, "pid": 7, "command": "pi", "at": NOON / 1_000_000}),
        "the day's most, its parent and when, in seconds, as cadence reads it"
    );
    assert_eq!(
        kept["figures"]["defunct.most-under-one-parent"],
        serde_json::json!(3.0),
        "and the figure cadence holds to its bar"
    );

    let mut again = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .minding(proc.path())
        .resuming(read(into.path(), "2026-11-12"));
    again.mind(NOON + 2);
    assert_eq!(again.day.defunct.most, 3, "a restart keeps the day's most");
}

#[test]
fn a_reported_crash_of_the_day_is_counted_once() {
    let into = tempfile::tempdir().expect("a folder");
    let reports = tempfile::tempdir().expect("a reports folder");
    let report = |when: &str| {
        format!(
            "{{\"bug_type\":\"309\",\"name\":\"nix\",\"timestamp\":\"{when}\"}}\n\
             {{\"procPath\":\"/nix/store/abc-nix/bin/nix\"}}"
        )
    };
    fs::write(
        reports.path().join("nix-1.ips"),
        report("2026-11-12 07:00:00.00 -0500"),
    )
    .expect("the day's report");
    fs::write(
        reports.path().join("nix-2.ips"),
        report("2026-11-11 07:00:00.00 -0500"),
    )
    .expect("the day before's");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .reporting(vec![reports.path().to_owned()]);
    follower.read_reports();
    follower.read_reports();
    follower.write().expect("the day is written");
    assert_eq!(
        read(into.path(), "2026-11-12").things["nix"].crashes,
        1,
        "the day's crash counted once, and the day before's not at all"
    );
}

#[test]
fn a_death_of_the_day_is_kept_though_its_presence_goes() {
    let into = tempfile::tempdir().expect("a folder");
    let proc = tempfile::tempdir().expect("a stand-in /proc");
    let presences = tempfile::tempdir().expect("the presences");
    let presence = |session: &str, at: &str| {
        let said = format!(
            r#"{{"session":"{session}","state":"busy","at":"{at}","owner":{{"pid":9,"startTicks":1}}}}"#
        );
        fs::write(presences.path().join(format!("{session}.json")), said).expect("a presence");
    };
    presence("yesterday", "2026-11-11T23:00:00Z");
    presence("today", "2026-11-12T11:00:00Z");
    let mut follower = Follower::new("2026-11-12", into.path(), TimeZone::UTC)
        .minding(proc.path())
        .counting_the_dead(presences.path());
    follower.count_the_dead(NOON);
    fs::remove_file(presences.path().join("today.json")).expect("revived");
    follower.count_the_dead(NOON + 1);
    follower.write().expect("the day is written");
    let agents = read(into.path(), "2026-11-12").agents;
    assert_eq!(
        agents.gone.keys().collect::<Vec<_>>(),
        ["yesterday"],
        "gone at the last look"
    );
    assert_eq!(
        agents.died.keys().collect::<Vec<_>>(),
        ["today"],
        "the day's death, kept after its presence went"
    );
}

/// A failed bash call at a moment in microseconds, as pi logs it.
fn failed(at: i64) -> String {
    format!(
        r#"{{"type":"message","message":{{"role":"toolResult","toolName":"bash","isError":true,"timestamp":{},"content":[{{"type":"text","text":"error: no"}}]}}}}"#,
        at / 1_000
    )
}

#[test]
fn the_sessions_logs_are_read_on_and_never_twice() {
    let into = tempfile::tempdir().expect("a folder");
    let sessions = tempfile::tempdir().expect("the sessions' folder");
    let folder = sessions.path().join("--home-joel-src-core.nix--");
    fs::create_dir_all(&folder).expect("a place");
    let log = folder.join("a.jsonl");
    // A line from the day before, then one of the day and half a line;
    // the day is today's, since only a log written to today is read.
    let today = date_of(Timestamp::now().as_microsecond(), &TimeZone::UTC);
    let noon = |at: i64| at + (Timestamp::now().as_microsecond() / DAY - NOON / DAY) * DAY;
    fs::write(
        &log,
        format!(
            "{}\n{}\n{{\"type",
            failed(noon(NOON - DAY)),
            failed(noon(NOON))
        ),
    )
    .expect("a log of today");
    let mut follower = Follower::new(&today, into.path(), TimeZone::UTC).hearing(sessions.path());
    follower.hear(1 << 20);
    follower.hear(1 << 20);
    follower.write().expect("the day is written");
    let day = read(into.path(), &today);
    let bash = &day.things["pi bash"];
    assert_eq!(bash.kind, Some(Kind::Tools), "a tool's");
    assert_eq!(bash.errors, 1, "the day's line, once: {bash:?}");
    let kept = day.read[&log.to_string_lossy().into_owned()];
    let written = fs::read(&log).expect("the log");
    let whole = written
        .iter()
        .rposition(|&byte| byte == b'\n')
        .expect("a line")
        + 1;
    assert_eq!(kept, whole as u64, "read to its last whole line");

    let mut again = Follower::new(&today, into.path(), TimeZone::UTC)
        .hearing(sessions.path())
        .resuming(day);
    again.hear(1 << 20);
    again.write().expect("the day is written again");
    assert_eq!(
        read(into.path(), &today).things["pi bash"].errors,
        1,
        "a restart reads on from where the last run stopped"
    );
}

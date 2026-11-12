use super::*;

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
    follower
            .take(&format!(
                r#"{{"__REALTIME_TIMESTAMP":"10000000","_BOOT_ID":"old","MESSAGE_ID":"{}","MESSAGE":"System is rebooting."}}"#,
                journal::SHUTTING_DOWN
            ))
            .expect("taken");
    assert_eq!(
        follower.day.boots["old"].shutdown(),
        Some(30_000_000),
        "from logind's line to the boot's last entry"
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
    // Each look at the clock finds ten seconds gone, so the lines come
    // far faster than the half minute a quiet journal would wait.
    let start = Instant::now();
    let mut asked = 0;
    let now = || {
        asked += 1;
        start + Duration::from_secs(10 * asked)
    };
    let mut looks = 0;
    run(&mut follower, &read, 1, now, || {
        looks += 1;
        Looked::default()
    })
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

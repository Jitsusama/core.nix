use std::cell::{Cell, RefCell};

use super::spell::Spell;
use super::*;
use crate::journal;

// When a run ends, and how often it ticks.
mod ends;
// The faults health sees itself, rather than hears.
mod faults;
// A restart across midnight, and the old day's lines heard after it.
mod midnight;
// A spell of failed writes, and what of it health says when.
mod spells;
// A day health can't write, as on a full disk.
mod writes;

/// The wall clock `seconds` into a test, in microseconds, as a spell is
/// given it beside the awake clock.
fn wall(seconds: u64) -> i64 {
    i64::try_from(seconds).expect("seconds a test runs") * 1_000_000
}

/// A warning line at a moment in microseconds.
fn warning(at: i64) -> String {
    format!(
        r#"{{"__REALTIME_TIMESTAMP":"{at}","PRIORITY":"4","SYSLOG_IDENTIFIER":"people","MESSAGE":"slow"}}"#
    )
}

/// A follower's line: a warning `at` microseconds past noon.
fn line(at: i64) -> String {
    warning(NOON + at)
}

/// 2026-11-12 at noon, UTC, in microseconds.
const NOON: i64 = 1_794_484_800_000_000;
const DAY: i64 = 86_400_000_000;

fn read(into: &Path, date: &str) -> Day {
    let file = fs::read(file_of(into, date)).expect("the day was written");
    serde_json::from_slice(&file).expect("the day's file is a day")
}

/// The day written into `into` for `date`, if one was.
fn written(into: &Path, date: &str) -> Option<Day> {
    file_of(into, date).exists().then(|| read(into, date))
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
    assert_eq!(
        (start_of(NOON, &TimeZone::UTC), start_of(NOON, &west)),
        (NOON - 12 * 3_600_000_000, NOON - 7 * 3_600_000_000),
        "a day starts at its own zone's midnight"
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

/// A run of health over followers that send `before` lines, `ends` ends,
/// `after` lines and `tomorrow` lines of the next day, all at once, every
/// sender then gone unless one is
/// held, as a quiet follower holds it, until the clock passes `held_for`.
/// Each look at the clock moves it on `step`, and each wait as far as it
/// asks and `OVERSLEPT` more. While a sender is held `run` never waits,
/// so only a `step` of a half minute or more moves the clock past each
/// half minute for it to hear at once, rather than in real time. The day
/// can't be written, as on a full disk, until the lid has been looked at
/// `unwritable_for` times, or ever at `usize::MAX`; then a held sender
/// sends `tomorrow_after_room` more of the next day's lines. A `busy`
/// held sender sends a line of the day at every look, as a busy journal
/// does, and one sends a line of the day at look `late`, if any. At each
/// look in `flaps`, the day turns writable if it wasn't, or unwritable if
/// it was, as a disk at the edge of full flaps between the two. The wall
/// clock runs with the awake one but for a `slept`: after so many looks,
/// the machine sleeps so long, which only the wall clock sees, from the
/// next look at the clock, as `run` looks at it before the lid in a tick.
/// Once the clock has been looked at `stops` times, if any, the run is
/// stopped, as by a signal.
#[derive(Default)]
struct Ran {
    before: i64,
    ends: usize,
    after: i64,
    held_for: Duration,
    step: Duration,
    unwritable_for: usize,
    tomorrow: i64,
    tomorrow_after_room: i64,
    busy: bool,
    late: usize,
    flaps: &'static [usize],
    slept: Option<(usize, Duration)>,
    stops: usize,
}

/// What a `Ran` came to: what `run` returned, what it left unread, how
/// long health ran, how often it looked at the lid, what it said and
/// noticed, how often it waited nothing, how many lines a busy journal
/// sent, and the day and the next as written.
struct Outcome {
    result: io::Result<()>,
    unread: usize,
    ran: Duration,
    looks: usize,
    said: Vec<String>,
    noticed: Vec<String>,
    idle_waits: usize,
    busy_sent: u64,
    today: Option<Day>,
    tomorrow: Option<Day>,
}

impl Ran {
    /// What `run` left unread, how long health ran, from its first look
    /// at the clock to its last, and how often it looked at the lid, of a
    /// run that ended well.
    fn ran(&self) -> (usize, Duration, usize) {
        let Outcome {
            result,
            unread,
            ran,
            looks,
            ..
        } = self.outcome();
        result.expect("the run");
        (unread, ran, looks)
    }

    /// What the run came to. A run that would never end fails at two
    /// hours, or at ten thousand looks at the clock, for one that spins
    /// with the clock still.
    fn outcome(&self) -> Outcome {
        assert!(
            self.held_for.is_zero() || self.step >= EVERY,
            "a held sender with a step under the half minute waits in real time"
        );
        let into = tempfile::tempdir().expect("a folder");
        let days = self.days(into.path());
        let mut follower = Follower::new("2026-11-12", &days, TimeZone::UTC);
        let (send, read) = self.sent();
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
        let now = || {
            let at = moved(self.step);
            (
                at,
                self.on_the_wall(at.saturating_duration_since(start), looks.get()),
            )
        };
        let idle_waits = Cell::new(0);
        let wait = |time: Duration| {
            idle_waits.set(idle_waits.get() + usize::from(time.is_zero()));
            moved(time + OVERSLEPT);
        };
        let busy_sent = Cell::new(0);
        let send_held = |line: String| {
            if let Some(send) = held.borrow().as_ref() {
                send.send(Some(line)).expect("a line sent");
            }
        };
        let look = || {
            looks.set(looks.get() + 1);
            if looks.get() == self.unwritable_for {
                fs::remove_file(&days).expect("room made for the day");
                for second in 0..self.tomorrow_after_room {
                    send_held(line(DAY + self.tomorrow + second));
                }
            }
            if self.flaps.contains(&looks.get()) {
                flap(&days);
            }
            if self.late > 0 && looks.get() == self.late {
                send_held(line(self.before + self.after));
            }
            if self.busy && held.borrow().is_some() {
                let second = i64::try_from(looks.get()).expect("a few looks");
                send_held(line(self.before + self.after + second));
                busy_sent.set(busy_sent.get() + 1);
            }
            Looked::default()
        };
        let said = RefCell::new(Vec::new());
        let say = |words: &str| said.borrow_mut().push(words.to_owned());
        let noticed = RefCell::new(Vec::new());
        let notice = |words: &str| noticed.borrow_mut().push(words.to_owned());
        let stopped = || self.stops > 0 && times.get() >= self.stops;
        let result = run(
            &mut follower,
            &read,
            now,
            wait,
            look,
            stopped,
            &say,
            &notice,
        );
        let started = first.get().expect("a look at the clock");
        Outcome {
            result,
            unread: read.try_iter().count(),
            ran: clock.get().saturating_duration_since(started),
            looks: looks.get(),
            said: said.take(),
            noticed: noticed.take(),
            idle_waits: idle_waits.get(),
            busy_sent: busy_sent.get(),
            today: written(&days, "2026-11-12"),
            tomorrow: written(&days, "2026-11-13"),
        }
    }

    /// Where the day is written, in `into`: while it can't be, a file
    /// stands in its folder's place, so no day can be written.
    fn days(&self, into: &Path) -> PathBuf {
        let days = into.join("days");
        if self.unwritable_for > 0 {
            fs::write(&days, "").expect("a file in the folder's place");
        }
        days
    }

    /// The lines sent before the run starts, and the followers' ends among
    /// them, with the sender that sent them.
    fn sent(&self) -> (mpsc::Sender<Option<String>>, mpsc::Receiver<Option<String>>) {
        let (send, read) = mpsc::channel();
        for second in 0..self.before {
            send.send(Some(line(second))).expect("a line sent");
        }
        for _ in 0..self.ends {
            send.send(None).expect("a follower's end sent");
        }
        for second in 0..self.after {
            send.send(Some(line(self.before + second)))
                .expect("a line sent");
        }
        for second in 0..self.tomorrow {
            send.send(Some(line(DAY + second))).expect("a line sent");
        }
        (send, read)
    }

    /// The wall clock, in microseconds, once the run has been awake
    /// `awake` and looked at the lid `looks` times.
    fn on_the_wall(&self, awake: Duration, looks: usize) -> i64 {
        let asleep = self
            .slept
            .filter(|&(after, _)| looks >= after)
            .map_or(Duration::ZERO, |(_, slept)| slept);
        NOON + i64::try_from((awake + asleep).as_micros()).expect("a test's length")
    }
}

/// Turn `days` writable if it isn't, or unwritable if it is, as a disk
/// at the edge of full flaps between the two.
fn flap(days: &Path) {
    if days.is_file() {
        fs::remove_file(days).expect("room made for the day");
    } else {
        if days.exists() {
            fs::remove_dir_all(days).expect("the day's folder taken away");
        }
        fs::write(days, "").expect("a file in the folder's place");
    }
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

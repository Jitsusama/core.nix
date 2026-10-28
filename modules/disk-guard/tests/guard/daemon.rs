use disk_guard::band::Band;
use disk_guard::clock::Tick;
use disk_guard::config::Config;
use disk_guard::daemon::{Explained, Guard, Places, Senses};
use disk_guard::moments::{Power, Pressure};
use disk_guard::replaced::Uses;
use disk_guard::socket::{Reply, Request};
use disk_guard::units::Size;
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::fs;
use std::path::{Path, PathBuf};

fn wall() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

/// A tick an hour into a boot that hasn't been suspended.
fn tick() -> Tick {
    Tick {
        wall: wall(),
        monotonic: 3600.0,
        suspended: 0.0,
        boot: "test".into(),
    }
}

const CALM: Senses = Senses {
    power: Power {
        on_charger: true,
        battery: None,
    },
    saver: false,
    pressure: Pressure {
        io: disk_guard::moments::Stall {
            some: 0.0,
            full: 0.0,
        },
        memory: disk_guard::moments::Stall {
            some: 0.0,
            full: 0.0,
        },
        cpu: disk_guard::moments::Stall {
            some: 0.0,
            full: 0.0,
        },
    },
};

fn touch(path: &Path, seconds: i64) {
    let at = Timespec {
        tv_sec: seconds,
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

struct Home {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Home {
    fn new() -> Home {
        Home::within(tempfile::tempdir().unwrap())
    }

    fn within(dir: tempfile::TempDir) -> Home {
        let root = fs::canonicalize(dir.path()).unwrap();
        fs::create_dir_all(root.join("state")).unwrap();
        // No processes, so nothing is in use.
        fs::create_dir_all(root.join("proc")).unwrap();
        Home { _dir: dir, root }
    }

    /// A scratch entry nothing has written to since 2001.
    fn cold_scratch(&self, name: &str) -> PathBuf {
        let entry = self.root.join("scratch").join(name);
        fs::create_dir_all(&entry).unwrap();
        fs::write(entry.join("out"), "x".repeat(8192)).unwrap();
        touch(&entry.join("out"), 1_000_000_000);
        touch(&entry, 1_000_000_000);
        entry
    }

    fn trashed(&self, name: &str, deleted: &str) -> PathBuf {
        let trash = self.root.join("Trash");
        fs::create_dir_all(trash.join("files")).unwrap();
        fs::create_dir_all(trash.join("info")).unwrap();
        fs::write(trash.join("files").join(name), "old").unwrap();
        fs::write(
            trash.join("info").join(format!("{name}.trashinfo")),
            format!("[Trash Info]\nPath=/x/{name}\nDeletionDate={deleted}\n"),
        )
        .unwrap();
        trash.join("files").join(name)
    }

    /// A user config whose bands are shares of the filesystem, so `100%`
    /// puts any disk in pressure and `0%` keeps it healthy. Its Nix store
    /// doesn't exist: with no system daemon answering, a user daemon
    /// collects garbage itself from urgent on, and the machine's own store
    /// is no test's to collect.
    fn config(&self, pressure: &str, target: &str) -> Config {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true,
                 "bands": {{ "pressure": "{pressure}", "urgent": "0%", "critical": "0%", "target": "{target}" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{
                   "scratch": {{ "kind": "scratch", "path": "{0}/scratch" }},
                   "trash": {{ "kind": "trash", "path": "{0}/Trash" }}
                 }} }}"#,
            self.root.display()
        ))
        .unwrap()
    }

    fn guard(&self, config: Config) -> Guard {
        Guard::new(crate::unpaced(config), &self.places()).unwrap()
    }

    fn places(&self) -> Places {
        Places {
            state: self.root.join("state"),
            status: Some(self.root.join("status.json")),
            processes: self.root.join("proc"),
            system: None,
            running: Vec::new(),
        }
    }

    /// A process of root's this account can't read.
    fn hidden(&self, pid: u32, start: u64) {
        let proc = self.root.join("proc");
        let at = |file: &str| format!("{pid}/{file}");
        crate::procs::text(
            &proc,
            &at("stat"),
            &crate::procs::stat(pid, "rustc", 0, start),
        );
        crate::procs::text(&proc, &at("status"), "Name:\trustc\nUid:\t0\t0\t0\t0\n");
        crate::procs::text(&proc, &at("cmdline"), "");
    }

    fn journal(&self) -> Vec<serde_json::Value> {
        fs::read_to_string(self.root.join("state/journal.jsonl"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }

    fn status(&self) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.root.join("status.json")).unwrap()).unwrap()
    }
}

/// Somewhere short for a socket: the temp directory as named, not
/// resolved. A TMPDIR that is a link, as a pi quest's is, can resolve to
/// a path that leaves a socket past its 108-byte limit.
pub(crate) fn socket_dir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// A system daemon at `socket` that vouches for process 300, says it
/// collects on every filesystem watched when `collects`, and collects
/// whenever asked, telling what it heard.
pub(crate) fn fake_system(socket: &Path, collects: bool) -> std::sync::mpsc::Receiver<Request> {
    let mut server = disk_guard::socket::Server::bind(socket, std::time::Duration::ZERO).unwrap();
    let (telling, heard) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        loop {
            server
                .serve(&mut |_, request, responder| {
                    let reply = match &request {
                        Request::Processes { .. } => Reply::Answer(disk_guard::vouch::Answer {
                            vouched: vec![(300, 77)],
                            touching: vec![],
                        }),
                        Request::Watching { paths } if collects => Reply::Collects(paths.clone()),
                        Request::Watching { .. } => Reply::Collects(Vec::new()),
                        Request::Collect { .. } => Reply::Collected("reached".to_owned()),
                    };
                    let _ = telling.send(request);
                    responder.send(&reply);
                })
                .unwrap();
        }
    });
    heard
}

#[test]
fn a_user_daemon_asks_the_system_for_the_store_garbage_in_its_turn() {
    let home = Home::new();
    home.cold_scratch("run-1");
    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let heard = fake_system(&socket, true);
    let places = Places {
        system: Some(socket),
        ..home.places()
    };
    let mut healthy = Guard::new(crate::unpaced(home.config("0%", "0%")), &places).unwrap();
    healthy.step(&tick(), &CALM).unwrap();
    let asked: Vec<Request> = heard.try_iter().collect();
    let [Request::Watching { paths }] = asked.as_slice() else {
        panic!("a heartbeat, and nothing collected while healthy: {asked:?}");
    };
    let mount = paths[0].clone();
    assert!(home.root.starts_with(&mount), "{mount:?}");
    let mut pressed = Guard::new(crate::unpaced(home.config("100%", "100%")), &places).unwrap();
    pressed.step(&tick(), &CALM).unwrap();
    let asked: Vec<Request> = heard.try_iter().collect();
    let collect = Request::Collect {
        path: mount,
        band: Band::Pressure,
    };
    assert!(asked.contains(&collect), "{asked:?}");
    let garbage = home
        .journal()
        .into_iter()
        .any(|line| line["kind"] == "garbage" && line["outcome"] == "reclaimed");
    assert!(garbage, "{:?}", home.journal());
}

#[test]
fn the_system_daemon_vouches_for_what_this_account_cant_read() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    home.hidden(300, 77);
    let mut alone = home.guard(home.config("100%", "100%"));
    alone.step(&tick(), &CALM).unwrap();
    assert!(cold.exists(), "kept while process 300 can't be judged");

    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let _heard = fake_system(&socket, false);
    let places = Places {
        system: Some(socket),
        ..home.places()
    };
    let mut vouched = Guard::new(crate::unpaced(home.config("100%", "100%")), &places).unwrap();
    vouched.step(&tick(), &CALM).unwrap();
    assert!(
        !cold.exists(),
        "process 300 touches nothing of this account's"
    );
}

#[test]
fn a_directory_is_begun_in_the_journal_before_it_is_moved() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let trashed = home.trashed("old.txt", "2001-09-09T01:46:40");
    let mut guard = home.guard(home.config("100%", "100%"));
    guard.step(&tick(), &CALM).unwrap();
    let said: Vec<(String, String)> = home
        .journal()
        .iter()
        .map(|line| {
            (
                line["event"].as_str().unwrap().to_owned(),
                line["path"].as_str().unwrap_or("").to_owned(),
            )
        })
        .collect();
    let at = |event: &str, path: &Path| {
        said.iter()
            .position(|(e, p)| e == event && p == path.to_str().unwrap())
    };
    let (begun, done) = (at("begin", &cold).unwrap(), at("action", &cold).unwrap());
    assert!(begun < done, "{said:?}");
    assert!(
        at("begin", &trashed).is_none(),
        "a file goes in one unlink: {said:?}"
    );
    assert!(at("action", &trashed).unwrap() < at("pass", Path::new("")).unwrap());
    let beginning = &home.journal()[begun];
    assert_eq!(
        beginning["root"],
        home.root.join("scratch").to_str().unwrap()
    );
}

#[test]
fn what_a_guard_left_moved_aside_is_emptied_when_the_next_starts() {
    let home = Home::new();
    let left = |name: &str| {
        let entry = home.root.join("scratch").join(name);
        let trash = entry.join(".disk-guard-trash-abc123");
        fs::create_dir_all(trash.join("deep")).unwrap();
        fs::write(trash.join("deep/out"), "built").unwrap();
        (entry, trash)
    };
    let (unfinished, its_trash) = left("run-1");
    let (finished, kept_trash) = left("run-2");
    let scratch = home.root.join("scratch");
    let line = |event: &str, path: &Path| {
        format!(
            r#"{{"event":"{event}","at":1.0,"band":"pressure","path":"{}","root":"{}","kind":"scratch","size":5,"outcome":"reclaimed"}}"#,
            path.display(),
            scratch.display()
        )
    };
    fs::write(
        home.root.join("state/journal.jsonl"),
        [
            line("begin", &unfinished),
            line("begin", &finished),
            line("action", &finished),
        ]
        .join("\n")
            + "\n",
    )
    .unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    assert!(!its_trash.exists(), "emptied");
    assert!(unfinished.exists(), "only what was moved aside goes");
    assert!(kept_trash.exists(), "what was ended is no one's to resume");
    let resumed = home
        .journal()
        .into_iter()
        .find(|line| line["event"] == "action" && line["kind"] == "resumed")
        .unwrap();
    assert_eq!(resumed["path"], unfinished.to_str().unwrap());
    assert_eq!(resumed["outcome"], "reclaimed");
    guard.step(&tick(), &CALM).unwrap();
    let resumptions = home
        .journal()
        .iter()
        .filter(|line| line["kind"] == "resumed")
        .count();
    assert_eq!(
        resumptions, 1,
        "what was resumed is ended, and not resumed again"
    );
}

#[test]
fn a_producer_that_ran_is_not_run_again_within_the_hour_even_by_the_next_guard() {
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let marker = home.root.join("ran");
    let mut config = home.config("100%", "100%");
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!("echo ran >> {}", marker.display()),
            ],
            path: home.root.clone(),
            every: None,
            from: Some(Band::Pressure),
            watch: Vec::new(),
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    let runs = || fs::read_to_string(&marker).map_or(0, |text| text.lines().count());
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut guard = home.guard(config.clone());
    guard.step(&tick_at(3600.0), &CALM).unwrap();
    assert_eq!((runs(), passes()), (1, 1));
    // Past the rest that pass earned, and within the hour.
    guard.step(&tick_at(3661.0), &CALM).unwrap();
    assert_eq!((runs(), passes()), (1, 2));
    drop(guard);
    let mut guard = home.guard(config);
    guard.step(&tick_at(3700.0), &CALM).unwrap();
    assert_eq!((runs(), passes()), (1, 3), "remembered across guards");
    let ran = home
        .journal()
        .into_iter()
        .find(|line| line["kind"] == "producer")
        .unwrap();
    assert_eq!(ran["path"], format!("{}#gc", home.root.display()));
    assert_eq!(ran["outcome"], "reclaimed");
}

#[test]
fn a_producer_runs_again_once_what_it_watches_changes() {
    // A tool's cache that gains a version every few minutes keeps every old
    // one until its producer runs: run every six hours, a cache of zone
    // builds held 24 versions of each tool and 17.7 GiB of the store on 28
    // October, where its own rule keeps four.
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let marker = home.root.join("ran");
    let stamp = home.root.join("stamp");
    fs::write(&stamp, "").unwrap();
    touch(&stamp, 1_000_000_000);
    let mut config = home.config("0%", "0%");
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!("echo ran >> {}", marker.display()),
            ],
            path: home.root.clone(),
            every: Some(Span(std::time::Duration::from_hours(6))),
            from: None,
            watch: vec![stamp.clone()],
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    let runs = || fs::read_to_string(&marker).map_or(0, |text| text.lines().count());
    let start = wall().floor();
    let at = |seconds: f64| Tick {
        wall: start + seconds,
        monotonic: 3600.0 + seconds,
        ..tick()
    };
    let mut guard = home.guard(config);
    guard.step(&at(0.0), &CALM).unwrap();
    assert_eq!(runs(), 1, "the first upkeep runs it");
    guard.step(&at(61.0), &CALM).unwrap();
    assert_eq!(runs(), 1, "nothing it watches has changed");
    #[expect(clippy::cast_possible_truncation, reason = "a wall clock in seconds")]
    touch(&stamp, start as i64 + 100);
    guard.step(&at(122.0), &CALM).unwrap();
    assert_eq!(runs(), 2, "what it watches changed since it ran");
    guard.step(&at(183.0), &CALM).unwrap();
    assert_eq!(runs(), 2, "once for each change");
}

/// Senses a disk and memory stalling half the time, as a machine building
/// all day does.
fn busy() -> Senses {
    let stalling = disk_guard::moments::Stall {
        some: 50.0,
        full: 5.0,
    };
    Senses {
        pressure: Pressure {
            io: stalling,
            memory: stalling,
            ..CALM.pressure
        },
        ..CALM
    }
}

/// A machine's pressure files, stalling as `busy` says, so whatever reads
/// them rather than the senses finds the machine busy too.
fn stalling(home: &Home) {
    let pressure = home.root.join("proc/pressure");
    fs::create_dir_all(&pressure).unwrap();
    for resource in ["io", "memory"] {
        fs::write(
            pressure.join(resource),
            "some avg10=50.00 avg60=50.00 avg300=50.00 total=0\n\
             full avg10=5.00 avg60=5.00 avg300=5.00 total=0\n",
        )
        .unwrap();
    }
}

#[test]
fn routine_upkeep_takes_what_is_due_however_busy_the_machine() {
    // A workstation that's always busy would never be tidied if upkeep
    // waited for calm, so it paces its deletes instead.
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    let older = home.trashed("older.txt", "2001-09-08T01:46:40");
    stalling(&home);
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &busy()).unwrap();
    assert!(!old.exists() && !older.exists());
    let outcomes: Vec<_> = home
        .journal()
        .into_iter()
        .filter(|line| line["event"] == "action")
        .map(|line| line["outcome"].clone())
        .collect();
    assert_eq!(outcomes, ["reclaimed", "reclaimed"]);
}

/// The scheduling policy a producer ran under in one step of a guard
/// whose bands are `pressure` and `urgent`.
fn producer_policy(pressure: &str, urgent: f64) -> String {
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let marker = home.root.join("policy");
    let mut config = home.config(pressure, "100%");
    config.bands.urgent = Size::Percent(urgent);
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!("cut -d' ' -f41 /proc/$$/stat > {}", marker.display()),
            ],
            path: home.root.clone(),
            every: Some(Span(std::time::Duration::from_secs(60))),
            from: Some(Band::Urgent),
            watch: Vec::new(),
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    home.guard(config).step(&tick(), &CALM).unwrap();
    fs::read_to_string(&marker).unwrap().trim().to_owned()
}

#[test]
fn a_command_runs_at_idle_priority_unless_the_pass_is_urgent() {
    assert_eq!(
        producer_policy("0%", 0.0),
        libc::SCHED_IDLE.to_string(),
        "upkeep"
    );
    assert_eq!(
        producer_policy("100%", 100.0),
        crate::commands::guards_policy().to_string(),
        "urgent"
    );
}

#[test]
fn an_urgent_pass_never_gives_way() {
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    let busy = home.root.join("proc/pressure");
    fs::create_dir_all(&busy).unwrap();
    fs::write(
        busy.join("memory"),
        "some avg10=90.00 avg60=0.00 avg300=0.00 total=0\n",
    )
    .unwrap();
    let mut config = home.config("100%", "100%");
    config.bands.urgent = Size::Percent(100.0);
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    assert!(!old.exists());
    let pass = home
        .journal()
        .into_iter()
        .find(|line| line["event"] == "pass")
        .unwrap();
    assert_eq!(pass["band"], "urgent");
}

fn tick_at(monotonic: f64) -> Tick {
    Tick {
        monotonic,
        ..tick()
    }
}

#[test]
fn a_pass_that_falls_short_rests_before_the_next_and_longer_each_time() {
    let home = Home::new();
    home.cold_scratch("run-1");
    // Kept every time, since process 300 can't be judged.
    home.hidden(300, 77);
    let mut guard = home.guard(home.config("100%", "100%"));
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3605.0, 3655.0, 3661.0, 3700.0, 3775.0, 3782.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(seen, [1, 1, 1, 2, 2, 2, 3], "a minute's rest, then two");
}

#[test]
fn a_routine_pass_rests_after_each_batch_it_unlinks() {
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    let older = home.trashed("older.txt", "2001-09-08T01:46:40");
    let mut config = home.config("0%", "0%");
    // Not through `guard`, which takes the rest away: a batch of one
    // file, and a second's rest after each.
    config.pacing = disk_guard::config::Pacing {
        files: 1,
        rest: disk_guard::units::Span(std::time::Duration::from_secs(1)),
    };
    let mut guard = Guard::new(config, &home.places()).unwrap();
    let began = std::time::Instant::now();
    guard.step(&tick(), &CALM).unwrap();
    assert!(!old.exists() && !older.exists());
    let took = began.elapsed();
    assert!(took >= std::time::Duration::from_secs(2), "{took:?}");
}

#[test]
fn a_pressed_disk_is_reclaimed_at_once_however_busy_the_machine() {
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    stalling(&home);
    let mut guard = home.guard(home.config("100%", "100%"));
    guard.step(&tick(), &busy()).unwrap();
    assert!(!old.exists());
    let pass = home
        .journal()
        .into_iter()
        .find(|line| line["event"] == "pass")
        .unwrap();
    assert_eq!(pass["band"], "pressure");
}

#[test]
fn a_pass_that_falls_short_at_critical_rests_a_minute_at_most() {
    let home = Home::new();
    home.cold_scratch("run-1");
    home.hidden(300, 77);
    let mut config = home.config("100%", "100%");
    config.bands.urgent = Size::Percent(100.0);
    config.bands.critical = Size::Percent(100.0);
    let mut guard = home.guard(config);
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3661.0, 3782.0, 3843.0, 3904.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(seen, [1, 2, 3, 4, 5], "a minute's rest each time");
}

#[test]
fn a_preview_says_how_many_processes_it_could_not_judge_and_why() {
    let home = Home::new();
    home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("0%", "100%"));
    guard.preview(&tick(), Some(Band::Pressure), None).unwrap();
    assert_eq!(guard.unread(), (0, None));
    home.hidden(300, 77);
    // And one of the account's, read whole.
    let proc = home.root.join("proc");
    let me = unsafe { libc::geteuid() };
    crate::procs::text(&proc, "400/stat", &crate::procs::stat(400, "zsh", 0, 90));
    crate::procs::text(
        &proc,
        "400/status",
        &format!("Name:\tzsh\nUid:\t{me}\t{me}\t{me}\t{me}\n"),
    );
    crate::procs::text(&proc, "400/cmdline", "zsh");
    crate::procs::text(&proc, "400/maps", "");
    crate::procs::link(&proc, "400/cwd", "/");
    fs::create_dir_all(proc.join("400/fd")).unwrap();
    let places = Places {
        system: Some(home.root.join("no-system.sock")),
        ..home.places()
    };
    let mut guard = Guard::new(crate::unpaced(home.config("0%", "100%")), &places).unwrap();
    guard.preview(&tick(), Some(Band::Pressure), None).unwrap();
    let (unread, why) = guard.unread();
    assert_eq!(unread, 1);
    assert!(why.is_some(), "the system daemon didn't answer");
}

#[test]
fn a_preview_says_what_a_pass_would_take_and_takes_nothing() {
    let home = Home::new();
    let first = home.cold_scratch("run-1");
    let second = home.cold_scratch("run-2");
    let trashed = home.trashed("old.txt", "2001-09-09T01:46:40");
    let mut guard = home.guard(home.config("0%", "100%"));
    let previews = guard.preview(&tick(), Some(Band::Pressure), None).unwrap();
    assert_eq!(previews.len(), 1);
    let preview = &previews[0];
    assert_eq!(preview.band, Band::Pressure);
    let mut taken: Vec<_> = preview.taken.iter().map(|c| c.path.clone()).collect();
    taken.sort();
    let mut expected = vec![first.clone(), second.clone(), trashed.clone()];
    expected.sort();
    assert_eq!(taken, expected);
    assert_eq!(preview.end, "exhausted");
    assert!(first.exists() && second.exists() && trashed.exists());
    assert!(
        home.journal().is_empty(),
        "nothing done, nothing journalled"
    );
}

/// A step can run for minutes, and its status was stamped with the
/// moment it began: on 28 October a system guard's read 189 seconds old
/// as it was written, and the desktop said the disks weren't watched.
#[test]
fn a_status_is_stamped_when_it_is_written_not_when_its_step_began() {
    let home = Home::new();
    let mut guard = home.guard(home.config("0%", "0%"));
    let begun = Tick {
        wall: wall() - 300.0,
        ..tick()
    };
    guard.step(&begun, &CALM).unwrap();
    let age = wall() - home.status()["heartbeat"].as_f64().unwrap();
    assert!(age < 5.0, "written {age:.0} seconds ago");
}

/// A restarted guard's first step can take minutes, in which the status
/// the last one left went quiet and the desktop took a restart for a
/// guard that had stopped. It beats from its first moment, on what the
/// last one said until it has said something itself.
#[test]
fn a_restarted_guard_beats_on_the_last_status_before_its_first_step() {
    let home = Home::new();
    fs::write(
        home.root.join("status.json"),
        r#"{ "version": 2, "heartbeat": 1, "says": "left by the last guard" }"#,
    )
    .unwrap();
    let guard = home.guard(home.config("0%", "0%"));
    guard.beating()();
    let status = home.status();
    let age = wall() - status["heartbeat"].as_f64().unwrap();
    assert!(age < 5.0, "beat {age:.0} seconds ago");
    assert_eq!(status["says"], "left by the last guard");
}

/// Once a guard has written its own, a beat rewrites that, fresh.
#[test]
fn a_beat_rewrites_the_status_last_written_with_a_fresh_heartbeat() {
    let home = Home::new();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    let written = home.status();
    fs::write(home.root.join("status.json"), r#"{ "heartbeat": 1 }"#).unwrap();
    guard.beating()();
    let beaten = home.status();
    assert!(wall() - beaten["heartbeat"].as_f64().unwrap() < 5.0);
    assert_eq!(beaten["says"], written["says"]);
    assert_eq!(
        beaten["filesystems"][0]["path"],
        written["filesystems"][0]["path"]
    );
}

#[test]
fn pretending_free_space_bands_the_preview() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("50%", "60%"));
    let short = guard.preview(&tick(), None, Some(Size::Bytes(0))).unwrap();
    assert_eq!(short[0].band, Band::Pressure);
    assert_eq!(short[0].free, 0);
    assert_eq!(short[0].taken[0].path, cold);
    let total = short[0].total;
    let roomy = guard
        .preview(&tick(), None, Some(Size::Percent(100.0)))
        .unwrap();
    assert_eq!(roomy[0].free, total);
    let mut low = home.guard(home.config("1%", "2%"));
    let banded = low.preview(&tick(), Some(Band::Pressure), None).unwrap();
    assert!(
        banded[0].free < total / 100,
        "a band alone pretends free space just short of it: {} of {total}",
        banded[0].free
    );
    assert_eq!(banded[0].taken[0].path, cold);
    assert_eq!(roomy[0].band, Band::Healthy);
    assert!(
        roomy[0].taken.is_empty(),
        "scratch with no healthy window stays while healthy"
    );
    assert!(cold.exists());
}

#[test]
fn a_disk_in_a_band_is_answered_by_a_pass() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("100%", "100%"));
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        !cold.exists(),
        "a day idle goes at pressure: {:?}",
        home.journal()
    );
    let journal = home.journal();
    let action = journal
        .iter()
        .find(|line| line["event"] == "action")
        .unwrap();
    assert_eq!(action["path"], cold.to_str().unwrap());
    assert_eq!(action["outcome"], "reclaimed");
    let pass = journal.iter().find(|line| line["event"] == "pass").unwrap();
    assert_eq!(pass["band"], "pressure");
    assert_eq!(pass["end"], "exhausted");
    let status = home.status();
    assert_eq!(status["filesystems"][0]["band"], "pressure");
    assert!(status["heartbeat"].as_f64().unwrap() > 0.0);
    assert!(home.root.join("state/memory.json").exists());
    assert_eq!(status["version"], 2);
    let disk = &status["filesystems"][0];
    assert_eq!(
        disk["used"].as_u64().unwrap() + disk["free"].as_u64().unwrap(),
        disk["size"].as_u64().unwrap()
    );
    assert!(
        disk["says"].as_str().unwrap().contains("is filling"),
        "{disk}"
    );
    assert_eq!(
        status["says"], disk["says"],
        "the machine speaks for its one disk"
    );
    let cleared = &status["cleared"][0];
    assert_eq!(cleared["path"], cold.to_str().unwrap());
    assert_eq!(cleared["kind"], "scratch");
    assert!(
        cleared["says"]
            .as_str()
            .unwrap()
            .starts_with("Cleared scratch space, "),
        "{cleared}"
    );
}

#[test]
fn a_fresh_measure_moves_the_band_a_status_shows() {
    use disk_guard::band::Space;
    use disk_guard::config::Bands;
    use disk_guard::daemon::FilesystemStatus;
    let bands = Bands {
        pressure: Size::Percent(25.0),
        urgent: Size::Percent(10.0),
        critical: Size::Percent(4.0),
        target: Size::Percent(35.0),
    };
    let space = |free| Space {
        total: 1000,
        free,
        btrfs: None,
    };
    let mut status = FilesystemStatus {
        path: PathBuf::from("/"),
        band: Band::Pressure,
        free: 200,
        total: 1000,
        size: 1000,
        used: 800,
        forecast: Some(60.0),
        fill_per_hour: Some(36_000.0),
        full_at: Some(1960.0),
        says: String::new(),
        exhausted: false,
    };
    status.measured(&space(210), &bands, 1900.0, None);
    assert_eq!(status.band, Band::Pressure);
    assert_eq!(status.free, 210);
    assert_eq!(status.used, 790);
    assert_eq!(
        status.says,
        "The disk is filling, with 210 B free. \
         The guard is clearing builds and caches nobody has used lately. \
         At this rate it fills in 60 s.",
        "said again from the fresh measure, filling to the moment it was due"
    );
    assert_eq!(
        status.forecast,
        Some(60.0),
        "still the band it was made for"
    );
    status.measured(&space(300), &bands, 1900.0, None);
    assert_eq!(status.band, Band::Healthy);
    assert_eq!(status.free, 300);
    assert_eq!(status.forecast, None);
    status.band = Band::Pressure;
    status.exhausted = true;
    status.full_at = None;
    status.measured(&space(200), &bands, 1900.0, None);
    assert!(status.exhausted, "the same band's pass still holds");
    assert!(
        status.says.contains("has cleared all it may"),
        "{}",
        status.says
    );
    status.measured(&space(50), &bands, 1900.0, None);
    assert_eq!(status.band, Band::Urgent);
    assert!(!status.exhausted, "a worse band's pass hasn't run yet");
    assert!(
        status.says.contains("is clearing everything"),
        "{}",
        status.says
    );
}

#[test]
fn a_filesystem_is_named_by_where_it_is_mounted() {
    use disk_guard::pool::pool;
    use std::os::unix::fs::MetadataExt;
    let named = |home: &Home| {
        home.cold_scratch("run-1");
        home.guard(home.config("0%", "0%"))
            .step(&tick(), &CALM)
            .unwrap();
        PathBuf::from(home.status()["filesystems"][0]["path"].as_str().unwrap())
    };
    let device = |path: &Path| fs::metadata(path).unwrap().dev();
    let pooled = |path: &Path| pool(path).unwrap();
    let home = Home::new();
    let mount = named(&home);
    assert!(
        home.root.starts_with(&mount),
        "{mount:?} holds {:?}",
        home.root
    );
    assert_eq!(pooled(&mount), pooled(&home.root));
    if let Some(above) = mount.parent() {
        assert_ne!(
            pooled(above),
            pooled(&mount),
            "{mount:?} is the top of its filesystem"
        );
    }
    // A mount whose parent lies on another device, which a temporary
    // directory on the root filesystem can't show.
    let shm = Path::new("/dev/shm");
    if shm.is_dir() && device(shm) != device(Path::new("/dev")) {
        let home = Home::within(tempfile::tempdir_in(shm).unwrap());
        assert_eq!(named(&home), shm);
    }
}

/// Btrfs numbers each subvolume as a device of its own, but they share
/// the pool's space, so sources in two of them are on one filesystem.
#[test]
fn the_subvolumes_of_one_pool_are_watched_as_one_filesystem() {
    let home = Home::new();
    if !crate::pool::on_btrfs(&home.root) {
        eprintln!("skipped: {} isn't on Btrfs", home.root.display());
        return;
    }
    crate::pool::subvolume(&home.root, "scratch");
    home.cold_scratch("run-1");
    home.trashed("old", "2001-01-01T00:00:00");
    home.guard(home.config("0%", "0%"))
        .step(&tick(), &CALM)
        .unwrap();
    let filesystems = &home.status()["filesystems"];
    assert_eq!(filesystems.as_array().unwrap().len(), 1, "{filesystems}");
}

#[test]
fn a_band_is_answered_when_the_machine_could_not_spare_upkeep() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("100%", "100%"));
    let drained = Senses {
        power: Power {
            on_charger: false,
            battery: Some(5),
        },
        ..CALM
    };
    guard.step(&tick(), &drained).unwrap();
    assert!(!cold.exists());
}

#[test]
fn a_healthy_disk_gets_its_upkeep_and_then_rests() {
    let home = Home::new();
    let expired = home.trashed("old.txt", "2020-01-01T00:00:00");
    let recent = home.trashed("new.txt", "2999-01-01T00:00:00");
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    assert!(!expired.exists(), "trash kept past 30 days goes in upkeep");
    assert!(recent.exists());
    assert!(cold.exists(), "scratch without a healthy window stays");
    assert_eq!(home.status()["filesystems"][0]["band"], "healthy");
    let later = home.trashed("later.txt", "2020-01-01T00:00:00");
    guard.step(&tick(), &CALM).unwrap();
    assert!(later.exists(), "upkeep isn't due again for six hours");
    // A guard starting again remembers when upkeep last ran.
    let mut again = home.guard(home.config("0%", "0%"));
    again.step(&tick(), &CALM).unwrap();
    assert!(later.exists());
}

/// A user config watching cargo builds under `trees`, each tree cut by a
/// broker whose records are in `cut`, and a healthy disk.
fn owned_trees(home: &Home) -> Config {
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true, "notify": false,
             "bands": {{ "pressure": "0%", "urgent": "0%", "critical": "0%", "target": "0%" }},
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{
               "src": {{ "kind": "cargo", "roots": ["{0}/trees"] }},
               "workspaces": {{ "kind": "workspaces", "path": "{0}/workspaces",
                                "quests": "{0}/quests", "trees": "{0}/cut" }} }} }}"#,
        home.root.display()
    ))
    .unwrap()
}

/// A tree with a cargo build in it, held by process `pid`, which runs: the
/// build's one file.
fn held_tree(home: &Home, pid: u32) -> PathBuf {
    let tree = home.root.join("trees/app");
    fs::create_dir_all(tree.join("src")).unwrap();
    fs::write(tree.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
    let target = tree.join("target");
    fs::create_dir_all(target.join("debug")).unwrap();
    fs::write(
        target.join("CACHEDIR.TAG"),
        "Signature: 8a477f597d28d172789f06886806bc55\n# created by cargo\n",
    )
    .unwrap();
    fs::write(target.join("debug/app"), "x".repeat(8192)).unwrap();
    fs::create_dir_all(home.root.join("workspaces")).unwrap();
    fs::create_dir_all(home.root.join("quests")).unwrap();
    fs::create_dir_all(home.root.join("cut")).unwrap();
    let start = 9_000;
    let boot = disk_guard::sys::boot(Path::new("/proc/stat")).unwrap();
    let started = (boot.wall + disk_guard::units::float(start) / boot.ticks) * 1000.0;
    fs::write(
        home.root.join("cut/app.json"),
        format!(
            r#"{{"path":"{}","owners":[{{"pid":{pid},"startedAt":{started}}}]}}"#,
            tree.display()
        ),
    )
    .unwrap();
    let proc = home.root.join("proc");
    let me = unsafe { libc::geteuid() };
    let at = |file: &str| format!("{pid}/{file}");
    crate::procs::text(&proc, &at("stat"), &crate::procs::stat(pid, "pi", 0, start));
    crate::procs::text(
        &proc,
        &at("status"),
        &format!("Name:\tpi\nUid:\t{me}\t{me}\t{me}\t{me}\n"),
    );
    crate::procs::text(&proc, &at("cmdline"), "pi\0");
    crate::procs::text(&proc, &at("maps"), "");
    crate::procs::link(&proc, &at("cwd"), home.root.to_str().unwrap());
    crate::procs::link(&proc, &at("exe"), "/bin/pi");
    fs::create_dir_all(proc.join(at("fd"))).unwrap();
    target.join("debug/app")
}

#[test]
fn a_trees_build_goes_within_a_minute_of_its_last_holder_ending() {
    // Upkeep comes every six hours, but what an owner leaves is spent the
    // moment it ends: waiting for routine upkeep kept 190 GiB of builds
    // whose sessions had ended for hours on 27 October.
    let home = Home::new();
    let built = held_tree(&home, 400);
    let mut guard = home.guard(owned_trees(&home));
    guard.step(&tick(), &CALM).unwrap();
    assert!(built.exists(), "a held tree's build stays through upkeep");
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    guard.step(&tick_at(3630.0), &CALM).unwrap();
    assert!(built.exists(), "owners are looked at once a minute");
    guard.step(&tick_at(3661.0), &CALM).unwrap();
    assert!(!built.exists(), "the build goes once its holder has ended");
}

#[test]
fn a_new_boot_makes_upkeep_due_and_a_restart_does_not() {
    // A boot ends every program, so what they left in temp is spent; a
    // guard merely restarting in the same boot ends nothing.
    let home = Home::new();
    let expired = home.trashed("old.txt", "2020-01-01T00:00:00");
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    assert!(!expired.exists(), "the first guard's upkeep");
    let later = home.trashed("later.txt", "2020-01-01T00:00:00");
    let mut again = home.guard(home.config("0%", "0%"));
    again.step(&tick(), &CALM).unwrap();
    assert!(later.exists(), "the same boot and guard owe no upkeep");
    let rebooted = Tick {
        boot: "another".into(),
        wall: wall() + 7200.0,
        ..tick()
    };
    let mut booted = home.guard(home.config("0%", "0%"));
    booted.step(&rebooted, &CALM).unwrap();
    assert!(!later.exists(), "a new boot makes upkeep due");
}

#[test]
fn upkeep_the_system_daemon_could_not_vouch_for_comes_again_soon() {
    // A switch restarts both guards, and on 28 October the user guard's
    // upkeep began in the seven seconds the system's was down: it kept
    // 3,036 temp entries for a process of root's it couldn't read, and
    // would have left them for six hours.
    let home = Home::new();
    let built = held_tree(&home, 400);
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    home.hidden(300, 77);
    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let places = Places {
        system: Some(socket.clone()),
        ..home.places()
    };
    let mut guard = Guard::new(crate::unpaced(owned_trees(&home)), &places).unwrap();
    guard.step(&tick(), &CALM).unwrap();
    assert!(built.exists(), "kept while nothing vouches for process 300");
    let at = |seconds: f64| Tick {
        wall: wall() + seconds,
        monotonic: 3600.0 + seconds,
        ..tick()
    };
    let _heard = fake_system(&socket, false);
    guard.step(&at(60.0), &CALM).unwrap();
    assert!(
        built.exists(),
        "an unheard system daemon costs a pass now and then, not every minute"
    );
    guard.step(&at(660.0), &CALM).unwrap();
    assert!(
        !built.exists(),
        "upkeep comes again once the system daemon can vouch"
    );
}

#[test]
fn a_tree_kept_for_its_use_is_explained_as_kept_and_why() {
    let home = Home::new();
    let (config, deps) = home.replaced_build("0%");
    let target = deps.parent().unwrap().parent().unwrap().to_owned();
    // cargo, building in the checkout.
    let proc = home.root.join("proc");
    let me = unsafe { libc::geteuid() };
    crate::procs::text(&proc, "400/stat", &crate::procs::stat(400, "cargo", 0, 90));
    crate::procs::text(
        &proc,
        "400/status",
        &format!("Name:\tcargo\nUid:\t{me}\t{me}\t{me}\t{me}\n"),
    );
    crate::procs::text(&proc, "400/cmdline", "cargo\0build\0");
    crate::procs::text(&proc, "400/maps", "");
    crate::procs::link(&proc, "400/cwd", target.join("debug").to_str().unwrap());
    crate::procs::link(&proc, "400/exe", "/bin/cargo");
    fs::create_dir_all(proc.join("400/fd")).unwrap();
    let mut guard = home.guard(config);
    let inside = deps.join(format!("libapp-{OLD}.rlib"));
    let explained = guard.explain(&tick(), &inside).unwrap();
    assert_eq!(
        explained,
        Explained::Kept {
            path: target,
            used: disk_guard::activity::Use::Busy(400)
        }
    );
}

#[test]
fn a_predicates_answer_is_kept_for_the_next_guard() {
    use crate::finished::git;
    use std::os::unix::fs::PermissionsExt;
    let home = Home::new();
    let (_, deps) = home.replaced_build("0%");
    let app = home.root.join("src/app");
    git(&app, &["init", "-q", "-b", "work"]);
    git(
        &app,
        &["remote", "add", "origin", "https://example.com/app.git"],
    );
    git(&app, &["add", "Cargo.toml"]);
    git(&app, &["commit", "-q", "-m", "one"]);
    let asked = home.root.join("asked");
    let predicate = home.root.join("merged");
    fs::write(
        &predicate,
        format!(
            "#!/bin/sh\necho asked >> {}\necho '{{\"finished\": true}}'\n",
            asked.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&predicate, fs::Permissions::from_mode(0o755)).unwrap();
    let config = || {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "notify": false,
                 "bands": {{ "pressure": "0%", "urgent": "0%", "critical": "0%", "target": "0%" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{
                   "src": {{ "kind": "cargo", "roots": ["{0}/src"] }},
                   "merged": {{ "kind": "finished", "remote": "^https://example\\.com/",
                                "command": ["{1}"], "timeout": "5s" }} }} }}"#,
            home.root.display(),
            predicate.display()
        ))
        .unwrap()
    };
    let _ = deps;
    let mut first = home.guard(config());
    first.step(&tick(), &CALM).unwrap();
    let times = || {
        fs::read_to_string(&asked)
            .unwrap_or_default()
            .lines()
            .count()
    };
    assert_eq!(times(), 1, "routine upkeep asks once");
    drop(first);
    let mut second = home.guard(config());
    second.preview(&tick(), Some(Band::Pressure), None).unwrap();
    assert_eq!(times(), 1, "a finished head is known still");
}

#[test]
fn a_preview_leaves_a_damaged_memory_as_it_found_it() {
    let home = Home::new();
    fs::write(home.root.join("state/memory.json"), "{ \"learned\": ").unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.preview(&tick(), Some(Band::Pressure), None).unwrap();
    let kept = fs::read_to_string(home.root.join("state/memory.json")).unwrap();
    assert_eq!(kept, "{ \"learned\": ");
    assert!(!home.root.join("state/memory.json.damaged").exists());
}

#[test]
fn a_damaged_memory_is_set_aside_and_begun_afresh() {
    let home = Home::new();
    fs::write(home.root.join("state/memory.json"), "{ \"learned\": ").unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    let kept = fs::read_to_string(home.root.join("state/memory.json.damaged")).unwrap();
    assert_eq!(kept, "{ \"learned\": ");
    let fresh: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(home.root.join("state/memory.json")).unwrap())
            .unwrap();
    assert_eq!(fresh["version"], 1);
}

#[test]
fn upkeep_waits_for_a_machine_that_can_spare_it() {
    let home = Home::new();
    let expired = home.trashed("old.txt", "2020-01-01T00:00:00");
    let mut guard = home.guard(home.config("0%", "0%"));
    let saving = Senses {
        saver: true,
        ..CALM
    };
    guard.step(&tick(), &saving).unwrap();
    assert!(expired.exists());
    guard.step(&tick(), &CALM).unwrap();
    assert!(!expired.exists());
}

/// A service manager that freezes nothing, saying what it was asked.
#[derive(Clone, Default)]
struct Manager {
    calls: std::sync::Arc<std::sync::Mutex<Vec<String>>>,
    /// Whether what it froze stays frozen, until somebody thaws it.
    thawed: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Manager {
    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl disk_guard::freezer::Freeze for Manager {
    fn freeze(&mut self, unit: &str, pids: &[u32], _: &str) -> Result<(), String> {
        self.calls
            .lock()
            .unwrap()
            .push(format!("freeze {unit} {pids:?}"));
        Ok(())
    }

    fn thaw(&mut self, unit: &str) -> Result<(), String> {
        self.calls.lock().unwrap().push(format!("thaw {unit}"));
        Ok(())
    }

    fn frozen(&mut self, _: &str) -> Result<bool, String> {
        Ok(!self.thawed.load(std::sync::atomic::Ordering::Relaxed))
    }
}

impl Home {
    /// A config whose disk is critical, or healthy, with an empty scratch
    /// source on it, so a pass finds nothing to take.
    fn critical(&self, critical: bool) -> Config {
        let share = if critical { "100%" } else { "0%" };
        fs::create_dir_all(self.root.join("scratch")).unwrap();
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true, "notify": false,
                 "bands": {{ "pressure": "{share}", "urgent": "{share}", "critical": "{share}", "target": "{share}" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{ "scratch": {{ "kind": "scratch", "path": "{0}/scratch" }} }} }}"#,
            self.root.display()
        ))
        .unwrap()
    }

    /// A process of the account's in `cgroup`, with a child, writing a
    /// file on the guarded disk, having written `written` bytes so far.
    fn writer(&self, pid: u32, cgroup: &str, written: u64) {
        let proc = self.root.join("proc");
        let me = unsafe { libc::geteuid() };
        for (pid, parent, name) in [(pid, 1, "dd"), (pid + 1, pid, "dd-helper")] {
            let at = |file: &str| format!("{pid}/{file}");
            let mut fields = vec!["0".to_string(); 49];
            fields[0] = "S".into();
            fields[1] = parent.to_string();
            fields[19] = "500".into();
            crate::procs::text(
                &proc,
                &at("stat"),
                &format!("{pid} ({name}) {}\n", fields.join(" ")),
            );
            crate::procs::text(
                &proc,
                &at("status"),
                &format!("Name:\t{name}\nUid:\t{me}\t{me}\t{me}\t{me}\n"),
            );
            crate::procs::text(&proc, &at("cmdline"), name);
            crate::procs::text(&proc, &at("cgroup"), &format!("0::{cgroup}\n"));
            // The child writes nothing itself.
            let mine = if parent == 1 { written } else { 0 };
            crate::procs::text(&proc, &at("io"), &format!("write_bytes: {mine}\n"));
        }
        let out = self.root.join("filling");
        fs::write(&out, "x").unwrap();
        let fd = proc.join(format!("{pid}/fd/3"));
        if fd.symlink_metadata().is_err() {
            crate::procs::link(&proc, &format!("{pid}/fd/3"), out.to_str().unwrap());
        }
    }

    fn memory(&self) -> serde_json::Value {
        serde_json::from_str(&fs::read_to_string(self.root.join("state/memory.json")).unwrap())
            .unwrap()
    }
}

fn uid() -> u32 {
    unsafe { libc::geteuid() }
}

fn app(uid: u32) -> String {
    format!("/user.slice/user-{uid}.slice/user@{uid}.service/app.slice/app-kitty.scope")
}

const GIB: u64 = 1 << 30;

/// Steps `guard` as a runaway writer fills its disk: a step with nothing
/// to take, then one 30 seconds later after 2 GiB more.
fn running_away(home: &Home, guard: &mut Guard, cgroup: &str) {
    home.writer(700, cgroup, 0);
    guard.step(&tick(), &CALM).unwrap();
    home.writer(700, cgroup, 2 * GIB);
    let later = Tick {
        wall: wall() + 30.0,
        monotonic: 3630.0,
        ..tick()
    };
    guard.step(&later, &CALM).unwrap();
}

#[test]
fn at_critical_with_nothing_left_the_runaway_writer_is_frozen_with_its_children() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    running_away(&home, &mut guard, &app(uid()));
    assert_eq!(
        manager.calls(),
        ["freeze disk-guard-freeze-700.scope [700, 701]"]
    );
    let frozen = &home.status()["frozen"];
    assert_eq!(frozen[0]["name"], "dd", "{frozen}");
    assert_eq!(frozen[0]["unit"], "disk-guard-freeze-700.scope");
    assert!(
        home.journal().iter().any(|line| line["kind"] == "freeze"),
        "{:?}",
        home.journal()
    );
}

#[test]
fn a_writer_with_nothing_open_on_the_disk_or_writing_slowly_is_left_alone() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    home.writer(700, &app(uid()), 0);
    // Its only file is on another filesystem.
    fs::remove_file(home.root.join("proc/700/fd/3")).unwrap();
    crate::procs::link(&home.root.join("proc"), "700/fd/3", "/dev/null");
    running_away(&home, &mut guard, &app(uid()));
    assert!(
        manager.calls().is_empty(),
        "another disk's: {:?}",
        manager.calls()
    );

    let home = Home::new();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    home.writer(700, &app(uid()), 0);
    guard.step(&tick(), &CALM).unwrap();
    // 300 KiB in 30 seconds is nothing that fills a disk.
    home.writer(700, &app(uid()), 300 << 10);
    let later = Tick {
        wall: wall() + 30.0,
        monotonic: 3630.0,
        ..tick()
    };
    guard.step(&later, &CALM).unwrap();
    assert!(
        manager.calls().is_empty(),
        "a slow one: {:?}",
        manager.calls()
    );
}

#[test]
fn the_status_names_who_writes_most_while_a_disk_is_short_freezing_or_not() {
    let home = Home::new();
    let mut guard = home.guard(home.critical(true));
    running_away(&home, &mut guard, &app(uid()));
    let writer = &home.status()["writer"];
    assert_eq!(writer["name"], "dd", "{writer}");
    assert_eq!(writer["pid"], 700);
    let mut calm = home.guard(home.critical(false));
    calm.step(&tick(), &CALM).unwrap();
    assert_eq!(home.status()["writer"], serde_json::Value::Null);
}

#[test]
fn nothing_in_a_session_is_ever_frozen() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    let session = format!(
        "/user.slice/user-{0}.slice/user@{0}.service/session.slice/foot.service",
        uid()
    );
    running_away(&home, &mut guard, &session);
    assert!(manager.calls().is_empty(), "{:?}", manager.calls());
}

#[test]
fn a_frozen_writer_is_thawed_once_the_disk_has_room() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    running_away(&home, &mut guard, &app(uid()));
    drop(guard);
    // The next guard, the disk now healthy, remembers what it froze.
    let mut guard = home.guard(home.critical(false));
    guard.freeze_with(Box::new(manager.clone()));
    guard.step(&tick(), &CALM).unwrap();
    assert_eq!(
        manager.calls().last().unwrap(),
        "thaw disk-guard-freeze-700.scope"
    );
    assert_eq!(home.memory()["frozen"], serde_json::json!([]));
}

#[test]
fn a_writer_thawed_by_hand_is_never_frozen_again() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critical(true));
    guard.freeze_with(Box::new(manager.clone()));
    running_away(&home, &mut guard, &app(uid()));
    manager
        .thawed
        .store(true, std::sync::atomic::Ordering::Relaxed);
    for (i, written) in [(1.0, 4 * GIB), (2.0, 6 * GIB), (3.0, 8 * GIB)] {
        home.writer(700, &app(uid()), written);
        let later = Tick {
            wall: wall() + 30.0 + 30.0 * i,
            monotonic: 3630.0 + 30.0 * i,
            ..tick()
        };
        guard.step(&later, &CALM).unwrap();
    }
    assert_eq!(
        manager.calls(),
        ["freeze disk-guard-freeze-700.scope [700, 701]"]
    );
    assert_eq!(home.status()["frozen"], serde_json::json!([]));
    // Once it's gone, there's nobody left to spare.
    fs::remove_dir_all(home.root.join("proc/700")).unwrap();
    let gone = Tick {
        wall: wall() + 1_000.0,
        monotonic: 4_600.0,
        ..tick()
    };
    guard.step(&gone, &CALM).unwrap();
    assert_eq!(home.memory()["spared"], serde_json::json!([]));
}

const OLD: &str = "1111111111111111";
const NEW: &str = "2222222222222222";

impl Home {
    /// A cargo source over `src`, whose app's target holds an older
    /// build of it, last written five days ago, beside the newest.
    fn replaced_build(&self, pressure: &str) -> (Config, PathBuf) {
        let app = self.root.join("src/app");
        fs::create_dir_all(app.join("src")).unwrap();
        fs::write(app.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        let deps = app.join("target/debug/deps");
        fs::create_dir_all(&deps).unwrap();
        fs::write(
            app.join("target/CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55\n# created by cargo\n",
        )
        .unwrap();
        for digest in [OLD, NEW] {
            fs::write(deps.join(format!("libapp-{digest}.rlib")), "x".repeat(4096)).unwrap();
        }
        touch(
            &deps.join(format!("libapp-{OLD}.rlib")),
            crate::whole_seconds(wall()) - 5 * 86_400,
        );
        let config = Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true, "notify": false,
                 "bands": {{ "pressure": "{pressure}", "urgent": "0%", "critical": "0%", "target": "{pressure}" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{ "src": {{ "kind": "cargo", "roots": ["{0}/src"] }} }} }}"#,
            self.root.display()
        ))
        .unwrap();
        (config, deps)
    }
}

#[test]
fn a_replaced_build_unopened_for_days_goes_at_pressure_and_the_newest_stays() {
    let home = Home::new();
    let (config, deps) = home.replaced_build("100%");
    // Watched for three days, and nothing opened it.
    let mut uses = Uses::default();
    uses.watching(&deps, wall() - 3.0 * 86_400.0);
    disk_guard::state::save(&home.root.join("state/uses.json"), &uses).unwrap();
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    assert!(!deps.join(format!("libapp-{OLD}.rlib")).exists());
    assert!(deps.join(format!("libapp-{NEW}.rlib")).exists());
    let taken = deps.join(format!("app-{OLD}"));
    assert!(
        home.journal()
            .iter()
            .any(|line| line["path"] == taken.to_string_lossy().as_ref()),
        "{:?}",
        home.journal()
    );
}

#[test]
fn the_opens_a_watch_hears_are_kept_for_the_next_guard() {
    let Ok(watch) = disk_guard::opens::Watch::new() else {
        eprintln!("no fanotify here, so nothing to check");
        return;
    };
    let home = Home::new();
    let (config, deps) = home.replaced_build("0%");
    let mut guard = home.guard(config);
    guard.listen(std::sync::Arc::new(watch));
    guard.step(&tick(), &CALM).unwrap();
    let rlib = deps.join(format!("libapp-{OLD}.rlib"));
    std::process::Command::new("cat")
        .arg(&rlib)
        .output()
        .unwrap();
    // Each step late enough after the last that the guard saves.
    let mut steps = 0.0;
    let mut later = || {
        steps += 1.0;
        Tick {
            wall: wall() + 400.0 * steps,
            ..tick()
        }
    };
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let kept = loop {
        guard.step(&later(), &CALM).unwrap();
        let kept: Uses = disk_guard::state::load(&home.root.join("state/uses.json"))
            .unwrap()
            .unwrap_or_default();
        if kept.opened.contains_key(&rlib) || std::time::Instant::now() > deadline {
            break kept;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(kept.watched.contains_key(&deps), "{kept:?}");
    assert!(kept.opened.contains_key(&rlib), "{kept:?}");
}

#[test]
fn a_deps_made_anew_is_marked_again_within_the_hour() {
    let Ok(watch) = disk_guard::opens::Watch::new() else {
        eprintln!("no fanotify here, so nothing to check");
        return;
    };
    let home = Home::new();
    let (config, deps) = home.replaced_build("0%");
    let mut guard = home.guard(config);
    guard.listen(std::sync::Arc::new(watch));
    guard.step(&tick(), &CALM).unwrap();
    // Cleaned and built again: the old mark went with the old directory.
    fs::remove_dir_all(&deps).unwrap();
    home.replaced_build("0%");
    let at = |seconds: f64| Tick {
        wall: wall() + seconds,
        ..tick()
    };
    guard.step(&at(3601.0), &CALM).unwrap();
    let rlib = deps.join(format!("libapp-{NEW}.rlib"));
    std::process::Command::new("cat")
        .arg(&rlib)
        .output()
        .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut seconds = 3601.0;
    let kept = loop {
        // Each step late enough after the last that the guard saves.
        seconds += 400.0;
        guard.step(&at(seconds), &CALM).unwrap();
        let kept: Uses = disk_guard::state::load(&home.root.join("state/uses.json"))
            .unwrap()
            .unwrap_or_default();
        if kept.opened.contains_key(&rlib) || std::time::Instant::now() > deadline {
            break kept;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(kept.opened.contains_key(&rlib), "{kept:?}");
}

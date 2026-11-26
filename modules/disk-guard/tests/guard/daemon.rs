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
            full_minute: 0.0,
        },
        memory: disk_guard::moments::Stall {
            some: 0.0,
            full: 0.0,
            full_minute: 0.0,
        },
        cpu: disk_guard::moments::Stall {
            some: 0.0,
            full: 0.0,
            full_minute: 0.0,
        },
        memory_room: None,
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
    /// has any disk filling and `0%` keeps it healthy. Its Nix store
    /// doesn't exist: with no system daemon answering, a user daemon
    /// collects garbage itself once nearly full, and the machine's own store
    /// is no test's to collect.
    fn config(&self, filling: &str, target: &str) -> Config {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true,
                 "bands": {{ "filling": "{filling}", "nearly_full": "0%", "critically_full": "0%",
                            "target": "{target}" }},
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
    fake_system_ending(socket, collects, "reached")
}

/// A fake system daemon as [`fake_system`], whose every collect ends as
/// `ended` says, in the journal's words.
pub(crate) fn fake_system_ending(
    socket: &Path,
    collects: bool,
    ended: &'static str,
) -> std::sync::mpsc::Receiver<Request> {
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
                        Request::Collect { .. } => Reply::Collected(ended.to_owned()),
                        Request::Footprint { paths } => Reply::Footprints(vec![None; paths.len()]),
                    };
                    let _ = telling.send(request);
                    responder.send(&reply);
                })
                .unwrap();
        }
    });
    heard
}

/// A system daemon at `socket` that hears every heartbeat and collects
/// nothing, but answers an ask about processes with something else, as
/// one too busy to scan them might fail it.
fn fake_system_vouching_nothing(socket: &Path) -> std::thread::JoinHandle<()> {
    let mut server = disk_guard::socket::Server::bind(socket, std::time::Duration::ZERO).unwrap();
    std::thread::spawn(move || {
        loop {
            server
                .serve(&mut |_, _, responder| responder.send(&Reply::Collects(Vec::new())))
                .unwrap();
        }
    })
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
        band: Band::Filling,
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

/// A config whose disk is filling, or nearly full where `nearly_full` is
/// `100%`, and which heeds the writers' hold `mark`.
fn held_by(home: &Home, mark: &Path, nearly_full: &str) -> Config {
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "status_file": true,
             "bands": {{ "filling": "100%", "nearly_full": "{nearly_full}",
                        "critically_full": "0%", "target": "100%" }},
             "writer_holds": ["{1}"],
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{ "scratch": {{ "kind": "scratch", "path": "{0}/scratch" }} }} }}"#,
        home.root.display(),
        mark.display()
    ))
    .unwrap()
}

#[test]
fn a_writers_hold_stops_a_pass_until_the_disk_is_nearly_full() {
    // A cell holding heavy disk writers, as while Joel is on a call, leaves
    // a mark; only its presence counts, never what it says.
    let home = Home::new();
    let mark = home.root.join("velocity-hold/disk-writers");
    fs::create_dir_all(mark.parent().unwrap()).unwrap();
    fs::write(&mark, "a call is on").unwrap();
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(held_by(&home, &mark, "0%"));
    guard.step(&tick_at(0.0), &CALM).unwrap();
    assert!(
        cold.exists(),
        "a filling disk passed under the writers' hold"
    );
    fs::remove_file(&mark).unwrap();
    guard.step(&tick_at(60.0), &CALM).unwrap();
    assert!(!cold.exists(), "no pass came once the writers' hold ended");

    fs::write(&mark, "").unwrap();
    let cold = home.cold_scratch("run-2");
    let mut nearly_full = home.guard(held_by(&home, &mark, "100%"));
    nearly_full.step(&tick_at(0.0), &CALM).unwrap();
    assert!(
        !cold.exists(),
        "a nearly full disk waited on the writers' hold"
    );
}

/// Asks a clear of cold scratch under a writers' hold, giving back the
/// guard and the scratch entry it should take.
fn clear_asked_under_a_hold(home: &Home) -> (Guard, PathBuf) {
    let mark = home.root.join("velocity-hold/disk-writers");
    fs::create_dir_all(mark.parent().unwrap()).unwrap();
    fs::write(&mark, "").unwrap();
    let scratch = home.cold_scratch("build");
    disk_guard::clears::ask(&home.root.join("state"), scratch.parent().unwrap(), 0.0).unwrap();
    (home.guard(held_by(home, &mark, "0%")), scratch)
}

/// Steps a clear asked under a writers' hold, giving back the guard and,
/// each time its survey looked, what the status said of the held disk's
/// waiting (left out when false) and of the pass at work.
fn clear_under_a_hold(home: &Home) -> (Guard, Vec<(serde_json::Value, serde_json::Value)>) {
    let (mut guard, scratch) = clear_asked_under_a_hold(home);
    // Asked once its survey has walked, before any pass judges.
    let status = home.root.join("status.json");
    let seen = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let seeing = std::rc::Rc::clone(&seen);
    guard.stop_when(Box::new(move || {
        if let Ok(text) = fs::read_to_string(&status) {
            let read: serde_json::Value = serde_json::from_str(&text).unwrap();
            seeing.borrow_mut().push((
                read["filesystems"][0]["waiting"].clone(),
                read["passing"]["why"].clone(),
            ));
        }
        false
    }));
    guard.step(&tick_at(0.0), &CALM).unwrap();
    assert!(
        !scratch.exists(),
        "the clear asked for didn't run under the hold"
    );
    guard.stop_when(Box::new(|| false));
    let seen = seen.borrow().clone();
    assert!(
        !seen.is_empty(),
        "nothing read the status while the clear surveyed"
    );
    (guard, seen)
}

#[test]
fn a_clear_asked_under_a_writers_hold_says_it_clears_from_before_its_survey() {
    // Its survey can walk for minutes, in which a status still saying the
    // disk waits tells Joel his clear never started.
    let home = Home::new();
    let (mut guard, seen) = clear_under_a_hold(&home);
    assert!(
        seen.iter()
            .all(|(waiting, why)| *waiting != true && why == "clear"),
        "a disk whose clear was surveying wasn't told at work clearing: {seen:?}"
    );
    // A clear of a folder with nothing in it to clear runs no pass, and
    // the disk is told at work no longer once it ends.
    let empty = home.root.join("scratch/nothing-here");
    fs::create_dir_all(&empty).unwrap();
    disk_guard::clears::ask(&home.root.join("state"), &empty, 60.0).unwrap();
    guard.step(&tick_at(60.0), &CALM).unwrap();
    assert!(
        home.status()["passing"].is_null(),
        "a clear with nothing to clear left its disk told at work: {}",
        home.status()["passing"]
    );
    assert_eq!(
        home.status()["filesystems"][0]["waiting"],
        true,
        "the held disk didn't say it waits once its clear ended"
    );
}

/// A subvolume has a device number of its own, which names no watched
/// disk: they're known by their pool.
#[test]
fn a_clear_in_a_subvolume_under_a_writers_hold_says_it_clears() {
    let home = Home::new();
    if !crate::pool::on_btrfs(&home.root) {
        eprintln!("skipped: {} isn't on Btrfs", home.root.display());
        return;
    }
    crate::pool::subvolume(&home.root, "scratch");
    let (_, seen) = clear_under_a_hold(&home);
    assert!(
        seen.iter()
            .all(|(waiting, why)| *waiting != true && why == "clear"),
        "a disk whose clear in a subvolume was surveying wasn't told at work clearing: {seen:?}"
    );
}

/// A clear that fails still ends, and the held disk says it waits again
/// rather than neither waiting nor clearing until its next status.
#[test]
fn a_clear_that_fails_under_a_writers_hold_leaves_its_disk_waiting() {
    use std::os::unix::fs::PermissionsExt;
    let home = Home::new();
    let (mut guard, scratch) = clear_asked_under_a_hold(&home);
    // Its ask can't be taken back, so the clear fails once it has run.
    let asks = home.root.join("state").join(disk_guard::clears::DIR);
    fs::set_permissions(&asks, fs::Permissions::from_mode(0o555)).unwrap();
    let stepped = guard.step(&tick_at(0.0), &CALM);
    fs::set_permissions(&asks, fs::Permissions::from_mode(0o755)).unwrap();
    assert!(stepped.is_err(), "a clear whose ask stayed didn't fail");
    assert!(!scratch.exists(), "the clear didn't run before it failed");
    assert!(
        home.status()["passing"].is_null(),
        "a failed clear left its disk told at work: {}",
        home.status()["passing"]
    );
    assert_eq!(
        home.status()["filesystems"][0]["waiting"],
        true,
        "the held disk didn't say it waits once its clear failed"
    );
}

#[test]
fn a_filling_disk_under_a_writers_hold_says_its_clearing_waits() {
    let home = Home::new();
    let mark = home.root.join("velocity-hold/disk-writers");
    fs::create_dir_all(mark.parent().unwrap()).unwrap();
    fs::write(&mark, "").unwrap();
    home.cold_scratch("run-1");
    let mut guard = home.guard(held_by(&home, &mark, "0%"));
    guard.step(&tick_at(0.0), &CALM).unwrap();
    let says = home.status()["says"].as_str().unwrap().to_owned();
    assert!(
        says.contains("waits") && !says.contains("is clearing"),
        "a held guard said it was clearing: {says}"
    );
    assert_eq!(
        home.status()["filesystems"][0]["says"].as_str(),
        Some(says.as_str()),
        "the disk's own sentence said otherwise"
    );
    assert_eq!(
        home.status()["filesystems"][0]["waiting"],
        true,
        "the status's flag didn't say the disk waits"
    );

    fs::remove_file(&mark).unwrap();
    guard.step(&tick_at(60.0), &CALM).unwrap();
    let says = home.status()["says"].as_str().unwrap().to_owned();
    assert!(
        !says.contains("waits"),
        "the guard still said it waited once the hold ended: {says}"
    );
    assert!(
        home.status()["filesystems"][0].get("waiting").is_none(),
        "a status with no hold carried the flag: {}",
        home.status()["filesystems"][0]
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
            r#"{{"event":"{event}","at":1.0,"band":"filling","path":"{}","root":"{}","kind":"scratch","size":5,"outcome":"reclaimed"}}"#,
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
    // Nearly full, so a pass that finds nothing rests a minute, not the
    // hour a filling disk would, and the same guard is asked again.
    config.bands.nearly_full = disk_guard::units::Size::Percent(100.0);
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
            from: Some(Band::Filling),
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
    // Past the rest that pass earned, and within the hour. Rests count
    // from when a pass really ended, so each step after one leaves ten
    // seconds for it to take on a loaded builder.
    guard.step(&tick_at(3670.0), &CALM).unwrap();
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
    // What it watches is said by what it holds, not when it was written:
    // a janitor that rewrote its links and touched its marker each time it
    // looked set off a walk of every source every ten minutes on 29
    // October.
    let home = Home::new();
    let marker = home.root.join("ran");
    let cache = home.root.join("cache");
    let latest = cache.join("tool/default/latest");
    fs::create_dir_all(latest.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink("sha/1", &latest).unwrap();
    let stamp = home.root.join("stamp");
    fs::write(&stamp, "").unwrap();
    touch(&stamp, 1_000_000_000);
    let point = |at: &str| {
        fs::remove_file(&latest).unwrap();
        std::os::unix::fs::symlink(at, &latest).unwrap();
    };
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
            watch: vec![stamp.clone(), cache.clone()],
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
    // Due at the next upkeep, six hours on, and not before: a producer's
    // news is its own, and walked every source each time on 29 October.
    let trashed = home.trashed("old.txt", "2001-09-09T01:46:40");
    guard.step(&at(61.0), &CALM).unwrap();
    assert_eq!(runs(), 1, "nothing it watches has changed");
    #[expect(clippy::cast_possible_truncation, reason = "a wall clock in seconds")]
    touch(&stamp, start as i64 + 100);
    point("sha/1");
    guard.step(&at(122.0), &CALM).unwrap();
    assert_eq!(runs(), 1, "written again, saying the same");
    point("sha/2");
    guard.step(&at(183.0), &CALM).unwrap();
    assert_eq!(runs(), 2, "what it watches says something new");
    guard.step(&at(244.0), &CALM).unwrap();
    assert_eq!(runs(), 2, "once for each change");
    fs::write(&stamp, "a version").unwrap();
    guard.step(&at(305.0), &CALM).unwrap();
    assert_eq!(runs(), 3, "a file saying something new");
    assert!(trashed.exists(), "only the producer ran, not all of upkeep");
    // Journaled as the producers' own pass, with its survey's seconds.
    let journal = home.journal();
    let pass = journal
        .iter()
        .rfind(|line| line["event"] == "pass")
        .unwrap();
    assert_eq!(pass["why"], "producers", "{journal:?}");
    assert!(pass["surveyed"].as_f64() > Some(0.0), "{journal:?}");
    if cfg!(target_os = "linux") {
        assert!(
            pass["read"].is_u64() && pass["survey_read"].is_u64(),
            "a pass didn't say what it and its survey read from disk: {pass}"
        );
    }
}

/// The walks outside passes, each journaled with what it read from disk.
fn walks(home: &Home, step: &str) -> Vec<serde_json::Value> {
    home.journal()
        .into_iter()
        .filter(|line| line["event"] == "walk" && line["step"] == step)
        .collect()
}

#[test]
fn a_measure_of_what_holds_the_disks_journals_what_it_read() {
    // On 25 November a guard read 9.2 GB in the eleven minutes after its
    // start, and nothing said whether that was its survey or this.
    let home = Home::new();
    home.cold_scratch("build");
    let mut guard = home.guard(home.config("100%", "100%"));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    let mut at = 0.0;
    let mut walls = Vec::new();
    while walks(&home, "holders").is_empty() && std::time::Instant::now() < deadline {
        let tick = tick_at(at);
        walls.push(tick.wall);
        guard.step(&tick, &CALM).unwrap();
        at += 5.0;
        // Its thread is joined at the next step that finds it done.
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let walked = walks(&home, "holders");
    assert_eq!(
        walked.len(),
        1,
        "the measure wasn't journaled once: {walked:?}"
    );
    assert!(walked[0]["seconds"].as_f64() >= Some(0.0), "{walked:?}");
    // Stamped when it began, so it sorts beside what it ran with, not
    // at the later step that found it done.
    assert!(walls.len() > 1, "no later step joined the measure");
    assert_eq!(
        walked[0]["at"].as_f64(),
        walls.first().copied(),
        "the measure wasn't stamped when it began: {walked:?}"
    );
    if cfg!(target_os = "linux") {
        assert!(
            walked[0]["read"].is_u64(),
            "it didn't say what it read: {walked:?}"
        );
    }
}

/// Senses a disk and memory stalling half the time, as a machine building
/// all day does.
fn busy() -> Senses {
    let stalling = disk_guard::moments::Stall {
        some: 50.0,
        full: 5.0,
        full_minute: 5.0,
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
/// whose filling and nearly full bands start at `filling` and `nearly_full`.
fn producer_policy(filling: &str, nearly_full: f64) -> String {
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let marker = home.root.join("policy");
    let mut config = home.config(filling, "100%");
    config.bands.nearly_full = Size::Percent(nearly_full);
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
            from: Some(Band::NearlyFull),
            watch: Vec::new(),
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    home.guard(config).step(&tick(), &CALM).unwrap();
    fs::read_to_string(&marker).unwrap().trim().to_owned()
}

#[test]
fn a_command_runs_at_idle_priority_unless_the_disk_is_nearly_full() {
    assert_eq!(
        producer_policy("0%", 0.0),
        libc::SCHED_IDLE.to_string(),
        "upkeep"
    );
    assert_eq!(
        producer_policy("100%", 100.0),
        crate::commands::guards_policy().to_string(),
        "nearly full"
    );
}

#[test]
fn a_pass_on_a_nearly_full_disk_never_gives_way() {
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
    config.bands.nearly_full = Size::Percent(100.0);
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    assert!(!old.exists());
    let pass = home
        .journal()
        .into_iter()
        .find(|line| line["event"] == "pass")
        .unwrap();
    assert_eq!(pass["band"], "nearly-full");
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
    // Nearly full, since a filling disk left with nothing rests an hour at
    // once; no writers' hold is ever marked here.
    let mut guard = home.guard(held_by(&home, &home.root.join("never-held"), "100%"));
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3605.0, 3655.0, 3670.0, 3700.0, 3785.0, 3800.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(seen, [1, 1, 1, 2, 2, 2, 3], "a minute's rest, then two");
}

#[test]
fn a_filling_disk_whose_passes_twice_running_leave_nothing_to_clear_rests_an_hour() {
    // On 25 November a reach pass freed 0 B and found nothing left, and
    // the next came 17 minutes on and read 15 GB of the disk's metadata
    // in 24 minutes, while evaluations and pushes needed it. The first
    // such pass still rests only a minute: the machine's guard expiring
    // generations leaves garbage only its next pass can take.
    let home = Home::new();
    fs::create_dir_all(home.root.join("scratch")).unwrap();
    let mut guard = home.guard(home.config("100%", "100%"));
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3670.0, 4600.0, 7260.0, 7280.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
        assert_eq!(
            home.status()["filesystems"][0]["exhausted"],
            true,
            "a resting disk didn't say it has cleared all it may at {at} s"
        );
    }
    assert_eq!(
        seen,
        [1, 2, 2, 2, 3],
        "a filling disk rested other than a minute, then an hour once left with nothing twice"
    );
}

#[test]
fn passes_kept_short_by_an_unheard_system_daemon_never_rest_a_filling_disk_an_hour() {
    // What they kept wanted only the system daemon's word, which a restart
    // of it gives back in a minute or two, so they found nothing left only
    // in that they couldn't ask.
    let home = Home::new();
    home.cold_scratch("run-1");
    home.hidden(300, 77);
    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let places = Places {
        system: Some(socket.clone()),
        ..home.places()
    };
    let mut guard = Guard::new(crate::unpaced(home.config("100%", "100%")), &places).unwrap();
    let _heard = fake_system_vouching_nothing(&socket);
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3670.0, 3800.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(
        seen,
        [1, 2, 3],
        "two unheard passes rested the disk longer than two minutes"
    );
}

#[test]
fn passes_that_find_nothing_far_apart_are_not_two_in_a_row() {
    // A hold of hours between them lets builds leave garbage the second
    // is the first to see, as after expiring generations.
    let home = Home::new();
    fs::create_dir_all(home.root.join("scratch")).unwrap();
    let mut guard = home.guard(home.config("100%", "100%"));
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 5000.0, 5130.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(
        seen,
        [1, 2, 3],
        "passes 23 minutes apart counted as two in a row"
    );
}

#[test]
fn passes_that_find_nothing_are_in_a_row_only_within_a_filling_disk_s_longest_rest() {
    // The first pass rests a minute, to 3660 s. The second comes 890 s
    // or 1200 s after that rest ended, either side of the 15 minutes a
    // filling disk rests longest. In a row, it rests the hour, so a step
    // past the two minutes it would otherwise rest runs no third pass.
    // The rest counts from the pass's real end, which a loaded builder
    // delays, so the far side keeps five minutes in hand.
    for (gap, third) in [(890.0, 2), (1200.0, 3)] {
        let home = Home::new();
        fs::create_dir_all(home.root.join("scratch")).unwrap();
        let mut guard = home.guard(home.config("100%", "100%"));
        let passes = || {
            home.journal()
                .iter()
                .filter(|line| line["event"] == "pass")
                .count()
        };
        let second = 3660.0 + gap;
        let mut seen = Vec::new();
        for at in [3600.0, second, second + 150.0] {
            guard.step(&tick_at(at), &CALM).unwrap();
            seen.push(passes());
        }
        assert_eq!(
            seen,
            [1, 2, third],
            "a pass {gap} s after its rest ended counted other than as the window says"
        );
    }
}

#[test]
fn a_rest_counts_from_when_its_pass_ended() {
    // On 25 November a pass ran 24 minutes, and a rest counted from its
    // start had ended before it did, so the next came at once and never
    // counted as in a row.
    let home = Home::new();
    home.trashed("old.txt", "2001-09-09T01:46:40");
    home.trashed("older.txt", "2001-09-08T01:46:40");
    let mut config = home.config("100%", "100%");
    // Not through `guard`, which takes the rest away: a pass of four
    // seconds at least, resting between its two deletes.
    config.pacing = disk_guard::config::Pacing {
        files: 1,
        rest: disk_guard::units::Span(std::time::Duration::from_secs(4)),
    };
    let mut guard = Guard::new(config, &home.places()).unwrap();
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    // A rest counted from the step's start ended at 3660 s; from the
    // pass's end, at 3664 s at the earliest. The last step leaves the
    // pass ten minutes, for a commit's wait on a loaded builder, and
    // still comes before an hour's rest could have ended.
    for at in [3600.0, 3663.0, 4300.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert_eq!(
        seen,
        [1, 1, 2],
        "the minute's rest counted from before its pass's four seconds"
    );
}

#[test]
fn each_pass_journals_what_set_it_going_what_it_was_offered_and_its_survey() {
    // In the day to 03:00 on 21 November half the passes freed nothing,
    // and their lines couldn't say whether each reached, upkept or
    // collected, or whether it had anything to take. Reach and upkeep
    // are here; producers, clear, garbage and collect are with their own
    // cases.
    let passes = |home: &Home| -> Vec<serde_json::Value> {
        home.journal()
            .into_iter()
            .filter(|line| line["event"] == "pass")
            .collect()
    };
    let filling = Home::new();
    filling.cold_scratch("run-1");
    // Kept, since process 300 can't be judged, so the pass judges
    // something and frees nothing: the waste its line is there to show.
    filling.hidden(300, 77);
    let mut guard = filling.guard(filling.config("100%", "100%"));
    guard.step(&tick_at(3600.0), &CALM).unwrap();
    let reach = passes(&filling);
    assert_eq!(reach.len(), 1, "{reach:?}");
    assert_eq!(reach[0]["why"], "reach");
    assert_eq!(reach[0]["offered"], 1, "{reach:?}");
    assert_eq!(reach[0]["judged"], 1, "{reach:?}");
    assert_eq!(reach[0]["freed"], 0, "{reach:?}");
    assert!(reach[0]["surveyed"].as_f64() > Some(0.0), "{reach:?}");
    let healthy = Home::new();
    healthy.trashed("old.txt", "2020-01-01T00:00:00");
    let mut guard = healthy.guard(healthy.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    let upkeep = passes(&healthy);
    assert_eq!(upkeep.len(), 1, "{upkeep:?}");
    assert_eq!(upkeep[0]["why"], "upkeep");
    assert!(upkeep[0]["offered"].as_u64() >= Some(1), "{upkeep:?}");
    assert_eq!(upkeep[0]["judged"], 1, "the old trash was due: {upkeep:?}");
    assert!(upkeep[0]["surveyed"].as_f64() > Some(0.0), "{upkeep:?}");
    // Trash deleted after now is inside every window, so the routine is
    // offered it and has nothing due: not the waste of one that kept it.
    let young = Home::new();
    young.trashed("new.txt", "2099-01-01T00:00:00");
    let mut guard = young.guard(young.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    let undue = passes(&young);
    assert_eq!(undue.len(), 1, "{undue:?}");
    assert!(undue[0]["offered"].as_u64() >= Some(1), "{undue:?}");
    assert_eq!(undue[0]["judged"], 0, "nothing was due: {undue:?}");
    // A scratch folder with nothing in it is watched, and offers nothing.
    let empty = Home::new();
    fs::create_dir_all(empty.root.join("scratch")).unwrap();
    let mut guard = empty.guard(empty.config("100%", "100%"));
    guard.step(&tick_at(3600.0), &CALM).unwrap();
    let nothing = passes(&empty);
    assert_eq!(nothing.len(), 1, "{nothing:?}");
    assert_eq!(
        (&nothing[0]["offered"], &nothing[0]["judged"]),
        (&serde_json::json!(0), &serde_json::json!(0)),
        "a pass with nothing to take says so"
    );
}

#[test]
fn a_pass_kept_short_by_an_unheard_system_daemon_looks_again_once_it_answers() {
    // The first pass after the account's daemon starts may find the system
    // daemon not answering in time: on 8 November that left a critical
    // disk's cache and an idle build target a full minute's rest.
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    home.hidden(300, 77);
    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let places = Places {
        system: Some(socket.clone()),
        ..home.places()
    };
    let mut guard = Guard::new(crate::unpaced(home.config("100%", "100%")), &places).unwrap();
    guard.step(&tick_at(3600.0), &CALM).unwrap();
    assert!(cold.exists(), "kept while no one vouches for process 300");

    let _heard = fake_system(&socket, false);
    guard.step(&tick_at(3605.0), &CALM).unwrap();
    assert!(
        !cold.exists(),
        "looked at again once the system daemon answers, not a minute on"
    );
}

#[test]
fn a_pass_kept_short_twice_by_an_unheard_system_daemon_rests() {
    // A system daemon whose heartbeat answers but whose vouching keeps
    // failing would otherwise have every step survey again.
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    home.hidden(300, 77);
    let sockets = socket_dir();
    let socket = sockets.path().join("system.socket");
    let places = Places {
        system: Some(socket.clone()),
        ..home.places()
    };
    let mut guard = Guard::new(crate::unpaced(home.config("100%", "100%")), &places).unwrap();
    let _heard = fake_system_vouching_nothing(&socket);
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3605.0, 3610.0, 3615.0] {
        guard.step(&tick_at(at), &CALM).unwrap();
        seen.push(passes());
    }
    assert!(cold.exists());
    assert_eq!(seen, [1, 2, 2, 2], "looked at once more, then rested");
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
    assert_eq!(pass["band"], "filling");
}

/// A stop that comes while the survey walks ends the step there, and what
/// the survey found is taken by nobody, however full the disk.
#[test]
fn a_stop_during_the_survey_ends_the_step_taking_nothing() {
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    let mut guard = home.guard(home.config("100%", "100%"));
    guard.stop_when(Box::new(|| true));
    let stepped = guard.step(&tick(), &CALM);
    assert_eq!(stepped.unwrap_err().kind(), std::io::ErrorKind::Interrupted);
    assert!(old.exists());
    assert!(home.journal().iter().all(|line| line["event"] != "pass"));
}

#[test]
fn a_pass_that_falls_short_when_critically_full_rests_a_minute_at_most() {
    let home = Home::new();
    home.cold_scratch("run-1");
    home.hidden(300, 77);
    let mut config = home.config("100%", "100%");
    config.bands.nearly_full = Size::Percent(100.0);
    config.bands.critically_full = Size::Percent(100.0);
    let mut guard = home.guard(config);
    let passes = || {
        home.journal()
            .iter()
            .filter(|line| line["event"] == "pass")
            .count()
    };
    let mut seen = Vec::new();
    for at in [3600.0, 3670.0, 3790.0, 3860.0, 3930.0] {
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
    guard
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
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
    guard
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
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
    let (surveyed, previews) = guard
        .preview(&tick(), Some(Band::Filling), None, false)
        .unwrap();
    assert_eq!(surveyed, None, "no pass kept a survey, so it surveyed");
    assert_eq!(previews.len(), 1);
    let preview = &previews[0];
    assert_eq!(preview.band, Band::Filling);
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

/// One disk that has written its bytes and is 3% worn.
struct OneDisk(u64);

impl disk_guard::disks::Disks for OneDisk {
    fn read(&mut self, health: bool) -> Vec<disk_guard::disks::Reading> {
        vec![disk_guard::disks::Reading {
            name: "nvme0n1".into(),
            written: self.0,
            wear: health.then_some(disk_guard::disks::Wear {
                used: 3,
                written: 60 * self.0,
            }),
        }]
    }
}

#[test]
fn a_guard_watching_disks_says_their_writes_and_wear_and_one_not_says_none() {
    let home = Home::new();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    assert_eq!(home.status().get("disks"), None, "{}", home.status());

    guard.watch_disks(Box::new(OneDisk(1 << 30)));
    guard.step(&tick(), &CALM).unwrap();
    let disk = &home.status()["disks"][0];
    assert_eq!(disk["name"], "nvme0n1", "{disk}");
    assert_eq!(disk["written_this_boot"], 1u64 << 30);
    assert_eq!(disk["used_percent"], 3);
    assert_eq!(disk["written_in_life"], 60u64 << 30);
    assert!(disk["rated_wear_in"].as_f64().is_some(), "{disk}");
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

/// A guard only starting has no pass at work, so its first beats never
/// carry on telling the pass the guard before it left mid-way.
#[test]
fn a_restarted_guard_beats_without_the_pass_the_last_one_left() {
    let home = Home::new();
    fs::write(
        home.root.join("status.json"),
        r#"{ "version": 2, "heartbeat": 1, "says": "left by the last guard",
              "passing": { "since": 1, "path": "/", "band": "filling", "why": "reach" } }"#,
    )
    .unwrap();
    let guard = home.guard(home.config("0%", "0%"));
    guard.beating()();
    let status = home.status();
    assert_eq!(
        status["says"], "left by the last guard",
        "the rest of what the last guard said is kept"
    );
    assert!(
        status["passing"].is_null(),
        "beat on as at work: {}",
        status["passing"]
    );
}

/// A restarted guard's first beats, on the status the last one left,
/// never pass on a moment it told that has since passed, which the broker
/// would forward as still to come; one still to come is kept.
#[test]
fn a_restarted_guard_beats_without_the_last_one_s_passed_moments() {
    let home = Home::new();
    // A whole second, since serde_json, without its float_roundtrip
    // feature, can read a long fraction back a little off what was written.
    let ahead = (wall() + 3600.0).floor();
    fs::write(
        home.root.join("status.json"),
        format!(
            r#"{{ "version": 2, "heartbeat": 1, "says": "left by the last guard",
                  "filesystems": [{{ "path": "/", "full_at": 2, "floor_at": 3 }},
                                   {{ "path": "/b", "full_at": {ahead}, "floor_at": {ahead} }}] }}"#
        ),
    )
    .unwrap();
    let guard = home.guard(home.config("0%", "0%"));
    guard.beating()();
    let status = home.status();
    let filesystems = &status["filesystems"];
    for moment in ["full_at", "floor_at"] {
        assert!(
            filesystems[0][moment].is_null(),
            "beat on with a passed {moment}: {filesystems}"
        );
        assert_eq!(
            filesystems[1][moment].as_f64(),
            Some(ahead),
            "dropped a {moment} still to come: {filesystems}"
        );
    }
}

/// A pass says it's at work in the status for as long as it judges, so
/// the desktop and the notice it withholds read it, and says so no longer
/// once it's done.
#[test]
fn a_pass_is_told_at_work_while_it_runs_and_not_after() {
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let seen = home.root.join("seen.json");
    let mut config = home.config("100%", "100%");
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!(
                    "cp {} {}",
                    home.root.join("status.json").display(),
                    seen.display()
                ),
            ],
            path: home.root.clone(),
            every: None,
            from: Some(Band::Filling),
            watch: Vec::new(),
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    let mut guard = home.guard(config);
    guard.step(&tick_at(3600.0), &CALM).unwrap();
    let during: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&seen).expect("the producer never ran")).unwrap();
    assert_eq!(
        during["passing"]["why"], "reach",
        "not told at work mid-pass: {}",
        during["passing"]
    );
    assert!(
        home.status()["passing"].is_null(),
        "still told at work once the pass ended: {}",
        home.status()["passing"]
    );
    guard.beating()();
    assert!(
        home.status()["passing"].is_null(),
        "a beat told the ended pass again: {}",
        home.status()["passing"]
    );
}

#[test]
fn a_status_the_disk_cant_hold_stops_no_pass() {
    // A disk too full for the status file is the one most in need of its
    // pass; telling it at work is only a courtesy.
    use disk_guard::config::Source;
    use disk_guard::units::Span;
    let home = Home::new();
    let ran = home.root.join("ran");
    let mut config = home.config("100%", "100%");
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                format!("echo once >> {}", ran.display()),
            ],
            path: home.root.clone(),
            every: None,
            from: Some(Band::Filling),
            watch: Vec::new(),
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    let mut guard = home.guard(config);
    // A folder holding a file, which no rename can put the status over.
    let status = home.root.join("status.json");
    let _ = fs::remove_file(&status);
    fs::create_dir_all(status.join("held")).unwrap();
    // The step's last write fails as well, once its pass is done.
    let stepped = guard.step(&tick_at(3600.0), &CALM);
    assert!(stepped.is_err(), "the status was written into a folder");
    let runs = fs::read_to_string(&ran).unwrap_or_default().lines().count();
    assert_eq!(runs, 1, "an unwritable status stopped the pass");
    // A clear Joel asked for, standing as the disk filled, writes the
    // status before it clears, and stops neither the clear nor the step.
    let scratch = home.cold_scratch("build");
    disk_guard::clears::ask(&home.root.join("state"), scratch.parent().unwrap(), 3600.0).unwrap();
    let stepped = guard.step(&tick_at(7200.0), &CALM);
    assert!(stepped.is_err(), "the status was written into a folder");
    assert!(!scratch.exists(), "an unwritable status stopped the clear");
}

/// A restarted guard's first measure of home takes minutes, in which its
/// status named no holders: on 13 November a switch left the user guard's
/// list empty while the system guard's came back. It says what the ledger
/// last measured until its own measure ends.
#[test]
fn a_restarted_guard_names_the_holders_it_last_measured() {
    let home = Home::new();
    let scratch = home.cold_scratch("build").parent().unwrap().to_owned();
    fs::write(
        home.root.join("state").join(disk_guard::ledger::FILE),
        format!(
            r#"[{{ "at": 1000.0, "sizes": {{ "{}": 5368709120 }} }}]"#,
            scratch.display()
        ),
    )
    .unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    let status = home.status();
    assert_eq!(status["holders_at"], 1000.0, "when the ledger measured it");
    assert_eq!(
        status["holders"][0]["path"],
        scratch.display().to_string(),
        "a healthy disk measures nothing new, so the last measure stands"
    );
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

/// A disk's estimates are remade from its fill's trend at every step, and
/// written each time they woke the desktop 26 times a minute for figures
/// it showed no differently (2026-10-31). They're news only once they
/// move far, or come or go.
#[test]
fn a_status_whose_estimates_only_drift_is_no_news() {
    use disk_guard::daemon::{FilesystemStatus, Status, news};
    let now = 1_000_000.0;
    let disk = |forecast: Option<f64>, fill: f64, full_in: f64, says: &str| FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Healthy,
        free: 500,
        total: 1000,
        size: 1000,
        used: 500,
        forecast,
        fill_per_hour: Some(fill),
        full_at: Some(now + full_in),
        nix_floor: None,
        floor_at: None,
        says: says.to_string(),
        exhausted: false,
    };
    let status = |one: FilesystemStatus, heartbeat: f64| Status {
        version: 2,
        heartbeat,
        says: one.says.clone(),
        filesystems: vec![one],
        cleared: Vec::new(),
        last: None,
        passing: None,
        failed: None,
        frozen: Vec::new(),
        writer: None,
        notice: None,
        holders: Vec::new(),
        holders_at: None,
        disks: Vec::new(),
    };
    let said = "The disk has room.";
    let written = status(
        disk(Some(507_791.0), 205_352_626.0, 9_000_000.0, said),
        now - 5.0,
    );
    let drifted = status(disk(Some(508_357.0), 205_124_083.0, 9_000_817.0, said), now);
    assert!(!news(&written, &drifted, now), "half a percent is no news");
    let moved = status(disk(Some(400_000.0), 205_124_083.0, 9_000_817.0, said), now);
    assert!(
        news(&written, &moved, now),
        "a forecast a fifth sooner is news"
    );
    let gone = status(disk(None, 205_124_083.0, 9_000_817.0, said), now);
    assert!(news(&written, &gone, now), "a forecast that goes is news");
    let told = status(
        disk(Some(508_357.0), 205_124_083.0, 9_000_817.0, "Now full."),
        now,
    );
    assert!(news(&written, &told, now), "what's said is news");
    // Nix's store's disk above Nix's floor counts to the floor instead.
    let floored = |floor_in: f64| FilesystemStatus {
        full_at: None,
        nix_floor: Some(100),
        floor_at: Some(now + floor_in),
        ..disk(Some(507_791.0), 205_352_626.0, 0.0, said)
    };
    let written = status(floored(9_000_000.0), now - 5.0);
    assert!(
        !news(&written, &status(floored(9_000_817.0), now), now),
        "half a percent nearer the floor is no news"
    );
    assert!(
        news(&written, &status(floored(7_000_000.0), now), now),
        "the floor a fifth sooner is news"
    );
}

/// A disk's counts move at every step and are said at each beat, so they
/// alone aren't news; its health read, or a disk coming or going, is.
#[test]
fn a_disks_counts_alone_are_no_news_and_its_health_or_a_disk_coming_is() {
    use disk_guard::daemon::{Status, news};
    use disk_guard::disks::DiskStatus;
    use disk_guard::units::float;
    let now = 1_000_000.0;
    let disk = |name: &str, written: u64, used: Option<u8>| DiskStatus {
        name: name.into(),
        written_this_boot: written,
        written_per_hour: Some(float(written) / 10.0),
        used_percent: used,
        // What it wrote in its life is read with its health, an hour
        // apart, so it moves with its wear here.
        written_in_life: used.map(|used| u64::from(used) * 20_000),
        rated_wear_in: used.map(|_| float(written)),
    };
    let status = |disks: Vec<DiskStatus>, heartbeat: f64| Status {
        version: 2,
        heartbeat,
        says: String::new(),
        filesystems: Vec::new(),
        cleared: Vec::new(),
        last: None,
        passing: None,
        failed: None,
        frozen: Vec::new(),
        writer: None,
        notice: None,
        holders: Vec::new(),
        holders_at: None,
        disks,
    };
    let written = status(vec![disk("nvme0n1", 1000, Some(2))], now - 5.0);
    let counted = status(vec![disk("nvme0n1", 9000, Some(2))], now);
    assert!(!news(&written, &counted, now), "counts alone");
    let worn = status(vec![disk("nvme0n1", 9000, Some(3))], now);
    assert!(news(&written, &worn, now), "a percent more worn");
    let unread = status(vec![disk("nvme0n1", 9000, None)], now);
    assert!(news(&written, &unread, now), "its health gone unread");
    let more = status(
        vec![disk("nvme0n1", 9000, Some(2)), disk("sda", 10, None)],
        now,
    );
    assert!(news(&written, &more, now), "a disk come");
    let other = status(vec![disk("nvme1n1", 9000, Some(2))], now);
    assert!(news(&written, &other, now), "another disk");
}

#[test]
fn a_preview_reads_the_survey_the_last_pass_kept_rather_than_walking_again() {
    // On 9 November a plan walked every source itself and ran past 200
    // seconds on a full machine without a word.
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let at = tick().wall;
    let mut daemon = home.guard(home.config("100%", "100%"));
    daemon.step(&tick(), &CALM).unwrap();
    assert!(!cold.exists(), "the pass took it");
    drop(daemon);
    let mut planner = home.guard(home.config("100%", "100%"));
    let (surveyed, kept) = planner
        .preview(&tick(), Some(Band::Filling), None, false)
        .unwrap();
    assert!(
        surveyed.is_some_and(|when| (when - at).abs() < 60.0),
        "it answers from the pass's survey: {surveyed:?}"
    );
    assert_eq!(kept[0].taken[0].path, cold, "as that survey saw it");
    let (surveyed, fresh) = planner
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
    assert_eq!(surveyed, None, "asked fresh, it surveys");
    assert_eq!(fresh[0].taken, [], "and finds it gone");
}

#[test]
fn pretending_free_space_bands_the_preview() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    let mut guard = home.guard(home.config("50%", "60%"));
    let (_, short) = guard
        .preview(&tick(), None, Some(Size::Bytes(0)), true)
        .unwrap();
    assert_eq!(short[0].band, Band::Filling);
    assert_eq!(short[0].free, 0);
    assert_eq!(short[0].taken[0].path, cold);
    let total = short[0].total;
    let (_, roomy) = guard
        .preview(&tick(), None, Some(Size::Percent(100.0)), true)
        .unwrap();
    assert_eq!(roomy[0].free, total);
    let mut low = home.guard(home.config("1%", "2%"));
    let (_, banded) = low
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
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
        "a day idle goes once filling: {:?}",
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
    assert_eq!(pass["band"], "filling");
    assert_eq!(pass["end"], "exhausted");
    let status = home.status();
    assert_eq!(status["filesystems"][0]["band"], "filling");
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
        filling: Size::Percent(25.0),
        nearly_full: Size::Percent(10.0),
        critically_full: Size::Percent(4.0),
        target: Size::Percent(35.0),
    };
    let space = |free| Space {
        total: 1000,
        free,
        btrfs: None,
    };
    let mut status = FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Filling,
        free: 200,
        total: 1000,
        size: 1000,
        used: 800,
        forecast: Some(60.0),
        fill_per_hour: Some(36_000.0),
        full_at: Some(1960.0),
        nix_floor: None,
        floor_at: None,
        says: String::new(),
        exhausted: false,
    };
    status.measured(&space(210), &bands, 1900.0, None);
    assert_eq!(status.band, Band::Filling);
    assert_eq!(status.free, 210);
    assert_eq!(status.used, 790);
    assert_eq!(
        status.says,
        "The disk is filling, with 210 B free. \
         The guard is clearing builds and caches nobody has used lately. \
         At this rate it fills in 1 m.",
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
    status.band = Band::Filling;
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
    assert_eq!(status.band, Band::NearlyFull);
    assert!(!status.exhausted, "a worse band's pass hasn't run yet");
    assert!(
        status.says.contains("is clearing everything"),
        "{}",
        status.says
    );
    // Under a writers' hold a beat's measure works out whether the disk
    // waits from its band, whichever way the band moved.
    status.held = true;
    status.measured(&space(50), &bands, 1900.0, None);
    assert!(!status.waiting, "a nearly full disk waited on a hold");
    assert!(
        status.says.contains("is clearing everything"),
        "{}",
        status.says
    );
    status.measured(&space(200), &bands, 1900.0, None);
    assert_eq!(status.band, Band::Filling);
    assert!(status.waiting, "a held disk back to filling didn't wait");
    assert!(status.says.contains("waits to clear"), "{}", status.says);
}

/// Every reader of the status file takes `full_at` as a moment still to
/// come: the bar's Disks piece turned amber and said "full in 0 s" beside
/// 104 GiB free once a long pass outlived its forecast.
#[test]
fn a_fresh_measure_drops_a_forecast_whose_moment_has_passed() {
    use disk_guard::band::Space;
    use disk_guard::config::Bands;
    use disk_guard::daemon::FilesystemStatus;
    let bands = Bands {
        filling: Size::Percent(25.0),
        nearly_full: Size::Percent(10.0),
        critically_full: Size::Percent(4.0),
        target: Size::Percent(35.0),
    };
    let mut status = FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Filling,
        free: 200,
        total: 1000,
        size: 1000,
        used: 800,
        forecast: Some(60.0),
        fill_per_hour: Some(36_000.0),
        full_at: Some(1960.0),
        nix_floor: None,
        floor_at: None,
        says: String::new(),
        exhausted: false,
    };
    let space = Space {
        total: 1000,
        free: 200,
        btrfs: None,
    };
    status.measured(&space, &bands, 1960.0, None);
    assert_eq!(
        status.full_at, None,
        "a disk with room left at its forecast's moment hasn't filled"
    );
    assert!(
        !status.says.contains("fills in"),
        "said no fill once its moment passed: {}",
        status.says
    );
    // A time to Nix's floor goes the same way, since the broker passes it
    // on as a moment still to come. Its sentence was never said for a
    // moment already come, so only the moment is held here.
    status.nix_floor = Some(100);
    status.floor_at = Some(1960.0);
    status.measured(&space, &bands, 1960.0, None);
    assert_eq!(
        status.floor_at, None,
        "a disk above Nix's floor at its forecast's moment hasn't reached it"
    );
}

/// Above the free space Nix keeps, Nix collects before the disk fills, so
/// a fresh measure says when it reaches that floor; one at or under it
/// has reached it, and when it's full wasn't worked out.
#[test]
fn a_fresh_measure_above_nixs_floor_says_when_it_reaches_it() {
    use disk_guard::band::Space;
    use disk_guard::config::Bands;
    use disk_guard::daemon::FilesystemStatus;
    let bands = Bands {
        filling: Size::Percent(25.0),
        nearly_full: Size::Percent(10.0),
        critically_full: Size::Percent(4.0),
        target: Size::Percent(35.0),
    };
    let space = |free| Space {
        total: 1000,
        free,
        btrfs: None,
    };
    let mut status = FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Healthy,
        free: 500,
        total: 1000,
        size: 1000,
        used: 500,
        forecast: None,
        fill_per_hour: Some(36_000.0),
        full_at: None,
        nix_floor: Some(100),
        floor_at: Some(1960.0),
        says: String::new(),
        exhausted: false,
    };
    status.measured(&space(490), &bands, 1900.0, None);
    assert_eq!(
        status.says,
        "The disk has 490 B free. At this rate it reaches the 100 B Nix keeps free in 1 m, \
         where Nix collects its own garbage as it builds.",
        "above the floor, when it reaches it"
    );
    status.measured(&space(300), &bands, 1900.0, None);
    status.measured(&space(100), &bands, 1900.0, None);
    assert!(
        !status.says.contains("At this rate"),
        "at the floor, neither it nor when it fills: {}",
        status.says
    );
    // Worked out under the floor, it said when it fills, and a pass that
    // lifts it back above has Nix collecting, so it fills at no time told.
    status.floor_at = None;
    status.full_at = Some(1960.0);
    status.measured(&space(90), &bands, 1900.0, None);
    assert!(
        status.says.contains("At this rate it fills in 1 m."),
        "under the floor, when it fills: {}",
        status.says
    );
    status.measured(&space(200), &bands, 1900.0, None);
    assert!(
        !status.says.contains("At this rate"),
        "back above the floor, not when it fills: {}",
        status.says
    );
}

/// The heartbeat's fresh measures write the machine's sentence from the
/// room each filesystem tells, so it says what the filesystem's own does,
/// on either side of Nix's floor and across it.
#[test]
fn a_fresh_measures_machine_sentence_says_what_its_own_does() {
    use disk_guard::band::Space;
    use disk_guard::config::Bands;
    use disk_guard::daemon::FilesystemStatus;
    use disk_guard::words::machine;
    let bands = Bands {
        filling: Size::Percent(25.0),
        nearly_full: Size::Percent(10.0),
        critically_full: Size::Percent(4.0),
        target: Size::Percent(35.0),
    };
    let space = |free| Space {
        total: 1000,
        free,
        btrfs: None,
    };
    let mut status = FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Healthy,
        free: 500,
        total: 1000,
        size: 1000,
        used: 500,
        forecast: None,
        fill_per_hour: Some(36_000.0),
        full_at: None,
        nix_floor: Some(100),
        floor_at: Some(1960.0),
        says: String::new(),
        exhausted: false,
    };
    let at_rate = |said: &str| said.split("At this rate").nth(1).map(str::to_owned);
    let agrees = |status: &FilesystemStatus, why: &str| {
        let said = machine(&[status.told(1900.0)], None);
        assert_eq!(
            at_rate(&said),
            at_rate(&status.says),
            "{why}: the machine said {said:?} where the disk said {:?}",
            status.says
        );
    };
    status.measured(&space(490), &bands, 1900.0, None);
    agrees(&status, "above the floor");
    status.measured(&space(100), &bands, 1900.0, None);
    agrees(&status, "at the floor");
    status.floor_at = None;
    status.full_at = Some(1960.0);
    status.measured(&space(90), &bands, 1900.0, None);
    agrees(&status, "under the floor");
    status.measured(&space(200), &bands, 1900.0, None);
    agrees(&status, "back above the floor");
}

/// A status counts to Nix's floor while above it with a time to reach
/// it, in its own sentence and the machine's, and to full at or under it.
#[test]
fn a_status_tells_the_time_to_nixs_floor_and_otherwise_to_full() {
    use disk_guard::daemon::FilesystemStatus;
    use disk_guard::words::{filesystem, machine};
    let mut status = FilesystemStatus {
        waiting: false,
        held: false,
        path: PathBuf::from("/"),
        band: Band::Healthy,
        free: 500,
        total: 1000,
        size: 1000,
        used: 500,
        forecast: None,
        fill_per_hour: Some(36_000.0),
        full_at: None,
        nix_floor: Some(100),
        floor_at: Some(1960.0),
        says: String::new(),
        exhausted: false,
    };
    let floor = "it reaches the 100 B Nix keeps free in 1 m";
    let said = filesystem(&status.told(1900.0), None);
    assert!(said.contains(floor), "its own sentence: {said}");
    let said = machine(&[status.told(1900.0)], None);
    assert!(said.contains(floor), "the machine's sentence: {said}");
    status.free = 90;
    status.floor_at = None;
    status.full_at = Some(1960.0);
    let said = filesystem(&status.told(1900.0), None);
    assert!(
        said.contains("At this rate it fills in 1 m."),
        "under the floor, when it fills: {said}"
    );
    // Above the floor Nix collects before the disk fills, so a time to
    // full alone is told as no time at all.
    status.free = 500;
    let said = filesystem(&status.told(1900.0), None);
    assert!(
        !said.contains("At this rate"),
        "above the floor, not when it fills: {said}"
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
             "bands": {{ "filling": "0%", "nearly_full": "0%", "critically_full": "0%",
                        "target": "0%" }},
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{
               "src": {{ "kind": "cargo", "roots": ["{0}/trees"] }},
               "trash": {{ "kind": "trash", "path": "{0}/Trash" }},
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

/// A committed checkout with a build, and a config whose predicate says
/// every checkout is finished, writing to the file returned each time
/// it's asked.
fn merged_app(home: &Home) -> (impl Fn() -> Config, PathBuf) {
    use crate::finished::git;
    use std::os::unix::fs::PermissionsExt;
    home.replaced_build("0%");
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
    let root = home.root.clone();
    let config = move || {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "notify": false,
                 "bands": {{ "filling": "0%", "nearly_full": "0%", "critically_full": "0%",
                            "target": "0%" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{
                   "src": {{ "kind": "cargo", "roots": ["{0}/src"] }},
                   "merged": {{ "kind": "finished", "remote": "^https://example\\.com/",
                                "command": ["{1}"], "timeout": "5s" }} }} }}"#,
            root.display(),
            predicate.display()
        ))
        .unwrap()
    };
    (config, asked)
}

fn times_asked(asked: &Path) -> usize {
    fs::read_to_string(asked)
        .unwrap_or_default()
        .lines()
        .count()
}

#[test]
fn a_predicates_answer_is_kept_for_the_next_guard() {
    let home = Home::new();
    let (config, asked) = merged_app(&home);
    let mut first = home.guard(config());
    first.step(&tick(), &CALM).unwrap();
    assert_eq!(times_asked(&asked), 1, "routine upkeep asks once");
    drop(first);
    let mut second = home.guard(config());
    second
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
    assert_eq!(times_asked(&asked), 1, "a finished head is known still");
}

#[test]
fn a_checkout_with_work_not_committed_is_never_finished() {
    // On 28 October a session went on editing a tree whose head had just
    // merged, and the guard took its packages from under it twice.
    let home = Home::new();
    let (config, asked) = merged_app(&home);
    let manifest = home.root.join("src/app/Cargo.toml");
    fs::write(&manifest, "[package]\nname = \"app2\"\n").unwrap();
    let mut guard = home.guard(config());
    guard.step(&tick(), &CALM).unwrap();
    assert_eq!(times_asked(&asked), 0, "its head isn't what's in it");
}

#[test]
fn a_held_snapshots_build_goes_an_hour_after_its_last_use() {
    // A gate's snapshot is held by the session that cut it, which goes on
    // working long after the gate: on 17 November 34 such snapshots held
    // 86 GiB of builds on a filling disk. A worktree held as long keeps its
    // build, since a person may come back to edit it.
    for shareable in [true, false] {
        let home = Home::new();
        let built = held_tree(&home, 400);
        let record = home.root.join("cut/app.json");
        let body = fs::read_to_string(&record).unwrap().replacen(
            '{',
            &format!(r#"{{"identity":{{"key":"app","shareable":{shareable}}},"#),
            1,
        );
        fs::write(&record, body).unwrap();
        let mut guard = home.guard(owned_trees(&home));
        guard.step(&tick(), &CALM).unwrap();
        assert!(built.exists(), "a held tree's build stays while it's fresh");
        // Trash long expired goes at any upkeep, so whether it's still
        // there says whether one came.
        let probe = home.trashed("old.txt", "2001-09-09T01:46:40");
        let after = |seconds: f64| Tick {
            wall: tick().wall + seconds,
            monotonic: 3_600.0 + seconds,
            ..tick()
        };
        guard.step(&after(1_800.0), &CALM).unwrap();
        assert!(probe.exists(), "no upkeep comes before the hour");
        assert!(built.exists(), "half an hour unused, the build stays");
        guard.step(&after(3_700.0), &CALM).unwrap();
        if shareable {
            assert!(!probe.exists(), "the snapshot's hour made upkeep due");
            assert!(
                !built.exists(),
                "a snapshot's build goes after an idle hour"
            );
        } else {
            assert!(probe.exists(), "a worktree's build makes nothing due");
            guard.step(&after(7.0 * 3_600.0), &CALM).unwrap();
            assert!(!probe.exists(), "routine upkeep came");
            assert!(built.exists(), "a worktree's build stays through upkeep");
        }
    }
}

#[test]
fn a_builds_disk_past_its_budget_makes_upkeep_due_at_most_hourly() {
    // On 17 November a shared builds disk held 63 GiB against the 20 its
    // budget allows while the root disk was healthy, so no band, and no
    // holder's end, brought the upkeep that would hold it to its budget.
    for (budget, past) in [("1", true), (r#""100%""#, false)] {
        let home = Home::new();
        fs::create_dir_all(home.root.join("builds")).unwrap();
        let config = Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "notify": false,
                 "bands": {{ "filling": "0%", "nearly_full": "0%", "critically_full": "0%",
                            "target": "0%" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{
                   "builds": {{ "kind": "cargo", "roots": ["{0}/builds"], "depth": 2,
                                "budget": {budget} }},
                   "trash": {{ "kind": "trash", "path": "{0}/Trash" }} }} }}"#,
            home.root.display()
        ))
        .unwrap();
        let mut guard = home.guard(config);
        let after = |seconds: f64| Tick {
            wall: tick().wall + seconds,
            monotonic: 3_600.0 + seconds,
            ..tick()
        };
        // Trash long expired goes at any upkeep, so whether it's still
        // there says whether one came.
        let probe = || home.trashed("old.txt", "2001-09-09T01:46:40");
        let first = probe();
        guard.step(&after(0.0), &CALM).unwrap();
        guard.step(&after(61.0), &CALM).unwrap();
        assert!(!first.exists(), "the guard's first upkeep ran");
        let within = probe();
        guard.step(&after(1_800.0), &CALM).unwrap();
        assert!(
            within.exists(),
            "a disk still past its budget asks again only an hour on"
        );
        guard.step(&after(3_800.0), &CALM).unwrap();
        assert_eq!(
            !within.exists(),
            past,
            "a disk past its budget makes upkeep due an hour on; one within it doesn't"
        );
    }
}

#[test]
fn a_snapshot_cut_since_the_last_survey_is_looked_at_an_hour_on() {
    // On 17 November a gate's snapshot was cut, and built in, after the
    // guard's upkeep, so no survey had timed its build's hour, and it
    // would have waited six for routine upkeep.
    let home = Home::new();
    let mut guard = home.guard(owned_trees(&home));
    let after = |seconds: f64| Tick {
        wall: tick().wall + seconds,
        monotonic: 3_600.0 + seconds,
        ..tick()
    };
    // The first step starts the guard; the second is its first upkeep,
    // which the probe's going shows.
    let probe = home.trashed("old.txt", "2001-09-09T01:46:40");
    guard.step(&after(0.0), &CALM).unwrap();
    guard.step(&after(61.0), &CALM).unwrap();
    assert!(!probe.exists(), "upkeep ran before the snapshot was cut");
    let built = held_tree(&home, 400);
    let record = home.root.join("cut/app.json");
    let body = fs::read_to_string(&record).unwrap().replacen(
        '{',
        r#"{"identity":{"key":"app","shareable":true},"#,
        1,
    );
    fs::write(&record, body).unwrap();
    guard.step(&after(122.0), &CALM).unwrap();
    assert!(built.exists(), "a fresh snapshot's build stays");
    guard.step(&after(3_900.0), &CALM).unwrap();
    assert!(
        !built.exists(),
        "a snapshot cut after upkeep loses its idle build an hour on"
    );
}

#[test]
fn a_held_trees_build_stays_though_its_branch_merged() {
    // On 12 November a session merged its branch and went on in the tree
    // its broker recorded it holding, and twice the guard took the tree's
    // packages within the half hour after a merge.
    use crate::finished::git;
    use std::os::unix::fs::PermissionsExt;
    let home = Home::new();
    let built = held_tree(&home, 400);
    let app = home.root.join("trees/app");
    git(&app, &["init", "-q", "-b", "work"]);
    git(
        &app,
        &["remote", "add", "origin", "https://example.com/app.git"],
    );
    git(&app, &["add", "Cargo.toml"]);
    git(&app, &["commit", "-q", "-m", "one"]);
    let predicate = home.root.join("merged");
    fs::write(&predicate, "#!/bin/sh\necho '{\"finished\": true}'\n").unwrap();
    fs::set_permissions(&predicate, fs::Permissions::from_mode(0o755)).unwrap();
    let mut config = owned_trees(&home);
    let merged = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
             "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
             "sources": {{
               "merged": {{ "kind": "finished", "remote": "^https://example\\.com/",
                            "command": ["{1}"], "timeout": "5s" }} }} }}"#,
        home.root.display(),
        predicate.display()
    ))
    .unwrap();
    config.sources.extend(merged.sources);
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        built.exists(),
        "a tree held by a running process was taken as finished by its branch"
    );
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    guard.step(&tick_at(3661.0), &CALM).unwrap();
    assert!(
        !built.exists(),
        "a merged tree's build stayed once its holder had ended"
    );
}

#[test]
fn a_preview_leaves_a_damaged_memory_as_it_found_it() {
    let home = Home::new();
    fs::write(home.root.join("state/memory.json"), "{ \"learned\": ").unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard
        .preview(&tick(), Some(Band::Filling), None, true)
        .unwrap();
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
    /// A config whose disk is critically full, or healthy, with an empty scratch
    /// source on it, so a pass finds nothing to take.
    fn critically_full(&self, critically_full: bool) -> Config {
        let share = if critically_full { "100%" } else { "0%" };
        fs::create_dir_all(self.root.join("scratch")).unwrap();
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true, "notify": false,
                 "bands": {{ "filling": "{share}", "nearly_full": "{share}",
                            "critically_full": "{share}", "target": "{share}" }},
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
fn critically_full_with_nothing_left_the_runaway_writer_is_frozen_with_its_children() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critically_full(true));
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
    let mut guard = home.guard(home.critically_full(true));
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
    let mut guard = home.guard(home.critically_full(true));
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
    let mut guard = home.guard(home.critically_full(true));
    running_away(&home, &mut guard, &app(uid()));
    let writer = &home.status()["writer"];
    assert_eq!(writer["name"], "dd", "{writer}");
    assert_eq!(writer["pid"], 700);
    let mut calm = home.guard(home.critically_full(false));
    calm.step(&tick(), &CALM).unwrap();
    assert_eq!(home.status()["writer"], serde_json::Value::Null);
}

#[test]
fn nothing_in_a_session_is_ever_frozen() {
    let home = Home::new();
    let manager = Manager::default();
    let mut guard = home.guard(home.critically_full(true));
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
    let mut guard = home.guard(home.critically_full(true));
    guard.freeze_with(Box::new(manager.clone()));
    running_away(&home, &mut guard, &app(uid()));
    drop(guard);
    // The next guard, the disk now healthy, remembers what it froze.
    let mut guard = home.guard(home.critically_full(false));
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
    let mut guard = home.guard(home.critically_full(true));
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
    /// A crate `name` under `src`, built twice, its older build five days
    /// old, answering its `deps` folder.
    fn replaced_crate(&self, name: &str) -> PathBuf {
        let app = self.root.join("src").join(name);
        fs::create_dir_all(app.join("src")).unwrap();
        fs::write(
            app.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\n"),
        )
        .unwrap();
        let deps = app.join("target/debug/deps");
        fs::create_dir_all(&deps).unwrap();
        fs::write(
            app.join("target/CACHEDIR.TAG"),
            "Signature: 8a477f597d28d172789f06886806bc55\n# created by cargo\n",
        )
        .unwrap();
        for digest in [OLD, NEW] {
            fs::write(
                deps.join(format!("lib{name}-{digest}.rlib")),
                "x".repeat(4096),
            )
            .unwrap();
        }
        touch(
            &deps.join(format!("lib{name}-{OLD}.rlib")),
            crate::whole_seconds(wall()) - 5 * 86_400,
        );
        deps
    }

    fn replaced_build(&self, filling: &str) -> (Config, PathBuf) {
        let deps = self.replaced_crate("app");
        let config = Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true, "notify": false,
                 "bands": {{ "filling": "{filling}", "nearly_full": "0%", "critically_full": "0%",
                            "target": "{filling}" }},
                 "nix": {{ "bin": "{0}/no-nix/nix", "store": "{0}/no-nix/store" }},
                 "sources": {{ "src": {{ "kind": "cargo", "roots": ["{0}/src"] }} }} }}"#,
            self.root.display()
        ))
        .unwrap();
        (config, deps)
    }
}

#[test]
fn a_replaced_build_unopened_for_days_goes_once_filling_and_the_newest_stays() {
    // A guard that never listened, as `plan` and `explain` don't, reads
    // what the daemon watched, so they say what it would take.
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
fn a_reach_that_only_kept_teaches_nothing_of_how_long_a_pass_takes() {
    // A reach that judged its one build and kept it, as cargo held it,
    // ends in moments; learned, it drew the forecast's lead to its floor.
    let home = Home::new();
    let (config, deps) = home.replaced_build("100%");
    let lock_path = deps.parent().unwrap().join(".cargo-lock");
    fs::write(&lock_path, "").unwrap();
    let lock = fs::File::open(&lock_path).unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockShared).unwrap();
    let mut uses = Uses::default();
    uses.watching(&deps, wall() - 3.0 * 86_400.0);
    disk_guard::state::save(&home.root.join("state/uses.json"), &uses).unwrap();
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    let later = Tick {
        wall: wall() + 301.0,
        ..tick()
    };
    guard.step(&later, &CALM).unwrap();
    let journal = home.journal();
    let actions: Vec<_> = journal
        .iter()
        .filter(|line| line["event"] == "action")
        .collect();
    assert!(
        !actions.is_empty() && actions.iter().all(|line| line["outcome"] == "kept"),
        "the reach didn't judge and keep alone, so this case tests nothing: {journal:?}"
    );
    assert_eq!(
        home.memory()["pass_seconds"],
        serde_json::Value::Null,
        "learned from a reach that only kept: {journal:?}"
    );
    drop(lock);
}

#[test]
fn a_pass_journals_how_many_replaced_builds_went_from_each_target_and_their_bytes() {
    // So whoever reads how fast a build fills sees what each sweep took
    // from it, not only a line per artifact.
    let home = Home::new();
    let (config, deps) = home.replaced_build("100%");
    // A second older build of the crate, so the line sums two.
    let second = deps.join("libapp-3333333333333333.rlib");
    fs::write(&second, "x".repeat(4096)).unwrap();
    touch(&second, crate::whole_seconds(wall()) - 5 * 86_400);
    // Another target, whose cargo holds its lock, so its older build is
    // kept and must not be counted as gone.
    let held = home.replaced_crate("held");
    let lock_path = held.parent().unwrap().join(".cargo-lock");
    fs::write(&lock_path, "").unwrap();
    let lock = fs::File::open(&lock_path).unwrap();
    rustix::fs::flock(&lock, rustix::fs::FlockOperation::LockShared).unwrap();
    let mut uses = Uses::default();
    uses.watching(&deps, wall() - 3.0 * 86_400.0);
    uses.watching(&held, wall() - 3.0 * 86_400.0);
    disk_guard::state::save(&home.root.join("state/uses.json"), &uses).unwrap();
    let mut guard = home.guard(config);
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        held.join(format!("libheld-{OLD}.rlib")).exists(),
        "the build cargo's lock holds was taken, so this case tests nothing"
    );
    let target = deps.parent().unwrap().parent().unwrap();
    let journal = home.journal();
    let tried = held.join(format!("held-{OLD}"));
    assert!(
        journal.iter().any(|line| line["event"] == "action"
            && line["path"] == tried.to_string_lossy().as_ref()
            && line["outcome"] == "kept"),
        "the held build was never tried, so this case tests nothing: {journal:?}"
    );
    let swept: Vec<_> = journal
        .iter()
        .filter(|line| line["event"] == "replaced")
        .collect();
    assert_eq!(
        swept.len(),
        1,
        "a pass left other than one line for the one target it took from: {journal:?}"
    );
    assert_eq!(
        (&swept[0]["target"], &swept[0]["count"], &swept[0]["bytes"]),
        (
            &serde_json::json!(target),
            &serde_json::json!(2),
            &serde_json::json!(8192)
        ),
        "the target's line says the wrong target, count or bytes"
    );
    let passed = journal.iter().position(|line| line["event"] == "pass");
    let replaced = journal.iter().position(|line| line["event"] == "replaced");
    assert!(
        passed.is_some() && passed < replaced,
        "the replaced lines came before the pass's own: {journal:?}"
    );
    assert!(
        swept[0]["band"].is_string()
            && passed.is_some_and(|at| swept[0]["band"] == journal[at]["band"]),
        "a replaced line didn't name its pass's band: {journal:?}"
    );
}

#[test]
fn a_daemon_that_cant_hear_keeps_replaced_builds_and_forgets_what_was_watched() {
    // Earlier guards' dates stop where a daemon with no watch starts, so
    // trusting them would date a build by opens nobody heard.
    let home = Home::new();
    let (config, deps) = home.replaced_build("100%");
    let mut uses = Uses::default();
    uses.watching(&deps, wall() - 3.0 * 86_400.0);
    disk_guard::state::save(&home.root.join("state/uses.json"), &uses).unwrap();
    let mut guard = home.guard(config);
    guard.hears_nothing();
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        deps.join(format!("libapp-{OLD}.rlib")).exists(),
        "a daemon that can't hear took a replaced build by an earlier guard's dates"
    );
    let kept: Uses = disk_guard::state::load(&home.root.join("state/uses.json"))
        .unwrap()
        .unwrap_or_default();
    assert!(
        kept.watched.is_empty(),
        "a daemon that can't hear saved earlier guards' dates for the next: {kept:?}"
    );
}

#[test]
fn the_opens_a_watch_hears_are_kept_and_its_own_marks_heard() {
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
    // Every cargo opens its profile's lock, even one that writes nothing.
    let lock = deps.parent().unwrap().join(".cargo-build-lock");
    fs::write(&lock, "").unwrap();
    std::process::Command::new("cat")
        .arg(&rlib)
        .arg(&lock)
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
        let heard = kept.opened.contains_key(&rlib) && kept.opened.contains_key(&lock);
        if heard || std::time::Instant::now() > deadline {
            break kept;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    };
    assert!(kept.watched.contains_key(&deps), "{kept:?}");
    assert!(
        kept.watched.contains_key(deps.parent().unwrap()),
        "the profile, whose lock a fresh cargo opens, went unwatched: {kept:?}"
    );
    assert!(kept.opened.contains_key(&rlib), "{kept:?}");
    assert!(
        kept.opened.contains_key(&lock),
        "a cargo opening its profile's lock went unheard: {kept:?}"
    );
    let hears = guard.hears();
    assert!(
        hears.contains(&deps) && hears.contains(&deps.parent().unwrap().to_owned()),
        "the guard's marks on the profile and its deps weren't kept as heard: {hears:?}"
    );
}

/// How a listener's reads go, once a test says they start going wrong:
/// failing (`Err`), passing (`Ok`) or panicking, in turn, and then as the
/// last says for good.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Read {
    Fails,
    Passes,
    Panics,
}

/// A guard listening through reads that go as `reads` says once they turn,
/// its builds' deps, and how many reads went since they turned.
struct Listening {
    home: Home,
    guard: Guard,
    deps: PathBuf,
    read: std::sync::Arc<std::sync::atomic::AtomicUsize>,
}

/// Starts a guard on a real watch whose reads, after its first step has
/// marked the build, go as `reads` says; nothing when there's no
/// fanotify here.
fn listening(reads: &'static [Read]) -> Option<Listening> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    let Ok(watch) = disk_guard::opens::Watch::new() else {
        eprintln!("no fanotify here, so nothing to check");
        return None;
    };
    let home = Home::new();
    let (config, deps) = home.replaced_build("0%");
    let mut guard = home.guard(config);
    let turned = Arc::new(AtomicBool::new(false));
    let read = Arc::new(AtomicUsize::new(0));
    let (turns, reading) = (Arc::clone(&turned), Arc::clone(&read));
    guard.listen_to(Arc::new(watch), move |within| {
        std::thread::sleep(within.min(std::time::Duration::from_millis(20)));
        if !turns.load(Ordering::SeqCst) {
            return Ok(Vec::new());
        }
        let at = reading.fetch_add(1, Ordering::SeqCst);
        match reads[at.min(reads.len() - 1)] {
            Read::Fails => Err(std::io::Error::other("a read that fails")),
            Read::Passes => Ok(Vec::new()),
            Read::Panics => panic!("a read that panics"),
        }
    });
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        guard.hears().contains(&deps),
        "the guard didn't hear the deps it marked, before anything went wrong"
    );
    turned.store(true, Ordering::SeqCst);
    Some(Listening {
        home,
        guard,
        deps,
        read,
    })
}

/// Waits up to `seconds` for `done`.
fn within(seconds: u64, mut done: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    while !done() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn a_guard_hears_through_failed_reads_that_pass_before_five() {
    // A read can fail and pass, as running short of memory does, so only
    // five failures in a row end the listener; each counts as lost events.
    use Read::{Fails, Passes};
    use std::sync::atomic::Ordering;
    static READS: [Read; 11] = [
        Fails, Fails, Fails, Fails, Passes, Fails, Fails, Fails, Fails, Passes, Passes,
    ];
    let started = wall();
    let Some(listening) = listening(&READS) else {
        return;
    };
    within(20, || listening.read.load(Ordering::SeqCst) >= READS.len());
    assert!(
        listening.read.load(Ordering::SeqCst) >= READS.len(),
        "the listener stopped reading after {} reads, though none failed five in a row",
        listening.read.load(Ordering::SeqCst)
    );
    let hears = listening.guard.hears();
    assert!(
        hears.contains(&listening.deps),
        "failed reads that passed before five left the guard hearing nothing: {hears:?}"
    );
    assert!(
        listening.guard.lost_at() >= started,
        "a failed read didn't count as lost events"
    );
}

/// Checks a guard whose listener ended hears nothing, saves no watched
/// dates for the next guard, and marks nothing an hour on; answers how
/// many reads it made once they turned.
fn ends_deaf(reads: &'static [Read]) -> Option<usize> {
    let mut listening = listening(reads)?;
    let tries = u64::from(disk_guard::daemon::LISTEN_TRIES);
    within(tries + 3, || listening.guard.hears().is_empty());
    let hears = listening.guard.hears();
    assert!(
        hears.is_empty(),
        "a guard whose listener ended ({reads:?}) still heard {hears:?}"
    );
    let later = Tick {
        wall: wall() + 3700.0,
        monotonic: 7300.0,
        ..tick()
    };
    listening.guard.step(&later, &CALM).unwrap();
    let kept: Uses = disk_guard::state::load(&listening.home.root.join("state/uses.json"))
        .unwrap()
        .unwrap_or_default();
    assert!(
        kept.watched.is_empty(),
        "a deaf guard saved what it watched for the next guard to trust: {kept:?}"
    );
    let hears = listening.guard.hears();
    assert!(
        hears.is_empty(),
        "a deaf guard marked its builds again an hour on: {hears:?}"
    );
    Some(listening.read.load(std::sync::atomic::Ordering::SeqCst))
}

#[test]
fn a_guard_whose_reads_keep_failing_hears_nothing() {
    if let Some(reads) = ends_deaf(&[Read::Fails]) {
        assert_eq!(
            reads, 5,
            "the listener didn't stop at its fifth failed read"
        );
    }
}

#[test]
fn a_guard_whose_listener_panicked_hears_nothing() {
    ends_deaf(&[Read::Panics]);
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

#[test]
fn marking_the_builds_journals_what_its_walk_read() {
    let Ok(watch) = disk_guard::opens::Watch::new() else {
        eprintln!("no fanotify here, so nothing to check");
        return;
    };
    let home = Home::new();
    let (config, _deps) = home.replaced_build("0%");
    let mut guard = home.guard(config);
    guard.listen(std::sync::Arc::new(watch));
    guard.step(&tick(), &CALM).unwrap();
    let walked = walks(&home, "marks");
    assert_eq!(
        walked.len(),
        1,
        "the marking walk wasn't journaled once: {walked:?}"
    );
    assert!(
        walked[0]["read"].is_u64(),
        "it didn't say what it read: {walked:?}"
    );
}

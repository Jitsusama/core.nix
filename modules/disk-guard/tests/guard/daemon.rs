use disk_guard::band::Band;
use disk_guard::clock::Tick;
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places, Senses};
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
        mains: true,
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
        Guard::new(config, &self.places()).unwrap()
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
    let socket = home.root.join("system.socket");
    let heard = fake_system(&socket, true);
    let places = Places {
        system: Some(socket),
        ..home.places()
    };
    let mut healthy = Guard::new(home.config("0%", "0%"), &places).unwrap();
    healthy.step(&tick(), &CALM).unwrap();
    let asked: Vec<Request> = heard.try_iter().collect();
    let [Request::Watching { paths }] = asked.as_slice() else {
        panic!("a heartbeat, and nothing collected while healthy: {asked:?}");
    };
    let mount = paths[0].clone();
    assert!(home.root.starts_with(&mount), "{mount:?}");
    let mut pressed = Guard::new(home.config("100%", "100%"), &places).unwrap();
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

    let socket = home.root.join("system.socket");
    let _heard = fake_system(&socket, false);
    let places = Places {
        system: Some(socket),
        ..home.places()
    };
    let mut vouched = Guard::new(home.config("100%", "100%"), &places).unwrap();
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
    let begin = &home.journal()[begun];
    assert_eq!(begin["root"], home.root.join("scratch").to_str().unwrap());
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
    let resumes = home
        .journal()
        .iter()
        .filter(|line| line["kind"] == "resumed")
        .count();
    assert_eq!(
        resumes, 1,
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
fn a_routine_pass_gives_way_when_the_disk_gets_busy_and_comes_back_later() {
    let home = Home::new();
    let old = home.trashed("old.txt", "2001-09-09T01:46:40");
    let older = home.trashed("older.txt", "2001-09-08T01:46:40");
    let busy = home.root.join("proc/pressure");
    fs::create_dir_all(&busy).unwrap();
    fs::write(
        busy.join("io"),
        "some avg10=50.00 avg60=0.00 avg300=0.00 total=0\n\
         full avg10=0.00 avg60=0.00 avg300=0.00 total=0\n",
    )
    .unwrap();
    let mut guard = home.guard(home.config("0%", "0%"));
    guard.step(&tick(), &CALM).unwrap();
    assert!(old.exists() && older.exists(), "given way");
    let kept: Vec<_> = home
        .journal()
        .into_iter()
        .filter(|line| line["event"] == "action")
        .collect();
    assert_eq!(kept.len(), 2, "{kept:?}");
    assert!(kept.iter().all(|line| line["outcome"] == "kept"
        && line["reason"] == "the machine got busy; left for a calmer moment"));
    fs::remove_dir_all(&busy).unwrap();
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        !old.exists() && !older.exists(),
        "upkeep that gave way isn't done"
    );
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
    let mut guard = Guard::new(home.config("0%", "100%"), &places).unwrap();
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
            mains: false,
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
            wall() as i64 - 5 * 86_400,
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

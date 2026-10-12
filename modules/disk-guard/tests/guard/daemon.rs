use disk_guard::clock::Tick;
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places, Senses};
use disk_guard::moments::{Power, Pressure};
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
        let dir = tempfile::tempdir().unwrap();
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
    /// puts any disk in pressure and `0%` keeps it healthy.
    fn config(&self, pressure: &str, target: &str) -> Config {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "user", "status_file": true,
                 "bands": {{ "pressure": "{pressure}", "urgent": "0%", "critical": "0%", "target": "{target}" }},
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

#[test]
fn the_system_daemon_vouches_for_what_this_account_cant_read() {
    let home = Home::new();
    let cold = home.cold_scratch("run-1");
    home.hidden(300, 77);
    let mut alone = home.guard(home.config("100%", "100%"));
    alone.step(&tick(), &CALM).unwrap();
    assert!(cold.exists(), "kept while process 300 can't be judged");

    let socket = home.root.join("system.socket");
    let mut server = disk_guard::socket::Server::bind(&socket, std::time::Duration::ZERO).unwrap();
    std::thread::spawn(move || {
        server
            .serve(&mut |_, _| {
                Ok(disk_guard::vouch::Answer {
                    vouched: vec![(300, 77)],
                    touching: vec![],
                })
            })
            .unwrap();
    });
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

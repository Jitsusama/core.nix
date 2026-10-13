//! The system daemon collecting Nix's garbage, against a fake store and a
//! fake nix that does to it what Nix would.

use disk_guard::clock::Tick;
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Places, Senses};
use disk_guard::moments::{Power, Pressure};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

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

fn now() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn tick() -> Tick {
    Tick {
        wall: now(),
        monotonic: 3600.0,
        suspended: 0.0,
        boot: "test".into(),
    }
}

struct Store {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Store {
    /// A store, and a nix and nix-store over it that log what they're asked
    /// to `log`, each with the scheduling policy it ran under.
    fn new() -> Store {
        let dir = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(dir.path()).unwrap();
        for sub in ["store", "bin", "state", "proc", "boot"] {
            fs::create_dir_all(root.join(sub)).unwrap();
        }
        let log = root.join("log");
        let policy = "$(cut -d' ' -f41 /proc/$$/stat)";
        let script = |name: &str, body: String| {
            crate::garbage::script(&root.join("bin").join(name), &body);
        };
        script(
            "nix-store",
            format!(
                "[ \"$*\" = '--gc --print-dead' ] || exit 9\n\
                 echo \"list {policy}\" >> {log}\n\
                 for p in {store}/*; do [ -e \"$p/meta\" ] && echo \"$p\"; done\n\
                 exit 0\n",
                log = log.display(),
                store = root.join("store").display(),
            ),
        );
        script(
            "nix",
            format!(
                "case \"$1 $2\" in\n\
                 'path-info --json') shift 2; printf '{{'; sep=''\n\
                   for p in \"$@\"; do read t s < \"$p/meta\"\n\
                     printf '%s\"%s\":{{\"narSize\":%s,\"registrationTime\":%s,\"references\":[]}}' \
                       \"$sep\" \"$p\" \"$s\" \"$t\"; sep=,\n\
                   done; printf '}}' ;;\n\
                 'store delete') shift 2; echo \"delete $*\" >> {log}; rm -r \"$@\" ;;\n\
                 'store gc') shift 2; echo \"gc $* {policy}\" >> {log} ;;\n\
                 *) exit 9 ;;\n\
                 esac\n",
                log = log.display(),
            ),
        );
        Store { _dir: dir, root }
    }

    /// A dead store path registered `ago` seconds since, of `size` bytes.
    fn dead(&self, name: &str, ago: f64, size: u64) -> PathBuf {
        let path = self.root.join("store").join(name);
        fs::create_dir_all(&path).unwrap();
        fs::write(
            path.join("meta"),
            format!("{} {size}\n", (now() - ago) as u64),
        )
        .unwrap();
        path
    }

    fn config(&self, pressure: &str, urgent: &str) -> Config {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "system", "status_file": true,
                 "bands": {{ "pressure": "{pressure}", "urgent": "{urgent}", "critical": "0%",
                            "target": "{pressure}" }},
                 "garbage": {{ "nix": "{0}/bin/nix", "store": "{0}/store", "keep": "3d",
                              "budget": "10%" }},
                 "boot": "{0}/boot",
                 "sources": {{}} }}"#,
            self.root.display()
        ))
        .unwrap()
    }

    fn guard(&self, config: Config) -> Guard {
        Guard::new(
            config,
            &Places {
                state: self.root.join("state"),
                status: Some(self.root.join("status.json")),
                processes: self.root.join("proc"),
                system: None,
            },
        )
        .unwrap()
    }

    fn log(&self) -> Vec<String> {
        let log = fs::read_to_string(self.root.join("log")).unwrap_or_default();
        log.lines().map(str::to_owned).collect()
    }

    /// Steps `guard` until `done` holds, as the listing runs apart.
    fn step_until(&self, guard: &mut Guard, done: impl Fn(&Store) -> bool) {
        let started = Instant::now();
        while !done(self) {
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "{:?}",
                self.log()
            );
            guard.step(&tick(), &CALM).unwrap();
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

const DAY: f64 = 86400.0;

#[test]
fn routine_upkeep_lists_garbage_at_idle_and_takes_what_is_old() {
    let store = Store::new();
    let old = store.dead("aaa-old", 10.0 * DAY, 4096);
    let young = store.dead("bbb-young", 3600.0, 4096);
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |_| !old.exists());
    let log = store.log();
    assert_eq!(log[0], "list 5", "listed at idle priority");
    assert_eq!(log[1], format!("delete {}", old.display()));
    assert!(young.exists(), "young garbage within budget stays");
    assert_eq!(log.len(), 2, "{log:?}");
}

#[test]
fn a_listing_is_kept_for_the_next_guard() {
    let store = Store::new();
    store.dead("aaa-young", 3600.0, 4096);
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| {
        store.root.join("state/garbage.json").exists()
    });
    drop(guard);
    let mut next = store.guard(store.config("0%", "0%"));
    // Long enough for a listing begun to say so.
    for _ in 0..10 {
        next.step(&tick(), &CALM).unwrap();
        std::thread::sleep(Duration::from_millis(30));
    }
    let lists = store
        .log()
        .iter()
        .filter(|line| line.starts_with("list"))
        .count();
    assert_eq!(lists, 1, "a day old at most, so not listed again");
}

#[test]
fn urgent_without_a_listing_collects_by_nix_store_gc_at_once() {
    let store = Store::new();
    let mut guard = store.guard(store.config("100%", "100%"));
    guard.step(&tick(), &CALM).unwrap();
    let log = store.log();
    assert!(
        log.iter().any(|line| line == "gc --max 8589934592 0"),
        "a step of 8 GiB, at the guard's priority: {log:?}"
    );
}

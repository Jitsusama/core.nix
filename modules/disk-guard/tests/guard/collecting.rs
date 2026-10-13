//! The system daemon collecting Nix's garbage, against a fake store and a
//! fake nix that does to it what Nix would.

use disk_guard::band::Band;
use disk_guard::clock::Tick;
use disk_guard::config::Config;
use disk_guard::daemon::{Guard, Hearing, Places, Senses};
use disk_guard::moments::{Power, Pressure};
use disk_guard::socket::Responder;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, mpsc};
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
        for sub in [
            "store",
            "bin",
            "state",
            "proc",
            "boot/EFI/Linux",
            "profiles",
        ] {
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
        script(
            "nix-env",
            format!(
                "[ \"$1 $3\" = '-p --delete-generations' ] || exit 9\n\
                 echo \"expire $* {policy}\" >> {log}\n\
                 profile=$2; shift 3\n\
                 for n in \"$@\"; do rm \"$profile-$n-link\"; done\n",
                log = log.display(),
            ),
        );
        Store { _dir: dir, root }
    }

    /// The profile `name`, under `profiles/` or deeper, with a generation
    /// for each `(number, days ago)`, `current` being the one in use.
    fn profile(&self, name: &str, generations: &[(u32, f64)], current: u32) -> PathBuf {
        let path = self.root.join("profiles").join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let file = path.file_name().unwrap().to_str().unwrap();
        crate::generations::profile_at(path.parent().unwrap(), file, generations, current, now())
    }

    /// A boot entry for the system's generation `number`, as lanzaboote
    /// names it.
    fn entry(&self, number: u32) {
        let name = format!("nixos-generation-{number}-abcdefgh.efi");
        fs::write(self.root.join("boot/EFI/Linux").join(name), "").unwrap();
    }

    /// The `expire` lines logged, each with its policy.
    fn expired(&self) -> Vec<String> {
        self.log()
            .into_iter()
            .filter(|line| line.starts_with("expire"))
            .collect()
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
                 "nix": {{ "bin": "{0}/bin/nix", "store": "{0}/store",
                          "profiles": "{0}/profiles", "generations": {{ "keep": 2 }} }},
                 "garbage": {{ "keep": "3d", "budget": "10%" }},
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
                running: vec![self.root.join("running")],
            },
        )
        .unwrap()
    }

    /// Has the running system be what generation `number` links to.
    fn running(&self, number: u32) {
        std::os::unix::fs::symlink(
            format!("/nix/store/{number}-system"),
            self.root.join("running"),
        )
        .unwrap();
    }

    /// Has `guard` hear that a user daemon spoke for the store's filesystem
    /// `ago` since, giving where to ask it to collect.
    fn spoken_for(
        &self,
        guard: &mut Guard,
        ago: Duration,
    ) -> mpsc::Sender<(PathBuf, Band, Responder)> {
        let device = disk_guard::pool::pool(&self.root.join("store")).unwrap();
        let spoken = HashMap::from([(device, Instant::now().checked_sub(ago).unwrap())]);
        let (asking, collects) = mpsc::channel();
        guard.hear(Hearing {
            spoken: Arc::new(Mutex::new(spoken)),
            collects,
        });
        asking
    }

    fn gc_ran(&self) -> bool {
        self.log().iter().any(|line| line.starts_with("gc "))
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
fn a_filesystem_a_user_daemon_speaks_for_is_collected_when_it_asks() {
    let store = Store::new();
    let old = store.dead("aaa-old", 10.0 * DAY, 4096);
    let mut guard = store.guard(store.config("100%", "100%"));
    let asking = store.spoken_for(&mut guard, Duration::ZERO);
    store.step_until(&mut guard, |store| {
        store.root.join("state/garbage.json").exists()
    });
    assert!(
        old.exists(),
        "the user daemon drives the order: {:?}",
        store.log()
    );
    assert!(!store.gc_ran());
    let (mut ours, theirs) = UnixStream::pair().unwrap();
    asking
        .send((
            store.root.join("store"),
            Band::Urgent,
            Responder::new(theirs),
        ))
        .unwrap();
    guard.step(&tick(), &CALM).unwrap();
    assert!(!old.exists(), "{:?}", store.log());
    let mut reply = String::new();
    ours.read_to_string(&mut reply).unwrap();
    assert!(reply.contains("collected"), "{reply}");
}

#[test]
fn a_user_daemon_quiet_for_a_minute_leaves_the_system_to_pass_alone() {
    let store = Store::new();
    let mut guard = store.guard(store.config("100%", "100%"));
    let _asking = store.spoken_for(&mut guard, Duration::from_secs(61));
    guard.step(&tick(), &CALM).unwrap();
    assert!(store.gc_ran(), "{:?}", store.log());
}

/// Six system generations: 6 is current, 2 is running, 3 has a boot
/// entry, 5 and 6 are the newest two, 4 is under a week old and 1 is old.
fn generations(store: &Store) -> PathBuf {
    let ages = [
        (1, 30.0),
        (2, 29.0),
        (3, 20.0),
        (4, 3.0),
        (5, 2.0),
        (6, 1.0),
    ];
    let system = store.profile("system", &ages, 6);
    store.running(2);
    store.entry(3);
    system
}

#[test]
fn routine_upkeep_expires_old_generations_and_lists_the_garbage_again() {
    let store = Store::new();
    let system = generations(&store);
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| {
        let log = store.log();
        let expired = log.iter().position(|line| line.starts_with("expire"));
        expired.is_some_and(|at| log[at..].iter().any(|line| line.starts_with("list")))
    });
    let expected = format!("expire -p {} --delete-generations 1 5", system.display());
    assert_eq!(
        store.expired(),
        [expected],
        "old ones only, at idle priority"
    );
}

#[test]
fn pressure_takes_young_generations_too_but_never_one_that_boots_or_runs() {
    let store = Store::new();
    let system = generations(&store);
    let mut guard = store.guard(store.config("100%", "0%"));
    store.step_until(&mut guard, |store| store.expired().len() == 2);
    let mut expired = store.expired();
    expired.sort();
    let expire = |numbers: &str| {
        format!(
            "expire -p {} --delete-generations {numbers} 5",
            system.display()
        )
    };
    assert_eq!(expired, [expire("1"), expire("4")]);
}

#[test]
fn an_unreadable_boot_partition_keeps_every_system_generation() {
    use std::os::unix::fs::PermissionsExt;
    // SAFETY: geteuid only reads the process's own credentials.
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipped: root reads any directory");
        return;
    }
    let store = Store::new();
    generations(&store);
    let root = store.profile(
        "per-user/root/profile",
        &[(1, 30.0), (2, 20.0), (3, 1.0)],
        3,
    );
    let boot = store.root.join("boot");
    fs::set_permissions(&boot, fs::Permissions::from_mode(0o000)).unwrap();
    let mut guard = store.guard(store.config("100%", "0%"));
    store.step_until(&mut guard, |store| !store.expired().is_empty());
    for _ in 0..3 {
        guard.step(&tick(), &CALM).unwrap();
    }
    fs::set_permissions(&boot, fs::Permissions::from_mode(0o755)).unwrap();
    let expected = format!("expire -p {} --delete-generations 1 5", root.display());
    assert_eq!(
        store.expired(),
        [expected],
        "a profile nothing boots still expires"
    );
}

#[test]
fn a_collect_asked_expires_young_generations_ahead_of_the_garbage() {
    let store = Store::new();
    let system = generations(&store);
    let old = store.dead("aaa-old", 10.0 * DAY, 4096);
    let mut guard = store.guard(store.config("100%", "0%"));
    let asking = store.spoken_for(&mut guard, Duration::ZERO);
    store.step_until(&mut guard, |store| {
        store.root.join("state/garbage.json").exists()
    });
    assert!(
        store.expired().is_empty(),
        "the user daemon drives the order: {:?}",
        store.log()
    );
    let (mut ours, theirs) = UnixStream::pair().unwrap();
    asking
        .send((
            store.root.join("store"),
            Band::Pressure,
            Responder::new(theirs),
        ))
        .unwrap();
    guard.step(&tick(), &CALM).unwrap();
    let mut reply = String::new();
    ours.read_to_string(&mut reply).unwrap();
    let mut expired = store.expired();
    expired.sort();
    let expire = |numbers: &str| {
        format!(
            "expire -p {} --delete-generations {numbers} 5",
            system.display()
        )
    };
    assert_eq!(expired, [expire("1"), expire("4")], "{reply}");
    let log = store.log();
    let deleted = log
        .iter()
        .position(|line| line == &format!("delete {}", old.display()));
    let last = log.iter().rposition(|line| line.starts_with("expire"));
    assert!(
        deleted > last,
        "generations make garbage, so go first: {log:?}"
    );
}

#[test]
fn a_user_daemon_expires_its_own_profiles_generations() {
    let store = Store::new();
    let ages = [(1, 30.0), (2, 20.0), (3, 1.0)];
    let home = store.profile("home-manager", &ages, 3);
    let config = Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
             "bands": {{ "pressure": "0%", "urgent": "0%", "critical": "0%",
                        "target": "0%" }},
             "nix": {{ "bin": "{0}/bin/nix", "store": "{0}/store",
                      "profiles": "{0}/profiles", "generations": {{ "keep": 2 }} }},
             "sources": {{}} }}"#,
        store.root.display()
    ))
    .unwrap();
    let mut guard = store.guard(config);
    store.step_until(&mut guard, |store| !store.expired().is_empty());
    let expected = format!("expire -p {} --delete-generations 1 5", home.display());
    assert_eq!(store.expired(), [expected]);
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

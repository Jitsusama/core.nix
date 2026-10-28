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
    /// to `log`, each with the scheduling policy it ran under. A path with
    /// `held` beside its `meta` is listed as dead every time and refused by
    /// every delete, as what a build roots again after a listing; while
    /// `listing-slow` is there, every listing after the first waits on it.
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
            "temproots",
        ] {
            fs::create_dir_all(root.join(sub)).unwrap();
        }
        let log = root.join("log");
        let policy = "$(cut -d' ' -f41 /proc/$$/stat)";
        let script = |name: &str, body: String| {
            crate::garbage::script(&root.join("bin").join(name), &body);
        };
        // Linking logs apart, so the garbage's log reads as it did.
        let linked = root.join("linked");
        script(
            "systemd-run",
            "[ \"$1\" = --scope ] || exit 9\n\
             while [ \"$1\" != -- ]; do shift; done; shift\n\
             exec \"$@\"\n"
                .to_owned(),
        );
        script(
            "nix-store",
            format!(
                "if [ \"$*\" = --optimise ]; then\n\
                   echo \"optimise {policy}\" >> {linked}\n\
                   [ -e {slow}-broken ] && {{ echo 'error: database is locked' >&2; exit 1; }}\n\
                   if [ -e {slow} ]; then\n\
                     trap 'echo stopped >> {linked}; exit 1' TERM\n\
                     sleep 30 & wait\n\
                   fi\n\
                   echo '1.0 GiB freed by hard-linking 3 files' >&2; exit 0\n\
                 fi\n\
                 [ \"$*\" = '--gc --print-dead' ] || exit 9\n\
                 echo \"list {policy}\" >> {log}\n\
                 if [ -e {listed} ]; then while [ -e {slow_list} ]; do sleep 0.05; done; fi\n\
                 : > {listed}\n\
                 [ -e {broken} ] && {{ echo 'error: a temporary file went' >&2; exit 1; }}\n\
                 for p in {store}/*; do [ -e \"$p/meta\" ] || continue\n\
                   [ -e \"$p/alive\" ] && [ ! -e {stale} ] || echo \"$p\"\n\
                 done\n\
                 rm -f {stale}; exit 0\n",
                log = log.display(),
                store = root.join("store").display(),
                linked = linked.display(),
                slow = root.join("slow").display(),
                broken = root.join("listing-broken").display(),
                listed = root.join("listed").display(),
                slow_list = root.join("listing-slow").display(),
                stale = root.join("listing-stale").display(),
            ),
        );
        script(
            "nix",
            format!(
                "[ \"$1 $2\" = '--extra-experimental-features nix-command' ] || exit 9\nshift 2\n\
                 case \"$1 $2\" in\n\
                 'path-info --json') [ \"$3\" = --offline ] || exit 9; shift 3\n\
                   printf '{{'; sep=''\n\
                   for p in \"$@\"; do read t s < \"$p/meta\"\n\
                     printf '%s\"%s\":{{\"narSize\":%s,\"registrationTime\":%s,\"references\":[]}}' \
                       \"$sep\" \"$p\" \"$s\" \"$t\"; sep=,\n\
                   done; printf '}}' ;;\n\
                 'store delete') shift 2; echo \"delete $*\" >> {log}\n\
                   for p in \"$@\"; do [ -e \"$p/alive\" ] || [ -e \"$p/held\" ] || continue\n\
                     echo \"error: Cannot delete path '$p' since it is still alive.\" >&2; exit 1\n\
                   done; rm -r \"$@\" ;;\n\
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
            format!("{} {size}\n", disk_guard::units::whole(now() - ago)),
        )
        .unwrap();
        path
    }

    /// A config keeping young garbage as a rebuild cache, as the guard did
    /// by default, so the policy's order can be shown.
    fn config(&self, pressure: &str, urgent: &str) -> Config {
        self.config_keeping(
            pressure,
            urgent,
            r#""garbage": { "keep": "3d", "budget": "10%" },"#,
        )
    }

    /// A config with the guard's own garbage policy, as a machine has.
    fn config_as_given(&self, pressure: &str, urgent: &str) -> Config {
        self.config_keeping(pressure, urgent, "")
    }

    fn config_keeping(&self, pressure: &str, urgent: &str, garbage: &str) -> Config {
        Config::parse(&format!(
            r#"{{ "version": 1, "scope": "system", "status_file": true,
                 "bands": {{ "pressure": "{pressure}", "urgent": "{urgent}", "critical": "0%",
                            "target": "{pressure}" }},
                 "nix": {{ "bin": "{0}/bin/nix", "store": "{0}/store",
                          "temp_roots": "{0}/temproots", "roots": "{0}/gcroots",
                          "profiles": "{0}/profiles", "generations": {{ "keep": 2 }} }},
                 {garbage}
                 "dedupe": {{ "systemd_run": "{0}/bin/systemd-run" }},
                 "boot": "{0}/boot",
                 "sources": {{}} }}"#,
            self.root.display()
        ))
        .unwrap()
    }

    fn guard(&self, config: Config) -> Guard {
        Guard::new(
            crate::unpaced(config),
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

    /// What linking the store's files logged, each run with its policy.
    fn linked(&self) -> Vec<String> {
        let log = fs::read_to_string(self.root.join("linked")).unwrap_or_default();
        log.lines().map(str::to_owned).collect()
    }

    /// The journal's lines for linking runs, oldest first.
    fn deduped(&self) -> Vec<serde_json::Value> {
        let journal = fs::read_to_string(self.root.join("state/journal.jsonl")).unwrap_or_default();
        journal
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .filter(|line| line["kind"] == "dedupe")
            .collect()
    }

    /// Has the guard remember a linking run that finished `ago` since.
    fn remember_deduped(&self, ago: f64) {
        let memory = disk_guard::state::Memory {
            deduped: Some(disk_guard::dedupe::Deduped {
                at: now() - ago,
                freed: 1 << 40,
            }),
            ..Default::default()
        };
        disk_guard::state::save(&self.root.join("state/memory.json"), &memory).unwrap();
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
fn garbage_a_running_build_holds_is_left_for_a_later_pass() {
    // Nix would refuse it, and each refusal costs a delete its whole look
    // for roots; on a busy machine that was most of a core freeing nothing.
    let store = Store::new();
    let held = store.dead("aaa-held.drv", 10.0 * DAY, 4096);
    let free = store.dead("bbb-free", 10.0 * DAY, 4096);
    fs::write(
        store.root.join("temproots/4242"),
        format!("{}\0", held.display()),
    )
    .unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |_| !free.exists());
    let deletes: Vec<String> = store
        .log()
        .into_iter()
        .filter(|line| line.starts_with("delete"))
        .collect();
    assert_eq!(deletes, [format!("delete {}", free.display())]);
    assert!(held.exists(), "a temporary root holds it");
}

#[test]
fn garbage_a_running_process_has_open_is_left_for_a_later_pass() {
    // As a test VM's virtiofsd keeps open each store path its machine
    // looked at, which Nix counts as a root.
    let store = Store::new();
    let open = store.dead("aaa-open", 10.0 * DAY, 4096);
    let free = store.dead("bbb-free", 10.0 * DAY, 4096);
    fs::create_dir_all(store.root.join("proc/4242/fd")).unwrap();
    std::os::unix::fs::symlink(open.join("meta"), store.root.join("proc/4242/fd/7")).unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |_| !free.exists());
    let deletes: Vec<String> = store
        .log()
        .into_iter()
        .filter(|line| line.starts_with("delete"))
        .collect();
    assert_eq!(deletes, [format!("delete {}", free.display())]);
    assert!(open.exists(), "a process has it open");
}

#[test]
fn many_small_paths_go_in_one_delete() {
    // Each delete costs Nix a whole look for roots, which a store of tens of
    // thousands takes most of a second over, whatever the batch holds.
    let store = Store::new();
    let dead: Vec<PathBuf> = (0..1000)
        .map(|n| store.dead(&format!("{n:04}-small.drv"), 10.0 * DAY, 4096))
        .collect();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |_| dead.iter().all(|path| !path.exists()));
    let deletes = store
        .log()
        .into_iter()
        .filter(|line| line.starts_with("delete"))
        .count();
    assert_eq!(deletes, 1, "a thousand small paths took {deletes} deletes");
}

#[test]
fn a_batch_that_meets_garbage_come_alive_has_the_store_listed_again() {
    // A build that roots a closure the listing called garbage makes Nix
    // refuse its batch a path at a time, each refusal a whole look for
    // roots; a batch made again from the same listing met them again, and
    // the guard spent most of a core for hours freeing nothing.
    let store = Store::new();
    let alive: Vec<PathBuf> = (0..4)
        .map(|n| store.dead(&format!("{n}-revived"), 10.0 * DAY, 4096))
        .collect();
    for path in &alive {
        fs::write(path.join("alive"), "").unwrap();
    }
    let free = store.dead("zzz-free", 10.0 * DAY, 4096);
    fs::write(store.root.join("listing-stale"), "").unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |_| !free.exists());
    assert_eq!(lists(&store), 2, "{:?}", store.log());
    assert!(alive.iter().all(|path| path.exists()), "a root holds them");
}

#[test]
fn a_listing_found_stale_offers_no_more_batches_until_the_next_is_heard() {
    // Builds root again what a listing called dead, hundreds of roots
    // reaching into a batch within minutes, so on 28 October batch after
    // batch from one listing was refused, each a whole look for roots,
    // while the listing that would cure it waited on the lock they took.
    let store = Store::new();
    for n in 0..4 {
        let path = store.dead(&format!("{n}-held"), 10.0 * DAY, 4096);
        fs::write(path.join("held"), "").unwrap();
    }
    let free = store.dead("zzz-free", 10.0 * DAY, 4096);
    fs::write(store.root.join("listing-slow"), "").unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| batches_begun(store) > 0);
    // Processes holding paths come and go all the while, each a reason to
    // collect again.
    for pid in 9000..9020 {
        let file = store.root.join("temproots").join(pid.to_string());
        fs::write(&file, "").unwrap();
        guard.step(&tick(), &CALM).unwrap();
        fs::remove_file(&file).unwrap();
        guard.step(&tick(), &CALM).unwrap();
    }
    let begun = batches_begun(&store);
    fs::remove_file(store.root.join("listing-slow")).unwrap();
    assert_eq!(begun, 1, "{:?}", store.log());
    assert!(free.exists(), "refused with the batch");
}

#[test]
fn a_pass_offers_nix_no_batch_after_one_is_found_stale() {
    // Builds root again what a listing called dead, so on 28 October a
    // pass that met it in one batch went on to meet it in every other,
    // 37 deletes a minute each refused whole.
    let store = Store::new();
    // Largest first, so the four largest fill the first batch of 8 GiB.
    for n in 0..8u64 {
        let path = store.dead(&format!("held-{n}"), 10.0 * DAY, (2 << 30) + n);
        fs::write(path.join("held"), "").unwrap();
    }
    fs::write(store.root.join("listing-slow"), "").unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| {
        store.log().iter().any(|line| line.starts_with("delete"))
    });
    for _ in 0..10 {
        guard.step(&tick(), &CALM).unwrap();
    }
    let log = store.log();
    fs::remove_file(store.root.join("listing-slow")).unwrap();
    let later = |line: &&String| (0..4).any(|n| line.contains(&format!("/held-{n}")));
    assert!(
        !log.iter()
            .filter(|line| line.starts_with("delete"))
            .any(|line| later(&line)),
        "{log:?}"
    );
}

/// The deletes that began a batch, naming all five of its paths: each try
/// after a refusal leaves the refused path out.
fn batches_begun(store: &Store) -> usize {
    store
        .log()
        .iter()
        .filter(|line| line.starts_with("delete") && line.split(' ').count() == 6)
        .count()
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

/// How many times the store has been listed.
fn lists(store: &Store) -> usize {
    store
        .log()
        .iter()
        .filter(|line| line.starts_with("list"))
        .count()
}

/// When the listing the guard last kept began, once it has kept one.
fn kept(store: &Store) -> Option<f64> {
    let text = fs::read_to_string(store.root.join("state/garbage.json")).ok()?;
    serde_json::from_str::<serde_json::Value>(&text).ok()?["listed_at"].as_f64()
}

/// Steps `guard` at `seconds` past now until `done` holds of the store and
/// that moment, as the listing runs apart. A machine busy with other
/// builds can take a while to start one, so this waits on what it sees.
fn step_later_until(
    store: &Store,
    guard: &mut Guard,
    seconds: f64,
    senses: &Senses,
    done: impl Fn(&Store, f64) -> bool,
) {
    let at = Tick {
        wall: now() + seconds,
        ..tick()
    };
    let started = Instant::now();
    while !done(store, at.wall) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{seconds} s on: {:?}",
            store.log()
        );
        guard.step(&at, senses).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Steps `guard` at `seconds` past now until it keeps a listing begun then.
/// JSON can bring a moment back a hair off, so near enough is the same.
fn listed_later(store: &Store, guard: &mut Guard, seconds: f64, senses: &Senses) {
    step_later_until(store, guard, seconds, senses, |store, at| {
        kept(store).is_some_and(|kept| (kept - at).abs() < 0.001)
    });
}

/// Steps `guard` at `seconds` past now long enough for a listing begun to
/// say so. That it said nothing is borne out by the next `listed_later`,
/// which waits for a listing begun at its own moment rather than this one.
fn step_later(guard: &mut Guard, seconds: f64, senses: &Senses) {
    let at = Tick {
        wall: now() + seconds,
        ..tick()
    };
    for _ in 0..10 {
        guard.step(&at, senses).unwrap();
        std::thread::sleep(Duration::from_millis(30));
    }
}

const BUSY: Senses = Senses {
    pressure: Pressure {
        io: disk_guard::moments::Stall {
            some: 50.0,
            full: 0.0,
        },
        ..CALM.pressure
    },
    ..CALM
};

#[test]
fn a_healthy_store_is_listed_again_six_hours_on_in_a_spare_moment() {
    // Upkeep comes every six hours and takes what the listing holds, so a
    // listing a day old misses a day of builds.
    let store = Store::new();
    store.dead("aaa-young", 3600.0, 4096);
    let mut guard = store.guard(store.config("0%", "0%"));
    listed_later(&store, &mut guard, 0.0, &CALM);
    assert_eq!(lists(&store), 1);
    step_later(&mut guard, 5.0 * 3600.0, &CALM);
    assert_eq!(lists(&store), 1, "five hours on");
    listed_later(&store, &mut guard, 7.0 * 3600.0, &CALM);
    assert_eq!(lists(&store), 2, "seven hours on");
}

#[test]
fn a_listing_is_taken_again_six_hours_on_however_busy_the_machine() {
    // A machine that builds all day is never calm, and its upkeep needs a
    // listing as fresh as a calm one's.
    let store = Store::new();
    store.dead("aaa-young", 3600.0, 4096);
    let mut guard = store.guard(store.config("0%", "0%"));
    listed_later(&store, &mut guard, 0.0, &CALM);
    step_later(&mut guard, 5.0 * 3600.0, &BUSY);
    assert_eq!(lists(&store), 1, "five hours on");
    listed_later(&store, &mut guard, 7.0 * 3600.0, &BUSY);
    assert_eq!(lists(&store), 2, "seven hours on");
    assert_eq!(store.log()[1], "list 5", "at idle priority");
}

#[test]
fn a_listing_kept_by_names_is_listed_again() {
    // As the guard kept them before it held references by place.
    let store = Store::new();
    store.dead("aaa-young", 3600.0, 4096);
    let old = format!(
        r#"{{"listed_at":{},"dead":[{{"path":"/nix/store/x","registered":1.0,"size":1,"references":[]}}]}}"#,
        now()
    );
    fs::write(store.root.join("state/garbage.json"), old).unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| {
        store.log().iter().any(|line| line.starts_with("list"))
    });
}

#[test]
fn a_listing_that_failed_under_pressure_is_tried_again_within_minutes() {
    let store = Store::new();
    store.dead("aaa-old", 10.0 * DAY, 4096);
    fs::write(store.root.join("listing-broken"), "").unwrap();
    let mut guard = store.guard(store.config("100%", "0%"));
    step_later_until(&store, &mut guard, 0.0, &CALM, |store, _| lists(store) == 1);
    fs::remove_file(store.root.join("listing-broken")).unwrap();
    step_later(&mut guard, 60.0, &CALM);
    assert_eq!(lists(&store), 1, "not at once: a listing costs the disk");
    // Kept, so this one listed, and listed six minutes on, not one.
    listed_later(&store, &mut guard, 6.0 * 60.0, &CALM);
    assert_eq!(lists(&store), 2, "{:?}", store.log());
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

/// A user daemon's config watching the store, at `urgent` and below.
fn user_config(store: &Store, urgent: &str) -> Config {
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user",
             "bands": {{ "pressure": "{urgent}", "urgent": "{urgent}", "critical": "0%",
                        "target": "{urgent}" }},
             "nix": {{ "bin": "{0}/bin/nix", "store": "{0}/store",
                      "profiles": "{0}/profiles" }},
             "sources": {{}} }}"#,
        store.root.display()
    ))
    .unwrap()
}

#[test]
fn a_user_daemon_with_no_system_daemon_collects_garbage_itself_from_urgent() {
    let store = Store::new();
    store
        .guard(user_config(&store, "0%"))
        .step(&tick(), &CALM)
        .unwrap();
    assert!(!store.gc_ran(), "healthy: {:?}", store.log());
    let mut guard = store.guard(user_config(&store, "100%"));
    guard.step(&tick(), &CALM).unwrap();
    let log = store.log();
    let gc = format!("gc --max 8589934592 {}", crate::commands::guards_policy());
    assert!(log.contains(&gc), "{log:?}");
}

#[test]
fn a_user_daemon_collects_itself_while_the_system_daemon_is_down() {
    let store = Store::new();
    let mut guard = Guard::new(
        crate::unpaced(user_config(&store, "100%")),
        &Places {
            state: store.root.join("state"),
            status: None,
            processes: store.root.join("proc"),
            system: Some(store.root.join("nobody.socket")),
            running: Vec::new(),
        },
    )
    .unwrap();
    guard.step(&tick(), &CALM).unwrap();
    assert!(store.gc_ran(), "{:?}", store.log());
}

#[test]
fn a_user_daemon_leaves_garbage_to_a_system_daemon_that_answers() {
    let store = Store::new();
    let sockets = crate::daemon::socket_dir();
    let socket = sockets.path().join("system.socket");
    let _heard = crate::daemon::fake_system(&socket, false);
    let mut guard = Guard::new(
        crate::unpaced(user_config(&store, "100%")),
        &Places {
            state: store.root.join("state"),
            status: None,
            processes: store.root.join("proc"),
            system: Some(socket),
            running: Vec::new(),
        },
    )
    .unwrap();
    guard.step(&tick(), &CALM).unwrap();
    assert!(!store.gc_ran(), "{:?}", store.log());
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

/// What the moment is like on a desktop set to save power.
const SAVING: Senses = Senses {
    saver: true,
    ..CALM
};

/// Steps `guard` a few times, with time for anything it started apart.
fn steps(guard: &mut Guard, senses: &Senses) {
    for _ in 0..5 {
        guard.step(&tick(), senses).unwrap();
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Steps `guard` and then waits long enough for a run it started apart
/// to have said so even on a busy machine, for checking none did.
fn steps_then_wait(guard: &mut Guard, senses: &Senses) {
    steps(guard, senses);
    std::thread::sleep(Duration::from_millis(500));
}

#[test]
fn a_healthy_store_is_linked_in_a_spare_moment_and_then_left_a_week() {
    let store = Store::new();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| !store.deduped().is_empty());
    steps_then_wait(&mut guard, &CALM);
    assert_eq!(store.linked(), ["optimise 5"], "once, at idle priority");
    let deduped = store.deduped();
    assert_eq!(deduped.len(), 1);
    assert_eq!(deduped[0]["outcome"], "reclaimed");
    assert_eq!(deduped[0]["size"], 1u64 << 30);
    assert_eq!(
        deduped[0]["path"],
        store.root.join("store").to_str().unwrap()
    );
}

#[test]
fn linking_is_due_again_a_week_after_the_last_run() {
    let lately = Store::new();
    lately.remember_deduped(DAY);
    let mut guard = lately.guard(lately.config("0%", "0%"));
    lately.step_until(&mut guard, |store| {
        store.root.join("state/garbage.json").exists()
    });
    steps_then_wait(&mut guard, &CALM);
    assert!(lately.linked().is_empty(), "{:?}", lately.linked());

    let long_ago = Store::new();
    long_ago.remember_deduped(8.0 * DAY);
    let mut guard = long_ago.guard(long_ago.config("0%", "0%"));
    long_ago.step_until(&mut guard, |store| !store.deduped().is_empty());
    assert_eq!(long_ago.linked(), ["optimise 5"]);
}

#[test]
fn linking_waits_for_a_spare_moment_and_a_healthy_store() {
    let saving = Store::new();
    let mut guard = saving.guard(saving.config("0%", "0%"));
    steps_then_wait(&mut guard, &SAVING);
    assert!(saving.linked().is_empty(), "{:?}", saving.linked());

    let pressed = Store::new();
    let mut guard = pressed.guard(pressed.config("100%", "0%"));
    pressed.step_until(&mut guard, |store| {
        store.root.join("state/garbage.json").exists()
    });
    steps_then_wait(&mut guard, &CALM);
    assert!(pressed.linked().is_empty(), "{:?}", pressed.linked());
}

#[test]
fn a_run_stops_when_the_moment_ends_and_isnt_counted_done() {
    let store = Store::new();
    fs::write(store.root.join("slow"), "").unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| !store.linked().is_empty());
    let started = Instant::now();
    while !store.linked().contains(&"stopped".to_owned()) {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            store.linked()
        );
        steps(&mut guard, &SAVING);
    }
    // The run is at idle priority, so on a busy machine it can be a while
    // between saying it stopped and exiting, which is when it's journaled.
    while store.deduped().is_empty() {
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "{:?}",
            store.linked()
        );
        steps(&mut guard, &SAVING);
    }
    let stopped = store.deduped();
    assert_eq!(stopped.len(), 1, "{stopped:?}");
    assert_eq!(stopped[0]["outcome"], "kept", "{stopped:?}");

    fs::remove_file(store.root.join("slow")).unwrap();
    store.step_until(&mut guard, |store| store.deduped().len() == 2);
    assert_eq!(store.deduped()[1]["outcome"], "reclaimed");
    assert_eq!(store.linked(), ["optimise 5", "stopped", "optimise 5"]);
}

#[test]
fn a_failed_run_is_journaled_and_not_tried_again_at_once() {
    let store = Store::new();
    fs::write(store.root.join("slow-broken"), "").unwrap();
    let mut guard = store.guard(store.config("0%", "0%"));
    store.step_until(&mut guard, |store| !store.deduped().is_empty());
    steps_then_wait(&mut guard, &CALM);
    let failed = store.deduped();
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0]["outcome"], "failed");
    assert!(
        failed[0]["reason"]
            .as_str()
            .unwrap()
            .contains("database is locked"),
        "{failed:?}"
    );
    assert_eq!(store.linked(), ["optimise 5"]);
}

#[test]
fn a_healthy_disk_keeps_no_garbage_for_a_rebuild() {
    // Kept as a cache, garbage under three days old held 34 GiB of the
    // 43 nothing held on a laptop whose disk its owner expected far emptier.
    let store = Store::new();
    let young = store.dead("aaa-young", 60.0, 4096);
    let mut guard = store.guard(store.config_as_given("0%", "0%"));
    store.step_until(&mut guard, |_| !young.exists());
}

#[test]
fn garbage_goes_once_the_process_holding_it_ends() {
    let store = Store::new();
    let built = store.dead("aaa-built", 60.0, 4096);
    let holding = store.root.join("temproots/4242");
    fs::write(&holding, format!("{}\0", built.display())).unwrap();
    let mut guard = store.guard(store.config_as_given("0%", "0%"));
    listed_later(&store, &mut guard, 0.0, &CALM);
    step_later(&mut guard, 1.0, &CALM);
    assert!(built.exists(), "a running build's path stays");
    fs::remove_file(&holding).unwrap();
    // Long before the next upkeep, six hours on.
    step_later_until(&store, &mut guard, 10.0, &BUSY, |_, _| !built.exists());
}

#[test]
fn garbage_a_root_left_goes_within_a_minute() {
    let store = Store::new();
    let kept = store.dead("aaa-kept", 60.0, 4096);
    fs::write(kept.join("alive"), "").unwrap();
    let result = store.root.join("result");
    fs::write(&result, "").unwrap();
    fs::create_dir_all(store.root.join("gcroots/auto")).unwrap();
    std::os::unix::fs::symlink(&result, store.root.join("gcroots/auto/1")).unwrap();
    let mut guard = store.guard(store.config_as_given("0%", "0%"));
    listed_later(&store, &mut guard, 0.0, &CALM);
    step_later(&mut guard, 1.0, &CALM);
    assert!(kept.exists());
    // Its owner deletes the build's link, and nothing holds it now.
    fs::remove_file(&result).unwrap();
    fs::remove_file(kept.join("alive")).unwrap();
    step_later_until(&store, &mut guard, 45.0, &BUSY, |_, _| !kept.exists());
    assert_eq!(lists(&store), 2, "{:?}", store.log());
}

#[test]
fn urgent_without_a_listing_collects_by_nix_store_gc_at_once() {
    let store = Store::new();
    let mut guard = store.guard(store.config("100%", "100%"));
    guard.step(&tick(), &CALM).unwrap();
    let log = store.log();
    let gc = format!("gc --max 8589934592 {}", crate::commands::guards_policy());
    assert!(
        log.contains(&gc),
        "a step of 8 GiB, at the guard's priority: {log:?}"
    );
}

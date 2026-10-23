use crate::discover::{CARGO_TAG, checkout, dir, file};
use disk_guard::activity::Process;
use disk_guard::band::Band;
use disk_guard::clock::Awake;
use disk_guard::config::Config;
use disk_guard::logs::{KEEP, LIMIT};
use disk_guard::plan::Class;
use disk_guard::replaced::Uses;
use disk_guard::snapshots::Boot;
use disk_guard::survey::{Action, Offer, World, survey};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const BOOT: Boot = Boot {
    wall: 0.0,
    ticks: 100.0,
};

fn now() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64()
}

fn config(sources: &str) -> Config {
    Config::parse(&format!(
        r#"{{ "version": 1, "scope": "user", "sources": {{ {sources} }} }}"#
    ))
    .unwrap()
}

fn offers(config: &Config, processes: &[Process], finished: &dyn Fn(&Path) -> bool) -> Vec<Offer> {
    offers_after(config, processes, finished, &BTreeMap::new())
}

fn offers_after(
    config: &Config,
    processes: &[Process],
    finished: &dyn Fn(&Path) -> bool,
    ran: &BTreeMap<String, f64>,
) -> Vec<Offer> {
    offers_using(config, processes, finished, ran, &Uses::default())
}

fn offers_using(
    config: &Config,
    processes: &[Process],
    finished: &dyn Fn(&Path) -> bool,
    ran: &BTreeMap<String, f64>,
    uses: &Uses,
) -> Vec<Offer> {
    let awake = Awake::default();
    let mut finished = |dir: &Path| finished(dir);
    let mut world = World {
        processes,
        me: std::process::id(),
        uid: unsafe { libc::geteuid() },
        boot: BOOT,
        awake: &awake,
        now: now(),
        finished: &mut finished,
        ran,
        uses,
        kept: Vec::new(),
    };
    let mut found = survey(config, &mut world);
    found.sort_by(|a, b| a.candidate.path.cmp(&b.candidate.path));
    found
}

fn find<'a>(offers: &'a [Offer], path: &Path) -> &'a Offer {
    offers
        .iter()
        .find(|offer| offer.candidate.path == path)
        .unwrap_or_else(|| {
            let paths: Vec<_> = offers.iter().map(|offer| &offer.candidate.path).collect();
            panic!("{} isn't offered among {paths:?}", path.display())
        })
}

fn real(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap()
}

/// A checkout with a built target and an install beside it.
fn src(root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let app = checkout(root, "app", ".");
    let target = dir(&app.join("target"));
    file(&target.join("CACHEDIR.TAG"), CARGO_TAG);
    file(
        &target.join("debug/deps/libapp-0123456789abcdef.rlib"),
        "rlib",
    );
    file(&app.join("package.json"), "{}");
    file(&app.join("package-lock.json"), "{}");
    let modules = dir(&app.join("node_modules/left-pad"));
    (app, target, modules.parent().unwrap().to_owned())
}

fn written_ago(path: &Path, seconds: f64) {
    use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
    let at = Timespec {
        tv_sec: crate::whole_seconds(now() - seconds),
        tv_nsec: 0,
    };
    let times = Timestamps {
        last_access: at,
        last_modification: at,
    };
    utimensat(CWD, path, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
}

const DAY: f64 = 86_400.0;

#[test]
fn an_older_build_is_offered_once_its_deps_is_watched() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, _) = src(&roots);
    let deps = target.join("debug/deps");
    for (name, ago) in [
        ("libapp-1111111111111111.rlib", 5.0 * DAY),
        ("libapp-1111111111111111.rmeta", 5.0 * DAY),
        ("libapp-0123456789abcdef.rlib", 0.0),
    ] {
        file(&deps.join(name), "built");
        written_ago(&deps.join(name), ago);
    }
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let replaced = deps.join("app-1111111111111111");
    let none = offers(&config, &[], &|_| false);
    assert!(
        none.iter().all(|offer| offer.candidate.path != replaced),
        "nothing watched it, so nothing knows it's unused"
    );
    let mut uses = Uses::default();
    uses.watching(&deps, now() - 3.0 * DAY);
    let found = offers_using(&config, &[], &|_| false, &BTreeMap::new(), &uses);
    let older = find(&found, &replaced);
    assert_eq!(older.candidate.class, Class::Replaced);
    assert!(
        (older.candidate.idle - 3.0 * DAY).abs() < 60.0,
        "{}",
        older.candidate.idle
    );
    assert_eq!(older.candidate.size, 10);
    assert_eq!(
        older.action,
        Action::Build {
            root: roots.clone(),
            target: target.clone()
        }
    );
    let newest = deps.join("app-0123456789abcdef");
    assert!(found.iter().all(|offer| offer.candidate.path != newest));
}

#[test]
fn targets_and_installs_are_offered_under_their_roots() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, modules) = src(&roots);
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{0}"] }},
           "installs": {{ "kind": "installs", "roots": ["{0}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|dir| dir == app);
    let built = find(&found, &target);
    assert_eq!(built.candidate.class, Class::Target);
    assert!(built.candidate.spent, "its checkout's work is finished");
    assert_eq!(built.candidate.checkout.as_deref(), Some(app.as_path()));
    assert_eq!(
        built.action,
        Action::Target {
            root: roots.clone()
        }
    );
    assert_eq!(built.device, disk_guard::pool::pool(&roots).unwrap());
    let installed = find(&found, &modules);
    assert_eq!(installed.candidate.class, Class::Install);
    assert_eq!(
        installed.action,
        Action::Entry {
            root: roots.clone()
        }
    );
    let unfinished = offers(&config, &[], &|_| false);
    assert!(!find(&unfinished, &target).candidate.spent);
}

#[test]
fn a_target_holding_only_what_reclaiming_keeps_is_not_offered() {
    // Reclaiming empties a target in place and keeps its benchmarks, so
    // offering it again would free nothing, pass after pass.
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let emptied = checkout(&roots, "emptied", ".");
    file(&emptied.join("Cargo.toml"), "");
    let emptied_target = dir(&emptied.join("target"));
    let benched = checkout(&roots, "benched", ".");
    file(&benched.join("Cargo.toml"), "");
    let benched_target = benched.join("target");
    file(
        &benched_target.join("criterion/report/index.html"),
        "a report",
    );
    let built = checkout(&roots, "built", ".");
    file(&built.join("Cargo.toml"), "");
    let built_target = built.join("target");
    file(&built_target.join("debug/app"), "a binary");
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|_| true);
    find(&found, &built_target);
    for target in [&emptied_target, &benched_target] {
        assert!(
            found.iter().all(|offer| offer.candidate.path != *target),
            "{} is offered",
            target.display()
        );
    }
}

#[test]
fn a_tree_a_build_runs_in_is_never_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (app, target, _) = src(&roots);
    let variant = dir(&target.join("debug/incremental/app-1a2b3c"));
    dir(&variant.join("s-abcdefghi-0123456-0123456789abcdef"));
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    assert!(
        offers(&config, &[], &|_| false)
            .iter()
            .any(|offer| offer.candidate.path == variant)
    );
    let cargo = Process {
        pid: 4242,
        name: "cargo".into(),
        exe: Some(PathBuf::from("/nix/store/abc-cargo/bin/cargo")),
        cmdline: vec![OsString::from("cargo"), OsString::from("build")],
        cwd: Some(app.clone()),
        readable: true,
        ..Default::default()
    };
    let found = offers(&config, &[cargo], &|_| false);
    assert!(
        !found
            .iter()
            .any(|offer| offer.candidate.path.starts_with(&target))
    );
}

#[test]
fn stale_incremental_variants_are_offered_as_variants() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let roots = dir(&home.join("src"));
    let (_, target, _) = src(&roots);
    let variant = dir(&target.join("debug/incremental/app-1a2b3c"));
    dir(&variant.join("s-abcdefghi-0123456-0123456789abcdef"));
    let config = config(&format!(
        r#""cargo": {{ "kind": "cargo", "roots": ["{}"] }}"#,
        roots.display()
    ));
    let found = offers(&config, &[], &|_| false);
    let offered = find(&found, &variant);
    assert_eq!(offered.candidate.class, Class::Variant);
    assert_eq!(
        offered.action,
        Action::Variant {
            root: roots.clone()
        }
    );
}

#[test]
fn caches_are_offered_whole_or_by_entry_with_their_rank_and_band() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let npm = dir(&home.join(".npm/_cacache/index-v5"));
    file(&npm.join("x"), "index");
    let go = dir(&home.join(".cache/go-build"));
    let entry = go.join("ab").join(format!("{}-d", "ab".repeat(32)));
    file(&entry, "output");
    file(&go.join("README"), "go");
    let config = config(&format!(
        r#""npm": {{ "kind": "cache", "path": "{}", "rank": 3 }},
           "go": {{ "kind": "cache", "path": "{}", "protocol": "go-build", "rank": 1, "from": "pressure" }},
           "gone": {{ "kind": "cache", "path": "{}" }}"#,
        home.join(".npm/_cacache").display(),
        go.display(),
        home.join(".cache/missing").display()
    ));
    let found = offers(&config, &[], &|_| false);
    assert_eq!(found.len(), 2);
    let whole = find(&found, &home.join(".npm/_cacache"));
    assert_eq!(
        whole.candidate.class,
        Class::Cache {
            rank: 3,
            from: Band::Urgent
        }
    );
    assert_eq!(
        whole.action,
        Action::Entry {
            root: home.join(".npm")
        }
    );
    assert!(whole.candidate.size >= 5);
    let by_entry = find(&found, &entry);
    assert_eq!(
        by_entry.candidate.class,
        Class::Cache {
            rank: 1,
            from: Band::Pressure
        }
    );
    assert_eq!(by_entry.action, Action::Entry { root: go.clone() });
}

#[test]
fn temp_leftovers_trash_and_scratch_are_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let temp = dir(&home.join("tmp"));
    let leftover = dir(&temp.join("tmp.Ab3dE9fGh1"));
    file(&leftover.join("f"), "x");
    let living = dir(&temp.join("tmp.Zz9yX8wV7u"));
    let trash = dir(&home.join("Trash"));
    file(&trash.join("files/old.txt"), "old");
    file(
        &trash.join("info/old.txt.trashinfo"),
        "[Trash Info]\nPath=/x\nDeletionDate=2020-01-01T00:00:00\n",
    );
    let scratch = dir(&home.join("scratch"));
    let run = dir(&scratch.join("run-1"));
    file(&run.join("out"), "x");
    let config = config(&format!(
        r#""temp": {{ "kind": "temp", "path": "{}" }},
           "trash": {{ "kind": "trash", "path": "{}" }},
           "scratch": {{ "kind": "scratch", "path": "{}", "healthy": "30d" }}"#,
        temp.display(),
        trash.display(),
        scratch.display()
    ));
    let shell = Process {
        pid: 77,
        env: vec![(OsString::from("TMPDIR"), living.clone().into_os_string())],
        readable: true,
        ..Default::default()
    };
    let found = offers(&config, &[shell], &|_| false);
    let offered = find(&found, &leftover);
    assert_eq!(offered.candidate.class, Class::Temp);
    assert_eq!(offered.action, Action::Entry { root: temp.clone() });
    assert!(
        !found.iter().any(|offer| offer.candidate.path == living),
        "a shell's TMPDIR"
    );
    let trashed = find(&found, &trash.join("files/old.txt"));
    assert_eq!(trashed.candidate.class, Class::Trash);
    assert_eq!(
        trashed.action,
        Action::Trash {
            trash: trash.clone()
        }
    );
    let scratched = find(&found, &run);
    assert_eq!(
        scratched.candidate.class,
        Class::Scratch {
            healthy: Some(30.0 * 86_400.0)
        }
    );
    assert_eq!(
        scratched.action,
        Action::Entry {
            root: scratch.clone()
        }
    );
}

#[test]
fn orphaned_snapshots_and_finished_workspaces_are_offered() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let snapshots = dir(&home.join("snapshots"));
    let orphan = dir(&snapshots.join("repo_1_4242"));
    dir(&snapshots.join("repo_1_77"));
    let quests = dir(&home.join("quests"));
    file(
        &quests.join("QEST-20200101-ABCDEF/README.md"),
        "---\nid: QEST-20200101-ABCDEF\nstatus: concluded\nupdated: 2020-01-01\n---\n# Done\n",
    );
    let workspaces = dir(&home.join("workspaces"));
    let workspace = dir(&workspaces.join("QEST-20200101-ABCDEF"));
    file(&workspace.join("notes"), "x");
    let config = config(&format!(
        r#""snapshots": {{ "kind": "snapshots", "path": "{}", "pattern": "^[a-z]+_\\d+_(?P<pid>\\d+)$", "command": "snapd" }},
           "workspaces": {{ "kind": "workspaces", "path": "{}", "quests": "{}" }}"#,
        snapshots.display(),
        workspaces.display(),
        quests.display()
    ));
    let snapd = Process {
        pid: 77,
        exe: Some(PathBuf::from("/usr/bin/snapd")),
        cmdline: vec![OsString::from("snapd")],
        readable: true,
        ..Default::default()
    };
    let found = offers(&config, &[snapd], &|_| false);
    let orphaned = find(&found, &orphan);
    assert_eq!(orphaned.candidate.class, Class::Snapshot);
    assert_eq!(
        orphaned.action,
        Action::Entry {
            root: snapshots.clone()
        }
    );
    assert!(
        !found
            .iter()
            .any(|offer| offer.candidate.path == snapshots.join("repo_1_77"))
    );
    let done = find(&found, &workspace);
    assert_eq!(done.candidate.class, Class::Workspace);
    assert_eq!(
        done.action,
        Action::Workspace {
            root: workspaces.clone()
        }
    );
}

#[test]
fn producers_are_offered_where_they_free_idle_since_they_last_ran() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let trees = dir(&home.join("trees"));
    let config = config(&format!(
        r#""tec": {{ "kind": "producer", "command": ["/bin/tec", "cache", "gc"],
                    "path": "{0}", "every": "6h", "timeout": "5m" }},
           "fresh": {{ "kind": "producer", "command": ["/bin/fresh"],
                      "path": "{0}", "from": "urgent", "timeout": "1m" }}"#,
        trees.display()
    ));
    let ran = [("fresh".to_owned(), now() - 100.0)].into();
    let found = offers_after(&config, &[], &|_| false, &ran);
    assert_eq!(found.len(), 2);
    let tec = find(&found, &PathBuf::from(format!("{}#tec", trees.display())));
    assert_eq!(
        tec.candidate.class,
        Class::Producer {
            every: Some(6.0 * 3600.0),
            from: None
        }
    );
    // It never ran.
    crate::assert_exactly(tec.candidate.idle, f64::INFINITY);
    assert_eq!(tec.candidate.size, 0, "what it frees is measured");
    assert_eq!(
        tec.action,
        Action::Run {
            root: trees.clone(),
            source: "tec".to_owned(),
            argv: vec!["/bin/tec".into(), "cache".into(), "gc".into()],
            timeout: Duration::from_secs(300),
        }
    );
    assert_eq!(tec.device, disk_guard::pool::pool(&trees).unwrap());
    let fresh = find(&found, &PathBuf::from(format!("{}#fresh", trees.display())));
    assert!(
        (fresh.candidate.idle - 100.0).abs() < 5.0,
        "{}",
        fresh.candidate.idle
    );
}

#[test]
fn podmans_prunes_are_offered_where_its_storage_lies() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let storage = dir(&home.join("containers/storage"));
    let config = config(&format!(
        r#""podman": {{ "kind": "podman", "command": "/bin/podman", "path": "{}" }}"#,
        storage.display()
    ));
    let ran = [("podman".to_owned(), now() - 100.0)].into();
    let found = offers_after(&config, &[], &|_| false, &ran);
    assert_eq!(found.len(), 2);
    let named = |suffix: &str| PathBuf::from(format!("{}#{suffix}", storage.display()));
    let dangling = find(&found, &named("podman"));
    assert_eq!(dangling.candidate.class, Class::Podman { unused: false });
    assert!((dangling.candidate.idle - 100.0).abs() < 5.0);
    let argv = |args: &[&str]| -> Vec<String> {
        std::iter::once("/bin/podman")
            .chain(args.iter().copied())
            .map(str::to_owned)
            .collect()
    };
    let run = |source: &str, args: &[&str]| Action::Run {
        root: storage.clone(),
        source: source.to_owned(),
        argv: argv(args),
        timeout: Duration::from_secs(300),
    };
    assert_eq!(
        dangling.action,
        run("podman", &["image", "prune", "--force", "--build-cache"])
    );
    let unused = find(&found, &named("podman:unused"));
    assert_eq!(unused.candidate.class, Class::Podman { unused: true });
    // Its last run is remembered apart from the build cache's, so it never ran.
    crate::assert_exactly(unused.candidate.idle, f64::INFINITY);
    assert_eq!(
        unused.action,
        run("podman:unused", &["image", "prune", "--force", "--all"])
    );
}

#[test]
fn logs_past_their_limit_are_offered_to_be_trimmed() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let logs = dir(&home.join("logs"));
    let sized = |name: &str, len: u64| {
        let path = logs.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // Sparse, so the test writes nothing.
        fs::File::create(&path).unwrap().set_len(len).unwrap();
        path
    };
    let big = sized("dev/run.log", LIMIT + 1);
    sized("small.log", LIMIT);
    sized("big.txt", LIMIT + 1);
    let config = config(&format!(
        r#""logs": {{ "kind": "logs", "path": "{}" }}"#,
        logs.display()
    ));
    let found = offers(&config, &[], &|_| false);
    assert_eq!(found.len(), 1, "{found:?}");
    let offered = find(&found, &big);
    assert_eq!(offered.candidate.class, Class::Log);
    assert_eq!(
        offered.candidate.size,
        LIMIT + 1 - KEEP,
        "what trimming drops"
    );
    assert_eq!(offered.action, Action::Trim { root: logs.clone() });
}

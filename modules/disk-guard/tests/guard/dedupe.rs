//! Linking the store's identical files together: what Nix says it freed,
//! when a run is due, and a run confined to a scope of its own.

use crate::garbage::script;
use disk_guard::dedupe::{Deduped, due, freed, optimise};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

const DAY: f64 = 86400.0;
const GIB: u64 = 1 << 30;

#[test]
fn nix_says_what_linking_freed() {
    let said = "warning: something\n1.5 GiB freed by hard-linking 1234 files\n";
    assert_eq!(freed(said), Some(3 * GIB / 2));
    assert_eq!(freed("0.0 KiB freed by hard-linking 0 files\n"), Some(0));
    assert_eq!(
        freed("812.3 MiB freed by hard-linking 9 files"),
        Some(851_758_285)
    );
    assert_eq!(freed("error: interrupted by the user\n"), None);
}

#[test]
fn a_run_is_due_weekly_or_monthly_after_one_that_freed_little() {
    let now = 100.0 * DAY;
    let every = 7.0 * DAY;
    let total = 1000 * GIB;
    let ran = |days_ago: f64, freed: u64| Deduped {
        at: now - days_ago * DAY,
        freed,
    };
    assert!(due(None, now, every, total), "never run");
    assert!(!due(Some(&ran(6.0, 50 * GIB)), now, every, total));
    assert!(due(Some(&ran(8.0, 50 * GIB)), now, every, total));
    assert!(
        !due(Some(&ran(8.0, 9 * GIB)), now, every, total),
        "under 1% of the filesystem waits a month"
    );
    assert!(due(Some(&ran(31.0, 9 * GIB)), now, every, total));
}

/// A `systemd-run` that notes what it was asked and runs the command after
/// `--`, as its scope does, and a `nix-store` whose body is `optimise`.
fn fakes(dir: &Path, optimise: &str) -> (PathBuf, PathBuf) {
    let log = dir.join("log");
    let run = dir.join("systemd-run");
    script(
        &run,
        &format!(
            "[ \"$1\" = --scope ] || exit 9\n\
             echo \"$*\" > {}\n\
             while [ \"$1\" != -- ]; do shift; done; shift\n\
             exec \"$@\"\n",
            log.display()
        ),
    );
    script(
        &dir.join("nix-store"),
        &format!("[ \"$*\" = --optimise ] || exit 9\n{optimise}"),
    );
    (run, dir.join("nix"))
}

#[test]
fn a_run_is_confined_to_a_scope_of_its_own() {
    let dir = tempfile::tempdir().unwrap();
    let policy = dir.path().join("policy");
    let (run, nix) = fakes(
        dir.path(),
        &format!(
            "cut -d' ' -f41 /proc/$$/stat > {}\n\
             echo '2.0 GiB freed by hard-linking 7 files' >&2\n",
            policy.display()
        ),
    );
    let stop = AtomicBool::new(false);
    assert_eq!(optimise(&run, &nix, &stop), Ok(Some(2 * GIB)));
    let idle = libc::SCHED_IDLE.to_string();
    assert_eq!(
        fs::read_to_string(&policy).unwrap().trim(),
        idle,
        "at idle priority"
    );
    let asked = fs::read_to_string(dir.path().join("log")).unwrap();
    assert_eq!(
        asked.trim(),
        format!(
            "--scope --quiet --collect --description=disk-guard: linking the Nix store's \
             identical files -p MemoryHigh=1G -p MemoryMax=2G -p CPUWeight=idle \
             -p IOWeight=1 -- {}/nix-store --optimise",
            dir.path().display()
        )
    );
}

#[test]
fn a_stopped_run_frees_nothing_it_knows_of() {
    let dir = tempfile::tempdir().unwrap();
    let (run, nix) = fakes(dir.path(), "sleep 30 & wait\n");
    let stop = AtomicBool::new(false);
    let started = Instant::now();
    let stopped = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            stop.store(true, Ordering::Relaxed);
        });
        optimise(&run, &nix, &stop)
    });
    assert_eq!(stopped, Ok(None));
    assert!(started.elapsed() < Duration::from_secs(10));
}

#[test]
fn a_failed_run_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (run, nix) = fakes(dir.path(), "echo 'error: database is locked' >&2; exit 1\n");
    let stop = AtomicBool::new(false);
    let failed = optimise(&run, &nix, &stop).unwrap_err();
    assert!(failed.contains("database is locked"), "{failed}");
}

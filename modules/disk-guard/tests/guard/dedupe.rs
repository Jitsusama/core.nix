//! Linking the store's identical files together: what Nix says it freed,
//! when a run is due, and a run confined to a scope of its own.

use crate::garbage::script;
use disk_guard::config::Config;
use disk_guard::dedupe::{Deduped, due, freed, optimise};
use disk_guard::hours::Hours;
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

#[test]
fn a_run_keeps_to_its_hours_of_the_night_or_any_named() {
    let night = Hours::parse("01:30-07:00").unwrap();
    assert!(!night.hold(89), "01:29");
    assert!(night.hold(90), "01:30");
    assert!(night.hold(419), "06:59");
    assert!(!night.hold(420), "07:00 ends them");
    assert!(
        !night.hold(20 * 60 + 6),
        "20:06, when one began on 11 November"
    );
    let late = Hours::parse("23:00-06:00").unwrap();
    assert!(
        late.hold(23 * 60 + 30) && late.hold(0) && late.hold(5 * 60 + 59),
        "23:30, midnight and 05:59 fall within 23:00-06:00"
    );
    assert!(
        !late.hold(6 * 60) && !late.hold(12 * 60),
        "past midnight and back"
    );
    for bad in [
        "1:30-07:00",
        "01:30",
        "24:00-07:00",
        "01:60-07:00",
        "07:00-07:00",
    ] {
        assert!(Hours::parse(bad).is_err(), "{bad} is refused");
    }
    let config =
        Config::parse(r#"{"version": 1, "scope": "system", "boot": "/boot", "sources": {}}"#)
            .unwrap();
    assert_eq!(
        config.system.unwrap().dedupe.hours,
        Some(night),
        "the night unless told"
    );
    let any = Config::parse(
        r#"{"version": 1, "scope": "system", "boot": "/boot",
            "dedupe": {"hours": null}, "sources": {}}"#,
    );
    assert_eq!(any.unwrap().system.unwrap().dedupe.hours, None);
    let wrong = Config::parse(
        r#"{"version": 1, "scope": "system", "boot": "/boot",
            "dedupe": {"hours": "night"}, "sources": {}}"#,
    );
    let refused = wrong.unwrap_err();
    assert!(refused.contains("HH:MM"), "{refused}");
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
#[cfg(target_os = "linux")]
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
    assert_eq!(optimise(Some(&run), &nix, &stop), Ok(Some(2 * GIB)));
    let idle = libc::SCHED_IDLE.to_string();
    assert_eq!(
        std::fs::read_to_string(&policy).unwrap().trim(),
        idle,
        "at idle priority"
    );
    let asked = std::fs::read_to_string(dir.path().join("log")).unwrap();
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
    // A run that would take 15 minutes, so a stop that ended it is told
    // from one that didn't by minutes, not seconds: on 11 November a 10 s
    // bound failed on a Mac building at a load average of 66.
    let (run, nix) = fakes(dir.path(), "sleep 900 & wait\n");
    let stop = AtomicBool::new(false);
    let started = Instant::now();
    let stopped = std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(Duration::from_millis(200));
            stop.store(true, Ordering::Relaxed);
        });
        optimise(Some(&run), &nix, &stop)
    });
    assert_eq!(stopped, Ok(None));
    assert!(
        started.elapsed() < Duration::from_mins(5),
        "a stopped run went on for {:?}",
        started.elapsed()
    );
}

#[test]
fn a_failed_run_says_why() {
    let dir = tempfile::tempdir().unwrap();
    let (run, nix) = fakes(dir.path(), "echo 'error: database is locked' >&2; exit 1\n");
    let stop = AtomicBool::new(false);
    let failed = optimise(Some(&run), &nix, &stop).unwrap_err();
    assert!(failed.contains("database is locked"), "{failed}");
}

#[test]
fn a_run_with_nothing_to_confine_it_runs_alone() {
    // As on a Mac, which has no scopes.
    let dir = tempfile::tempdir().unwrap();
    let (_, nix) = fakes(
        dir.path(),
        "echo '1.0 GiB freed by hard-linking 3 files' >&2\n",
    );
    let stop = AtomicBool::new(false);
    assert_eq!(optimise(None, &nix, &stop), Ok(Some(GIB)));
    assert!(!dir.path().join("log").exists(), "systemd-run wasn't asked");
}

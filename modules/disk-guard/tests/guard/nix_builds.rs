use disk_guard::activity::Process;
use disk_guard::clock::Awake;
use disk_guard::nix_builds::{Build, candidate, find, orphaned};
use disk_guard::plan::Class;
use disk_guard::snapshots::{Boot, Snapshot};
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

const BOOT: Boot = Boot {
    wall: 1_000.0,
    ticks: 100.0,
};

fn build(kept: Option<f64>) -> Build {
    Build {
        snapshot: Snapshot {
            path: PathBuf::from("/nix/var/nix/builds/nix-42-7"),
            pid: 42,
            born: 3_000.0,
        },
        kept,
    }
}

fn running(pid: u32, exe: &str) -> Process {
    Process {
        pid,
        start_ticks: 100_000,
        exe: Some(PathBuf::from(exe)),
        cmdline: vec![OsString::from(exe)],
        readable: true,
        ..Default::default()
    }
}

#[test]
fn build_directories_are_found_and_kept_failures_told_apart() {
    let root = tempfile::tempdir().unwrap();
    for (name, mode) in [
        ("nix-25486-2662119134", 0o755),
        ("nix-387614-3268420062", 0o700),
        ("nix-1-x", 0o700),
        ("other", 0o700),
    ] {
        let path = root.path().join(name);
        fs::create_dir(&path).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    }
    let mut found = find(root.path());
    found.sort_by_key(|build| build.snapshot.pid);
    let summary: Vec<(u32, bool)> = found
        .iter()
        .map(|build| (build.snapshot.pid, build.kept.is_some()))
        .collect();
    assert_eq!(summary, [(25486, true), (387614, false)]);
}

#[test]
fn a_build_any_nix_process_runs_is_held() {
    for exe in [
        "/run/current-system/sw/bin/nix-daemon",
        "/nix/store/abc-nix/bin/nix",
        "/nix/store/abc-nix/bin/nix-build",
    ] {
        assert!(
            !orphaned(&build(None), &[running(42, exe)], BOOT),
            "{exe} didn't hold it"
        );
    }
    assert!(orphaned(&build(None), &[running(42, "/usr/bin/vim")], BOOT));
    assert!(orphaned(&build(None), &[], BOOT));
}

#[test]
fn an_abandoned_build_is_waste() {
    let made = candidate(&build(None), 7, &Awake::default(), 5_000.0);
    assert_eq!(made.class, Class::NixBuild);
    assert_eq!(made.idle, 2_000.0);
    assert_eq!(made.size, 7);
}

#[test]
fn a_failure_kept_on_purpose_ages_as_scratch_from_when_it_was_kept() {
    let made = candidate(&build(Some(4_000.0)), 7, &Awake::default(), 5_000.0);
    assert_eq!(
        made.class,
        Class::Scratch {
            healthy: Some(7.0 * 86_400.0)
        }
    );
    assert_eq!(made.idle, 1_000.0);
}

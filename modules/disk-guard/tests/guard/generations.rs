use disk_guard::generations::{Boot, entries, expire, expired, profiles};
use rustix::fs::{AtFlags, CWD, Timespec, Timestamps, utimensat};
use std::collections::BTreeSet;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

const DAY: f64 = 86400.0;
const NOW: f64 = 100.0 * DAY;

/// A profile `name` in `dir` with a generation for each `(number, days
/// ago)`, the current one being `current`, each pointing at its own system.
fn profile(dir: &Path, name: &str, generations: &[(u32, f64)], current: u32) -> PathBuf {
    profile_at(dir, name, generations, current, NOW)
}

/// The same, its ages counted back from `now`.
pub(crate) fn profile_at(
    dir: &Path,
    name: &str,
    generations: &[(u32, f64)],
    current: u32,
    now: f64,
) -> PathBuf {
    for &(number, ago) in generations {
        let link = dir.join(format!("{name}-{number}-link"));
        symlink(format!("/nix/store/{number}-system"), &link).unwrap();
        let at = Timespec {
            tv_sec: crate::whole_seconds(now - ago * DAY),
            tv_nsec: 0,
        };
        let times = Timestamps {
            last_access: at,
            last_modification: at,
        };
        utimensat(CWD, &link, &times, AtFlags::SYMLINK_NOFOLLOW).unwrap();
    }
    symlink(format!("{name}-{current}-link"), dir.join(name)).unwrap();
    dir.join(name)
}

#[test]
fn profiles_are_read_with_their_generations_and_the_current_one() {
    let dir = tempfile::tempdir().unwrap();
    profile(dir.path(), "system", &[(1, 9.0), (2, 8.0), (10, 1.0)], 10);
    fs::create_dir_all(dir.path().join("per-user/root")).unwrap();
    profile(&dir.path().join("per-user/root"), "profile", &[(3, 2.0)], 3);
    // A profile pointing straight into the store has no generations.
    symlink("/nix/store/abc-nix", dir.path().join("default")).unwrap();
    let mut found = profiles(dir.path()).unwrap();
    found.sort_by(|a, b| a.path.cmp(&b.path));
    assert_eq!(found.len(), 2, "{found:?}");
    let root = &found[0];
    assert_eq!(root.path, dir.path().join("per-user/root/profile"));
    let system = &found[1];
    assert_eq!(system.name, "system");
    assert_eq!(system.current, Some(10));
    let numbers: Vec<u32> = system.generations.iter().map(|g| g.number).collect();
    assert_eq!(numbers, [1, 2, 10], "oldest first, by number not by name");
    assert_eq!(
        system.generations[0].target,
        PathBuf::from("/nix/store/1-system")
    );
    crate::assert_exactly(system.generations[0].made, NOW - 9.0 * DAY);
}

#[test]
fn boot_entries_name_the_generations_they_boot() {
    let dir = tempfile::tempdir().unwrap();
    let linux = dir.path().join("EFI/Linux");
    let loader = dir.path().join("loader/entries");
    fs::create_dir_all(&linux).unwrap();
    fs::create_dir_all(&loader).unwrap();
    for name in [
        "nixos-generation-84-abcdefghijklmnop.efi",
        "nixos-generation-85-specialisation-quiet-abcdefgh.efi",
        "nixos-generation-86-qrstuvwx+3-1.efi",
        "other-loader.efi",
    ] {
        fs::write(linux.join(name), "").unwrap();
    }
    fs::write(loader.join("nixos-generation-12.conf"), "").unwrap();
    fs::write(loader.join("nixos-generation-13+2.conf"), "").unwrap();
    assert_eq!(
        entries(dir.path()).unwrap(),
        BTreeSet::from([12, 13, 84, 85, 86])
    );
    let empty = tempfile::tempdir().unwrap();
    assert_eq!(
        entries(empty.path()).unwrap(),
        BTreeSet::new(),
        "a loader of neither kind"
    );
    assert!(entries(&empty.path().join("gone")).is_err());
}

#[test]
fn what_stays_and_what_goes_routinely_and_under_pressure() {
    let dir = tempfile::tempdir().unwrap();
    let ages = [
        (1, 30.0),
        (2, 29.0),
        (3, 20.0),
        (4, 3.0),
        (5, 2.0),
        (6, 2.0),
        (7, 1.0),
        (8, 1.0),
    ];
    profile(dir.path(), "system", &ages, 4);
    let system = &profiles(dir.path()).unwrap()[0];
    let in_use = [PathBuf::from("/nix/store/2-system")];
    let boot = Boot::Entries(BTreeSet::from([3]));
    let (routine, pressed) = expired(system, &in_use, &boot, 2, 7.0 * DAY, NOW);
    // 4 is current, 2 is in use, 3 has a boot entry, 7 and 8 the newest two.
    assert_eq!(routine, [1], "and 5 and 6 are under a week old");
    assert_eq!(pressed, [5, 6], "which pressure takes too");
}

#[test]
fn an_unreadable_boot_partition_expires_nothing_of_the_system() {
    let dir = tempfile::tempdir().unwrap();
    profile(dir.path(), "system", &[(1, 30.0), (2, 29.0), (3, 1.0)], 3);
    let system = &profiles(dir.path()).unwrap()[0];
    let (routine, pressed) = expired(system, &[], &Boot::Unreadable, 1, 7.0 * DAY, NOW);
    assert!(routine.is_empty() && pressed.is_empty());
    let (routine, _) = expired(system, &[], &Boot::NotBooted, 1, 7.0 * DAY, NOW);
    assert_eq!(routine, [1, 2], "a profile nothing boots needs no entries");
}

#[test]
fn generations_are_deleted_through_nix_env() {
    let dir = tempfile::tempdir().unwrap();
    let bin = dir.path().join("bin");
    fs::create_dir_all(&bin).unwrap();
    crate::garbage::script(
        &bin.join("nix-env"),
        &format!(
            "[ \"$1\" = -p ] || exit 9\necho \"$*\" > {}\n",
            dir.path().join("asked").display()
        ),
    );
    expire(&bin.join("nix"), Path::new("/p/system"), &[1, 5], false).unwrap();
    let asked = fs::read_to_string(dir.path().join("asked")).unwrap();
    assert_eq!(asked, "-p /p/system --delete-generations 1 5\n");
    crate::garbage::script(
        &bin.join("nix-env"),
        "echo 'error: lock held' >&2; exit 1\n",
    );
    let failed = expire(&bin.join("nix"), Path::new("/p/system"), &[1], false).unwrap_err();
    assert_eq!(failed, "nix-env failed: error: lock held");
}

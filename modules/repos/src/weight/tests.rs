//! The weighing: how loose objects are reckoned from one folder or counted
//! whole, how packs are counted and their room weighed, and when git's
//! auto-gc would call a store due, at its defaults and as a checkout's
//! config sets it, over stores laid out as git lays them, and over one git
//! itself wrote and gathered.

use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use super::*;
use crate::upstreams::tests::git;

/// A git folder of the test's own, emptied first, its objects folder
/// laid out as `git init` leaves it.
fn store(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("repos-weight-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("objects/pack")).expect("a temporary store");
    fs::create_dir_all(dir.join("objects/info")).expect("a temporary store");
    dir
}

/// Writes `bytes` to `path`, giving the room the disk gave it, read here
/// from its blocks rather than through the weighing's own `room`.
fn written(path: &Path, bytes: usize) -> u64 {
    fs::write(path, vec![1; bytes]).expect("a file written");
    fs::metadata(path).expect("its metadata").blocks() * 512
}

/// `count` loose objects of `bytes` each in folder `first`, named as git
/// names them, giving the room they take.
fn loose(store: &Path, first: &str, count: usize, bytes: usize) -> u64 {
    let folder = store.join("objects").join(first);
    fs::create_dir_all(&folder).expect("a loose objects folder");
    (0..count)
        .map(|at| written(&folder.join(format!("{at:038x}")), bytes))
        .sum()
}

/// A pack of `bytes`, with its index, and kept where `kept` says, giving
/// the room its files take.
fn pack(store: &Path, name: &str, bytes: usize, kept: bool) -> u64 {
    let folder = store.join("objects/pack");
    let named = format!("pack-{name:0>40}");
    let mut taken = written(&folder.join(format!("{named}.pack")), bytes);
    taken += written(&folder.join(format!("{named}.idx")), 8);
    if kept {
        taken += written(&folder.join(format!("{named}.keep")), 0);
    }
    taken
}

/// Git's limits as a config sets them, key by key, each a section, its
/// name (empty for none), a key and its value.
fn limits(set: &[(&str, &str, &str, &str)]) -> Limits {
    let mut limits = Limits::default();
    for (section, name, key, value) in set {
        limits.set(section, name, key, value);
    }
    limits
}

fn weighed(store: &Path) -> Weight {
    weigh(store, &Limits::default()).expect("a store git laid out")
}

#[test]
fn an_empty_store_weighs_nothing_and_isnt_due() {
    let store = store("empty");
    assert_eq!(
        weighed(&store),
        Weight {
            loose: 0,
            estimated: false,
            loose_bytes: 0,
            packs: 0,
            packed_bytes: 0,
            due: false,
        }
    );
}

#[test]
fn few_loose_objects_are_counted_in_every_folder_by_the_room_they_take() {
    let store = store("few");
    let mut taken = loose(&store, "17", 3, 10);
    taken += loose(&store, "aa", 2, 9999);
    // A SHA-256 store's names are longer.
    fs::create_dir_all(store.join("objects/bb")).expect("a loose objects folder");
    taken += written(&store.join("objects/bb").join("c".repeat(62)), 10);
    // None is an object: git's half-written one, a name too short, and
    // names of an object's length that aren't lowercase hex.
    let strays = store.join("objects/17");
    for name in ["tmp_obj_a1b2c3", "abc", &"g".repeat(38), &"A".repeat(38)] {
        written(&strays.join(name), 99);
    }
    let weight = weighed(&store);
    assert_eq!(
        (weight.loose, weight.loose_bytes, weight.estimated),
        (6, taken, false)
    );
    assert!(taken >= 6 * 512, "each takes at least a block: {taken}");
}

#[test]
fn many_loose_objects_are_reckoned_from_one_folder_of_256() {
    let store = store("many");
    let taken = loose(&store, "17", 16, 10);
    // Another folder's objects are left to the estimate.
    loose(&store, "aa", 1, 99_999);
    // The largest of the sample is counted once, and the rest 256 times.
    let small = taken / 16;
    let weight = weighed(&store);
    assert_eq!(
        (weight.loose, weight.loose_bytes, weight.estimated),
        (16 * 256, 15 * small * 256 + small, true)
    );
    // So one large file among them doesn't swell the whole.
    let folder = store.join("objects/17");
    let big = written(&folder.join("f".repeat(38)), 4_000_000);
    let one = weighed(&store);
    assert_eq!(
        (one.loose, one.loose_bytes),
        (17 * 256, 16 * small * 256 + big),
        "{one:?}"
    );
    // While several large files are a part of the store, and count.
    let bigs: Vec<String> = (0..3).map(|at| format!("e{at:037x}")).collect();
    for name in &bigs {
        written(&folder.join(name), 4_000_000);
    }
    let several = weighed(&store);
    assert_eq!(
        (several.loose, several.loose_bytes),
        (20 * 256, (16 * small + 3 * big) * 256 + big),
        "{several:?}"
    );
    for name in bigs.iter().chain([&"f".repeat(38)]) {
        fs::remove_file(folder.join(name)).expect("a large one gone");
    }
    // One fewer in the folder it samples, and every folder is counted.
    fs::remove_file(store.join("objects/17").join(format!("{:038x}", 15))).expect("one gone");
    let counted = weighed(&store);
    assert_eq!(
        (counted.loose, counted.estimated),
        (16, false),
        "{counted:?}"
    );
}

#[test]
fn a_store_is_due_past_gits_loose_threshold_and_not_at_it() {
    // Git's default of 6700 loose objects is 27 in the one folder it counts.
    let store = store("threshold");
    loose(&store, "17", 27, 1);
    assert!(!weighed(&store).due, "27 in one folder is git's own limit");
    loose(&store, "17", 28, 1);
    assert!(weighed(&store).due, "one past git's limit is due");
}

#[test]
fn packs_are_counted_by_index_and_only_those_not_kept_make_a_store_due() {
    let store = store("packs");
    let mut taken = 0;
    for at in 0..50 {
        taken += pack(&store, &at.to_string(), 100, false);
    }
    taken += pack(&store, "kept", 1000, true);
    let weight = weighed(&store);
    assert_eq!(
        (weight.packs, weight.packed_bytes, weight.due),
        (51, taken, false),
        "packs kept aren't gathered, so they don't count towards git's 50"
    );
    pack(&store, "more", 100, false);
    assert!(weighed(&store).due, "one past git's limit is due");
}

#[test]
fn a_pack_counts_only_with_its_index_beside_it_though_every_file_takes_room() {
    let store = store("unindexed");
    let folder = store.join("objects/pack");
    let mut taken = pack(&store, "whole", 100, false);
    // A pack whose index isn't written yet, and an index whose pack is gone.
    taken += written(&folder.join(format!("pack-{:0>40}.pack", "half")), 9999);
    taken += written(&folder.join(format!("pack-{:0>40}.idx", "gone")), 8);
    let weight = weighed(&store);
    assert_eq!((weight.packs, weight.packed_bytes), (1, taken));
}

#[test]
fn a_checkouts_config_moves_gits_thresholds_or_turns_them_off() {
    let store = store("config");
    loose(&store, "17", 28, 1);
    let due = |set: &[(&str, &str, &str, &str)]| weigh(&store, &limits(set)).expect("a store").due;
    assert!(due(&[]), "28 is past git's default");
    assert!(
        !due(&[("gc", "", "auto", "10000")]),
        "a limit of 10000 is 40 in one folder"
    );
    assert!(
        !due(&[("gc", "", "auto", "1m")]),
        "a limit of 1m is 4096 in one folder"
    );
    // Each key's last value is git's, whatever came before it.
    let again: [(&str, &str, &str, &str, &str, bool); 4] = [
        ("gc", "", "auto", "10000", "1024", true),
        ("maintenance", "", "auto", "false", "true", true),
        ("maintenance", "", "strategy", "gc", "none", false),
        ("maintenance", "gc", "enabled", "false", "true", true),
    ];
    for (section, name, key, first, last, said) in again {
        let set = [(section, name, key, first), (section, name, key, last)];
        assert_eq!(due(&set), said, "{set:?}: the last value is git's");
    }
    assert!(
        due(&[("maintenance", "", "auto", "true")]),
        "upkeep left on"
    );
    assert!(
        due(&[("maintenance", "", "strategy", "gc")]),
        "the gc strategy runs it"
    );
    let on = [
        ("maintenance", "", "strategy", "geometric"),
        ("maintenance", "gc", "enabled", "true"),
    ];
    assert!(
        due(&on),
        "the gc task turned on by name runs under any strategy"
    );
    let reversed = [on[1], on[0]];
    assert!(
        due(&reversed),
        "in either order, as git judges them once all are read"
    );
    for at in 0..51 {
        pack(&store, &at.to_string(), 1, false);
    }
    let off: [&[(&str, &str, &str, &str)]; 7] = [
        &[("gc", "", "auto", "0")],
        &[("GC", "", "Auto", "-1")],
        &[("maintenance", "", "auto", "false")],
        &[("maintenance", "", "auto", "")],
        &[("maintenance", "gc", "enabled", "no")],
        &[("maintenance", "", "strategy", "none")],
        &[("maintenance", "", "strategy", "geometric")],
    ];
    for set in off {
        assert!(
            !due(set),
            "{set:?} turns git's automatic gc off, packs and all"
        );
    }
    let packs = [
        ("gc", "", "auto", "10000"),
        ("gc", "", "autoPackLimit", "50"),
    ];
    assert!(due(&packs), "51 packs are past a limit of 50");
    let first = [("gc", "", "autoPackLimit", "2"), ("gc", "", "auto", "0")];
    assert!(
        !due(&first),
        "gc.auto off turns the packs off too, set before or after"
    );
    let twice = [
        ("gc", "", "auto", "10000"),
        ("gc", "", "autoPackLimit", "0"),
        ("gc", "", "autoPackLimit", "50"),
    ];
    assert!(due(&twice), "the pack limit's last value is git's");
    let none = [
        ("gc", "", "auto", "10000"),
        ("gc", "", "autoPackLimit", "0"),
    ];
    assert!(!due(&none), "a pack limit of 0 turns that half off");
}

#[test]
fn a_folder_with_no_objects_isnt_weighed() {
    let store = store("none");
    fs::remove_dir_all(store.join("objects")).expect("no objects");
    assert_eq!(weigh(&store, &Limits::default()), None);
}

#[test]
fn a_loose_folder_that_cant_be_read_leaves_the_store_unweighed() {
    let store = store("unreadable");
    loose(&store, "17", 3, 10);
    let folder = store.join("objects/17");
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o000)).expect("closed");
    let readable = fs::read_dir(&folder).is_ok();
    let weight = weigh(&store, &Limits::default());
    fs::set_permissions(&folder, fs::Permissions::from_mode(0o755)).expect("opened");
    // Whoever may read any folder, as root may, sees it whole.
    if !readable {
        assert_eq!(weight, None, "not said as an empty store");
    }
}

#[test]
fn a_store_git_wrote_weighs_each_loose_object_and_once_gathered_one_pack() {
    let dir = store("git").join("work");
    fs::create_dir_all(&dir).expect("a work folder");
    git(&dir, &["init", "--quiet"]);
    for at in 0..40 {
        fs::write(dir.join("file"), format!("{at}\n")).expect("a file");
        git(&dir, &["add", "file"]);
        git(&dir, &["commit", "--quiet", "-m", &at.to_string()]);
    }
    // Each commit wrote its file, its tree and itself, each a loose object.
    let written = weighed(&dir.join(".git"));
    assert_eq!((written.loose, written.packs), (120, 0), "{written:?}");
    assert!(
        written.loose_bytes > 0,
        "loose objects take room: {written:?}"
    );
    git(&dir, &["gc", "--quiet"]);
    let gathered = weighed(&dir.join(".git"));
    assert_eq!(
        (gathered.loose, gathered.packs, gathered.due),
        (0, 1, false),
        "gathered into one pack: {gathered:?}"
    );
    assert!(gathered.packed_bytes > 0, "a pack takes room: {gathered:?}");
}

#[test]
fn a_store_said_due_is_one_git_gathers_when_it_next_runs_its_upkeep() {
    // The rule is git's, copied: this holds it to the git the machine runs,
    // so a git whose defaults moved fails here rather than saying a store is
    // due that git never gathers.
    let dir = store("git-due").join("work");
    fs::create_dir_all(&dir).expect("a work folder");
    git(&dir, &["init", "--quiet"]);
    // Ten thousand files, past gc.auto's 6,700 loose objects however their
    // names fall in the folder sampled.
    for at in 0..10_000 {
        fs::write(dir.join(format!("{at}")), format!("{at}\n")).expect("a file");
    }
    git(&dir, &["add", "."]);
    // Without its own upkeep, which would gather them detached, holding
    // the lock the run below then quietly yields to.
    git(
        &dir,
        &[
            "-c",
            "maintenance.auto=false",
            "commit",
            "--quiet",
            "-m",
            "many",
        ],
    );
    let before = weighed(&dir.join(".git"));
    assert!(before.due, "past git's limit: {before:?}");
    // As git runs it after a commit, but here, not detached.
    git(
        &dir,
        &[
            "-c",
            "maintenance.autoDetach=false",
            "-c",
            "gc.autoDetach=false",
            "maintenance",
            "run",
            "--auto",
            "--quiet",
        ],
    );
    let after = weighed(&dir.join(".git"));
    assert!(
        !after.due && after.loose < before.loose && after.packs > 0,
        "git gathered it: {before:?} then {after:?}"
    );
}

//! What a guard learns from a pass after a survey: which disks' spent
//! builds were judged, and when upkeep comes again.

use crate::daemon::{CALM, Home, held_tree, owned_trees, surveyed_before, tick, touch, wall};
use crate::steps::{DRAINED, fill, margin_and_free, on_shm, writes_a_share};
use disk_guard::clock::Tick;
use disk_guard::config::{Config, Source};
use disk_guard::pool::pool;
use disk_guard::state::Memory;
use disk_guard::units::{Size, Span};
use std::fs;
use std::path::{Path, PathBuf};

/// A tick `seconds` after `start`, in a boot an hour old then.
fn after(start: f64, seconds: f64) -> Tick {
    Tick {
        wall: start + seconds,
        monotonic: 3600.0 + seconds,
        ..tick()
    }
}

/// Changes what `home`'s guard remembers, as a guard restarted with that
/// memory would find it.
fn remember(home: &Home, change: impl FnOnce(&mut Memory)) {
    let path = home.root.join("state/memory.json");
    let mut memory: Memory = disk_guard::state::load(&path).unwrap().unwrap();
    change(&mut memory);
    disk_guard::state::save(&path, &memory).unwrap();
}

/// Where the filesystem holding `path` is mounted, as the guard names it.
fn mount(path: &Path) -> PathBuf {
    let mut mount = fs::canonicalize(path).unwrap();
    let device = pool(&mount).unwrap();
    while let Some(above) = mount.parent() {
        if pool(above).unwrap() != device {
            break;
        }
        mount = above.to_owned();
    }
    mount
}

/// Two folders on filesystems of their own, a tmpfs and the build's disk,
/// the first on the one whose device sorts first, as the guard passes
/// them.
fn two_disks() -> (tempfile::TempDir, tempfile::TempDir) {
    let shm = tempfile::tempdir_in("/dev/shm").unwrap();
    let target = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).unwrap();
    let (on_shm, on_target) = (pool(shm.path()).unwrap(), pool(target.path()).unwrap());
    assert_ne!(
        on_shm, on_target,
        "the build's temp folder shares /dev/shm's filesystem, so there's no second disk"
    );
    if on_shm < on_target {
        (shm, target)
    } else {
        (target, shm)
    }
}

/// `config` with the disk scratch at `path` on it too.
fn with_scratch(mut config: Config, path: &Path) -> Config {
    fs::create_dir_all(path).unwrap();
    config.sources.insert(
        "scratch".to_owned(),
        Source::Scratch {
            path: path.to_owned(),
            healthy: None,
        },
    );
    config
}

#[test]
fn a_disk_resting_while_another_judged_what_an_owner_left_still_judges_its_own() {
    // An owner's builds may lie on any disk, and a pass is offered only
    // what lies on its own, so another disk's pass judged none of these.
    let (first, second) = two_disks();
    let scratch = first.path().join("scratch");
    let home = Home::within(second);
    let built = held_tree(&home, 400);
    let start = wall();
    let memory = Memory {
        emptied: [(mount(&home.root), start + 100.0)].into(),
        ..Default::default()
    };
    disk_guard::state::save(&home.root.join("state/memory.json"), &memory).unwrap();
    let mut config = with_scratch(owned_trees(&home), &scratch);
    config.bands.filling = Size::Percent(100.0);
    config.bands.target = Size::Percent(100.0);
    let mut guard = home.guard(config.clone());
    guard.step(&after(start, 0.0), &CALM).unwrap();
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    guard.step(&after(start, 61.0), &CALM).unwrap();
    assert!(
        built.exists(),
        "the builds' disk rests while the other passes"
    );
    assert_eq!(
        home.memory()["spent_due"],
        true,
        "a pass on the other disk judged the builds an owner left on a resting one"
    );
    drop(guard);
    remember(&home, |memory| {
        memory.surveyed = Some((start + 61.0, 3600.0));
    });
    let mut again = home.guard(config);
    again.step(&after(start, 130.0), &CALM).unwrap();
    assert!(
        !built.exists(),
        "the builds an owner left waited out the spacing once their disk's rest ended"
    );
}

#[test]
fn a_producers_pass_leaves_what_an_owner_left_to_be_judged() {
    // A producer's news walks only its own sources, so its pass is
    // offered nothing an owner left.
    let home = Home::new();
    let built = held_tree(&home, 400);
    let stamp = home.root.join("stamp");
    fs::write(&stamp, "").unwrap();
    let mut config = owned_trees(&home);
    config.sources.insert(
        "gc".to_owned(),
        Source::Producer {
            command: vec!["/bin/sh".to_owned(), "-c".to_owned(), "true".to_owned()],
            path: home.root.clone(),
            every: Some(Span(std::time::Duration::from_hours(6))),
            from: None,
            watch: vec![stamp.clone()],
            timeout: Span(std::time::Duration::from_secs(10)),
        },
    );
    let start = wall();
    let mut guard = home.guard(config);
    guard.step(&after(start, 0.0), &CALM).unwrap();
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    fs::write(&stamp, "a version").unwrap();
    guard.step(&after(start, 61.0), &DRAINED).unwrap();
    let journal = home.journal();
    let last = journal.iter().rfind(|line| line["event"] == "pass");
    assert_eq!(
        last.map(|line| &line["why"]),
        Some(&serde_json::json!("producers")),
        "the producer's news brought no pass of its own: {journal:?}"
    );
    assert!(built.exists(), "a drained machine spared upkeep");
    assert_eq!(
        home.memory()["spent_due"],
        true,
        "a producers' pass judged the builds an owner left"
    );
}

#[test]
fn a_reach_that_met_its_band_first_leaves_what_an_owner_left_to_be_judged() {
    // A reach stops once free space reaches its target, so what an owner
    // left past what it took is left to the upkeep it brings.
    let _held = writes_a_share();
    let home = on_shm();
    held_tree(&home, 400);
    let (margin, _) = margin_and_free(&home.root);
    let entry = home.root.join("scratch/run");
    fs::create_dir_all(&entry).unwrap();
    fill(&entry.join("out"), margin);
    let (_, free) = margin_and_free(&home.root);
    let filling = free - margin / 2;
    let mut config = with_scratch(owned_trees(&home), &home.root.join("scratch"));
    config.bands.filling = Size::Bytes(filling);
    config.bands.target = Size::Bytes(filling);
    let filler = home.root.join("filler");
    fill(&filler, margin);
    let start = wall();
    let mut guard = home.guard(config);
    guard.step(&after(start, 0.0), &CALM).unwrap();
    assert!(
        entry.exists(),
        "scratch written to just now went while filling"
    );
    // Back within the margin, so still filling, and short of the target by
    // less than the entry, once it's old.
    fs::remove_file(&filler).unwrap();
    touch(&entry.join("out"), 1_000_000_000);
    touch(&entry, 1_000_000_000);
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    guard.step(&after(start, 61.0), &CALM).unwrap();
    let journal = home.journal();
    let last = journal.iter().rfind(|line| line["event"] == "pass");
    assert_eq!(
        last.map(|line| &line["end"]),
        Some(&serde_json::json!("reached")),
        "the reach didn't meet its band: {journal:?}"
    );
    assert_eq!(
        home.memory()["spent_due"],
        true,
        "a reach that met its band judged the builds an owner left"
    );
}

#[test]
fn a_guard_marks_every_disk_it_watches_when_it_starts_with_spent_builds_unjudged() {
    // Which disk held them isn't kept, so each judges its own again.
    let (first, second) = two_disks();
    let scratch = first.path().join("scratch");
    let home = Home::within(second);
    let built = held_tree(&home, 400);
    fs::remove_dir_all(home.root.join("proc/400")).unwrap();
    surveyed_before(&home, wall(), 3600.0);
    remember(&home, |memory| memory.spent_due = true);
    let mut guard = home.guard(with_scratch(owned_trees(&home), &scratch));
    guard.step(&tick(), &CALM).unwrap();
    assert!(
        !built.exists(),
        "a restarted guard waited out the spacing with an owner's builds unjudged"
    );
    assert_eq!(
        home.memory()["spent_due"],
        false,
        "every disk's upkeep judged its own, yet the builds were still kept as due"
    );
}

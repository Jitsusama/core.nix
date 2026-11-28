//! What a survey walks to size its candidates, and what it knows from the
//! surveys before.

use crate::discover::{dir, file, stop_after};
use crate::survey::{config, find, now, offers_sized, offers_stopping, real, src};
use disk_guard::config::Config;
use disk_guard::sizes::{SETTLED, Sizes};
use std::cell::Cell;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The folders of an entry a stop cuts short, enough that a stop counted
/// back from the end of its walk comes partway through it.
const FOLDERS: usize = 50;

/// Waits out the time a change must be past before a stamp holding it is
/// kept, as a survey just after the change would walk it again however
/// the record was kept.
fn settle() {
    std::thread::sleep(Duration::from_secs_f64(2.0 * SETTLED));
}

/// A home holding two built checkouts, one with an incremental variant,
/// and a scratch entry, with a config surveying them.
fn built(home: &Path) -> (Config, PathBuf, PathBuf) {
    let (_, one, _) = src(&home.join("one"));
    dir(&one.join("debug/incremental/app-0abc/s-abc-def"));
    file(
        &one.join("debug/incremental/app-0abc/s-abc-def/dep-graph.bin"),
        "graph",
    );
    let (_, two, _) = src(&home.join("two"));
    let scratch = dir(&home.join("scratch"));
    file(&scratch.join("run-1/deep/out"), &"x".repeat(20_000));
    let config = config(&format!(
        r#""src": {{ "kind": "cargo", "roots": ["{}"] }},
           "scratch": {{ "kind": "scratch", "path": "{}" }}"#,
        home.display(),
        scratch.display()
    ));
    (config, one, two)
}

// On 27 November seven whole surveys read 136 GB between them, each
// walking every candidate afresh though few had changed.
#[test]
fn a_second_survey_with_nothing_changed_walks_no_candidate() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let (config, one, two) = built(&home);
    settle();
    let (first, sizes) = offers_sized(&config, now(), Sizes::default());
    let walked = sizes.walked;
    assert!(
        walked >= 4,
        "two targets, a variant and a scratch entry weren't each walked: {walked}"
    );
    let (second, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(
        sizes.walked, walked,
        "an unchanged candidate was walked again"
    );
    for path in [&one, &two, &home.join("scratch/run-1")] {
        assert_eq!(
            find(&second, path).candidate.size,
            find(&first, path).candidate.size,
            "{} known unchanged changed its size",
            path.display()
        );
    }
}

// Cargo writes each new file of a build in a profile's deps, build,
// incremental or .fingerprint, two levels below the target, so a target
// written again is read by a few dozen stats rather than walked whole.
#[test]
fn a_file_written_in_a_targets_deps_walks_that_target_alone() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let (config, one, two) = built(&home);
    settle();
    let (first, sizes) = offers_sized(&config, now(), Sizes::default());
    let walked = sizes.walked;
    file(
        &two.join("debug/deps/libapp-fedcba9876543210.rlib"),
        &"y".repeat(20_000),
    );
    let (second, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(
        sizes.walked,
        walked + 1,
        "a build written again wasn't walked alone"
    );
    assert!(
        find(&second, &two).candidate.size > find(&first, &two).candidate.size,
        "the target written again kept its old size"
    );
    assert_eq!(
        find(&second, &one).candidate.size,
        find(&first, &one).candidate.size,
        "a target nobody wrote changed its size"
    );
}

// A walk a stop cut short counted part of its entry, and a stopped survey
// still hands its sizes back to the daemon, so a size kept from it would
// be the next survey's answer until the entry changed.
#[test]
fn a_walk_a_stop_cut_short_leaves_no_size_behind() {
    let home = tempfile::tempdir().unwrap();
    let home = real(home.path());
    let scratch = dir(&home.join("scratch"));
    let entry = scratch.join("run-1");
    for n in 0..FOLDERS {
        file(&entry.join(format!("{n}/out")), &"x".repeat(1_000));
    }
    let config = config(&format!(
        r#""scratch": {{ "kind": "scratch", "path": "{}" }}"#,
        scratch.display()
    ));
    settle();
    let asked = Cell::new(0);
    offers_stopping(
        &config,
        now(),
        Sizes::default(),
        &stop_after(usize::MAX, &asked),
    );
    let asks = asked.get();
    assert!(asks > FOLDERS, "the walk didn't ask at each folder: {asks}");
    // The entry's walk asks at each of its folders and the survey a few
    // times around it, so a stop half its folders from the end comes
    // partway through the walk.
    let asked = Cell::new(0);
    let stop = stop_after(asks - FOLDERS / 2, &asked);
    let (_, sizes) = offers_stopping(&config, now(), Sizes::default(), &stop);
    assert_eq!(sizes.walked, 1, "the stopped survey never walked the entry");
    let (found, sizes) = offers_sized(&config, now(), sizes);
    assert_eq!(
        sizes.walked, 2,
        "the entry a stop cut short wasn't walked again"
    );
    let size = find(&found, &entry).candidate.size;
    assert!(
        size >= (FOLDERS * 1_000) as u64,
        "the entry kept what its cut walk counted: {size}"
    );
}

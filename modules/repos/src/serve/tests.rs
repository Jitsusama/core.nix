use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use repos::read;

use serde_json::json;

use super::{Disk, LONGEST, Of, QUIET, SPACED, health, settle};

fn of(name: &str) -> Of {
    Of::Checkout(PathBuf::from(name))
}

/// How far past its bound a wait may run on a builder other work shares,
/// whose threads can wake late: the bounds below are what's proven, and
/// this only keeps a slow wake from reading as a wait too long.
const LATE: Duration = Duration::from_secs(1);

/// A reading that began long enough ago that no spacing is owed.
fn long_ago() -> Instant {
    Instant::now()
        .checked_sub(SPACED * 2)
        .expect("a time long enough ago")
}

#[test]
fn a_write_heard_is_read_once_the_disk_is_quiet_with_whatever_came_meanwhile() {
    let (tell, told) = mpsc::channel();
    let began = Instant::now();
    // Fifteen more writes, each a tenth of the quiet after the last, so the
    // quiet only begins after the last, past a whole quiet from the first:
    // a settle timing the quiet from the first write alone would read
    // before it. Gaps this short leave a late wake most of a quiet.
    let names: Vec<String> = (1..=15).map(|each| format!("write {each}")).collect();
    let telling = std::thread::spawn({
        let names = names.clone();
        move || {
            for name in names {
                std::thread::sleep(QUIET / 10);
                tell.send(BTreeSet::from([of(&name)])).expect("told");
            }
            tell
        }
    });
    let moved = settle(&told, BTreeSet::from([of("first")]), long_ago()).expect("settled");
    let _tell = telling.join().expect("the teller");
    let waited = began.elapsed();
    let every: BTreeSet<Of> = std::iter::once(of("first"))
        .chain(names.iter().map(|name| of(name)))
        .collect();
    assert_eq!(
        moved, every,
        "read before the writes ended, after {waited:?}"
    );
    assert!(
        waited >= QUIET * 2,
        "read before the disk was quiet after the last write: {waited:?}"
    );
    assert!(waited < LONGEST + LATE, "waited past the quiet: {waited:?}");
}

#[test]
fn writing_without_pause_is_read_within_the_longest_wait() {
    let (tell, told) = mpsc::channel();
    let until = Instant::now() + LONGEST * 3;
    let telling = std::thread::spawn(move || {
        while Instant::now() < until {
            if tell.send(BTreeSet::from([of("busy")])).is_err() {
                return;
            }
            std::thread::sleep(QUIET / 10);
        }
    });
    let began = Instant::now();
    let moved = settle(&told, BTreeSet::from([of("busy")]), long_ago()).expect("settled");
    let waited = began.elapsed();
    drop(told);
    telling.join().expect("the teller");
    assert_eq!(moved, BTreeSet::from([of("busy")]));
    assert!(
        waited >= LONGEST,
        "gave up before the longest wait: {waited:?}"
    );
    assert!(
        waited < LONGEST + LATE,
        "waited past the longest: {waited:?}"
    );
}

#[test]
fn a_reading_starts_no_sooner_than_its_spacing_after_the_last() {
    let (_tell, told) = mpsc::channel();
    let began = Instant::now();
    let moved = settle(&told, BTreeSet::from([of("first")]), began).expect("settled");
    assert_eq!(moved, BTreeSet::from([of("first")]));
    let waited = began.elapsed();
    assert!(
        waited >= SPACED,
        "read again within its spacing: {waited:?}"
    );
}

#[test]
fn a_checkout_gone_when_its_folder_is_heard_is_let_go() {
    // The record is the checkouts found, never the roots walked, merged
    // with the timers' kept files from the account's state folder, which
    // add GitHub's part to a repository and never a repository of their
    // own, so only this test's checkout is counted.
    let at = std::env::temp_dir().join(format!("repos-serve-gone-{}", std::process::id()));
    let _ = fs::remove_dir_all(&at);
    let git = at.join("notes/.git");
    fs::create_dir_all(git.join("refs/heads")).expect("a git folder");
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    let roots = vec![at.clone()];
    let mut disk = Disk {
        home: None,
        roots,
        machine: "laptop".to_owned(),
        found: BTreeMap::from([(git.clone(), read::checkout(&git, None))]),
    };
    assert_eq!(disk.record().repositories.len(), 1, "the checkout held");

    fs::remove_dir_all(&at).expect("the checkout taken away");
    let record = disk.read_again(&BTreeSet::from([Of::Checkout(git)]));
    assert!(
        record.repositories.is_empty(),
        "a gone checkout still said: {record:#?}"
    );
    assert!(disk.found.is_empty(), "a gone checkout still held");
}

#[test]
fn health_says_how_many_folders_went_unwatched_and_why_the_first_did() {
    let because = "/home/joel/src/a/.git/refs: the account's inotify watches are spent";
    let said = health(40, (2, Some(because.to_owned())), 12);
    assert_eq!(
        said,
        json!({"watched": 40, "unwatched": 2, "unwatched_because": because, "read_ms": 12}),
        "health hides what went unwatched"
    );
    let said = health(40, (0, None), 12);
    assert_eq!(
        said.get("unwatched_because"),
        None,
        "a reason said when nothing went unwatched: {said}"
    );
    assert_eq!(said["unwatched"], 0, "health leaves out what's unwatched");
}

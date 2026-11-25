use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use repos::read;

use serde_json::json;

use repos::using::{Process, Sample};

use super::{Disk, LONGEST, Of, QUIET, SAMPLED, SPACED, STALE, Working, health, ready, settle};

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

/// The record of one checkout, `notes`, in a folder of its own named for
/// `name`, which the caller removes.
fn one_checkout(name: &str) -> (PathBuf, repos::model::Record) {
    let at = std::env::temp_dir().join(format!("repos-serve-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&at);
    let git = at.join("notes/.git");
    fs::create_dir_all(git.join("refs/heads")).expect("a git folder");
    fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("HEAD");
    let disk = Disk {
        home: None,
        roots: vec![at.clone()],
        machine: "laptop".to_owned(),
        found: BTreeMap::from([(git.clone(), read::checkout(&git, None))]),
    };
    (at, disk.record())
}

/// A pace for what works in each tree short enough that a case sees
/// several readings in well under a second.
const BRISK: Duration = Duration::from_millis(20);

#[test]
fn what_works_is_read_at_its_own_pace_however_long_the_disk_takes() {
    // A whole reading of a busy disk took over three minutes on 25 Nov,
    // longer than two readings of /proc may be apart and still measure a
    // share, so they keep a clock of their own rather than waiting on it.
    // Each reading here says a pace of waking time passed, in which the
    // process worked all of it.
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counting = std::sync::Arc::clone(&calls);
    let folder = PathBuf::from("/home/someone/trees/fix");
    let mut working = Working::start(BRISK, vec![folder.clone()], move |folders: &[PathBuf]| {
        let call = counting.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Sample {
            at: 100 + call * 2,
            awake: Some(100 + call * 2),
            processes: vec![Process {
                pid: 1,
                parent: 0,
                folder: folders.first().cloned(),
                started: 1,
                own: call * 2,
                children: 0,
                memory: Some(4_096),
            }],
        }
    });
    working.read(vec![folder.clone()]);
    // The disk's whole reading takes ten paces.
    std::thread::sleep(BRISK * 10);
    working.read(vec![folder.clone()]);
    let read = calls.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        read >= 4,
        "/proc was read {read} times in ten paces, so only as the disk was"
    );
    assert!(working.measured, "a share went unmeasured");
    assert_eq!(
        working.used.get(&folder).and_then(|used| used.cpu),
        Some(1.0),
        "the share wasn't of the pace before the newest reading"
    );
}

#[test]
fn what_works_is_unread_until_a_first_reading_answers() {
    // A daemon restarted while a process holds its memory map reaches
    // that process again, so nothing it has read says nothing works.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start(BRISK, folders.clone(), move |_: &[PathBuf]| {
        let _ = freed.lock().map(|freed| freed.recv_timeout(SAMPLED * 4));
        Sample::default()
    });
    working.read(folders);
    let (at, record) = one_checkout("first");
    let said = working.said(&record);
    let using = said.iter().find(|said| said["kind"] == "using");
    assert!(
        using.is_some_and(|using| using["processes"].is_null()),
        "a reading not yet answered says nothing works: {using:?}"
    );
    drop(free);
    let _ = fs::remove_dir_all(&at);
}

#[test]
fn a_reading_stuck_in_proc_never_holds_up_the_loop() {
    // A process holding its memory map locked holds up a read of its
    // smaps_rollup until it lets go, which may be never. The first
    // reading answers, and each after it waits on `free`.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let calls = std::sync::atomic::AtomicU64::new(0);
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start(BRISK, folders.clone(), move |folders: &[PathBuf]| {
        let call = calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if call > 0 {
            let _ = freed.lock().map(|freed| freed.recv_timeout(SAMPLED * 4));
        }
        // As `sample` does, a reading held up says so, and each after the
        // first is held, however late this thread wakes (using.rs's own
        // tests time a real walk).
        Sample {
            at: 100 + call * 6_000,
            awake: (call == 0).then_some(100),
            processes: vec![Process {
                pid: 1,
                parent: 0,
                folder: Some(folders[0].clone()),
                started: 1,
                own: call * 600,
                children: 0,
                memory: Some(4_096),
            }],
        }
    });
    working.read(folders.clone());
    assert_eq!(working.used.len(), 1, "the first reading wasn't taken");

    let began = Instant::now();
    for _ in 0..STALE {
        working.read(folders.clone());
    }
    assert!(
        began.elapsed() < SAMPLED + LATE,
        "a stuck reading held up the loop {:?}",
        began.elapsed()
    );
    assert!(
        working.used.is_empty() && !working.measured,
        "what was read before a reading stuck is still said as now"
    );
    let (at, record) = one_checkout("stuck");
    let using: Vec<_> = working
        .said(&record)
        .into_iter()
        .filter(|said| said["kind"] == "using")
        .collect();
    assert_eq!(using.len(), 1, "the checkout's use went unsaid");
    assert!(
        using[0]["processes"].is_null(),
        "a reading stuck says nothing works: {}",
        using[0]
    );
    let _ = fs::remove_dir_all(&at);

    drop(free);
    let until = Instant::now() + SAMPLED * 4 + LATE;
    while working.used.is_empty() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(10));
        working.read(folders.clone());
    }
    assert_eq!(working.used.len(), 1, "a reading freed again wasn't taken");
    assert!(
        !working.measured,
        "a reading held up measured a share of time that wasn't its own"
    );
}

#[test]
fn it_says_how_long_it_took_to_be_ready_in_the_words_health_times() {
    // Health reads the first "in N ms" after the word ready as how long a restart
    // was gone (modules/health/src/launches.rs), so the line keeps those words.
    assert_eq!(
        ready(Duration::from_micros(1_234_567)),
        "repos: ready in 1234 ms, every repository read and the socket listening"
    );
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

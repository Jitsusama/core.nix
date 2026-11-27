//! What the daemon reads of the disk and of /proc, and what it says of
//! each, driven through stand-in readers on their own threads.
//!
//! Long because every case paces its readers by the same timings, LATE,
//! BRISK, ROOMY and SLOW, and the same stand-in checkouts, held once
//! here, so splitting it would copy those or part a reading from what's
//! said of it.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use repos::read;

use serde_json::{Value, json};

use repos::using::{LONGEST as LONGEST_SHARE, Process, Sample};

use super::{
    Disk, Journal, LONGEST, Of, QUIET, SAMPLED, SPACED, STALE, Working, folders_of, health,
    measures, ready, settle,
};

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

/// How old a reading may grow in a case that isn't about staleness, far
/// past any late wake on a shared builder.
const ROOMY: Duration = Duration::from_secs(10);

#[test]
fn what_works_is_read_at_its_own_pace_however_long_the_disk_takes() {
    // A whole reading of a busy disk took over three minutes on 25 Nov,
    // longer than two readings of /proc may be apart and still measure a
    // share, so they keep a clock of their own rather than waiting on it.
    // Each reading here is more than half the longest apart from the one
    // before, so only neighbours measure a share; the process works more
    // each pace, so the last pace's share is told from any longer span's;
    // and its memory says which call read it.
    const STEP: u64 = 10_000;
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counting = std::sync::Arc::clone(&calls);
    let folder = PathBuf::from("/home/someone/trees/fix");
    let started = Instant::now();
    let mut working = Working::start(
        BRISK,
        ROOMY,
        vec![folder.clone()],
        move |folders: &[PathBuf]| {
            let call = counting.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Sample {
                at: 100 + call * STEP,
                awake: Some(100 + call * STEP),
                processes: vec![Process {
                    pid: 1,
                    parent: 0,
                    folder: folders.first().cloned(),
                    started: 1,
                    own: call * call * 100,
                    children: 0,
                    memory: Some(4_096 * (call + 1)),
                }],
            }
        },
    );
    let _ = working.read(vec![folder.clone()]);
    // The disk's whole reading goes on while /proc is read four times,
    // which reading only as the disk is read never would.
    let until = Instant::now() + BRISK * 10 + LATE;
    while calls.load(std::sync::atomic::Ordering::Relaxed) < 4 && Instant::now() < until {
        std::thread::sleep(BRISK);
    }
    let read = calls.load(std::sync::atomic::Ordering::Relaxed);
    assert!(
        read >= 4,
        "/proc was read {read} times while the disk was, so only as the disk was"
    );
    // And no more often than its pace, since each walk reads every placed
    // process's memory.
    let paces = started.elapsed().as_millis() / BRISK.as_millis();
    assert!(
        u128::from(read) <= paces + 2,
        "/proc was read {read} times in {paces} paces"
    );
    let _ = working.read(vec![folder.clone()]);
    // The fourth call began only once the third's reading was held.
    let newest = working
        .used
        .get(&folder)
        .map(|used| used.memory / 4_096 - 1)
        .expect("the folder's use went unsaid");
    assert!(
        newest >= 2,
        "the reading taken was call {newest}'s, not the newest"
    );
    assert!(working.measured, "a share went unmeasured");
    let newest = u32::try_from(newest).expect("a call count");
    assert_eq!(
        working.used.get(&folder).and_then(|used| used.cpu),
        Some(f64::from(100 * (2 * newest - 1)) / 10_000.0),
        "the share wasn't of the pace before the newest reading"
    );
}

#[test]
fn a_folder_found_after_a_reading_is_said_unread_until_the_next_reads_it() {
    // Each reading of the disk hands the folders it holds to the next
    // reading of /proc, which places processes only in those it holds.
    let (at, record) = one_checkout("later");
    let mut working = Working::start(BRISK, ROOMY, Vec::new(), |folders: &[PathBuf]| Sample {
        at: 100,
        awake: None,
        processes: folders
            .iter()
            .map(|folder| Process {
                pid: 1,
                parent: 0,
                folder: Some(folder.clone()),
                started: 1,
                own: 0,
                children: 0,
                memory: Some(4_096),
            })
            .collect(),
    });
    // The first reading answers before the case judges what it said, so
    // the folder is unread for not being looked in, not for no answer.
    let until = Instant::now() + LATE;
    let _ = working.read(Vec::new());
    while working.unread() && Instant::now() < until {
        std::thread::sleep(BRISK);
        let _ = working.read(Vec::new());
    }
    assert!(!working.unread(), "the first reading never answered");
    let processes = |working: &Working| {
        let said = working.said(&record);
        let using = said.into_iter().find(|said| said["kind"] == "using");
        using.map(|using| using["processes"].clone())
    };
    assert_eq!(
        processes(&working),
        Some(Value::Null),
        "a folder no reading looked in reads as idle"
    );
    let found = folders_of(&record);
    let until = Instant::now() + BRISK * 10 + LATE;
    let _ = working.read(found.clone());
    while processes(&working) == Some(Value::Null) && Instant::now() < until {
        std::thread::sleep(BRISK);
        let _ = working.read(found.clone());
    }
    assert_eq!(
        processes(&working),
        Some(json!(1)),
        "a folder handed on was never read"
    );
    let _ = fs::remove_dir_all(&at);
}

#[test]
fn what_works_is_unread_until_a_first_reading_answers() {
    // A daemon restarted while a process holds its memory map reaches
    // that process again, so nothing it has read says nothing works.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let (at, record) = one_checkout("first");
    let folders = folders_of(&record);
    let mut working = Working::start(BRISK, ROOMY, folders.clone(), move |_: &[PathBuf]| {
        let _ = freed.lock().map(|freed| freed.recv_timeout(SAMPLED * 4));
        Sample::default()
    });
    let _ = working.read(folders);
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
fn a_walk_that_cannot_read_proc_says_what_works_unread_not_idle() {
    // A unit with /proc hidden answers at once, and fresh, with nothing
    // read: said as nothing working, it would read as a measurement of
    // none, however often it's taken.
    let (at, record) = one_checkout("hidden");
    let folders = folders_of(&record);
    let mut working = Working::start(BRISK, ROOMY, folders.clone(), |_: &[PathBuf]| {
        Sample::default()
    });
    for _ in 0..3 {
        let _ = working.read(folders.clone());
        std::thread::sleep(BRISK);
    }
    assert!(!working.unread(), "a fresh reading went stale");
    let said = working.said(&record);
    let using = said.iter().find(|said| said["kind"] == "using");
    assert!(
        using.is_some_and(|using| using["processes"].is_null()),
        "a walk that read nothing says nothing works: {using:?}"
    );
    let _ = fs::remove_dir_all(&at);
}

#[test]
fn a_first_reading_that_never_answers_is_said_late_once_and_its_answer_once() {
    // The restart above leaves the page unread, and the journal must say
    // why, as it does for a reading that stops, but as none that ever
    // answered. Grown old past SAMPLED, as the daemon's three paces are,
    // the first take waits SAMPLED out still young, and says nothing;
    // one past `old` is late and waits on nothing. What holds the
    // reading up is named only where one is, which a stand-in reading
    // never is, so the line is the late one whole. It's heard where the
    // daemon says it, as `take` tells it, and the thread says nothing
    // while its first walk is held.
    let old = YOUNG_PAST_A_WAIT;
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let (tell, told) = mpsc::channel();
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start_saying(
        BRISK,
        old,
        folders.clone(),
        move |_: &[PathBuf]| {
            let _ = freed.lock().map(|freed| freed.recv_timeout(SAMPLED * 8));
            Sample::default()
        },
        move |said| {
            let _ = tell.send(said);
        },
    );
    // Read once the reading's own clock has begun, so the late take is
    // past `old` however long the thread took to start.
    let began = Instant::now();
    working.take(folders.clone());
    assert!(working.unread(), "a reading not yet answered was said");
    assert_eq!(told.try_iter().count(), 0, "a young first wait was said");
    std::thread::sleep(old.saturating_sub(began.elapsed()) + BRISK);
    let late = Instant::now();
    working.take(folders.clone());
    let waited = late.elapsed();
    assert!(
        waited < SAMPLED / 2,
        "a take after the first waited {waited:?}"
    );
    let said: Vec<String> = told.try_iter().collect();
    let unread = " s, so what works in each repository is said unread until it answers";
    let seconds = said
        .first()
        .and_then(|said| said.strip_prefix("no reading of /proc has answered in "))
        .and_then(|said| said.strip_suffix(unread))
        .and_then(|seconds| seconds.parse::<u64>().ok());
    // It grew old, and at most a late wake more.
    assert!(
        said.len() == 1
            && seconds
                .is_some_and(|seconds| (old.as_secs()..=(old + LATE).as_secs()).contains(&seconds)),
        "the journal wasn't told once that no reading answered: {said:?}"
    );

    // Freed, the held walk lands already old, and the next answers: said
    // once, and not as answering again, as none had.
    drop(free);
    let answers = |said: &String| said.starts_with("the reading of /proc answers");
    let mut heard = Vec::new();
    let until = Instant::now() + LATE;
    while !heard.iter().any(answers) && Instant::now() < until {
        std::thread::sleep(BRISK);
        working.take(folders.clone());
        heard.extend(told.try_iter());
    }
    for _ in 0..5 {
        std::thread::sleep(BRISK);
        working.take(folders.clone());
    }
    heard.extend(told.try_iter());
    // Every age line after the freeing, so landing old says nothing again.
    let answered: Vec<&String> = heard.iter().filter(|said| of_the_reading(said)).collect();
    assert_eq!(
        answered,
        ["the reading of /proc answers at last"],
        "the journal wasn't told once that the first reading answered: {heard:?}"
    );
}

/// Whether `said` is a line of the reading's age, as `take` says it,
/// rather than the share's, as the thread says it of each walk.
fn of_the_reading(said: &str) -> bool {
    [
        "no reading of /proc has answered in ",
        "the reading of /proc is ",
        "the reading of /proc answers ",
    ]
    .iter()
    .any(|age| said.starts_with(age))
}

/// How old a reading may grow in a case whose first take must wait out
/// SAMPLED and still be young: a wake a whole LATE late leaves it LATE of
/// room yet.
const YOUNG_PAST_A_WAIT: Duration = SAMPLED.saturating_add(LATE.saturating_mul(2));

#[test]
fn a_first_reading_that_lands_old_is_said_as_none_answering_and_then_at_last() {
    // A first walk held past `old` and freed between two takes lands
    // already old: none has answered yet, so the journal says so, as the
    // next walk's answer says it answers at last. The second walk holds
    // until the case has taken the first, telling `landed` that the
    // first was put in the slot. Each walk reads nothing, so the thread
    // says the share unmeasured from the second on, which the case
    // doesn't judge. The first walk's age counts from its own start, so
    // the case times `old` from when the walk says it began, however late
    // its thread started.
    let old = YOUNG_PAST_A_WAIT;
    let (walking, walked) = mpsc::channel::<Instant>();
    let (free, freed) = mpsc::channel::<()>();
    let (again, freed_again) = mpsc::channel::<()>();
    let (landed, heard_landed) = mpsc::channel::<()>();
    let mut calls = 0_u32;
    let (tell, told) = mpsc::channel();
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start_saying(
        BRISK,
        old,
        folders.clone(),
        move |_: &[PathBuf]| {
            calls += 1;
            if calls == 1 {
                let _ = walking.send(Instant::now());
                let _ = freed.recv_timeout(SAMPLED * 8);
            } else if calls == 2 {
                let _ = landed.send(());
                let _ = freed_again.recv_timeout(SAMPLED * 8);
            }
            Sample::default()
        },
        move |said| {
            let _ = tell.send(said);
        },
    );
    working.take(folders.clone());
    assert_eq!(told.try_iter().count(), 0, "a young first wait was said");
    let began = walked
        .recv_timeout(LATE)
        .expect("the first walk never began");
    std::thread::sleep(old.saturating_sub(began.elapsed()) + BRISK);
    drop(free);
    heard_landed
        .recv_timeout(LATE)
        .expect("the first walk never landed");
    working.take(folders.clone());
    let said: Vec<String> = told
        .try_iter()
        .filter(|said| of_the_reading(said))
        .collect();
    assert!(
        said.len() == 1 && said[0].starts_with("no reading of /proc has answered in "),
        "a first reading landing old wasn't said once as none answering: {said:?}"
    );

    drop(again);
    let mut heard = Vec::new();
    let until = Instant::now() + LATE;
    while heard.is_empty() && Instant::now() < until {
        std::thread::sleep(BRISK);
        working.take(folders.clone());
        heard.extend(told.try_iter().filter(|said| of_the_reading(said)));
    }
    assert_eq!(
        heard,
        ["the reading of /proc answers at last"],
        "the journal wasn't told once that a reading answered at last"
    );
}

#[test]
fn a_first_reading_slower_than_a_wait_that_answers_young_says_nothing() {
    // The daemon's commonest slow start: its first walk outlasts the
    // wait, then answers long before it's old, so nothing was late and
    // nothing answers at last. The walk outlasts the wait by a whole late
    // wake, so the first take is sure to find nothing.
    let (tell, told) = mpsc::channel();
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start_saying(
        BRISK,
        ROOMY,
        folders.clone(),
        |_: &[PathBuf]| {
            std::thread::sleep(SAMPLED + LATE);
            Sample::default()
        },
        move |said| {
            let _ = tell.send(said);
        },
    );
    let until = Instant::now() + SAMPLED * 2 + LATE * 2;
    working.take(folders.clone());
    assert!(
        working.unread(),
        "the first take found a walk slower than its wait"
    );
    while working.unread() && Instant::now() < until {
        std::thread::sleep(BRISK);
        working.take(folders.clone());
    }
    assert!(!working.unread(), "the first reading never answered");
    let said: Vec<String> = told
        .try_iter()
        .filter(|said| of_the_reading(said))
        .collect();
    assert!(
        said.is_empty(),
        "a young answer after a wait was said: {said:?}"
    );
}

#[test]
fn a_folder_first_looked_in_has_no_share_until_a_second_reading_has() {
    // A process working there before was placed nowhere by the reading
    // before, so it has no time to measure its share from, and saying it
    // idle would read as a measurement. Once a reading places a process,
    // each after it waits on `free`, so the case sees that one.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let calls = std::sync::atomic::AtomicU64::new(0);
    let mut placed = false;
    let folder = PathBuf::from("/home/someone/trees/fix");
    let mut working = Working::start(BRISK, ROOMY, Vec::new(), move |folders: &[PathBuf]| {
        if placed {
            let _ = freed.lock().map(|freed| freed.recv_timeout(ROOMY));
        }
        placed = !folders.is_empty();
        let call = calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Sample {
            at: 100 + call * 100,
            awake: Some(100 + call * 100),
            processes: vec![Process {
                pid: 1,
                parent: 0,
                folder: folders.first().cloned(),
                started: 1,
                own: call * 50,
                children: 0,
                memory: Some(4_096),
            }],
        }
    });
    let _ = working.read(Vec::new());
    let until = Instant::now() + LATE;
    let _ = working.read(vec![folder.clone()]);
    while !working.used.contains_key(&folder) && Instant::now() < until {
        std::thread::sleep(BRISK);
        let _ = working.read(vec![folder.clone()]);
    }
    let used = working
        .used
        .get(&folder)
        .expect("the folder handed on was never read");
    assert!(working.measured, "the readings measured no share at all");
    assert_eq!(
        used.cpu, None,
        "a folder's first reading said a share it had nothing to measure from"
    );
    drop(free);
}

#[test]
fn a_reading_stuck_in_proc_never_holds_up_the_loop() {
    // A process holding its memory map locked holds up a read of its
    // smaps_rollup until it lets go, which may be never. The first
    // reading answers, and each after it waits on `free`.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let calls = std::sync::atomic::AtomicU64::new(0);
    // Placed in the record's own folder, so only the stale reading can
    // leave it unsaid.
    let (at, record) = one_checkout("stuck");
    let folders = folders_of(&record);
    // Old only after a late wake's whole margin, so the first reading is
    // taken however late this thread wakes, and stale well before the
    // held reading is freed.
    let old = LATE;
    let mut working = Working::start(BRISK, old, folders.clone(), move |folders: &[PathBuf]| {
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
    // What the loop is told: once as the reading grows stale, once as it
    // answers again, and nothing as the first is taken.
    let mut said = Vec::new();
    said.extend(working.read(folders.clone()));
    assert_eq!(working.used.len(), 1, "the first reading wasn't taken");

    // Only the first is waited on: each after takes what's there, or
    // nothing, until the last grows old.
    let until = Instant::now() + old + LATE;
    let mut slowest = Duration::ZERO;
    while !working.used.is_empty() && Instant::now() < until {
        let began = Instant::now();
        said.extend(working.read(folders.clone()));
        slowest = slowest.max(began.elapsed());
        std::thread::sleep(Duration::from_millis(10));
    }
    // Taken again while it's still stuck, it says nothing more, so a
    // stale line said twice is caught here whatever the freed loop below
    // races to.
    said.extend(working.read(folders.clone()));
    assert!(
        slowest < SAMPLED / 2,
        "a stuck reading held up the loop {slowest:?}"
    );
    assert!(
        working.used.is_empty() && !working.measured,
        "what was read before a reading stuck is still said as now"
    );
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
        said.extend(working.read(folders.clone()));
    }
    assert_eq!(working.used.len(), 1, "a reading freed again wasn't taken");
    assert!(
        !working.measured,
        "a reading held up measured a share of time that wasn't its own"
    );
    // The late line whole, so none names what held it up, and its age
    // from `old` to a late wake past it.
    let late = " s old with none newer, so what works in each repository is said unread until it \
                answers";
    let seconds = said
        .first()
        .and_then(|said| said.strip_prefix("the reading of /proc is "))
        .and_then(|said| said.strip_suffix(late))
        .and_then(|seconds| seconds.parse::<u64>().ok());
    assert!(
        said.len() == 2
            && seconds
                .is_some_and(|seconds| (old.as_secs()..=(old + LATE).as_secs()).contains(&seconds))
            && said[1] == "the reading of /proc answers again",
        "the journal wasn't told once as the reading stuck and once as it answered: {said:?}"
    );
}

/// A pace long enough that a burst of takings falls well within one.
const SLOW: Duration = Duration::from_millis(200);

/// A reading by call `call` of one process working in `folders`' first,
/// its memory saying which call read it.
fn read_by(call: u64, folders: &[PathBuf]) -> Sample {
    Sample {
        at: 100 + call,
        awake: None,
        processes: vec![Process {
            pid: 1,
            parent: 0,
            folder: folders.first().cloned(),
            started: 1,
            own: 0,
            children: 0,
            memory: Some(4_096 * (call + 1)),
        }],
    }
}

#[test]
fn a_reading_is_said_until_it_is_stale_paces_old_however_often_it_is_taken() {
    // The loop takes the reading after each reading of the disk, which may
    // come every few milliseconds or every few minutes, so a reading's age
    // is its own: counted in takings, it would go unsaid early on a quiet
    // disk and late on a busy one. Each reading after the first is stuck.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let calls = std::sync::atomic::AtomicU64::new(0);
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    // Old only past a late wake's whole margin, so the takings below
    // see the first reading fresh however late this thread wakes.
    let old = SLOW * STALE + LATE;
    let mut working = Working::start(SLOW, old, folders.clone(), move |folders: &[PathBuf]| {
        let call = calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        if call > 0 {
            let _ = freed.lock().map(|freed| freed.recv_timeout(old * 4));
        }
        read_by(call, folders)
    });
    let _ = working.read(folders.clone());
    for _ in 0..10 {
        let _ = working.read(folders.clone());
    }
    assert_eq!(
        working.used.len(),
        1,
        "a reading taken often went unsaid before it was {old:?} old"
    );
    std::thread::sleep(old);
    let _ = working.read(folders.clone());
    assert!(
        working.used.is_empty() && working.unread(),
        "a reading {old:?} old, taken once, is still said as now"
    );
    drop(free);
}

#[test]
fn a_reading_left_waiting_past_stale_paces_is_never_said_as_now() {
    // On a disk slower than a pace the loop takes a reading only as each
    // reading of the disk ends, by when the one waiting may be old. The
    // second reading answers, and each after it is stuck.
    let (free, freed) = mpsc::channel::<()>();
    let freed = std::sync::Mutex::new(freed);
    let calls = std::sync::atomic::AtomicU64::new(0);
    let landed = std::sync::Arc::new(std::sync::Mutex::new(None));
    let landing = std::sync::Arc::clone(&landed);
    let folders = vec![PathBuf::from("/home/someone/trees/fix")];
    let mut working = Working::start(
        SLOW,
        SLOW * STALE,
        folders.clone(),
        move |folders: &[PathBuf]| {
            let call = calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if call > 1 {
                let _ = freed
                    .lock()
                    .map(|freed| freed.recv_timeout(SLOW * STALE * 4));
            }
            if call == 1 {
                let _ = landing
                    .lock()
                    .map(|mut landed| *landed = Some(Instant::now()));
            }
            read_by(call, folders)
        },
    );
    let _ = working.read(folders.clone());
    let until = Instant::now() + SLOW * 2 + LATE;
    let second = loop {
        let second = landed.lock().ok().and_then(|landed| *landed);
        if second.is_some() || Instant::now() > until {
            break second.expect("the second reading never answered");
        }
        std::thread::sleep(BRISK);
    };
    // Its age counts from before its walk, so past this it's stale
    // however late the thread stamped it.
    std::thread::sleep((second + SLOW * STALE + SLOW).saturating_duration_since(Instant::now()));
    let _ = working.read(folders.clone());
    assert!(
        working.used.is_empty() && working.unread(),
        "a reading that waited {STALE} paces to be taken is said as now: {:?}",
        working.used
    );
    drop(free);
}

/// A case of the journal: what it's called, the reading before, the reading
/// now, whether the readings before measured a share, and what the journal
/// says.
type Case = (&'static str, Sample, Sample, Option<bool>, Option<String>);

#[test]
fn the_reading_thread_says_a_lost_share_once_and_its_return_once() {
    // The thread's own journal, heard over a run of readings: two slow
    // walks and the reading after them lose the share, and every reading
    // after measures it, so it's said once lost and once measured again.
    let (tell, told) = mpsc::channel();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicU64::new(0));
    let counted = std::sync::Arc::clone(&calls);
    let working = Working::start_saying(
        BRISK,
        ROOMY,
        Vec::new(),
        move |_: &[PathBuf]| {
            let call = counted.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            Sample {
                at: 100 + call * 100,
                awake: (!matches!(call, 2 | 3)).then_some(100 + call * 100),
                processes: Vec::new(),
            }
        },
        move |said| {
            let _ = tell.send(said);
        },
    );
    let mut said = Vec::new();
    let until = Instant::now() + BRISK * 10 + LATE;
    while said.len() < 2 && Instant::now() < until {
        said.extend(told.recv_timeout(BRISK).ok());
    }
    // Seven more readings judged with nothing more said, counted rather
    // than timed, so a starved builder can't pass it having read none:
    // the thirteenth walk starts only once the twelfth is judged.
    let until = Instant::now() + BRISK * 20 + LATE;
    let read = || calls.load(std::sync::atomic::Ordering::Relaxed);
    while read() < 13 && Instant::now() < until {
        std::thread::sleep(BRISK);
    }
    said.extend(told.try_iter());
    let judged = read();
    drop(working);
    assert!(
        judged >= 13,
        "only {judged} walks began, too few to hear that nothing more was said"
    );
    assert_eq!(
        said,
        [
            "the share of a core goes unmeasured, as this walk of /proc took over 2 s",
            "the share of a core is measured again",
        ],
    );
}

#[test]
fn the_journal_says_each_loss_and_each_return_once() {
    // The thread case above hears one loss and its end; over a longer
    // run the step holds what each reading measured, so a second loss
    // after a return is said again, and its end again, which a journal
    // that said a loss once ever would miss. A slow walk loses the share
    // for itself and the reading after it.
    let one = Process {
        pid: 1,
        parent: 0,
        folder: None,
        started: 1,
        own: 0,
        children: 0,
        memory: None,
    };
    let read = |awake: Option<u64>| Sample {
        at: 100,
        awake,
        processes: vec![one.clone()],
    };
    let run = [
        read(Some(100)),
        read(Some(200)),
        read(None),
        read(Some(300)),
        read(Some(400)),
        read(None),
        read(Some(500)),
        read(Some(600)),
    ];
    let lost = || {
        Some("the share of a core goes unmeasured, as this walk of /proc took over 2 s".to_owned())
    };
    let again = || Some("the share of a core is measured again".to_owned());
    let mut journal = Journal::default();
    let said: Vec<Option<String>> = (0..run.len())
        .map(|at| journal.step(at.checked_sub(1).map(|before| &run[before]), &run[at]))
        .collect();
    assert_eq!(
        said,
        [None, None, lost(), None, again(), lost(), None, again()]
    );
}

#[test]
fn the_journal_says_why_a_share_goes_unmeasured_and_only_what_changed() {
    let one = Process {
        pid: 1,
        parent: 0,
        folder: None,
        started: 1,
        own: 0,
        children: 0,
        memory: None,
    };
    let read = |awake: Option<u64>| Sample {
        at: 100,
        awake,
        processes: vec![one.clone()],
    };
    let unmeasured = |why: &str| Some(format!("the share of a core goes unmeasured, as {why}"));
    let cases: [Case; 9] = [
        (
            "measured from the start",
            read(Some(100)),
            read(Some(200)),
            None,
            None,
        ),
        (
            "measured again",
            read(Some(100)),
            read(Some(200)),
            Some(false),
            Some("the share of a core is measured again".to_owned()),
        ),
        (
            "no /proc",
            read(Some(100)),
            Sample::default(),
            None,
            unmeasured("/proc couldn't be read"),
        ),
        (
            "still measured",
            read(Some(100)),
            read(Some(200)),
            Some(true),
            None,
        ),
        (
            "still unmeasured",
            read(Some(100)),
            read(None),
            Some(false),
            None,
        ),
        (
            "a slow walk",
            read(Some(100)),
            read(None),
            None,
            unmeasured("this walk of /proc took over 2 s"),
        ),
        (
            "a slow walk before",
            read(None),
            read(Some(200)),
            None,
            unmeasured("the walk of /proc before this one wasn't read whole within 2 s"),
        ),
        (
            "no time",
            read(Some(100)),
            read(Some(100)),
            Some(true),
            unmeasured("no waking time passed since the reading before"),
        ),
        (
            "too long",
            read(Some(100)),
            read(Some(100 + LONGEST_SHARE + 1)),
            Some(true),
            unmeasured(
                "more than 180 s of waking time passed since the reading before, the most a \
                 share is measured over",
            ),
        ),
    ];
    for (case, before, now, was, said) in cases {
        assert_eq!(measures(Some(&before), &now, was), said, "{case}");
    }
    assert_eq!(
        measures(None, &read(None), None),
        None,
        "a first reading, with none to measure from, said why"
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

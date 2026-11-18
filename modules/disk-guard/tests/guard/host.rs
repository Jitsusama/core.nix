use crate::procs::{LOCKS, link, stat, text};
use disk_guard::activity::Use;
use disk_guard::band::{Band, Thresholds};
use disk_guard::host::{Elsewhere, Filesystem, Reclaim, Scan};
use disk_guard::pace::Pacer;
use disk_guard::pass::{Disk, Outcome, Reclaimer};
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::Looker;
use disk_guard::survey::{Action, Offer};
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::time::{Duration, SystemTime};

/// A readable process sitting in `cwd`, in a fake `/proc`.
pub(crate) fn sitting(proc: &Path, pid: u32, cwd: &str) {
    let at = |file: &str| format!("{pid}/{file}");
    text(proc, &at("stat"), &stat(pid, "sh", 0, 100));
    text(
        proc,
        &at("status"),
        "Name:\tsh\nUid:\t1000\t1000\t1000\t1000\n",
    );
    text(proc, &at("cmdline"), "sh\0");
    text(proc, &at("maps"), "");
    link(proc, &at("cwd"), cwd);
    link(proc, &at("exe"), "/bin/sh");
    std::fs::create_dir_all(proc.join(at("fd"))).unwrap();
}

pub(crate) const TARGET: &str = "/home/u/src/app/target";

#[test]
fn a_scan_is_reused_until_something_is_moved() {
    let proc = tempfile::tempdir().unwrap();
    let mut scan = Scan::new(
        proc.path(),
        &proc.path().join("locks"),
        42,
        Duration::from_secs(3600),
    );
    let target = Path::new(TARGET);
    assert_eq!(scan.judge(target, None).unwrap(), Use::Free);
    sitting(proc.path(), 100, "/home/u/src/app/target/debug");
    assert_eq!(
        scan.judge(target, None).unwrap(),
        Use::Free,
        "the scan was reused"
    );
    scan.moved();
    assert_eq!(scan.judge(target, None).unwrap(), Use::Busy(100));
}

#[test]
fn a_scan_older_than_its_freshness_is_taken_again() {
    let proc = tempfile::tempdir().unwrap();
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    let target = Path::new(TARGET);
    assert_eq!(scan.judge(target, None).unwrap(), Use::Free);
    sitting(proc.path(), 100, "/home/u/src/app/target/debug");
    assert_eq!(scan.judge(target, None).unwrap(), Use::Busy(100));
}

#[test]
fn a_scan_is_trusted_from_when_it_ended() {
    // On 12 November a scan of 1,900 processes took 1.8 s, longer than the
    // second a scan is trusted for, so counted from its start every scan
    // was stale by its first use and each entry paid for two.
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::OpenOptionsExt;
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 100, "/home/u");
    // A stat read from a pipe makes the scan as slow as its writer.
    let slow = proc.path().join("100/stat");
    fs::remove_file(&slow).unwrap();
    let name = std::ffi::CString::new(slow.as_os_str().as_bytes()).unwrap();
    // SAFETY: name is a valid C string naming a path in a temp directory.
    let made = unsafe { libc::mkfifo(name.as_ptr(), 0o600) };
    assert_eq!(made, 0, "the slow stat couldn't be made");
    let writing = slow.clone();
    let writer = std::thread::spawn(move || {
        // Twice the scan's freshness, so a scan counted from its start is
        // already stale when it ends.
        std::thread::sleep(Duration::from_millis(600));
        // Once for the scan, waiting for it to read.
        let _ = fs::write(&writing, stat(100, "sh", 0, 100));
        // Then to any reader for a second, for a second scan that
        // shouldn't come, so a failing one ends rather than hangs. A
        // blocking open here hung the suite when, under load, the test's
        // own reader came and went before it.
        // A moment first, so the first scan has read to its end and the
        // spare stat can't join its read.
        std::thread::sleep(Duration::from_millis(100));
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        while std::time::Instant::now() < deadline {
            let opened = fs::OpenOptions::new()
                .write(true)
                .custom_flags(libc::O_NONBLOCK)
                .open(&writing);
            if let Ok(mut fifo) = opened {
                let _ = std::io::Write::write_all(&mut fifo, stat(100, "sh", 0, 100).as_bytes());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        // A second scan later than that finds no stat and fails, rather
        // than waiting on a fifo nobody writes.
        let _ = fs::remove_file(&writing);
    });
    let mut scan = Scan::new(
        proc.path(),
        &proc.path().join("locks"),
        42,
        Duration::from_millis(300),
    );
    let target = Path::new(TARGET);
    assert_eq!(
        scan.judge(target, None).unwrap(),
        Use::Free,
        "nothing sat in it"
    );
    sitting(proc.path(), 200, "/home/u/src/app/target/debug");
    let second = scan.judge(target, None).unwrap();
    writer.join().unwrap();
    assert_eq!(
        second,
        Use::Free,
        "the scan ended just now, so it was reused"
    );
}

/// A stop waits for the guard to take it, and systemd kills a guard that
/// keeps it waiting more than five seconds, as a scan of a busy desktop
/// can. The stop is this thread's alone, and taken back before it ends.
#[test]
fn a_stop_ends_a_scan_and_its_judgement_fails() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 100, "/home/u");
    let judged = std::thread::spawn(move || {
        let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
        // SAFETY: set is a valid sigset_t; the signal goes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            libc::pthread_kill(libc::pthread_self(), libc::SIGTERM);
            let judged = scan
                .judge(Path::new(TARGET), None)
                .map_err(|error| error.kind());
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            judged
        }
    })
    .join()
    .unwrap();
    assert_eq!(
        judged,
        Err(std::io::ErrorKind::Interrupted),
        "the scan went on with a stop waiting"
    );
}

#[test]
fn the_guard_itself_judges_nothing() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 42, "/home/u/src/app/target");
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Free);
}

#[test]
fn waiters_are_read_from_the_lock_listing() {
    let proc = tempfile::tempdir().unwrap();
    text(proc.path(), "locks", LOCKS);
    let mut scan = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    assert_eq!(scan.waiters(&[7]).unwrap(), [500]);
}

/// A process of root's this account can't read, in a fake `/proc`. What
/// the system's guard says of such processes is proven in `unread`.
pub(crate) fn hidden(proc: &Path, pid: u32, start: u64) {
    let at = |file: &str| format!("{pid}/{file}");
    text(proc, &at("stat"), &stat(pid, "rustc", 0, start));
    text(proc, &at("status"), "Name:\trustc\nUid:\t0\t0\t0\t0\n");
    text(proc, &at("cmdline"), "");
}

fn offer(path: &Path, class: Class, action: Action) -> Offer {
    Offer {
        candidate: Candidate {
            path: path.to_owned(),
            class,
            idle: 0.0,
            size: 0,
            occupied: false,
            spent: false,
            checkout: None,
        },
        action,
        device: 0,
    }
}

/// A scan of a `/proc` with nothing in it, so nothing is in use, that
/// reads the real lock listing as this process, which takes the locks.
fn empty_scan(proc: &Path) -> Scan {
    Scan::new(
        proc,
        Path::new("/proc/locks"),
        std::process::id(),
        Duration::ZERO,
    )
}

#[test]
fn each_offer_is_reclaimed_as_its_action_says() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let target = crate::reclaim::built(&root);
    let crate_dir = root.join("app/target/debug/incremental/app-1a2b3c");
    crate::locks::session(&crate_dir, SystemTime::now() - Duration::from_secs(3600));
    let scratch = root.join("scratch/run-1");
    fs::create_dir_all(&scratch).unwrap();
    fs::write(scratch.join("out"), "x").unwrap();
    let trash = root.join("Trash");
    fs::create_dir_all(trash.join("files")).unwrap();
    fs::create_dir_all(trash.join("info")).unwrap();
    fs::write(trash.join("files/old.txt"), "old").unwrap();
    fs::write(trash.join("info/old.txt.trashinfo"), "[Trash Info]\n").unwrap();
    let offers = [
        offer(
            &crate_dir,
            Class::Variant,
            Action::Variant { root: root.clone() },
        ),
        offer(
            &target,
            Class::Target,
            Action::Target { root: root.clone() },
        ),
        offer(
            &scratch,
            Class::Scratch { healthy: None },
            Action::Entry {
                root: root.join("scratch"),
            },
        ),
        offer(
            &trash.join("files/old.txt"),
            Class::Trash,
            Action::Trash {
                trash: trash.clone(),
            },
        ),
    ];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    for offered in &offers {
        assert_eq!(
            reclaim.reclaim(&offered.candidate),
            Outcome::Reclaimed,
            "{:?}",
            offered.action
        );
    }
    assert!(!crate_dir.exists());
    assert_eq!(
        fs::read_dir(&target).unwrap().count(),
        1,
        "only the benchmarks are left"
    );
    assert!(!scratch.exists());
    assert!(!trash.join("files/old.txt").exists());
    assert!(!trash.join("info/old.txt.trashinfo").exists());
}

#[test]
fn a_cache_entry_goes_whoever_the_guard_cant_read() {
    // On 10 November the account's guard kept half of Go's build cache as
    // "process 1 can't be read", and scanned every process twice for each
    // entry it judged. Go trims its own cache by age alone, and a process
    // reading an entry keeps reading it once it's gone.
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 1, 100);
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let entry = root
        .join("go-build/3f")
        .join(format!("{}-d", "3f".repeat(32)));
    fs::create_dir_all(entry.parent().unwrap()).unwrap();
    fs::write(&entry, "output").unwrap();
    let offers = [offer(
        &entry,
        Class::Cache {
            rank: 40,
            from: disk_guard::band::Band::Healthy,
        },
        Action::CacheEntry {
            root: root.join("go-build"),
        },
    )];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    assert_eq!(reclaim.reclaim(&offers[0].candidate), Outcome::Reclaimed);
    assert!(!entry.exists(), "the entry is still there");
    assert!(
        entry.parent().unwrap().exists(),
        "Go's own directory went too"
    );
}

// On 16 November a pass begun hours before took a live session's script.
// A live quest's tmp/ entry is read whole again just before it goes, and
// kept if anything in it was written since its survey, or it holds work
// now; otherwise it goes as any entry does.
#[test]
fn a_live_quests_entry_is_read_again_before_it_goes() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let offer_for =
        |name: &str, root: &Path, action: Action| offer(&root.join(name), Class::QuestTemp, action);
    let surveyed = |name: &str| {
        let lab = root.join(name);
        fs::create_dir_all(lab.join("out")).unwrap();
        fs::write(lab.join("out/run.log"), "x").unwrap();
        let written = match disk_guard::scratch::live_reading(&lab) {
            disk_guard::scratch::Live::Written(written) => written,
            disk_guard::scratch::Live::Holds(why) => {
                panic!("{} was held at its survey: {why}", lab.display())
            }
        };
        let action = Action::LiveEntry {
            root: root.clone(),
            written,
        };
        (lab, offer_for(name, &root, action))
    };
    let (left, left_offer) = surveyed("left");
    let (back, back_offer) = surveyed("back");
    let (cloned, cloned_offer) = surveyed("cloned");
    // After the survey: the session writes deep in one, clones into another.
    std::thread::sleep(Duration::from_millis(20));
    fs::write(back.join("out/run.log"), "more").unwrap();
    fs::create_dir_all(cloned.join("out/repo/.git")).unwrap();
    let offers = [left_offer, back_offer, cloned_offer];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    let outcomes: Vec<_> = offers
        .iter()
        .map(|offer| reclaim.reclaim(&offer.candidate))
        .collect();
    assert_eq!(outcomes[0], Outcome::Reclaimed, "the left lab stayed");
    assert!(!left.exists(), "the left lab is still there");
    assert!(
        matches!(&outcomes[1], Outcome::Kept(why) if why.contains("written since")),
        "a lab written since its survey wasn't kept for it: {:?}",
        outcomes[1]
    );
    assert!(
        matches!(&outcomes[2], Outcome::Kept(why) if why.contains("repository")),
        "a lab cloned into since its survey wasn't kept for it: {:?}",
        outcomes[2]
    );
    assert!(
        back.join("out/run.log").exists() && cloned.join("out/repo/.git").exists(),
        "a kept lab lost its files: the written one or the clone is gone"
    );
}

#[test]
fn a_folder_offered_as_a_cache_entry_is_refused_untouched() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let folder = root.join("go-build/3f");
    fs::create_dir_all(&folder).unwrap();
    fs::write(folder.join("kept"), "x").unwrap();
    let offers = [offer(
        &folder,
        Class::Cache {
            rank: 40,
            from: disk_guard::band::Band::Healthy,
        },
        Action::CacheEntry {
            root: root.join("go-build"),
        },
    )];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    let outcome = reclaim.reclaim(&offers[0].candidate);
    assert!(
        matches!(outcome, Outcome::Failed(_)),
        "a folder wasn't refused: {outcome:?}"
    );
    assert!(folder.join("kept").exists(), "the folder's file went");
}

#[test]
fn a_workspace_goes_and_its_worktrees_are_pruned() {
    use crate::finished::git;
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let (repo, workspace) = crate::workspaces::linked_fixture(&root);
    let offers = [offer(
        &workspace,
        Class::Workspace,
        Action::Workspace { root: root.clone() },
    )];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    assert_eq!(reclaim.reclaim(&offers[0].candidate), Outcome::Reclaimed);
    assert!(!workspace.exists());
    let listed = git(&repo, &["worktree", "list", "--porcelain"]);
    assert_eq!(listed.matches("worktree ").count(), 1, "{listed}");
    assert_eq!(reclaim.notes(), [] as [String; 0]);
}

#[test]
fn a_log_is_trimmed_to_its_last_part_in_place() {
    use disk_guard::logs::KEEP;
    use std::io::Write;
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let root = fs::canonicalize(root.path()).unwrap();
    let log = root.join("dev/run.log");
    fs::create_dir_all(log.parent().unwrap()).unwrap();
    let mut writer = std::io::BufWriter::new(fs::File::create(&log).unwrap());
    let lines = (KEEP + (1 << 20)) / 16;
    for n in 0..lines {
        writeln!(writer, "line {n:010}").unwrap();
    }
    drop(writer);
    let offers = [offer(&log, Class::Log, Action::Trim { root: root.clone() })];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    assert_eq!(reclaim.reclaim(&offers[0].candidate), Outcome::Reclaimed);
    let kept = fs::read_to_string(&log).unwrap();
    assert!(kept.len() as u64 <= KEEP, "{}", kept.len());
    assert!(kept.starts_with("line "), "from a whole line");
    assert!(kept.ends_with(&format!("line {:010}\n", lines - 1)));
}

fn run(argv: &[&str], timeout: Duration) -> (Offer, Outcome, Duration) {
    run_paced(argv, timeout, None)
}

fn run_paced(
    argv: &[&str],
    timeout: Duration,
    pacer: Option<Rc<RefCell<Pacer>>>,
) -> (Offer, Outcome, Duration) {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let offers = [offer(
        &root.path().join("#gc"),
        Class::Producer {
            every: None,
            from: Some(disk_guard::band::Band::Filling),
        },
        Action::Run {
            root: root.path().to_owned(),
            source: "gc".to_owned(),
            argv: argv.iter().map(ToString::to_string).collect(),
            timeout,
        },
    )];
    let mut scan = empty_scan(proc.path());
    let reclaim = Reclaim::new(&offers, &mut scan);
    let mut reclaim = match pacer {
        Some(pacer) => reclaim.paced(pacer),
        None => reclaim,
    };
    let started = std::time::Instant::now();
    let outcome = reclaim.reclaim(&offers[0].candidate);
    let [offered] = offers;
    (offered, outcome, started.elapsed())
}

// On 14 November a switch stopped the guard while a pass ran a command,
// and the pass left an error line saying 1 of the 2 it tried failed, the
// last "stopped with the guard": a stop asked for isn't the guard
// failing. The stop is this thread's alone, and taken back before it ends.
#[test]
fn a_command_cut_short_by_the_guards_stop_is_kept_not_failed() {
    let outcome = std::thread::spawn(|| {
        // SAFETY: set is a valid sigset_t; the signal goes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            libc::pthread_kill(libc::pthread_self(), libc::SIGTERM);
            // A command that would outlast the test, so only the stop ends it.
            let (_, outcome, _) = run(&["/bin/sh", "-c", "exec sleep 30"], Duration::from_secs(60));
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            outcome
        }
    })
    .join()
    .unwrap();
    assert!(
        matches!(&outcome, Outcome::Kept(why) if why.contains("stopped with the guard")),
        "a stop asked for was told as {outcome:?}"
    );
}

#[test]
fn a_producer_is_run_and_its_failure_told() {
    let second = Duration::from_secs(1);
    let (_, outcome, _) = run(&["/bin/sh", "-c", "exit 0"], 10 * second);
    assert_eq!(outcome, Outcome::Reclaimed);
    let (_, outcome, _) = run(&["/bin/sh", "-c", "echo no room >&2; exit 3"], 10 * second);
    assert_eq!(outcome, Outcome::Failed("it exited 3: no room".to_owned()));
}

#[test]
fn a_producer_that_takes_its_time_keeps_the_guard_beating() {
    // A guard silent for two minutes reads as dead on the desktop, and a
    // command, a Nix deletion or the system daemon's answer can take that.
    let (pacer, _, beats) = crate::pace::hurried();
    let (_, outcome, _) = run_paced(
        &["/bin/sh", "-c", "exec sleep 0.2"],
        Duration::from_secs(10),
        Some(Rc::new(RefCell::new(pacer))),
    );
    assert_eq!(outcome, Outcome::Reclaimed);
    assert!(beats.get() > 0);
}

#[test]
fn a_hung_producer_is_given_up_on_at_its_timeout() {
    let (_, outcome, took) = run(
        &["/bin/sh", "-c", "exec sleep 30"],
        Duration::from_millis(200),
    );
    assert_eq!(
        outcome,
        Outcome::Failed("it ran past its 200ms and was killed".to_owned())
    );
    assert!(took < Duration::from_secs(5), "{took:?}");
}

#[test]
fn a_candidate_no_offer_names_fails_untouched() {
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let stray = root.path().join("stray");
    fs::create_dir_all(&stray).unwrap();
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&[], &mut scan);
    let named = offer(
        &stray,
        Class::Temp,
        Action::Entry {
            root: root.path().to_owned(),
        },
    );
    assert!(matches!(
        reclaim.reclaim(&named.candidate),
        Outcome::Failed(_)
    ));
    assert!(stray.exists());
}

/// Bands on another filesystem that read `band` whatever it holds.
fn reading(band: Band) -> Thresholds {
    let start = |from: Band| if band >= from { u64::MAX } else { 0 };
    Thresholds {
        filling: start(Band::Filling),
        nearly_full: start(Band::NearlyFull),
        critically_full: start(Band::CriticallyFull),
        target: 0,
        margin: 0,
    }
}

/// Whether a pass in `during` on one filesystem stops for another that
/// began it in `began` and reads `now`.
fn gives_way(during: Band, began: Band, now: Band) -> bool {
    let proc = tempfile::tempdir().unwrap();
    let other = tempfile::tempdir().unwrap();
    let mut scan = empty_scan(proc.path());
    Reclaim::new(&[], &mut scan)
        .during(during)
        .giving_way(vec![Elsewhere {
            path: other.path().to_owned(),
            thresholds: reading(now),
            band: began,
        }])
        .stopping()
}

#[test]
fn a_pass_gives_way_to_a_disk_grown_shorter_of_room_than_its_own() {
    assert!(
        gives_way(Band::Filling, Band::Healthy, Band::CriticallyFull),
        "a pass on a filling disk went on while another fell critically full"
    );
}

#[test]
fn a_pass_keeps_on_while_no_other_disk_is_worse_than_it_was_or_than_its_own() {
    assert!(
        !gives_way(Band::Filling, Band::Healthy, Band::Healthy),
        "a pass gave way to a disk with room"
    );
    assert!(
        !gives_way(Band::Filling, Band::Filling, Band::Filling),
        "a pass gave way to a disk no worse than when it began"
    );
    assert!(
        !gives_way(Band::CriticallyFull, Band::Healthy, Band::NearlyFull),
        "a pass on a critically full disk gave way to one less short of room"
    );
}

#[test]
fn a_filesystem_is_measured_and_settled_where_it_lies() {
    let root = tempfile::tempdir().unwrap();
    let mut disk = Filesystem::new(root.path());
    let space = disk.space().unwrap();
    assert!(space.total > 0 && space.free <= space.total);
    disk.settle().unwrap();
    let before = disk.now();
    std::thread::sleep(Duration::from_millis(5));
    assert!(disk.now() > before);
}

#[test]
fn the_processes_a_survey_judges_by_come_from_the_same_scan() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 100, "/home/u");
    let mut scan = Scan::new(
        proc.path(),
        &proc.path().join("locks"),
        42,
        Duration::from_secs(3600),
    );
    assert_eq!(scan.processes().unwrap().len(), 1);
    sitting(proc.path(), 101, "/home/u");
    assert_eq!(scan.processes().unwrap().len(), 1);
    scan.moved();
    assert_eq!(scan.processes().unwrap().len(), 2);
}

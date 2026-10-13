use crate::procs::{LOCKS, link, stat, text};
use disk_guard::activity::Use;
use disk_guard::host::{Filesystem, Reclaim, Scan};
use disk_guard::pass::{Disk, Outcome, Reclaimer};
use disk_guard::plan::{Candidate, Class};
use disk_guard::reclaim::Looker;
use disk_guard::survey::{Action, Offer};
use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

/// A readable process sitting in `cwd`, in a fake `/proc`.
fn sitting(proc: &Path, pid: u32, cwd: &str) {
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

const TARGET: &str = "/home/u/src/app/target";

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

/// A process of root's this account can't read, in a fake `/proc`.
fn hidden(proc: &Path, pid: u32, start: u64) {
    let at = |file: &str| format!("{pid}/{file}");
    text(proc, &at("stat"), &stat(pid, "rustc", 0, start));
    text(proc, &at("status"), "Name:\trustc\nUid:\t0\t0\t0\t0\n");
    text(proc, &at("cmdline"), "");
}

/// A system daemon answering `answers` asks with `touching`, counting them.
fn vouching(
    socket: &Path,
    touching: Vec<disk_guard::activity::Process>,
    answers: usize,
) -> std::sync::mpsc::Receiver<()> {
    let mut server = disk_guard::socket::Server::bind(socket, Duration::ZERO).unwrap();
    let (sender, asked) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for _ in 0..answers {
            server
                .serve(&mut |_, _, responder| {
                    sender.send(()).unwrap();
                    responder.send(&disk_guard::socket::Reply::Answer(
                        disk_guard::vouch::Answer {
                            vouched: touching.iter().map(|p| (p.pid, p.start_ticks)).collect(),
                            touching: touching.clone(),
                        },
                    ))
                })
                .unwrap();
        }
    });
    asked
}

fn building(pid: u32, start: u64) -> disk_guard::activity::Process {
    disk_guard::activity::Process {
        pid,
        start_ticks: start,
        name: "rustc".into(),
        cwd: Some("/home/u/src/app/target/debug".into()),
        readable: true,
        ..Default::default()
    }
}

fn vouched_scan(proc: &Path, socket: &Path, fresh: Duration) -> Scan {
    Scan::new(proc, &proc.join("locks"), 42, fresh)
        .vouched_by(socket, &[Path::new("/home/u/src").to_owned()])
}

#[test]
fn a_process_this_account_cant_read_is_judged_by_what_the_system_says() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let mut alone = Scan::new(proc.path(), &proc.path().join("locks"), 42, Duration::ZERO);
    assert_eq!(
        alone.judge(Path::new(TARGET), None).unwrap(),
        Use::Unknown(300)
    );
    let socket = proc.path().join("socket");
    let _asked = vouching(&socket, vec![building(300, 77)], 1);
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
}

#[test]
fn the_systems_word_is_kept_as_long_as_a_scan_is_fresh() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    let asked = vouching(&socket, vec![building(300, 77)], 2);
    let mut scan = vouched_scan(proc.path(), &socket, Duration::from_secs(3600));
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    scan.moved();
    hidden(proc.path(), 301, 90);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    assert_eq!(asked.try_iter().count(), 1, "asked once");
    let mut fresh = vouched_scan(proc.path(), &socket, Duration::ZERO);
    fs::remove_dir_all(proc.path().join("300")).unwrap();
    assert_eq!(
        fresh.judge(Path::new(TARGET), None).unwrap(),
        Use::Unknown(301),
        "a process started since the answer"
    );
}

#[test]
fn without_a_system_daemon_what_cant_be_read_stays_unknown() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let mut scan = vouched_scan(proc.path(), &proc.path().join("socket"), Duration::ZERO);
    assert_eq!(
        scan.judge(Path::new(TARGET), None).unwrap(),
        Use::Unknown(300)
    );
}

#[test]
fn an_account_that_reads_everything_never_asks() {
    let proc = tempfile::tempdir().unwrap();
    sitting(proc.path(), 100, "/home/u");
    let socket = proc.path().join("socket");
    let asked = vouching(&socket, vec![], 1);
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Free);
    assert_eq!(asked.try_iter().count(), 0);
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
    assert!(reclaim.notes().is_empty());
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
    let proc = tempfile::tempdir().unwrap();
    let root = tempfile::tempdir().unwrap();
    let offers = [offer(
        &root.path().join("#gc"),
        Class::Producer {
            every: None,
            from: Some(disk_guard::band::Band::Pressure),
        },
        Action::Run {
            root: root.path().to_owned(),
            source: "gc".to_owned(),
            argv: argv.iter().map(|arg| arg.to_string()).collect(),
            timeout,
        },
    )];
    let mut scan = empty_scan(proc.path());
    let mut reclaim = Reclaim::new(&offers, &mut scan);
    let started = std::time::Instant::now();
    let outcome = reclaim.reclaim(&offers[0].candidate);
    let [offered] = offers;
    (offered, outcome, started.elapsed())
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

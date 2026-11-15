//! Processes this account can't read, judged by what the system's guard
//! says of them: asked once a scan, asked again for a newcomer, waited on
//! while it scans, and left unknown when it can't or won't say.

use crate::host::{TARGET, hidden, sitting};
use disk_guard::activity::Use;
use disk_guard::host::Scan;
use disk_guard::reclaim::Looker;
use std::fs;
use std::path::Path;
use std::time::Duration;

/// A system daemon answering `answers` asks with `touching`, counting them.
fn vouching(
    socket: &Path,
    touching: Vec<disk_guard::activity::Process>,
    answers: usize,
) -> std::sync::mpsc::Receiver<()> {
    vouching_each(socket, vec![touching; answers], Duration::ZERO)
}

/// A system daemon answering each ask with the next of `answers`, letting
/// an account ask once `every`, and counting the asks it answers.
fn vouching_each(
    socket: &Path,
    answers: Vec<Vec<disk_guard::activity::Process>>,
    every: Duration,
) -> std::sync::mpsc::Receiver<()> {
    let mut server = disk_guard::socket::Server::bind(socket, every).unwrap();
    let (sender, asked) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut answers = answers.into_iter();
        // A refused ask takes a connection without an answer.
        while answers.len() > 0 {
            server
                .serve(&mut |_, _, responder| {
                    let touching = answers.next().unwrap();
                    sender.send(()).unwrap();
                    responder.send(&disk_guard::socket::Reply::Answer(
                        disk_guard::vouch::Answer {
                            vouched: touching.iter().map(|p| (p.pid, p.start_ticks)).collect(),
                            touching,
                        },
                    ));
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
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    assert_eq!(asked.try_iter().count(), 1, "asked once");
    let mut fresh = vouched_scan(proc.path(), &socket, Duration::ZERO);
    fs::remove_dir_all(proc.path().join("300")).unwrap();
    hidden(proc.path(), 301, 90);
    assert_eq!(
        fresh.judge(Path::new(TARGET), None).unwrap(),
        Use::Unknown(301),
        "a process the system didn't vouch for"
    );
}

#[test]
fn a_process_started_since_the_answer_has_the_system_asked_again() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    let asked = vouching_each(
        &socket,
        vec![
            vec![building(300, 77)],
            vec![building(300, 77), building(301, 90)],
        ],
        Duration::ZERO,
    );
    let mut scan = vouched_scan(proc.path(), &socket, Duration::from_secs(3600));
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    fs::remove_dir_all(proc.path().join("300")).unwrap();
    hidden(proc.path(), 301, 90);
    scan.moved();
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(301));
    assert_eq!(asked.try_iter().count(), 2);
}

#[test]
fn a_process_the_system_cant_vouch_for_has_it_asked_only_once() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    hidden(proc.path(), 302, 95);
    let socket = proc.path().join("socket");
    let asked = vouching(&socket, vec![building(300, 77)], 2);
    let mut scan = vouched_scan(proc.path(), &socket, Duration::from_secs(3600));
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    scan.moved();
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    assert_eq!(asked.try_iter().count(), 1, "asked once");
}

#[test]
fn an_ask_refused_as_too_soon_is_asked_again_once_allowed() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    let asked = vouching_each(
        &socket,
        vec![vec![building(300, 77)], vec![building(300, 77)]],
        Duration::from_millis(500),
    );
    // Another asker of the same account, as a plan beside the daemon is.
    let mut first = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(
        first.judge(Path::new(TARGET), None).unwrap(),
        Use::Busy(300)
    );
    let mut second = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(
        second.judge(Path::new(TARGET), None).unwrap(),
        Use::Busy(300)
    );
    assert_eq!(asked.try_iter().count(), 2);
}

/// A system daemon whose scan takes 3 s, as one of 2,100 processes does on
/// a loaded machine, waited on for its answer rather than given up on.
#[test]
fn a_system_daemon_slow_to_scan_is_waited_for() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    let mut server = disk_guard::socket::Server::bind(&socket, Duration::ZERO).unwrap();
    std::thread::spawn(move || {
        server
            .serve(&mut |_, _, responder| {
                // The scan's own time, which a loaded machine stretches.
                std::thread::sleep(Duration::from_secs(3));
                let touching = vec![building(300, 77)];
                responder.send(&disk_guard::socket::Reply::Answer(
                    disk_guard::vouch::Answer {
                        vouched: vec![(300, 77)],
                        touching,
                    },
                ));
            })
            .unwrap();
    });
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Busy(300));
    assert!(!scan.went_unheard(), "gave up on it: {:?}", scan.unheard());
}

/// A system daemon that finds `pid` gone by the time it scans, as a short
/// build step is once it's been asked about, and so can't vouch for it.
fn outlived(proc: &Path, socket: &Path, pid: u32, then: impl FnOnce(&Path) + Send + 'static) {
    let mut server = disk_guard::socket::Server::bind(socket, Duration::ZERO).unwrap();
    let dir = proc.join(pid.to_string());
    std::thread::spawn(move || {
        let mut then = Some(then);
        server
            .serve(&mut |_, _, responder| {
                fs::remove_dir_all(&dir).unwrap();
                (then.take().unwrap())(dir.parent().unwrap());
                responder.send(&disk_guard::socket::Reply::Answer(
                    disk_guard::vouch::Answer::default(),
                ));
            })
            .unwrap();
    });
}

#[test]
fn a_process_gone_before_the_system_could_vouch_for_it_keeps_nothing() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    outlived(proc.path(), &socket, 300, |_| {});
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Free);
}

#[test]
fn a_pid_taken_again_since_the_scan_is_another_process() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    let socket = proc.path().join("socket");
    // The process the scan read exits and another starts under its pid.
    outlived(proc.path(), &socket, 300, |proc| hidden(proc, 300, 91));
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(scan.judge(Path::new(TARGET), None).unwrap(), Use::Free);
}

#[test]
fn a_process_still_running_that_the_system_didnt_vouch_for_stays_unknown() {
    let proc = tempfile::tempdir().unwrap();
    hidden(proc.path(), 300, 77);
    hidden(proc.path(), 301, 80);
    let socket = proc.path().join("socket");
    outlived(proc.path(), &socket, 300, |_| {});
    let mut scan = vouched_scan(proc.path(), &socket, Duration::ZERO);
    assert_eq!(
        scan.judge(Path::new(TARGET), None).unwrap(),
        Use::Unknown(301)
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

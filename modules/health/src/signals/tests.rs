use std::os::unix::process::ExitStatusExt as _;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use super::*;
use crate::source::Followers;

/// Where the copy of these tests that's stopped says its follower, set
/// only for that copy.
const TOLD_IN: &str = "HEALTH_STOPPED_TELLS_IN";

/// The test the stopped copy runs.
const STOPPED_TEST: &str = "signals::tests::a_health_to_stop";

/// How long any one step may take before the test fails rather than hangs.
const WAITS: Duration = Duration::from_secs(10);

/// Whether the process `id` is gone and waited on, so no signal reaches it.
fn gone(id: u32) -> bool {
    let id = libc::pid_t::try_from(id).expect("a process's id");
    // SAFETY: signal 0 only asks whether the process is there.
    let asked = unsafe { libc::kill(id, 0) };
    asked == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// Which of the stopping signals the process `id` blocks, and which it
/// catches, as its status says; none on a Mac, which has no such status.
fn blocking_and_catching(id: u32) -> (u64, u64) {
    let Ok(status) = std::fs::read_to_string(format!("/proc/{id}/status")) else {
        return (0, 0);
    };
    let stops: u64 = STOPPING.iter().map(|&signal| 1 << (signal - 1)).sum();
    let mask = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
            .map_or(u64::MAX, |mask| mask & stops)
    };
    (mask("SigBlk:"), mask("SigCgt:"))
}

/// What `told` says once it's written: the follower's id, and which of
/// the stops it blocks and catches.
fn told(told: &Path, health: &mut Child) -> (u32, u64, u64) {
    let since = Instant::now();
    loop {
        let said = std::fs::read_to_string(told).unwrap_or_default();
        let words: Vec<u64> = said
            .split_whitespace()
            .filter_map(|word| word.parse().ok())
            .collect();
        if let [id, blocked, caught] = words[..] {
            return (u32::try_from(id).expect("a process's id"), blocked, caught);
        }
        if let Ok(Some(ended)) = health.try_wait() {
            panic!("the health to stop ended before it was stopped: {ended}");
        }
        assert!(
            since.elapsed() < WAITS,
            "the health to stop never said its follower"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

/// The health to stop, run only as the copy that
/// `a_stop_ends_the_followers_then_health_by_its_signal` starts, which
/// says where to say its follower: it catches health's stops, starts a
/// follower and waits to be stopped.
#[test]
fn a_health_to_stop() {
    let Some(told) = std::env::var_os(TOLD_IN) else {
        return;
    };
    let followers = Followers::default();
    ending(followers.ender()).expect("the stops caught");
    let mut sleep = Command::new("sleep");
    sleep.arg("600");
    let follower = followers.following(sleep);
    let (blocked, caught) = blocking_and_catching(follower);
    let saying = Path::new(&told).with_extension("saying");
    std::fs::write(&saying, format!("{follower} {blocked} {caught}")).expect("said");
    std::fs::rename(&saying, &told).expect("said");
    thread::sleep(WAITS * 6);
}

#[test]
fn a_stop_ends_the_followers_then_health_by_its_signal() {
    let folder = tempfile::tempdir().expect("a folder");
    let said = folder.path().join("follower");
    let mut health = Command::new(std::env::current_exe().expect("this test"))
        .args([STOPPED_TEST, "--exact", "--quiet"])
        .env(TOLD_IN, &said)
        .stdout(Stdio::null())
        .spawn()
        .expect("the health to stop starts");
    let (follower, blocked, caught) = told(&said, &mut health);
    // SAFETY: the health to stop is this test's child, not yet waited on.
    unsafe {
        libc::kill(
            libc::pid_t::try_from(health.id()).expect("its id"),
            libc::SIGTERM,
        )
    };
    let since = Instant::now();
    let ended = loop {
        if let Some(ended) = health.try_wait().expect("the health to stop waited on") {
            break ended;
        }
        if since.elapsed() > WAITS {
            let _ = health.kill();
            panic!("a stop left health running");
        }
        thread::sleep(Duration::from_millis(20));
    };
    assert_eq!(
        (ended.signal(), gone(follower)),
        (Some(libc::SIGTERM), true),
        "a stop didn't end health by its signal ({ended}), or left its follower running"
    );
    // Were the stops blocked rather than caught, every follower would block
    // them too, as a run that blocked them showed, and a service manager's
    // stop would wait out its timeout to kill each.
    assert_eq!(
        (blocked, caught),
        (0, 0),
        "a follower started while health catches its stops blocks or catches one"
    );
}

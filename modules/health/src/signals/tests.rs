use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
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

/// How long the stopped copy's ender waits, so its follower's own end by
/// a stop to the whole group comes first, as it may in health.
const ENDS_LATE: Duration = Duration::from_millis(200);

/// How long a hangup the copy was started ignoring is given to end it.
const HANGS_UP: Duration = Duration::from_millis(300);

/// How the copy leaves once its follower ends, unless the stop holds it,
/// as health's own way out after its run.
const LEAVES: i32 = 3;

/// The mask a process's status gives for `signal`.
fn bit(signal: c_int) -> u64 {
    1 << (signal - 1)
}

/// Whether the process `id` is gone and waited on, so no signal reaches it.
fn gone(id: u32) -> bool {
    let id = libc::pid_t::try_from(id).expect("a process's id");
    // SAFETY: signal 0 only asks whether the process is there.
    let asked = unsafe { libc::kill(id, 0) };
    asked == -1 && io::Error::last_os_error().raw_os_error() == Some(libc::ESRCH)
}

/// Which of the stopping signals the process `id` blocks, and which it
/// ignores, as its status says; none on a Mac, which has no such status.
fn blocking_and_ignoring(id: u32) -> Option<(u64, u64)> {
    let status = std::fs::read_to_string(format!("/proc/{id}/status")).ok()?;
    let stops: u64 = STOPPING.iter().copied().map(bit).sum();
    let mask = |name: &str| {
        status
            .lines()
            .find_map(|line| line.strip_prefix(name))
            .and_then(|mask| u64::from_str_radix(mask.trim(), 16).ok())
            .map_or(u64::MAX, |mask| mask & stops)
    };
    Some((mask("SigBlk:"), mask("SigIgn:")))
}

/// The health to stop, with its process group, which ends the whole
/// group should the test fail before the stop does.
struct Stopped {
    health: Child,
    ended: Option<ExitStatus>,
}

impl Stopped {
    fn send(&self, signal: c_int, group: bool) {
        let id = libc::pid_t::try_from(self.health.id()).expect("its id");
        // SAFETY: the health to stop leads its group, and isn't waited on
        // while it hasn't ended.
        unsafe { libc::kill(if group { -id } else { id }, signal) };
    }

    /// How it ended, if it has.
    fn ended(&mut self) -> Option<ExitStatus> {
        if self.ended.is_none() {
            self.ended = self
                .health
                .try_wait()
                .expect("the health to stop waited on");
        }
        self.ended
    }

    /// What `told` says once it's written: the follower's id, and which
    /// of the stops it blocks and ignores.
    fn told(&mut self, told: &Path) -> (u32, Option<(u64, u64)>) {
        let since = Instant::now();
        loop {
            let said = std::fs::read_to_string(told).unwrap_or_default();
            let words: Vec<u64> = said
                .split_whitespace()
                .filter_map(|word| word.parse().ok())
                .collect();
            let follower = |id| u32::try_from(id).expect("a process's id");
            match words[..] {
                [id] => return (follower(id), None),
                [id, blocked, ignored] => return (follower(id), Some((blocked, ignored))),
                _ => {}
            }
            if let Some(ended) = self.ended() {
                panic!("the health to stop ended before it was stopped: {ended}");
            }
            assert!(
                since.elapsed() < WAITS,
                "the health to stop never said its follower"
            );
            thread::sleep(Duration::from_millis(20));
        }
    }

    /// How it ended once stopped.
    fn ending(&mut self) -> ExitStatus {
        let since = Instant::now();
        loop {
            if let Some(ended) = self.ended() {
                return ended;
            }
            assert!(since.elapsed() < WAITS, "a stop left health running");
            thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Stopped {
    fn drop(&mut self) {
        if self.ended().is_none() {
            self.send(libc::SIGKILL, true);
            let _ = self.health.wait();
        }
    }
}

/// The health to stop, run only as the copy that
/// `a_stop_ends_the_followers_then_health_by_its_signal` starts, which
/// says where to say its follower: started as under `nohup`, it catches
/// health's stops, starts a follower, and leaves once its follower ends.
#[test]
#[ignore = "run only as the copy a_stop_ends_the_followers_then_health_by_its_signal stops"]
fn a_health_to_stop() {
    let Some(told) = std::env::var_os(TOLD_IN) else {
        return;
    };
    let told = Path::new(&told);
    // SAFETY: ignoring a signal is safe, and no catch is set yet.
    unsafe { libc::signal(libc::SIGHUP, libc::SIG_IGN) };
    let followers = Followers::default();
    let ender = followers.ender();
    ending(move || {
        thread::sleep(ENDS_LATE);
        ender();
    })
    .expect("the stops caught");
    let said = told.with_extension("said");
    let mut sleep = Command::new("sleep");
    sleep.arg("600");
    let (follower, sent) = followers.following(sleep, move |words| {
        std::fs::write(&said, words).expect("its end said");
    });
    let masks = blocking_and_ignoring(follower).map_or(String::new(), |(blocked, ignored)| {
        format!(" {blocked} {ignored}")
    });
    let saying = told.with_extension("saying");
    std::fs::write(&saying, format!("{follower}{masks}")).expect("said");
    std::fs::rename(&saying, told).expect("said");
    while let Ok(Some(_)) = sent.recv() {}
    await_stop();
    std::process::exit(LEAVES);
}

#[test]
fn a_stop_ends_the_followers_then_health_by_its_signal() {
    let folder = tempfile::tempdir().expect("a folder");
    let told = folder.path().join("follower");
    let health = Command::new(std::env::current_exe().expect("this test"))
        .args([STOPPED_TEST, "--exact", "--ignored", "--quiet"])
        .env(TOLD_IN, &told)
        .process_group(0)
        .stdout(Stdio::null())
        .spawn()
        .expect("the health to stop starts");
    let mut stopped = Stopped {
        health,
        ended: None,
    };
    let (follower, masks) = stopped.told(&told);
    stopped.send(libc::SIGHUP, false);
    thread::sleep(HANGS_UP);
    assert_eq!(
        stopped.ended(),
        None,
        "health ended on a hangup it was started ignoring"
    );
    // To the whole group, as a terminal's interrupt is, so the follower's
    // own end comes too, which says nothing, and health ends by the stop
    // rather than its own way out.
    stopped.send(libc::SIGTERM, true);
    let ended = stopped.ending();
    let said = std::fs::read_to_string(told.with_extension("said")).ok();
    assert_eq!(
        (ended.signal(), gone(follower), said),
        (Some(libc::SIGTERM), true, None),
        "a stop didn't end health by its signal ({ended}), left its follower running, \
         or had it say its end"
    );
    // Were the stops blocked rather than caught, every follower would block
    // them too, as a run that blocked them showed, and a service manager's
    // stop would wait out its timeout to kill each. One health was started
    // ignoring, its follower ignores too.
    if let Some(masks) = masks {
        assert_eq!(
            masks,
            (0, bit(libc::SIGHUP)),
            "a follower started while health catches its stops blocks or ignores one"
        );
    }
}

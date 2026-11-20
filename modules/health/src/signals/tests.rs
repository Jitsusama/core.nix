use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::*;
use crate::source::Followers;

/// Where the copy of these tests that's stopped says its follower, set
/// only for that copy.
const TOLD_IN: &str = "HEALTH_STOPPED_TELLS_IN";

/// Set only for a copy whose run never returns once its follower ends.
const HANGS: &str = "HEALTH_STOPPED_HANGS";

/// How long the stopped copy's run takes to write once its follower
/// ends, so a stop that didn't wait for it would end health first.
const WRITES_LATE: Duration = Duration::from_millis(500);

/// How long a follower stopped on its own has ended before health is
/// stopped, well within the `CAUGHT_WITHIN` it waits to say so.
const ENDED_BEFORE: Duration = Duration::from_millis(20);

/// The test the stopped copy runs.
const STOPPED_TEST: &str = "signals::tests::a_health_to_stop";

/// How long any one step may take before the test fails rather than hangs.
const WAITS: Duration = Duration::from_secs(10);

/// How long the stopped copy's ender waits, so its follower's own end by
/// a stop to the whole group comes first, as it may in health, and a
/// second stop comes while the first's ender runs.
const ENDS_LATE: Duration = Duration::from_secs(1);

/// How long a hangup the copy was started ignoring is given to end it,
/// well past the `ENDS_LATE` in which a caught one would.
const HANGS_UP: Duration = Duration::from_secs(2);

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

/// A copy of these tests run as the health to stop, leading a process
/// group of its own with its follower, both killed should the test fail
/// before a stop ends them.
struct Stopped {
    health: Child,
    ended: Option<ExitStatus>,
    follower: u32,
    /// Which of the stops the follower blocks and ignores, where its
    /// status says.
    masks: Option<(u64, u64)>,
    folder: TempDir,
}

impl Stopped {
    /// The copy started, once it has said its follower.
    fn start() -> Self {
        Self::starting(false)
    }

    /// The copy started, its run never returning once its follower ends
    /// if it `hangs`, once it has said its follower.
    fn starting(hangs: bool) -> Self {
        let folder = tempfile::tempdir().expect("a folder");
        let health = Command::new(std::env::current_exe().expect("this test"))
            .args([STOPPED_TEST, "--exact", "--ignored", "--quiet"])
            .env(TOLD_IN, folder.path().join("follower"))
            .envs(hangs.then_some((HANGS, "")))
            .process_group(0)
            .stdout(Stdio::null())
            .spawn()
            .expect("the health to stop starts");
        let mut stopped = Self {
            health,
            ended: None,
            follower: 0,
            masks: None,
            folder,
        };
        stopped.told();
        stopped
    }

    /// Where the copy says its follower.
    fn told_in(&self) -> PathBuf {
        self.folder.path().join("follower")
    }

    /// What the follower said of its end, if it said anything.
    fn said(&self) -> Option<String> {
        std::fs::read_to_string(self.told_in().with_extension("said")).ok()
    }

    /// Whether the copy's run wrote as it ended.
    fn wrote(&self) -> bool {
        self.told_in().with_extension("written").exists()
    }

    /// Wait for the copy to have caught a stop, as its ender marks.
    fn caught(&self) {
        let since = Instant::now();
        while !self.told_in().with_extension("caught").exists() {
            assert!(
                since.elapsed() < WAITS,
                "the health to stop never caught a stop"
            );
            thread::sleep(Duration::from_millis(5));
        }
    }

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

    /// Wait for the copy to say its follower, and which of the stops it
    /// blocks and ignores.
    fn told(&mut self) {
        let since = Instant::now();
        loop {
            let said = std::fs::read_to_string(self.told_in()).unwrap_or_default();
            let words: Vec<u64> = said
                .split_whitespace()
                .filter_map(|word| word.parse().ok())
                .collect();
            let follower = |id| u32::try_from(id).expect("a process's id");
            match words[..] {
                [id] => {
                    self.follower = follower(id);
                    return;
                }
                [id, blocked, ignored] => {
                    self.follower = follower(id);
                    self.masks = Some((blocked, ignored));
                    return;
                }
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
        let follower = libc::pid_t::try_from(self.follower).expect("its id");
        let group = libc::pid_t::try_from(self.health.id()).expect("its id");
        // SAFETY: getpgid only asks a process's group.
        let the_copys = follower != 0 && unsafe { libc::getpgid(follower) } == group;
        if the_copys {
            // SAFETY: still in the copy's group, so the follower and never
            // another process that took its id.
            unsafe { libc::kill(follower, libc::SIGKILL) };
        }
    }
}

/// The health to stop, run only as the copy `Stopped` starts, which says
/// where to say its follower: started as under `nohup`, it catches
/// health's stops, starts a follower, and once its follower ends, writes
/// a little later, as health writes the day, and leaves, or never returns
/// if it hangs.
#[test]
#[ignore = "run only as the copy the stop tests start"]
fn a_health_to_stop() {
    let Some(told) = std::env::var_os(TOLD_IN) else {
        return;
    };
    let told = Path::new(&told);
    // SAFETY: setting a signal's action is safe, and no catch is set yet.
    // Whatever ran the tests may ignore a stop, as a script's `&` ignores
    // interrupts, so each is set here.
    unsafe {
        libc::signal(libc::SIGTERM, libc::SIG_DFL);
        libc::signal(libc::SIGINT, libc::SIG_DFL);
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
    let followers = Followers::default();
    let ender = followers.ender();
    let caught = told.with_extension("caught");
    // As follow() runs, so this is how health ends.
    let ran = until_stopped(
        move || {
            std::fs::write(&caught, "").expect("the stop marked");
            thread::sleep(ENDS_LATE);
            ender();
        },
        || {
            let said = told.with_extension("said");
            let mut sleep = Command::new("sleep");
            sleep.arg("600");
            let (follower, sent) = followers.following(sleep, move |words| {
                std::fs::write(&said, words).expect("its end said");
            });
            let masks = blocking_and_ignoring(follower)
                .map_or(String::new(), |(blocked, ignored)| {
                    format!(" {blocked} {ignored}")
                });
            let saying = told.with_extension("saying");
            std::fs::write(&saying, format!("{follower}{masks}")).expect("said");
            std::fs::rename(&saying, told).expect("said");
            while let Ok(Some(_)) = sent.recv() {}
            if std::env::var_os(HANGS).is_some() {
                loop {
                    thread::park();
                }
            }
            thread::sleep(WRITES_LATE);
            std::fs::write(told.with_extension("written"), "").expect("its write");
            Err::<(), _>(io::Error::other("the follower ended"))
        },
    );
    std::process::exit(if ran.is_ok() { 0 } else { LEAVES });
}

#[test]
fn a_stop_ends_the_followers_then_health_by_its_signal() {
    let mut stopped = Stopped::start();
    stopped.send(libc::SIGHUP, false);
    thread::sleep(HANGS_UP);
    assert_eq!(
        stopped.ended(),
        None,
        "health ended on a hangup it was started ignoring"
    );
    // To health alone, as `kill` sends it, so only health's own end of its
    // follower ends that.
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), gone(stopped.follower), stopped.wrote()),
        (Some(libc::SIGTERM), true, true),
        "a stop didn't end health by its signal ({ended}), left its follower running, or \
         didn't wait for health's run to write the day"
    );
    // Were the stops blocked rather than caught, every follower would block
    // them too, as a run that blocked them showed (`SigBlk` 0x4003, though
    // std's Command started it), and a service manager's stop would wait
    // out its timeout to kill each. One health was started
    // ignoring, its follower ignores too.
    assert!(
        stopped.masks.is_some() || !cfg!(target_os = "linux"),
        "the follower's masks went unread"
    );
    if let Some(masks) = stopped.masks {
        assert_eq!(
            masks,
            (0, bit(libc::SIGHUP)),
            "a follower started while health catches its stops blocks or ignores one"
        );
    }
}

#[test]
fn a_stop_to_the_whole_group_ends_health_by_it_and_its_follower_unsaid() {
    // As a terminal's interrupt is sent, so the follower ends of it before
    // health's end of it, and health's own way out comes before the stop.
    let mut stopped = Stopped::start();
    stopped.send(libc::SIGTERM, true);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), gone(stopped.follower), stopped.said()),
        (Some(libc::SIGTERM), true, None),
        "a stop to health's group didn't end health by its signal ({ended}), left its \
         follower running, or had it say its end"
    );
}

#[test]
fn a_stop_ends_health_by_its_signal_though_its_run_never_returns() {
    let mut stopped = Stopped::starting(true);
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), stopped.wrote()),
        (Some(libc::SIGTERM), false),
        "a stop waited on a run that never returns ({ended})"
    );
}

#[test]
fn a_follower_that_ends_just_before_health_is_stopped_says_nothing() {
    // As a stop to the whole group may reach the follower and end it
    // before health's catch runs.
    let mut stopped = Stopped::start();
    let follower = libc::pid_t::try_from(stopped.follower).expect("its id");
    // SAFETY: the copy's follower, which it hasn't waited on while it runs.
    unsafe { libc::kill(follower, libc::SIGTERM) };
    thread::sleep(ENDED_BEFORE);
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), stopped.said()),
        (Some(libc::SIGTERM), None),
        "a follower that ended just before health's stop said its end ({ended})"
    );
}

#[test]
fn a_second_stop_ends_health_at_once_leaving_its_follower() {
    // The first stop's end of the followers is late, so a second that
    // waited on it would end health by the first, its follower ended.
    let mut stopped = Stopped::start();
    stopped.send(libc::SIGTERM, false);
    stopped.caught();
    stopped.send(libc::SIGINT, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), gone(stopped.follower)),
        (Some(libc::SIGINT), false),
        "a second stop waited for the first to end health ({ended})"
    );
}

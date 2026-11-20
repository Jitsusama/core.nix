use std::os::unix::process::{CommandExt as _, ExitStatusExt as _};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

use super::*;
use crate::source::{Followers, History};

/// Where the copy of these tests that's stopped says its follower, set
/// only for that copy.
const TOLD_IN: &str = "HEALTH_STOPPED_TELLS_IN";

/// Set only for a copy whose run never returns once its follower ends.
const HANGS: &str = "HEALTH_STOPPED_HANGS";

/// Set only for a copy that replays a history too, as health replays the
/// day, its run waiting on that history's end.
const REPLAYS: &str = "HEALTH_STOPPED_REPLAYS";

/// Set only for a copy whose run returns its error at once, the copy
/// then waiting to be stopped before `main` would say it.
const FAILS_FIRST: &str = "HEALTH_STOPPED_FAILS_FIRST";

/// Set only for a copy whose ender never returns, as one waiting on a
/// follower stuck on a stalled disk.
const ENDS_NEVER: &str = "HEALTH_STOPPED_ENDS_NEVER";

/// Set only for a copy whose run returns once a stop wakes its rest,
/// writing without waiting for its follower's end, as health's run does;
/// a copy whose ender never returns does too.
const RETURNS_AT_ONCE: &str = "HEALTH_STOPPED_RETURNS_AT_ONCE";

/// Set only for a copy started hearing hangups, as in a terminal, rather
/// than ignoring them, as under `nohup`.
const HEARS_HANGUPS: &str = "HEALTH_STOPPED_HEARS_HANGUPS";

/// The error the copy's run returns once its follower ends.
const FAILS_ENDING: &str = "the follower ended";

/// The error the copy's run returns at once, if it fails first.
const FAILS_BEFORE: &str = "the copy's run failed before the stop";

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

/// What a stop that ended health before its run returned says.
const UNWRITTEN: &str = "a stop ended health before it wrote the day";

/// What a stop that ended health before a follower's end says.
const LEFT: &str = "a stop left a follower running";

/// What a second stop says as it ends health.
const SECOND: &str = "a second stop ended health";

/// How far past its bound a stop may end health on a busy builder, well
/// short of a second bound's worth.
const LATE_BY: Duration = Duration::from_secs(1);

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
        Self::starting(&[])
    }

    /// The copy started with each of `asked` set (`HANGS`, `REPLAYS`,
    /// `FAILS_FIRST`, `ENDS_NEVER`, `RETURNS_AT_ONCE` or `HEARS_HANGUPS`),
    /// once it has said its follower.
    fn starting(asked: &[&str]) -> Self {
        let folder = tempfile::tempdir().expect("a folder");
        let health = Command::new(std::env::current_exe().expect("this test"))
            .args([STOPPED_TEST, "--exact", "--ignored", "--quiet"])
            .env(TOLD_IN, folder.path().join("follower"))
            .envs(asked.iter().map(|&name| (name, "")))
            .process_group(0)
            .stdout(Stdio::null())
            .stderr(std::fs::File::create(folder.path().join("stderr")).expect("its stderr"))
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

    /// The history the copy replays, if `REPLAYS`.
    fn history(&self) -> libc::pid_t {
        let said = std::fs::read_to_string(self.told_in().with_extension("history"));
        said.ok()
            .and_then(|id| id.parse().ok())
            .expect("the copy said its history")
    }

    /// Wait for the copy's run to have returned, as it marks if it fails
    /// first.
    fn returned(&self) {
        let since = Instant::now();
        while !self.told_in().with_extension("returned").exists() {
            assert!(since.elapsed() < WAITS, "the copy's run never returned");
            thread::sleep(Duration::from_millis(5));
        }
    }

    /// What the copy said on its stderr.
    fn failed(&self) -> String {
        std::fs::read_to_string(self.folder.path().join("stderr")).unwrap_or_default()
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
/// health's stops, starts a follower, `rest`s until a stop wakes it, as
/// health rests between its looks, and once its follower ends, writes a
/// little later, as health writes the day, and leaves with an error, or
/// never returns if it hangs. Asked, it replays a history too, its run
/// waiting on that history's end, or returns its error at once and waits
/// for a stop where `main` would say it, or returns on a stop without
/// waiting for its follower, whose end may never come, or hears hangups
/// as in a terminal.
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
        let hangups = if std::env::var_os(HEARS_HANGUPS).is_some() {
            libc::SIG_DFL
        } else {
            libc::SIG_IGN
        };
        libc::signal(libc::SIGHUP, hangups);
    }
    let followers = Followers::default();
    let ender = followers.ender();
    let caught = told.with_extension("caught");
    let fails_first = std::env::var_os(FAILS_FIRST).is_some();
    let ends_never = std::env::var_os(ENDS_NEVER).is_some();
    let returns_at_once = ends_never || std::env::var_os(RETURNS_AT_ONCE).is_some();
    // As follow() runs, so this is how health ends.
    let ran = until_stopped(
        move || {
            std::fs::write(&caught, "").expect("the stop marked");
            thread::sleep(ENDS_LATE);
            if ends_never {
                loop {
                    thread::park();
                }
            }
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
            let replaying = std::env::var_os(REPLAYS).is_some().then(|| {
                let mut sleep = Command::new("sleep");
                sleep.arg("600");
                let mut history = History::of(sleep).expect("the history starts");
                let id = history.child.id().to_string();
                std::fs::write(told.with_extension("history"), id).expect("said");
                thread::spawn(move || history.ended())
            });
            let saying = told.with_extension("saying");
            std::fs::write(&saying, format!("{follower}{masks}")).expect("said");
            std::fs::rename(&saying, told).expect("said");
            if fails_first {
                return Err(io::Error::other(FAILS_BEFORE));
            }
            rest(WAITS);
            while !returns_at_once && matches!(sent.recv(), Ok(Some(_))) {}
            if std::env::var_os(HANGS).is_some() {
                loop {
                    thread::park();
                }
            }
            if let Some(replaying) = replaying {
                replaying.join().expect("the history waited on");
            }
            thread::sleep(WRITES_LATE);
            std::fs::write(told.with_extension("written"), "").expect("its write");
            Err::<(), _>(io::Error::other(FAILS_ENDING))
        },
    );
    if fails_first {
        // Where `main` would say the run's error, a stop coming first.
        std::fs::write(told.with_extension("returned"), "").expect("its return marked");
        loop {
            thread::park();
        }
    }
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
    // follower ends that. The run returns about `WRITES_LATE` after it, so
    // a stop that waited out `WRITES_WITHIN` anyway would end health late.
    let since = Instant::now();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    let took = since.elapsed();
    assert_eq!(
        (ended.signal(), gone(stopped.follower), stopped.wrote()),
        (Some(libc::SIGTERM), true, true),
        "a stop didn't end health by its signal ({ended}), left its follower running, or \
         didn't wait for health's run to write the day"
    );
    assert!(
        took < WRITES_WITHIN
            && !stopped.failed().contains(UNWRITTEN)
            && !stopped.failed().contains(LEFT)
            && !stopped.failed().contains(SECOND)
            && stopped
                .failed()
                .contains(&format!("health: {FAILS_ENDING}")),
        "a stop waited {took:?} though health's run had returned, said it hadn't, or left \
         its error unsaid: {}",
        stopped.failed()
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
        (
            ended.signal(),
            gone(stopped.follower),
            stopped.said(),
            stopped.wrote()
        ),
        (Some(libc::SIGTERM), true, None, true),
        "a stop to health's group didn't end health by its signal ({ended}), left its \
         follower running, had it say its end, or didn't wait for the day's write"
    );
    assert!(
        !stopped.failed().contains(UNWRITTEN),
        "a stop to health's group said the day went unwritten though its run returned: {}",
        stopped.failed()
    );
}

#[test]
fn a_stop_ends_health_by_its_signal_though_its_run_never_returns() {
    // `ending` fails past `WAITS`, well past the `WRITES_WITHIN` this stop
    // takes, so a stop that waited for good fails. The ender returns
    // `ENDS_LATE` on, so a stop counted from its return ends health late.
    let mut stopped = Stopped::starting(&[HANGS]);
    let since = Instant::now();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    let took = since.elapsed();
    assert_eq!(
        ended.signal(),
        Some(libc::SIGTERM),
        "a stop waited on a run that never returns ({ended})"
    );
    assert!(
        took >= WRITES_WITHIN
            && took < WRITES_WITHIN + ENDS_LATE
            && stopped.failed().contains(UNWRITTEN)
            && !stopped.failed().contains(LEFT),
        "a stop to a run that never returns ended health {took:?} on, didn't say the day \
         went unwritten, or said its ended follower was left: {}",
        stopped.failed()
    );
}

#[test]
fn a_stop_whose_run_and_followers_never_end_takes_one_bound() {
    // As a stalled disk may hold both a follower past its kill and the
    // run's write, so a bound for each wait would end health past the
    // reboot's own.
    let mut stopped = Stopped::starting(&[HANGS, ENDS_NEVER]);
    let since = Instant::now();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    let took = since.elapsed();
    assert!(
        ended.signal() == Some(libc::SIGTERM)
            && took >= WRITES_WITHIN
            && took < WRITES_WITHIN + LATE_BY
            && stopped.failed().contains(UNWRITTEN)
            && stopped.failed().contains(LEFT),
        "a stop whose run and followers never end ended health ({ended}) {took:?} on, or \
         didn't say both: {}",
        stopped.failed()
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
fn a_replay_that_ends_just_before_health_is_stopped_says_nothing() {
    // As a stop to the whole group may reach the day's replay and end it
    // before health's catch runs, as it may a follower.
    let mut stopped = Stopped::starting(&[REPLAYS]);
    // SAFETY: the copy's history, which it waits on only once it ends.
    unsafe { libc::kill(stopped.history(), libc::SIGTERM) };
    thread::sleep(ENDED_BEFORE);
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    assert!(
        ended.signal() == Some(libc::SIGTERM) && !stopped.failed().contains("ended early"),
        "a replay that ended just before health's stop said so ({ended}): {}",
        stopped.failed()
    );
}

#[test]
fn a_stop_says_the_error_a_run_returned_just_before_it() {
    // The run's error is left for `main`, which a stop's end of health
    // never lets run.
    let mut stopped = Stopped::starting(&[FAILS_FIRST]);
    stopped.returned();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    assert!(
        ended.signal() == Some(libc::SIGTERM)
            && gone(stopped.follower)
            && stopped
                .failed()
                .contains(&format!("health: {FAILS_BEFORE}")),
        "a stop ended health ({ended}) before its follower, or without saying the error its \
         run returned first: {}",
        stopped.failed()
    );
}

#[test]
fn a_stop_ends_the_followers_before_health_though_its_run_returns_at_once() {
    // As health's run writes and returns on a stop without waiting for its
    // followers' end, which the ender makes late.
    let mut stopped = Stopped::starting(&[RETURNS_AT_ONCE]);
    let since = Instant::now();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    let took = since.elapsed();
    assert!(
        ended.signal() == Some(libc::SIGTERM)
            && gone(stopped.follower)
            && stopped.wrote()
            && took >= ENDS_LATE
            && !stopped.failed().contains(UNWRITTEN)
            && !stopped.failed().contains(LEFT),
        "a stop ended health ({ended}) {took:?} on, before its follower or the day's write: {}",
        stopped.failed()
    );
}

#[test]
fn a_rest_with_no_stop_waits_its_whole_time() {
    // No stop is ever caught in this process, only in the copies, so a
    // rest here must wait it out, as a run's rest must, or it would spin.
    // On a thread of its own, so a rest that waits for good fails here
    // rather than hangs the tests.
    let time = Duration::from_millis(100);
    let (rested, waited) = std::sync::mpsc::channel();
    thread::spawn(move || {
        let since = Instant::now();
        rest(time);
        let _ = rested.send(since.elapsed());
    });
    let took = waited.recv_timeout(WAITS);
    assert!(
        took.is_ok_and(|took| took >= time),
        "a rest with no stop waited {took:?} rather than {time:?}"
    );
}

#[test]
fn a_hangup_heard_again_is_the_same_stop() {
    // As a terminal closing sends one hangup from its shell and another
    // from the kernel, so the second mustn't cut off the day's write.
    let mut stopped = Stopped::starting(&[HEARS_HANGUPS]);
    stopped.send(libc::SIGHUP, false);
    stopped.caught();
    stopped.send(libc::SIGHUP, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), gone(stopped.follower), stopped.wrote()),
        (Some(libc::SIGHUP), true, true),
        "a second hangup ended health ({ended}) before its follower or the day's write"
    );
    assert!(
        !stopped.failed().contains(SECOND),
        "a second hangup said it was a second stop: {}",
        stopped.failed()
    );
}

#[test]
fn a_hangup_after_another_stop_is_the_same_stop() {
    // As a session's scope stopped sends its stop and a hangup just after,
    // or a terminal is closed on a health already interrupted.
    let mut stopped = Stopped::starting(&[HEARS_HANGUPS]);
    stopped.send(libc::SIGTERM, false);
    stopped.caught();
    stopped.send(libc::SIGHUP, false);
    let ended = stopped.ending();
    assert_eq!(
        (ended.signal(), gone(stopped.follower), stopped.wrote()),
        (Some(libc::SIGTERM), true, true),
        "a hangup after a stop ended health ({ended}) before its follower or the day's write"
    );
    assert!(
        !stopped.failed().contains(SECOND),
        "a hangup after a stop said it was a second stop: {}",
        stopped.failed()
    );
}

#[test]
fn a_stop_whose_followers_never_end_still_ends_health_in_time() {
    // As a follower stuck on a stalled disk outlives its kill, so health's
    // end of it never returns, and the stop's bound must count without it.
    // The run writes all the same, not waiting on its followers.
    let mut stopped = Stopped::starting(&[ENDS_NEVER]);
    let since = Instant::now();
    stopped.send(libc::SIGTERM, false);
    let ended = stopped.ending();
    let took = since.elapsed();
    assert!(
        ended.signal() == Some(libc::SIGTERM)
            && took >= WRITES_WITHIN
            && stopped.wrote()
            && !stopped.failed().contains(UNWRITTEN)
            && stopped.failed().contains(LEFT),
        "a stop whose followers never end ended health ({ended}) {took:?} on, didn't write \
         the day, or didn't say it left one: {}",
        stopped.failed()
    );
}

#[test]
fn a_second_stop_ends_health_at_once_leaving_its_follower() {
    // The first stop's end of the followers is late, so a second that
    // waited on it would end health by the first, its follower ended. An
    // interrupt typed twice is a second stop too, unlike a hangup.
    for (first, second) in [(libc::SIGTERM, libc::SIGINT), (libc::SIGINT, libc::SIGINT)] {
        let mut stopped = Stopped::start();
        stopped.send(first, false);
        stopped.caught();
        stopped.send(second, false);
        let ended = stopped.ending();
        assert_eq!(
            (ended.signal(), gone(stopped.follower)),
            (Some(second), false),
            "a second stop ({second}) waited for the first ({first}) to end health ({ended})"
        );
        assert!(
            stopped.failed().contains(&format!(
                "{SECOND} at once, not waiting for the day's write"
            )),
            "a second stop before the day's write ended health without saying so: {}",
            stopped.failed()
        );
    }
}

#[test]
fn a_second_stop_after_the_run_returned_says_it_returned() {
    // As one interrupts again while a follower is slow to end, the day
    // written, so the second stop mustn't say a write was lost.
    let mut stopped = Stopped::starting(&[ENDS_NEVER]);
    stopped.send(libc::SIGTERM, false);
    let since = Instant::now();
    while !stopped.wrote() {
        assert!(since.elapsed() < WAITS, "the copy's run never wrote");
        thread::sleep(Duration::from_millis(5));
    }
    thread::sleep(ENDED_BEFORE);
    stopped.send(libc::SIGINT, false);
    let ended = stopped.ending();
    assert!(
        ended.signal() == Some(libc::SIGINT)
            && stopped
                .failed()
                .contains(&format!("{SECOND} at once, its run returned")),
        "a second stop after the run returned ended health ({ended}) saying otherwise: {}",
        stopped.failed()
    );
}

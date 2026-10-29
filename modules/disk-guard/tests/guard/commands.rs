use disk_guard::commands::{Job, Ran, run, stoppable};
use std::ffi::OsString;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The scheduling policy the guard runs under here, which a command not
/// started idle keeps: not always the default, since a Nix build runs at
/// the policy its daemon is given, batch on some machines.
#[cfg(target_os = "linux")]
pub(crate) fn guards_policy() -> u32 {
    // SAFETY: sched_getscheduler only reads the calling thread's policy.
    let policy = unsafe { libc::sched_getscheduler(0) };
    assert!(policy >= 0, "{}", std::io::Error::last_os_error());
    u32::try_from(policy).unwrap()
}

fn job(words: &[&str]) -> Job {
    Job {
        argv: words.iter().map(OsString::from).collect(),
        cwd: None,
        env: Vec::new(),
        timeout: Duration::from_secs(10),
        limit: 1 << 20,
        idle: false,
    }
}

/// The scheduling policy a command ran under, from its own stat line.
#[cfg(target_os = "linux")]
fn policy(job: &Job) -> u32 {
    let output = finished(run(job).unwrap());
    let stat = String::from_utf8(output.stdout).unwrap();
    let after_name = stat.rsplit_once(')').unwrap().1;
    // Field 41 of the line, counted from the state, which is field 3.
    after_name
        .split_whitespace()
        .nth(41 - 3)
        .unwrap()
        .parse()
        .unwrap()
}

#[test]
#[cfg(target_os = "linux")]
fn an_idle_command_runs_at_idle_priority_and_any_other_as_the_guard_does() {
    let mut job = job(&["cat", "/proc/self/stat"]);
    assert_eq!(policy(&job), guards_policy());
    job.idle = true;
    assert_eq!(policy(&job), libc::SCHED_IDLE as u32);
}

fn finished(ran: Ran) -> disk_guard::commands::Output {
    match ran {
        Ran::Finished(output) => output,
        other => panic!("{other:?}"),
    }
}

/// Runs `job` until `stop` is asked after `after`, with `grace` to end.
fn stopped_after(job: &Job, after: Duration, grace: Duration) -> (Ran, Duration) {
    let stop = AtomicBool::new(false);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            std::thread::sleep(after);
            stop.store(true, Ordering::Relaxed);
        });
        let started = Instant::now();
        let ran = stoppable(job, &stop, grace).unwrap();
        (ran, started.elapsed())
    })
}

#[test]
fn a_stopped_command_is_asked_to_end_and_given_its_grace() {
    let dir = tempfile::tempdir().unwrap();
    let said = dir.path().join("said");
    let script = format!(
        "trap 'sleep 0.3; echo ended > {}; exit 0' TERM; sleep 30 & wait",
        said.display()
    );
    let (ran, took) = stopped_after(
        &job(&["sh", "-c", &script]),
        Duration::from_millis(200),
        Duration::from_secs(5),
    );
    assert!(matches!(ran, Ran::Stopped), "{ran:?}");
    assert!(took < Duration::from_secs(3), "{took:?}");
    assert_eq!(std::fs::read_to_string(&said).unwrap(), "ended\n");
}

#[test]
fn a_stopped_command_that_wont_end_is_killed_after_its_grace() {
    let (ran, took) = stopped_after(
        &job(&["sh", "-c", "trap '' TERM; sleep 30 & wait; sleep 30"]),
        Duration::from_millis(100),
        Duration::from_millis(300),
    );
    assert!(matches!(ran, Ran::Stopped), "{ran:?}");
    assert!(took < Duration::from_secs(3), "{took:?}");
}

#[test]
fn a_command_never_stopped_finishes_as_any_other() {
    let stop = AtomicBool::new(false);
    let ran = stoppable(&job(&["echo", "done"]), &stop, Duration::from_secs(1)).unwrap();
    assert_eq!(finished(ran).stdout, b"done\n");
}

#[test]
fn a_commands_output_and_status_are_read() {
    let output = finished(run(&job(&["sh", "-c", "echo out; echo err >&2; exit 3"])).unwrap());
    assert_eq!(output.status, Some(3));
    assert_eq!(output.stdout, b"out\n");
    assert_eq!(output.stderr, b"err\n");
    assert!(!output.truncated);
}

#[test]
fn it_runs_where_and_with_what_it_is_told() {
    let dir = tempfile::tempdir().unwrap();
    let mut job = job(&["sh", "-c", "pwd; echo \"$GUARD_TEST\""]);
    job.cwd = Some(dir.path().to_owned());
    job.env = vec![(OsString::from("GUARD_TEST"), OsString::from("given"))];
    let output = finished(run(&job).unwrap());
    let real = dir.path().canonicalize().unwrap();
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("{}\ngiven\n", real.display())
    );
}

#[test]
fn it_reads_nothing_from_the_guard() {
    let output = finished(run(&job(&["cat"])).unwrap());
    assert_eq!(output.status, Some(0));
    assert_eq!(output.stdout, b"");
}

#[test]
fn output_past_the_limit_is_cut_and_the_rest_drained() {
    let mut job = job(&["head", "-c", "5000000", "/dev/zero"]);
    job.limit = 1000;
    let output = finished(run(&job).unwrap());
    assert_eq!(output.status, Some(0));
    assert_eq!(output.stdout.len(), 1000);
    assert!(output.truncated);
}

fn gone(pid: u32) -> bool {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat"));
    stat.map_or(true, |stat| {
        stat.rsplit(')')
            .next()
            .is_some_and(|rest| rest.trim_start().starts_with('Z'))
    })
}

#[test]
fn a_command_past_its_time_is_killed_with_everything_it_started() {
    let dir = tempfile::tempdir().unwrap();
    let pid_file = dir.path().join("pid");
    let script = format!("sleep 30 & echo $! > {}; sleep 30", pid_file.display());
    let mut job = job(&["sh", "-c", &script]);
    job.timeout = Duration::from_millis(500);
    let started = Instant::now();
    assert!(matches!(run(&job).unwrap(), Ran::TimedOut));
    assert!(started.elapsed() < Duration::from_secs(5));
    let pid: u32 = std::fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    // Killed is gone at once, but its exit has to be run: a sandbox build
    // at low weight on a machine at load 27 waited past 2 s for it on 28
    // October, so the bound is wide and the wait ends as soon as it's gone.
    let deadline = Instant::now() + Duration::from_secs(30);
    while !gone(pid) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(gone(pid), "the command's own child outlived it");
}

#[test]
fn a_child_holding_the_output_open_cant_hold_the_guard() {
    // It leaves its group, so killing the group misses it, and it keeps
    // the output pipe open.
    let mut job = job(&["sh", "-c", "setsid sleep 3 & exit 0"]);
    job.timeout = Duration::from_millis(500);
    let started = Instant::now();
    let _ = run(&job).unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
}

/// The guard blocks its stop signals in every thread, so it stops only
/// between steps; a command it starts answers them still, so a stop's
/// SIGTERM to the unit ends what a step is waiting on at once.
#[cfg(target_os = "linux")]
#[test]
fn a_command_answers_the_stop_signals_its_guard_blocks() {
    let mask = std::thread::spawn(|| {
        // SAFETY: set is a valid sigset_t, filled before it's read.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            for signal in [libc::SIGTERM, libc::SIGINT, libc::SIGHUP] {
                libc::sigaddset(&raw mut set, signal);
            }
            assert_eq!(
                libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut()),
                0
            );
        }
        let output = finished(run(&job(&["grep", "^SigBlk", "/proc/self/status"])).unwrap());
        String::from_utf8(output.stdout).unwrap()
    })
    .join()
    .unwrap();
    assert_eq!(mask.trim(), "SigBlk:\t0000000000000000");
}

/// A stop asked during a pass, waiting while the guard blocks it, means no
/// new command starts: systemd signals the unit once, so a command started
/// after it would run on unasked until systemd kills it.
#[cfg(target_os = "linux")]
#[test]
fn no_command_starts_once_a_stop_is_waiting() {
    let (refused, ran) = std::thread::spawn(|| {
        // SAFETY: set is a valid sigset_t; the signal goes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            libc::pthread_kill(libc::pthread_self(), libc::SIGTERM);
            let marker = tempfile::tempdir().unwrap();
            let made = marker.path().join("ran");
            let started = run(&job(&["touch", made.to_str().unwrap()]));
            let ran = made.exists();
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            (started.is_err(), ran)
        }
    })
    .join()
    .unwrap();
    assert!(refused, "a command started with a stop waiting");
    assert!(!ran, "the command ran");
}

/// A stop that comes while a command runs ends it at once, by SIGKILL, and
/// it's said to have stopped: the unit sends its stop to the guard alone,
/// since Nix sent SIGTERM mid-delete aborted and dumped core.
#[cfg(target_os = "linux")]
#[test]
fn a_stop_that_comes_while_a_command_runs_kills_it() {
    let (sender, receiver) = std::sync::mpsc::channel();
    let worker = std::thread::spawn(move || {
        // SAFETY: set is a valid sigset_t; the signal comes to this thread,
        // which blocks it, and is taken back before the thread ends.
        unsafe {
            let mut set = std::mem::zeroed::<libc::sigset_t>();
            libc::sigemptyset(&raw mut set);
            libc::sigaddset(&raw mut set, libc::SIGTERM);
            libc::pthread_sigmask(libc::SIG_BLOCK, &raw const set, std::ptr::null_mut());
            sender.send(libc::pthread_self()).unwrap();
            let started = Instant::now();
            let ran = run(&job(&["sleep", "30"])).unwrap();
            let took = started.elapsed();
            let mut taken = 0;
            libc::sigwait(&raw const set, &raw mut taken);
            (ran, took)
        }
    });
    let thread = receiver.recv().unwrap();
    std::thread::sleep(Duration::from_millis(300));
    // SAFETY: the worker blocks SIGTERM, so it waits for the worker to take it.
    unsafe { libc::pthread_kill(thread, libc::SIGTERM) };
    let (ran, took) = worker.join().unwrap();
    assert!(matches!(ran, Ran::Stopped), "{ran:?}");
    assert!(
        took < Duration::from_secs(5),
        "ran on {took:?} after the stop"
    );
}

#[test]
fn a_missing_program_is_an_error() {
    assert!(run(&job(&["/nonexistent/program"])).is_err());
    assert!(
        run(&Job {
            argv: Vec::new(),
            ..job(&[])
        })
        .is_err()
    );
}

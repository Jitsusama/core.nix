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
    policy as u32
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
    let deadline = Instant::now() + Duration::from_secs(2);
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

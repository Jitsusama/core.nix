//! night-shift turn: a heavy job by day waits for the one turn a night job
//! takes, keeps it while its command runs, and leaves its command's output
//! and status as they are.

mod common;

use common::Folder;
use std::fs::File;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// A runtime and state folder of the test's own.
struct Account {
    root: Folder,
}

impl Account {
    fn new() -> Self {
        Account {
            root: Folder::new("night-shift-turn"),
        }
    }

    fn path(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }

    fn command(&self, args: &[&str]) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_night-shift"));
        command
            .args(args)
            .env("XDG_STATE_HOME", self.path("state"))
            .env("XDG_RUNTIME_DIR", self.path("runtime"))
            .current_dir(&self.root);
        command
    }

    /// Asks for the turn as `name` with `options`, for the shell line `job`.
    fn turn(&self, name: &str, options: &[&str], job: &str) -> Command {
        let mut args = vec!["turn", name];
        args.extend_from_slice(options);
        args.extend_from_slice(&["--", "sh", "-c", job]);
        self.command(&args)
    }

    /// Starts `job` with the turn, back once it has it. The job waits for
    /// `go` before it ends.
    fn holding(&self, name: &str, job: &str) -> Holding {
        let child = self
            .turn(name, &[], &format!("touch has; {job}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let child = Holding {
            child,
            go: self.path("go"),
        };
        let began = Instant::now();
        while !self.path("has").exists() {
            assert!(
                began.elapsed() < Duration::from_secs(30),
                "the first job never took the turn"
            );
            // Polled: the stand-in job says it has the turn by a file.
            std::thread::sleep(Duration::from_millis(20));
        }
        child
    }
}

/// A job holding the turn, told to go on and end when its test ends, so a
/// failing test can't leave it waiting for ever.
struct Holding {
    child: Child,
    go: PathBuf,
}

impl Holding {
    fn wait(&mut self) -> std::io::Result<ExitStatus> {
        self.child.wait()
    }
}

impl Drop for Holding {
    fn drop(&mut self) {
        let _ = File::create(&self.go);
        let _ = self.child.wait();
    }
}

/// What a turn said on stderr, where it says everything, leaving stdout
/// to its command.
fn warned(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn a_turn_with_nobody_waiting_runs_its_command_as_it_is() {
    let account = Account::new();
    let output = account
        .turn("eval", &[], "echo out; echo err >&2; exit 3")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(3), "{}", warned(&output));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "out\n");
    assert_eq!(
        warned(&output),
        "err\n",
        "it said more than the command did"
    );
}

#[test]
fn a_second_heavy_job_waits_its_turn_and_says_whose_it_is() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done; touch first");
    let second = account
        .turn("eval", &[], "[ -e first ] && touch second")
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // Long enough for the second to have started beside the first, were
    // it going to.
    std::thread::sleep(Duration::from_millis(1500));
    assert!(
        !account.path("second").exists(),
        "the second job ran beside the first"
    );
    File::create(account.path("go")).unwrap();
    assert!(first.wait().unwrap().success(), "the first job failed");
    let output = second.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", warned(&output));
    assert!(account.path("second").exists(), "the second job never ran");
    assert!(
        warned(&output).contains("waiting for the turn, which stamp (process"),
        "{}",
        warned(&output)
    );
}

#[test]
fn a_job_waiting_past_its_most_gives_up_with_try_again_later() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let output = account
        .turn("eval", &["--wait-at-most", "0.5"], "touch ran")
        .output()
        .unwrap();
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    assert_eq!(output.status.code(), Some(75), "{}", warned(&output));
    assert!(!account.path("ran").exists(), "it ran without the turn");
}

#[test]
fn a_night_job_tried_beside_a_heavy_job_by_day_leaves_it_the_turn() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let output = account
        .command(&["run", "repack", "--", "touch", "ran"])
        .output()
        .unwrap();
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    let says = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "{says}");
    assert!(!account.path("ran").exists(), "the night job ran beside it");
    assert!(
        says.contains("another heavy job is running, stamp"),
        "{says}"
    );
}

/// Whether the process the file `pid` names still runs: there, and not a
/// zombie waiting to be reaped.
fn runs(account: &Account, pid: &str) -> bool {
    let pid = std::fs::read_to_string(account.path(pid)).unwrap();
    std::fs::read_to_string(format!("/proc/{}/stat", pid.trim()))
        .is_ok_and(|stat| !stat.rsplit(") ").next().unwrap_or("").starts_with('Z'))
}

/// Kills, as the test ends, the process the file `pid` names, so a
/// failing test leaves nothing running for a minute.
struct Leftover<'a>(&'a Account, &'a str);

impl Drop for Leftover<'_> {
    fn drop(&mut self) {
        if let Ok(pid) = std::fs::read_to_string(self.0.path(self.1)) {
            let _ = Command::new("kill").args(["-KILL", pid.trim()]).status();
        }
    }
}

/// Waits up to `most` for `what` to hold, polling, since a process ending
/// says so nowhere a test can wait on.
fn within(most: Duration, what: impl Fn() -> bool) -> bool {
    let began = Instant::now();
    while began.elapsed() < most {
        if what() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    what()
}

#[test]
fn a_command_that_ends_lets_go_of_the_turn_and_takes_its_leftovers() {
    // On 17 November a cargo test ended at 18:20 and two of its shells,
    // one ignoring TERM, kept the turn until they were killed at 19:26.
    let account = Account::new();
    let _leftover = Leftover(&account, "left");
    let first = account
        .turn(
            "cargo",
            &[],
            "sh -c 'trap \"\" TERM; echo $$ > left; exec sleep 60' & \
             until [ -s left ]; do sleep 0.05; done; exit 4",
        )
        .output()
        .unwrap();
    assert_eq!(first.status.code(), Some(4), "{}", warned(&first));
    assert!(
        !runs(&account, "left"),
        "the command's leftover outlived its turn"
    );
    let second = account
        .turn("eval", &["--wait-at-most", "2"], "touch ran")
        .output()
        .unwrap();
    assert!(second.status.success(), "{}", warned(&second));
    assert!(account.path("ran").exists(), "the turn stayed taken");
}

#[test]
fn a_caller_that_dies_takes_its_turn_and_its_command_with_it() {
    // A session killed mid-run kills the shell that asked for the turn,
    // and not always the turn's command.
    let account = Account::new();
    let _leftover = Leftover(&account, "top");
    let turn = format!(
        "{} turn cargo -- sh -c 'echo $$ > top; exec sleep 60' & wait",
        env!("CARGO_BIN_EXE_night-shift")
    );
    let mut caller = Command::new("sh")
        .args(["-c", &turn])
        .env("XDG_STATE_HOME", account.path("state"))
        .env("XDG_RUNTIME_DIR", account.path("runtime"))
        .current_dir(account.path(""))
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    assert!(
        within(Duration::from_secs(30), || account.path("top").exists()),
        "the command never started"
    );
    caller.kill().unwrap();
    caller.wait().unwrap();
    assert!(
        within(Duration::from_secs(10), || !runs(&account, "top")),
        "the command outlived its caller"
    );
    let second = account
        .turn("eval", &["--wait-at-most", "2"], "touch ran")
        .output()
        .unwrap();
    assert!(second.status.success(), "{}", warned(&second));
}

#[test]
fn a_job_joel_waits_on_goes_ahead_of_those_waiting_already() {
    // On 17 November a switch's build waited behind agents' tests.
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let tests = account
        .turn("cargo", &[], "echo tests >> order")
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    // Long enough for the tests to be waiting before the switch asks.
    std::thread::sleep(Duration::from_millis(1500));
    let switch = account
        .turn("switch", &["--ahead"], "echo switch >> order")
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    let switched = switch.wait_with_output().unwrap();
    assert!(switched.status.success(), "{}", warned(&switched));
    assert!(tests.wait_with_output().unwrap().status.success());
    assert_eq!(
        std::fs::read_to_string(account.path("order")).unwrap(),
        "switch\ntests\n",
        "the job Joel waits on went after the tests"
    );
}

#[test]
fn a_night_job_leaves_the_turn_to_a_job_joel_waits_on() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let switch = account
        .turn("switch", &["--ahead"], "sleep 1; touch switched")
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    let night = account
        .command(&[
            "run",
            "repack",
            "--wait-at-most",
            "20",
            // The machine's memory needn't be quiet for this.
            "--start-under",
            "100",
            "--",
            "sh",
            "-c",
            "[ -e switched ] && touch ran",
        ])
        .output()
        .unwrap();
    assert!(switch.wait_with_output().unwrap().status.success());
    assert!(
        night.status.success(),
        "{}",
        String::from_utf8_lossy(&night.stdout)
    );
    assert!(account.path("ran").exists(), "the night job went first");
}

#[test]
fn the_command_starts_with_no_signal_held() {
    // night-shift holds the signals that end a turn while it watches; its
    // command must still hear them.
    let account = Account::new();
    let output = account
        .turn("eval", &[], "grep SigBlk /proc/self/status")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", warned(&output));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "SigBlk:\t0000000000000000"
    );
}

#[test]
fn the_command_keeps_the_turn_until_it_ends() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let held = File::create(account.path("runtime/night-shift/lock")).unwrap();
    assert!(
        held.try_lock().is_err(),
        "the running command left the turn"
    );
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    assert!(held.try_lock().is_ok(), "the turn outlived its command");
}

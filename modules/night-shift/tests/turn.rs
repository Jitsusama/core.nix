//! night-shift turn: a heavy job by day waits for the one turn a night job
//! takes, keeps it while its command runs, and leaves its command's output
//! and status as they are.

mod common;

use common::Folder;
use std::fs::{self, File};
use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

/// How long a test gives a process to get where it's going, which a
/// loaded machine still reaches in well under it.
const AT_MOST: Duration = Duration::from_secs(30);

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

    /// Starts a turn as `name` with `options` for `job`, what it says
    /// going to the file `said`, where a test can watch for it.
    fn saying(&self, name: &str, options: &[&str], job: &str, said: &str) -> Child {
        self.turn(name, options, job)
            .stdout(Stdio::null())
            .stderr(File::create(self.path(said)).unwrap())
            .spawn()
            .unwrap()
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
        assert!(
            within(AT_MOST, || self.path("has").exists()),
            "the first job never took the turn"
        );
        child
    }

    /// Whether the file `said` holds `what`.
    fn says(&self, said: &str, what: &str) -> bool {
        fs::read_to_string(self.path(said)).is_ok_and(|text| text.contains(what))
    }

    /// Leaves a mark that `name`, a job Joel waits on, waits for the turn
    /// as this test's own process, started when `started` says.
    fn waits_ahead(&self, name: &str, started: &str) {
        let ahead = self.path("runtime/night-shift/ahead");
        fs::create_dir_all(&ahead).unwrap();
        fs::write(
            ahead.join(std::process::id().to_string()),
            format!("{name}\t{started}\n"),
        )
        .unwrap();
    }
}

/// When this test's process started, as night-shift reads it.
fn own_start() -> String {
    let stat = fs::read_to_string("/proc/self/stat").unwrap();
    let after = stat.rsplit_once(") ").unwrap().1;
    after.split(' ').nth(19).unwrap().to_owned()
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

/// Waits up to `most` for `what` to hold, polling, since what a test
/// waits on here says so nowhere it can wait on.
fn within(most: Duration, what: impl Fn() -> bool) -> bool {
    let began = Instant::now();
    while began.elapsed() < most {
        if what() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    what()
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
    let warned = warned(&output);
    let lines: Vec<&str> = warned.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "it said more than one line after the command: {warned}"
    );
    assert_eq!(lines[0], "err", "the command's own line changed");
    assert!(
        lines[1].contains("night-shift eval: the command failed: exit status: 3"),
        "it didn't say how the command failed: {warned}"
    );
}

#[test]
fn a_second_heavy_job_waits_its_turn_and_says_whose_it_is() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done; touch first");
    let second = account.saying("eval", &[], "[ -e first ] && touch second", "said");
    assert!(
        within(AT_MOST, || account.says("said", "waiting for the turn")),
        "the second job never said it waits"
    );
    assert!(
        !account.path("second").exists(),
        "the second job ran beside the first"
    );
    File::create(account.path("go")).unwrap();
    assert!(first.wait().unwrap().success(), "the first job failed");
    let status = second.wait_with_output().unwrap().status;
    let said = fs::read_to_string(account.path("said")).unwrap();
    assert!(status.success(), "{said}");
    assert!(account.path("second").exists(), "the second job never ran");
    assert!(
        said.contains("waiting for the turn, which stamp (process"),
        "{said}"
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
    let pid = fs::read_to_string(account.path(pid)).unwrap();
    fs::read_to_string(format!("/proc/{}/stat", pid.trim()))
        .is_ok_and(|stat| !stat.rsplit(") ").next().unwrap_or("").starts_with('Z'))
}

/// Kills, as the test ends, the process the file `pid` names, so a
/// failing test leaves nothing running for a minute.
struct Leftover<'a>(&'a Account, &'a str);

impl Drop for Leftover<'_> {
    fn drop(&mut self) {
        if let Ok(pid) = fs::read_to_string(self.0.path(self.1)) {
            let _ = Command::new("kill").args(["-KILL", pid.trim()]).status();
        }
    }
}

#[test]
fn a_command_that_ends_lets_go_of_the_turn_and_takes_its_leftovers() {
    // On 17 November a cargo test ended at 18:20 and two of its shells,
    // one ignoring TERM, kept the turn until they were killed at 19:26.
    // The leftover's output goes elsewhere, so the turn's own output ends
    // with the turn rather than with it.
    let account = Account::new();
    let _leftover = Leftover(&account, "left");
    let first = account
        .turn(
            "cargo",
            &[],
            "sh -c 'trap \"\" TERM; echo $$ > left; exec sleep 60' >/dev/null 2>&1 & \
             until [ -s left ]; do sleep 0.05; done; exit 4",
        )
        .output()
        .unwrap();
    assert_eq!(first.status.code(), Some(4), "{}", warned(&first));
    assert!(
        !runs(&account, "left"),
        "the command's leftover outlived its turn"
    );
    assert!(
        warned(&first).contains("ended 1 process its command left running"),
        "{}",
        warned(&first)
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
    // and not always the turn's command. The command's own child asked
    // the kernel for nothing, so only night-shift can end it.
    let account = Account::new();
    let _top = Leftover(&account, "top");
    let _left = Leftover(&account, "left");
    let turn = format!(
        "{} turn cargo -- sh -c 'sleep 60 >/dev/null 2>&1 & echo $! > left; echo $$ > top; wait' & wait",
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
        within(AT_MOST, || account.path("top").exists()),
        "the command never started"
    );
    caller.kill().unwrap();
    caller.wait().unwrap();
    assert!(
        within(Duration::from_secs(10), || !runs(&account, "top")),
        "the command outlived its caller"
    );
    assert!(
        within(Duration::from_secs(10), || !runs(&account, "left")),
        "the command's own child outlived its caller"
    );
    let second = account
        .turn("eval", &["--wait-at-most", "2"], "touch ran")
        .output()
        .unwrap();
    assert!(second.status.success(), "{}", warned(&second));
}

#[test]
fn a_turn_sent_term_passes_it_on_and_ends_by_it() {
    let account = Account::new();
    let turn = account.saying(
        "eval",
        &[],
        "trap 'touch heard; exit 0' TERM; touch started; while :; do sleep 0.05; done",
        "said",
    );
    assert!(
        within(AT_MOST, || account.path("started").exists()),
        "the command never started"
    );
    let pid = turn.id().to_string();
    Command::new("kill").args(["-TERM", &pid]).status().unwrap();
    let status = turn.wait_with_output().unwrap().status;
    assert!(
        account.path("heard").exists(),
        "the command never heard TERM"
    );
    assert_eq!(
        status.signal(),
        Some(15),
        "the turn didn't end by TERM: {status}"
    );
}

#[test]
fn a_turn_whose_command_a_signal_ends_ends_by_it() {
    let account = Account::new();
    let output = account.turn("eval", &[], "kill -HUP $$").output().unwrap();
    assert_eq!(
        output.status.signal(),
        Some(1),
        "the turn didn't end by HUP: {}",
        warned(&output)
    );
}

#[test]
fn ctrl_c_is_the_commands_to_hear() {
    // A terminal's Ctrl-C reaches the command itself; an INT sent to
    // night-shift alone leaves the command to finish.
    let account = Account::new();
    let turn = account.saying(
        "eval",
        &[],
        "touch started; until [ -e go ]; do sleep 0.05; done",
        "said",
    );
    assert!(
        within(AT_MOST, || account.path("started").exists()),
        "the command never started"
    );
    Command::new("kill")
        .args(["-INT", &turn.id().to_string()])
        .status()
        .unwrap();
    // Long enough for night-shift to have ended the command, were it
    // going to: it hears a signal the moment it comes.
    std::thread::sleep(Duration::from_millis(500));
    File::create(account.path("go")).unwrap();
    let status = turn.wait_with_output().unwrap().status;
    assert!(
        status.success(),
        "INT to night-shift ended the command: {status}"
    );
}

#[test]
fn a_free_turn_is_left_a_while_to_a_job_joel_waits_on() {
    let account = Account::new();
    account.waits_ahead("switch", &own_start());
    let output = account
        .turn("eval", &["--wait-at-most", "1"], "touch ran")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(75), "{}", warned(&output));
    assert!(
        !account.path("ran").exists(),
        "it took the turn left to the switch"
    );
    assert!(
        warned(&output).contains("leaving the turn to switch, which Joel waits on"),
        "{}",
        warned(&output)
    );
}

#[test]
fn a_job_joel_waits_on_that_never_takes_the_turn_holds_nobody_back_long() {
    // A switch stopped with Ctrl-Z, or stuck, still runs.
    let account = Account::new();
    account.waits_ahead("switch", &own_start());
    let began = Instant::now();
    let output = account
        .turn("eval", &["--wait-at-most", "20"], "touch ran")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", warned(&output));
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "it waited {:?} on a job that never took the turn",
        began.elapsed()
    );
}

#[test]
fn a_mark_left_by_a_job_gone_holds_nobody_back() {
    // Its number now names another process, started at another time.
    let account = Account::new();
    account.waits_ahead("switch", "1");
    let output = account
        .turn("eval", &["--wait-at-most", "0.5"], "touch ran")
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", warned(&output));
    assert!(
        account.path("ran").exists(),
        "a dead job's mark held the turn"
    );
}

#[test]
fn a_night_job_waits_a_while_for_a_job_joel_waits_on_then_goes() {
    let account = Account::new();
    account.waits_ahead("switch", &own_start());
    let night = account
        .command(&[
            "run",
            "repack",
            // The machine's memory needn't be quiet for this.
            "--start-under",
            "100",
            "--",
            "touch",
            "ran",
        ])
        .output()
        .unwrap();
    let says = String::from_utf8_lossy(&night.stdout);
    assert!(night.status.success(), "{says}");
    assert!(
        account.path("ran").exists(),
        "a job that never took the turn kept the night job off it: {says}"
    );
}

#[test]
fn a_job_joel_waits_on_goes_ahead_of_those_waiting_already() {
    // On 17 November a switch's build waited behind agents' tests. Three
    // wait, so one going first by chance is a quarter's.
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let tests: Vec<Child> = (0..3)
        .map(|n| {
            let said = format!("tests{n}");
            let child = account.saying("cargo", &[], "echo tests >> order", &said);
            assert!(
                within(AT_MOST, || account.says(&said, "waiting for the turn")),
                "the tests never waited"
            );
            child
        })
        .collect();
    let switch = account.saying("switch", &["--ahead"], "echo switch >> order", "switched");
    let mark = account.path(&format!("runtime/night-shift/ahead/{}", switch.id()));
    assert!(
        within(AT_MOST, || mark.exists()),
        "the switch never said it waits ahead"
    );
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    let status = switch.wait_with_output().unwrap().status;
    assert!(status.success(), "the switch failed: {status}");
    for test in tests {
        let status = test.wait_with_output().unwrap().status;
        assert!(status.success(), "the tests failed: {status}");
    }
    assert_eq!(
        fs::read_to_string(account.path("order")).unwrap(),
        "switch\ntests\ntests\ntests\n",
        "the job Joel waits on went after the tests"
    );
}

#[test]
fn a_night_job_leaves_the_turn_to_a_job_joel_waits_on() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let switch = account.saying("switch", &["--ahead"], "sleep 1; touch switched", "said");
    let mark = account.path(&format!("runtime/night-shift/ahead/{}", switch.id()));
    assert!(
        within(AT_MOST, || mark.exists()),
        "the switch never said it waits ahead"
    );
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
    let status = switch.wait_with_output().unwrap().status;
    assert!(status.success(), "the switch failed: {status}");
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

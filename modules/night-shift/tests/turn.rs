//! night-shift turn: a heavy job by day waits for the one turn a night job
//! takes, keeps it while its command runs, and leaves its command's output
//! and status as they are.
//!
//! Long because every case here drives the one turn's lock through the same
//! account and its helpers, so a split would copy them or move them into
//! `common`, where the run tests would build what they never use.

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

    /// Starts the night job `repack` with `options` for the shell line
    /// `job`, what it logs going to the file `night`.
    fn night(&self, options: &[&str], job: &str) -> Child {
        let mut args = vec![
            "run",
            "repack",
            // The machine's memory needn't be quiet for this, nor stay
            // so while it runs.
            "--start-under",
            "100",
            "--stop-machine",
            "100",
            "--every",
            "0.1",
        ];
        args.extend_from_slice(options);
        args.extend_from_slice(&["--", "sh", "-c", job]);
        self.command(&args)
            .stdout(File::create(self.path("night")).unwrap())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
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

// A turn asked for inside a turn's command, as nix-offload's cargo asks
// for one inside an agent's, has it already, where it once waited for its
// own parent's lock for ever. One beside a turn, not within it, still
// waits, as a_job_waiting_past_its_most_gives_up_with_try_again_later shows.
// The line goes on past the inner turn, so the shell forks it rather than
// running it in its own place, and the walk climbs past the shell.
#[test]
fn a_turn_asked_for_within_a_turn_runs_in_it() {
    let account = Account::new();
    let inner = format!(
        "'{}' turn cargo --wait-at-most 5 -- touch inner; :",
        env!("CARGO_BIN_EXE_night-shift")
    );
    let output = account.turn("test", &[], &inner).output().unwrap();
    let said = warned(&output);
    assert!(
        output.status.success(),
        "the turn within a turn failed: {said}"
    );
    assert!(
        account.path("inner").exists(),
        "the inner command never ran"
    );
    assert!(
        said.contains("night-shift cargo: runs in the turn test (process"),
        "the inner turn didn't say whose turn it ran in: {said}"
    );
}

/// When this test's process started, in the kernel's ticks, as a turn
/// notes its own.
fn started() -> String {
    let stat = fs::read_to_string("/proc/self/stat").unwrap();
    let (_, after) = stat.rsplit_once(") ").unwrap();
    after.split(' ').nth(19).unwrap().to_owned()
}

// A holder that is a later turn's parent, here this test, but holds no
// lock, as a night job paused for a job Joel waits on doesn't, leaves
// the turn to be taken.
#[test]
fn a_turn_below_a_pid_a_gone_holder_noted_takes_the_lock_itself() {
    let account = Account::new();
    let runtime = account.path("runtime/night-shift");
    fs::create_dir_all(&runtime).unwrap();
    fs::write(
        runtime.join("holder"),
        format!("gone\t{}\t0\t{}\n", std::process::id(), started()),
    )
    .unwrap();
    let output = account.turn("eval", &[], "touch ran").output().unwrap();
    let said = warned(&output);
    assert!(output.status.success(), "the turn failed: {said}");
    assert!(account.path("ran").exists(), "the command never ran");
    assert!(
        !said.contains("runs in the turn"),
        "it ran in a turn nobody held: {said}"
    );
    assert!(
        fs::read_to_string(runtime.join("holder")).is_ok_and(|noted| noted.starts_with("eval\t")),
        "it didn't note itself as the turn's holder"
    );
}

// A note's pid given again to one of a later turn's parents, while
// another job holds the lock and hasn't noted itself yet, is told apart
// by when it started: the turn waits for that job rather than running
// beside it.
#[test]
fn a_turn_below_a_pid_given_again_waits_while_another_holds_the_lock() {
    let account = Account::new();
    let mut first = account.holding("stamp", "until [ -e go ]; do sleep 0.05; done");
    let runtime = account.path("runtime/night-shift");
    fs::write(
        runtime.join("holder"),
        format!("gone\t{}\t0\t1\n", std::process::id()),
    )
    .unwrap();
    let output = account
        .turn("eval", &["--wait-at-most", "0.5"], "touch ran")
        .output()
        .unwrap();
    File::create(account.path("go")).unwrap();
    first.wait().unwrap();
    let said = warned(&output);
    assert_eq!(
        output.status.code(),
        Some(75),
        "it didn't wait for the job holding the lock: {said}"
    );
    assert!(
        !account.path("ran").exists(),
        "it ran beside the job holding the lock"
    );
    assert!(
        !said.contains("runs in the turn"),
        "it took a pid given again for the turn's holder: {said}"
    );
}

// The note is written whole, its holder's start beside its pid, and only
// the note is left in the turn's folder.
#[test]
fn a_turn_notes_when_its_holder_started() {
    let account = Account::new();
    let output = account
        .turn(
            "eval",
            &[],
            "cat \"$XDG_RUNTIME_DIR/night-shift/holder\" > noted",
        )
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "the turn failed: {}",
        warned(&output)
    );
    let noted = fs::read_to_string(account.path("noted")).unwrap();
    let fields: Vec<&str> = noted.trim().split('\t').collect();
    assert_eq!(
        fields.len(),
        4,
        "the note isn't name, pid, since and start: {noted}"
    );
    assert!(
        fields[3].parse::<u64>().is_ok_and(|ticks| ticks > 0),
        "the note's start isn't ticks: {noted}"
    );
    let left: Vec<_> = fs::read_dir(account.path("runtime/night-shift"))
        .unwrap()
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with('.'))
        .collect();
    assert!(
        left.is_empty(),
        "a note being written was left behind: {left:?}"
    );
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

/// A night job that adds to `ticks` until a job Joel waits on is done,
/// then says it ran.
const TICKING: &str = "until [ -e switched ]; do echo . >> ticks; sleep 0.05; done; touch ran";

/// A switch's build that says it ran alone if nothing added to `ticks`
/// while it ran.
const ALONE: &str = r#"a=$(wc -c < ticks); sleep 1; b=$(wc -c < ticks); [ "$a" = "$b" ] && touch alone; touch switched"#;

#[test]
fn a_running_night_job_pauses_for_a_job_joel_waits_on() {
    // On 18 November a switch's evaluation waited 17 minutes behind
    // World's nightly roll, then gave up.
    let account = Account::new();
    let mut night = account.night(&[], TICKING);
    assert!(
        within(AT_MOST, || account.path("ticks").exists()),
        "the night job never ran"
    );
    let switch = account
        .turn("switch", &["--ahead", "--wait-at-most", "10"], ALONE)
        .output()
        .unwrap();
    assert!(switch.status.success(), "{}", warned(&switch));
    assert!(
        account.path("alone").exists(),
        "the night job ran on beside the switch"
    );
    let status = night.wait().unwrap();
    let said = fs::read_to_string(account.path("night")).unwrap();
    assert!(status.success(), "{said}");
    assert!(account.path("ran").exists(), "the night job never went on");
    assert!(
        said.contains("pauses, leaving the turn to switch, which Joel waits on"),
        "{said}"
    );
    assert!(said.contains("goes on, having paused"), "{said}");
}

#[test]
fn a_paused_night_job_goes_before_jobs_that_came_to_wait_meanwhile() {
    let account = Account::new();
    let mut night = account.night(&[], TICKING);
    assert!(
        within(AT_MOST, || account.path("ticks").exists()),
        "the night job never ran"
    );
    let switch = account.saying(
        "switch",
        &["--ahead"],
        "until [ -e go ]; do sleep 0.05; done; touch switched",
        "said",
    );
    assert!(
        within(AT_MOST, || account.says("night", "pauses")),
        "the night job never paused"
    );
    let tests = account.saying("cargo", &[], "[ -e ran ] && touch after", "tests");
    assert!(
        within(AT_MOST, || account.says("tests", "waiting for the turn")),
        "the tests never waited"
    );
    File::create(account.path("go")).unwrap();
    let status = switch.wait_with_output().unwrap().status;
    assert!(status.success(), "the switch failed: {status}");
    assert!(night.wait().unwrap().success(), "the night job failed");
    let status = tests.wait_with_output().unwrap().status;
    assert!(status.success(), "the tests went before the night job");
    assert!(account.path("after").exists(), "the tests never ran");
}

#[test]
fn a_night_jobs_most_counts_not_its_pauses() {
    let account = Account::new();
    let mut night = account.night(
        &["--runs-at-most", "3"],
        "touch started; until [ -e switched ]; do sleep 0.05; done; touch ran",
    );
    assert!(
        within(AT_MOST, || account.path("started").exists()),
        "the night job never ran"
    );
    let switch = account
        .turn(
            "switch",
            &["--ahead", "--wait-at-most", "10"],
            "sleep 5; touch switched",
        )
        .output()
        .unwrap();
    assert!(switch.status.success(), "{}", warned(&switch));
    let status = night.wait().unwrap();
    let said = fs::read_to_string(account.path("night")).unwrap();
    assert!(status.success(), "{said}");
    assert!(
        account.path("ran").exists(),
        "the night job was stopped for the time it was paused: {said}"
    );
}

#[test]
fn a_night_job_stopped_while_paused_goes_on_to_hear_it() {
    // systemd stops a unit by TERM; a watcher ended by it would leave its
    // job paused, deaf to the TERM sent beside it.
    let account = Account::new();
    let mut night = account.night(
        &["--grace", "10"],
        "trap 'touch heard; exit 0' TERM; until [ -e switched ]; do echo . >> ticks; sleep 0.05; done",
    );
    assert!(
        within(AT_MOST, || account.path("ticks").exists()),
        "the night job never ran"
    );
    let switch = account.saying(
        "switch",
        &["--ahead"],
        "until [ -e go ]; do sleep 0.05; done; touch switched",
        "said",
    );
    assert!(
        within(AT_MOST, || account.says("night", "pauses")),
        "the night job never paused"
    );
    Command::new("kill")
        .args(["-TERM", &night.id().to_string()])
        .status()
        .unwrap();
    let heard = within(AT_MOST, || account.path("heard").exists());
    File::create(account.path("go")).unwrap();
    let _ = switch.wait_with_output();
    let _ = night.wait();
    let said = fs::read_to_string(account.path("night")).unwrap_or_default();
    assert!(heard, "the paused job never heard its stop: {said}");
    assert!(said.contains("stopping: it was asked to stop"), "{said}");
}

#[test]
fn a_night_job_run_its_most_is_stopped_and_a_later_try_goes_on() {
    let account = Account::new();
    let began = Instant::now();
    let night = account
        .command(&[
            "run",
            "repack",
            "--start-under",
            "100",
            "--runs-at-most",
            "0.5",
            "--grace",
            "0.5",
            "--",
            "sleep",
            "30",
        ])
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&night.stdout);
    assert!(night.status.success(), "{said}");
    assert!(
        began.elapsed() < Duration::from_secs(10),
        "it ran {:?}",
        began.elapsed()
    );
    assert!(said.contains("stopping: it has run"), "{said}");
    assert!(
        !account.path("state/night-shift/repack.done").exists(),
        "a stopped job was marked done"
    );
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

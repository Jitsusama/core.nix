//! night-shift run against a stand-in machine: its pressure files and its
//! own cgroup are files in a folder of the test's, and each job is a
//! line of shell that can raise them as a real job's load would. Its
//! cases while Joel is away are in tests/away.

mod away;
mod common;

use common::Folder;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

const MANAGER: &str = "user.slice/user-1000.slice/user@1000.service";

/// A stand-in machine, with every pressure at rest.
struct Machine {
    root: Folder,
}

impl Machine {
    fn new() -> Self {
        let machine = Machine {
            root: Folder::new("night-shift"),
        };
        fs::create_dir_all(machine.root.join("proc/self")).unwrap();
        fs::create_dir_all(machine.root.join("proc/pressure")).unwrap();
        fs::write(
            machine.root.join("proc/self/cgroup"),
            format!("0::/{MANAGER}/background.slice/job.service\n"),
        )
        .unwrap();
        for file in [
            "proc/pressure/memory".to_owned(),
            format!("cgroups/{MANAGER}/app.slice/memory.pressure"),
            format!("cgroups/{MANAGER}/agents.slice/memory.pressure"),
            format!("cgroups/{MANAGER}/background.slice/job.service/memory.pressure"),
        ] {
            machine.set(&file, 0.0, 0.0);
        }
        machine
    }

    fn path(&self, file: &str) -> PathBuf {
        self.root.join(file)
    }

    fn set(&self, file: &str, avg10: f64, avg60: f64) {
        let path = self.path(file);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, line(avg10, avg60)).unwrap();
    }

    fn state(&self, file: &str) -> PathBuf {
        self.root.join("state/night-shift").join(file)
    }

    /// Runs `night-shift run` with `options` and the shell line `job`.
    fn run(&self, options: &str, job: &str) -> Output {
        self.command(options, job).output().unwrap()
    }

    /// `night-shift run` with `options` and the shell line `job`, to start.
    fn command(&self, options: &str, job: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_night-shift"));
        command
            .args(["run", "test-job"])
            .args(options.split_whitespace())
            .args(["--every", "0.05", "--grace", "0.5", "--", "sh", "-c", job])
            .env("NIGHT_SHIFT_PROC", self.path("proc"))
            .env("NIGHT_SHIFT_CGROUPS", self.path("cgroups"))
            .env("XDG_STATE_HOME", self.path("state"))
            .env("XDG_RUNTIME_DIR", self.path("runtime"))
            // Its hours are read by the local clock, five and a half hours
            // ahead of UTC here, so hours read by UTC's clock by mistake
            // fall outside a test's.
            .env("TZ", "XST-5:30")
            .current_dir(&self.root)
            .stdout(Stdio::piped());
        command
    }

    /// Takes the turn as another heavy job would, until the file is dropped.
    fn hold_the_turn(&self) -> File {
        fs::create_dir_all(self.path("runtime/night-shift")).unwrap();
        let held = File::create(self.path("runtime/night-shift/lock")).unwrap();
        held.try_lock().unwrap();
        held
    }
}

/// A pressure file's text with `full` at these figures.
fn line(avg10: f64, avg60: f64) -> String {
    format!(
        "some avg10={avg10:.2} avg60={avg60:.2} avg300=0.00 total=1\n\
         full avg10={avg10:.2} avg60={avg60:.2} avg300=0.00 total=1\n"
    )
}

/// A shell line writing `file` with `full` at `avg10`, as the job's load
/// would raise it.
fn raise(machine: &Machine, file: &str, avg10: f64) -> String {
    format!(
        "printf '{}' > '{}'",
        line(avg10, 0.0).replace('\n', "\\n"),
        machine.path(file).display()
    )
}

/// What a run said: its log's lines on stdout, then a failure on stderr,
/// where the terminal kit says one.
fn said(output: &Output) -> String {
    [&output.stdout, &output.stderr]
        .map(|stream| String::from_utf8_lossy(stream).into_owned())
        .concat()
}

#[test]
fn a_quiet_machine_runs_the_job_once_a_night() {
    let machine = Machine::new();
    let job = "echo ran >> ran";
    let first = machine.run("", job);
    let second = machine.run("", job);
    assert!(
        first.status.success() && second.status.success(),
        "{}{}",
        said(&first),
        said(&second)
    );
    assert_eq!(fs::read_to_string(machine.path("ran")).unwrap(), "ran\n");
    assert!(
        machine.state("test-job.done").exists(),
        "a success left no done mark"
    );
    assert!(said(&second).contains("already done"), "{}", said(&second));
}

#[test]
fn a_done_job_runs_again_once_it_has_rested() {
    let machine = Machine::new();
    machine.run("", "echo ran >> ran");
    let again = machine.run("--rests 0", "echo ran >> ran");
    assert!(again.status.success(), "{}", said(&again));
    assert_eq!(
        fs::read_to_string(machine.path("ran")).unwrap(),
        "ran\nran\n"
    );
}

/// How far the tests' local clock is ahead of UTC's, in minutes.
const AHEAD_OF_UTC: i64 = 5 * 60 + 30;

/// The minute `minutes` from now, as the tests' local clock reads it,
/// `HH:MM`.
fn clock_in(minutes: i64) -> String {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("a clock after 1970");
    let now = i64::try_from(since.as_secs() / 60).expect("minutes that fit");
    let minute = (now + AHEAD_OF_UTC + minutes).rem_euclid(24 * 60);
    format!("{:02}:{:02}", minute / 60, minute % 60)
}

#[test]
fn a_start_within_its_hours_runs_the_job() {
    let machine = Machine::new();
    let hours = format!("--hours {}-{}", clock_in(-5), clock_in(5));
    let ran = machine.run(&hours, "echo ran >> ran");
    assert!(ran.status.success(), "{}", said(&ran));
    assert_eq!(
        fs::read_to_string(machine.path("ran")).ok().as_deref(),
        Some("ran\n"),
        "a start within its hours didn't run the job: {}",
        said(&ran)
    );
}

#[test]
fn a_start_outside_its_hours_runs_nothing_and_isnt_a_failure() {
    // A switch can start a night job's timer, and with it the job, by day.
    let machine = Machine::new();
    fs::create_dir_all(machine.state("")).unwrap();
    fs::write(machine.state("test-job.samples"), "last night's\n").unwrap();
    let hours = format!("--hours {}-{}", clock_in(60), clock_in(120));
    let ran = machine.run(&hours, "echo ran >> ran");
    assert!(
        ran.status.success(),
        "a start outside its hours failed its unit: {}",
        said(&ran)
    );
    assert!(
        !machine.path("ran").exists(),
        "a start outside its hours ran the job"
    );
    assert!(
        said(&ran).contains("outside its hours"),
        "a start outside its hours didn't say why it ran nothing: {}",
        said(&ran)
    );
    assert!(
        ran.stderr.is_empty(),
        "a start outside its hours said a failure: {}",
        said(&ran)
    );
    assert_eq!(
        fs::read_to_string(machine.state("test-job.samples"))
            .ok()
            .as_deref(),
        Some("last night's\n"),
        "a start outside its hours wrote over last night's samples"
    );
    assert!(
        !machine.path("runtime/night-shift/lock").exists(),
        "a start outside its hours took the turn"
    );
}

/// Moves the stand-in clock in one step, by a rename, so the job never
/// reads it half written.
fn set_the_clock(clock: &Path, time: &str) {
    let next = clock.with_extension("next");
    fs::write(&next, format!("{time}\n")).unwrap();
    fs::rename(&next, clock).unwrap();
}

/// A job within its hours by a stand-in clock at 03:00, waiting for a
/// turn the test holds, with the hours over by the time the test gives
/// the clock 06:00 and `then` has run. Gives what it said in all, and
/// whether it ended as a success.
fn wait_past_its_hours(machine: &Machine, then: impl FnOnce()) -> (String, bool) {
    let clock = machine.path("clock");
    set_the_clock(&clock, "03:00");
    let mut waiting = machine
        .command("--hours 01:30-05:15 --wait-at-most 10", "echo ran > ran")
        .env("NIGHT_SHIFT_CLOCK", &clock)
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = BufReader::new(waiting.stdout.take().expect("a piped stdout"));
    // Its own line comes after its look at the hours in the wait, so the
    // clock moves, and `then` runs, a whole look before the next one.
    let mut said = String::new();
    while !said.contains("waiting for the turn") {
        let read = stdout.read_line(&mut said).expect("a readable stdout");
        assert!(
            read > 0,
            "the job ended without waiting for the turn: {said}"
        );
    }
    set_the_clock(&clock, "06:00");
    then();
    stdout.read_to_string(&mut said).expect("a readable stdout");
    let output = waiting.wait_with_output().unwrap();
    said.push_str(&String::from_utf8_lossy(&output.stderr));
    (said, output.status.success())
}

#[test]
fn a_turn_had_once_its_hours_are_over_runs_nothing() {
    let machine = Machine::new();
    let held = machine.hold_the_turn();
    // The turn is free at the wait's next look, which takes it before it
    // reads the clock, so only the look after the turn can stop it.
    let (said, success) = wait_past_its_hours(&machine, || drop(held));
    assert!(success, "a turn had past its hours failed its unit: {said}");
    assert!(
        !machine.path("ran").exists(),
        "a turn had past its hours ran the job by day"
    );
    assert!(
        said.contains("06:00 is outside its hours, 01:30-05:15") && !said.contains("has the turn"),
        "a turn had past its hours didn't say why it ran nothing: {said}"
    );
}

#[test]
fn hours_closing_while_it_waits_end_the_wait() {
    let machine = Machine::new();
    let held = machine.hold_the_turn();
    let began = Instant::now();
    let (said, success) = wait_past_its_hours(&machine, || {});
    drop(held);
    assert!(success, "a wait its hours ended failed its unit: {said}");
    assert!(
        began.elapsed() < Duration::from_secs(8),
        "a wait its hours ended ran on to its most: {said}"
    );
    assert!(
        said.contains("06:00 is outside its hours, 01:30-05:15")
            && !said.contains("a later try goes on"),
        "a wait its hours ended didn't say why it ran nothing: {said}"
    );
}

#[test]
fn a_clock_it_cant_read_runs_nothing() {
    let machine = Machine::new();
    let clock = machine.path("clock");
    fs::write(&clock, "noon\n").unwrap();
    let ran = machine
        .command("--hours 01:30-05:15", "echo ran > ran")
        .env("NIGHT_SHIFT_CLOCK", &clock)
        .output()
        .unwrap();
    assert!(
        ran.status.success(),
        "an unread clock failed its unit: {}",
        said(&ran)
    );
    assert!(!machine.path("ran").exists(), "an unread clock ran the job");
    assert!(
        said(&ran).contains("an unread clock is outside its hours"),
        "an unread clock didn't say why it ran nothing: {}",
        said(&ran)
    );
}

#[test]
fn a_busy_machine_starts_nothing() {
    let machine = Machine::new();
    machine.set("proc/pressure/memory", 0.0, 3.5);
    let output = machine.run("--start-under 3", "echo ran > ran");
    assert!(output.status.success(), "{}", said(&output));
    assert!(!machine.path("ran").exists(), "the job ran");
    assert!(
        !machine.state("test-job.done").exists(),
        "a refused start left a done mark"
    );
    assert!(
        said(&output).contains("not started: memory full avg60 is 3.50%"),
        "{}",
        said(&output)
    );
}

#[test]
fn only_one_night_job_runs_at_a_time() {
    let machine = Machine::new();
    let _held = machine.hold_the_turn();
    let output = machine.run("", "echo ran > ran");
    assert!(output.status.success(), "{}", said(&output));
    assert!(!machine.path("ran").exists(), "the job ran beside another");
    assert!(
        said(&output).contains("another heavy job is running"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_night_job_told_to_wait_takes_the_turn_once_it_is_free() {
    let machine = Machine::new();
    let held = machine.hold_the_turn();
    let waiting = machine
        .command("--wait-at-most 10", "echo ran > ran")
        .spawn()
        .unwrap();
    // Long enough for the job to have started beside the held turn, were
    // it going to.
    thread::sleep(Duration::from_millis(300));
    assert!(!machine.path("ran").exists(), "the job ran beside another");
    drop(held);
    let output = waiting.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", said(&output));
    assert!(machine.path("ran").exists(), "{}", said(&output));
    // The holder left no note, so the kernel's holder, this test, is named.
    let holder = format!(
        "waiting for the turn, which process {} has",
        std::process::id()
    );
    assert!(
        said(&output).contains(&holder) && said(&output).contains("has the turn after"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_night_job_waiting_past_its_most_leaves_with_try_again_later() {
    let machine = Machine::new();
    let _held = machine.hold_the_turn();
    let output = machine.run("--wait-at-most 0.3", "echo ran > ran");
    assert_eq!(output.status.code(), Some(75), "{}", said(&output));
    assert!(!machine.path("ran").exists(), "the job ran beside another");
    assert!(
        said(&output).contains("not started, having waited as long as it was let"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_night_job_that_waited_still_starts_only_on_a_quiet_machine() {
    let machine = Machine::new();
    let held = machine.hold_the_turn();
    let waiting = machine
        .command("--wait-at-most 10 --start-under 3", "echo ran > ran")
        .spawn()
        .unwrap();
    // Long enough for the job to be waiting for the turn when the machine
    // grows busy; it reads the machine only once it has the turn.
    thread::sleep(Duration::from_millis(300));
    machine.set("proc/pressure/memory", 0.0, 3.5);
    drop(held);
    let output = waiting.wait_with_output().unwrap();
    assert!(output.status.success(), "{}", said(&output));
    assert!(
        !machine.path("ran").exists(),
        "the job ran on a busy machine"
    );
    assert!(
        said(&output).contains("not started: memory full avg60 is 3.50%"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_stall_in_a_watched_slice_stops_the_job_and_kills_what_ignores_term() {
    let machine = Machine::new();
    let app = format!("cgroups/{MANAGER}/app.slice/memory.pressure");
    let job = format!("trap '' TERM; {}; sleep 30", raise(&machine, &app, 1.5));
    let began = Instant::now();
    let output = machine.run("--stop app.slice=1 --stop agents.slice=10", &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        began.elapsed() < Duration::from_secs(20),
        "it waited for the job"
    );
    assert!(
        said.contains("stopping: app.slice's memory full avg10 is 1.50%, past 1%"),
        "{said}"
    );
    assert!(said.contains("killing what is left"), "{said}");
    assert!(
        !machine.state("test-job.done").exists(),
        "a stopped job left a done mark"
    );
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(
        samples.contains("app.slice 1.50 agents.slice 0.00 own 0.00"),
        "{samples}"
    );
}

#[test]
fn the_machine_stalling_past_its_backstop_stops_the_job() {
    let machine = Machine::new();
    let job = format!(
        "{}; sleep 30",
        raise(&machine, "proc/pressure/memory", 26.0)
    );
    let output = machine.run("--stop app.slice=1 --stop-machine 25", &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains(
            "stopping: memory full avg10 is 26.00%, 26.00% beyond the job's own, past 25%"
        ),
        "{said}"
    );
    assert!(said.contains("stopped; a later try goes on"), "{said}");
    assert!(
        !machine.state("test-job.done").exists(),
        "a stopped job left a done mark"
    );
}

#[test]
fn a_job_stalling_only_itself_runs_on() {
    let machine = Machine::new();
    let own = format!("cgroups/{MANAGER}/background.slice/job.service/memory.pressure");
    let job = format!("{}; sleep 0.5", raise(&machine, &own, 60.0));
    let output = machine.run("--stop app.slice=1 --stop agents.slice=10", &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(said.contains("finished"), "{said}");
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(samples.contains("own 60.00"), "{samples}");
}

#[test]
fn the_machines_backstop_is_held_to_what_it_waits_beyond_the_jobs_own() {
    let own = format!("cgroups/{MANAGER}/background.slice/job.service/memory.pressure");
    // Beyond the job's own 20%, the machine's 40% is 20, under the bar of
    // 25, while its 50% is 30, past it.
    for (machine_at, stopped) in [(40.0, false), (50.0, true)] {
        let machine = Machine::new();
        // Long enough for a sample or two at the machine's figure.
        let job = format!(
            "{}; {}; sleep 1",
            raise(&machine, &own, 20.0),
            raise(&machine, "proc/pressure/memory", machine_at)
        );
        let output = machine.run("--stop app.slice=1 --stop-machine 25", &job);
        let said = said(&output);
        assert!(output.status.success(), "at {machine_at}: {said}");
        let beyond = machine_at - 20.0;
        assert_eq!(
            said.contains(&format!(
                "stopping: memory full avg10 is {machine_at:.2}%, {beyond:.2}% beyond the job's own, past 25%"
            )),
            stopped,
            "at {machine_at}, with the job's own at 20: {said}"
        );
        assert_eq!(
            said.contains("finished"),
            !stopped,
            "at {machine_at}, it should finish only where it wasn't stopped: {said}"
        );
    }
}

#[test]
fn a_busy_disk_starts_nothing_only_where_it_has_an_io_bar() {
    let machine = Machine::new();
    machine.set("proc/pressure/io", 0.0, 6.0);
    let output = machine.run("--stop-io 5", "echo ran > ran");
    assert!(output.status.success(), "{}", said(&output));
    assert!(!machine.path("ran").exists(), "the job ran on a busy disk");
    assert!(
        said(&output).contains("not started: io full avg60 is 6.00%, not under 5%"),
        "{}",
        said(&output)
    );
    let unbarred = machine.run("", "echo ran > ran");
    assert!(
        machine.path("ran").exists(),
        "a job with no io bar didn't run on a busy disk: {}",
        said(&unbarred)
    );
}

#[test]
fn the_disk_stalling_past_its_io_bar_stops_the_job() {
    let machine = Machine::new();
    machine.set("proc/pressure/io", 0.0, 1.0);
    // The minute's figure, as a job reading the disk hard would raise it.
    let job = format!(
        "printf '{}' > '{}'; sleep 30",
        line(0.0, 6.0).replace('\n', "\\n"),
        machine.path("proc/pressure/io").display()
    );
    let output = machine.run("--stop-io 5", &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("stopping: io full avg60 is 6.00%, 6.00% beyond the job's own, past 5%"),
        "{said}"
    );
    assert!(said.contains("stopped; a later try goes on"), "{said}");
    assert!(
        !machine.state("test-job.done").exists(),
        "a job stopped for the disk left a done mark"
    );
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(samples.contains(" io 6.00 "), "{samples}");
    assert!(samples.contains(" own io 0.00"), "{samples}");
}

#[test]
fn a_job_waiting_on_the_disk_itself_runs_on_past_its_io_bar() {
    let machine = Machine::new();
    machine.set("proc/pressure/io", 0.0, 1.0);
    let own = machine.path(&format!(
        "cgroups/{MANAGER}/background.slice/job.service/io.pressure"
    ));
    let io = machine.path("proc/pressure/io");
    // The machine's minute at 8%, 6 of it the job's own: 2 beyond it, under
    // the bar of 5. The sleep gives it a sample or two at that figure.
    let job = format!(
        "printf '{}' > '{}'; printf '{}' > '{}'; sleep 1",
        line(0.0, 6.0).replace('\n', "\\n"),
        own.display(),
        line(0.0, 8.0).replace('\n', "\\n"),
        io.display()
    );
    let output = machine.run("--stop-io 5", &job);
    let said = said(&output);
    assert!(output.status.success(), "{said}");
    assert!(said.contains("finished"), "{said}");
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(samples.contains(" io 8.00 "), "{samples}");
    assert!(samples.contains(" own io 6.00"), "{samples}");
}

#[test]
fn an_io_bar_with_no_io_to_read_is_said() {
    let machine = Machine::new();
    let output = machine.run("--stop-io 5", "true");
    assert!(output.status.success(), "{}", said(&output));
    assert!(
        said(&output).contains("the machine has no io pressure to read"),
        "{}",
        said(&output)
    );
}

const UNIT: &str = "background.slice/job.service";

#[test]
fn a_job_with_limits_runs_in_a_cgroup_apart_from_its_watcher() {
    let machine = Machine::new();
    let output = machine.run(
        "--memory-high 12G --swap-max 2G --reads-at-most /dev/null=50M",
        "echo ran > ran",
    );
    assert!(output.status.success(), "{}", said(&output));
    assert!(machine.path("ran").exists(), "the job didn't run");
    let unit = format!("cgroups/{MANAGER}/{UNIT}");
    let read = |file: &str| fs::read_to_string(machine.path(&format!("{unit}/{file}"))).unwrap();
    assert!(
        read("watch/cgroup.procs").parse::<u32>().is_ok(),
        "the watcher didn't move into its own cgroup: {}",
        read("watch/cgroup.procs")
    );
    assert_eq!(read("cgroup.subtree_control"), "+memory +io");
    assert_eq!(read("job/memory.high"), "12G");
    assert_eq!(read("job/memory.swap.max"), "2G");
    assert_eq!(read("job/io.max"), "1:3 rbps=50000000");
    assert_eq!(
        read("job/cgroup.procs"),
        "0",
        "the job didn't move itself into its cgroup"
    );
}

#[test]
fn a_job_with_limits_is_watched_by_its_own_cgroup() {
    let machine = Machine::new();
    let own = format!("cgroups/{MANAGER}/{UNIT}/job/memory.pressure");
    let job = format!("{}; sleep 0.5", raise(&machine, &own, 60.0));
    let output = machine.run("--memory-high 12G --stop app.slice=1", &job);
    let said = said(&output);
    assert!(said.contains("finished"), "{said}");
    let samples = fs::read_to_string(machine.state("test-job.samples")).unwrap();
    assert!(samples.contains("own 60.00"), "{samples}");
}

#[test]
fn a_unit_that_cant_be_split_runs_nothing_and_says_what_it_needs() {
    let machine = Machine::new();
    fs::write(machine.path(&format!("cgroups/{MANAGER}/{UNIT}/job")), "").unwrap();
    let output = machine.run("--memory-high 12G", "echo ran > ran");
    assert!(
        !output.status.success(),
        "a job without its limits counted as run"
    );
    assert!(
        !machine.path("ran").exists(),
        "the job ran without its limits"
    );
    assert!(
        said(&output).contains("its unit needs Delegate=memory io"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_slice_with_nothing_to_read_is_said() {
    let machine = Machine::new();
    let output = machine.run("--stop session.slice=1", "true");
    assert!(output.status.success(), "{}", said(&output));
    assert!(
        said(&output).contains("session.slice has no pressure to read"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_failing_job_fails_the_run_with_its_status() {
    let machine = Machine::new();
    let output = machine.run("", "exit 3");
    assert_eq!(output.status.code(), Some(3));
    assert!(
        !machine.state("test-job.done").exists(),
        "a failed job left a done mark"
    );
    assert!(
        said(&output).contains("the job failed"),
        "{}",
        said(&output)
    );
}

#[test]
fn a_command_that_isnt_there_fails_the_run() {
    let machine = Machine::new();
    let output = Command::new(env!("CARGO_BIN_EXE_night-shift"))
        .args(["run", "test-job", "--", "no-such-program-anywhere"])
        .env("NIGHT_SHIFT_PROC", machine.path("proc"))
        .env("NIGHT_SHIFT_CGROUPS", machine.path("cgroups"))
        .env("XDG_STATE_HOME", machine.path("state"))
        .env("XDG_RUNTIME_DIR", machine.path("runtime"))
        .output()
        .unwrap();
    assert!(!output.status.success(), "a missing program counted as run");
    assert!(
        said(&output).contains("can't start no-such-program-anywhere"),
        "{}",
        said(&output)
    );
}

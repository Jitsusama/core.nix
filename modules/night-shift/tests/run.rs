//! night-shift run against a stand-in machine: its pressure files and its
//! own cgroup are files in a folder of the test's, and each job is a
//! line of shell that can raise them as a real job's load would.

use std::fs::{self, File};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

const MANAGER: &str = "user.slice/user-1000.slice/user@1000.service";

/// A stand-in machine, with every pressure at rest.
struct Machine {
    root: PathBuf,
}

impl Machine {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "night-shift-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        let machine = Machine { root };
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
        let mut command = Command::new(env!("CARGO_BIN_EXE_night-shift"));
        command
            .args(["run", "test-job"])
            .args(options.split_whitespace())
            .args(["--every", "0.05", "--grace", "0.5", "--", "sh", "-c", job])
            .env("NIGHT_SHIFT_PROC", self.path("proc"))
            .env("NIGHT_SHIFT_CGROUPS", self.path("cgroups"))
            .env("XDG_STATE_HOME", self.path("state"))
            .env("XDG_RUNTIME_DIR", self.path("runtime"))
            .current_dir(&self.root);
        command.output().unwrap()
    }
}

impl Drop for Machine {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
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

fn said(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
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
    fs::create_dir_all(machine.path("runtime/night-shift")).unwrap();
    let held = File::create(machine.path("runtime/night-shift/lock")).unwrap();
    held.try_lock().unwrap();
    let output = machine.run("", "echo ran > ran");
    assert!(output.status.success(), "{}", said(&output));
    assert!(!machine.path("ran").exists(), "the job ran beside another");
    assert!(
        said(&output).contains("another night job is running"),
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
        said.contains("stopping: memory full avg10 is 26.00%, past 25%"),
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

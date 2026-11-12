//! Runs a heavy job at night, once, only while the machine has memory to
//! spare, and stops it the moment what Joel uses waits for memory.
//!
//! A large repository's daily maintenance rewrote its 3.3 GB pack index
//! at a quarter past midnight on 11 November, holding 9.3 GB, with
//! memory's full pressure at 10.5%, and nothing held it back. So a job
//! like it starts only when the machine's full memory pressure over the
//! last minute is under `--start-under`, and only one such job runs at a
//! time. Once it runs, it is stopped if a slice's full pressure over the
//! last ten seconds passes what `--stop` gives it, or the machine's
//! passes `--stop-machine`.
//!
//! The slices are watched, rather than the machine alone, because a job
//! that stalls at its own memory limit counts in the machine's figure:
//! the same night, a 2 GB search took the machine's full pressure past
//! 10% twice within 30 seconds while the apps' and the desktop's slices
//! stalled not at all. The machine's bar is a backstop, and every sample,
//! with the job's own cgroup's, goes to `<state>/night-shift/NAME.samples`,
//! so a stop says whose stall it was.
//!
//! A stopped job is sent TERM in its own process group first, so git and
//! the like give back their locks, and is killed once `--grace` passes.
//! A success leaves `<state>/night-shift/NAME.done`, and for `--rests`
//! hours after it a start does nothing, so a timer can try every quarter
//! hour of a night and the job still runs once. A start refused, or a job
//! stopped, leaves the later tries their turn, and the run succeeds; only
//! the job's own failure fails it, with its status.
//!
//! The job's limits, `--memory-high`, `--memory-max`, `--swap-max` and
//! `--reads-at-most`, go on a cgroup of the job's own, apart from the one
//! night-shift watches from (src/cgroup.rs says why), so its unit needs
//! `Delegate=memory io` when any is given.
//!
//! A heavy job by day takes the same lock through `night-shift turn`,
//! which waits for it rather than leaving the turn (src/turn.rs says how),
//! so a night job tried meanwhile leaves it to a later try.
//!
//! Usage: night-shift turn NAME [--wait-at-most SECONDS] -- COMMAND [ARG]...
//!        night-shift run NAME [--start-under PCT] [--stop SLICE=PCT]...
//!                            [--stop-machine PCT] [--rests HOURS]
//!                            [--every SECONDS] [--grace SECONDS]
//!                            [--memory-high SIZE] [--memory-max SIZE]
//!                            [--swap-max SIZE] [--reads-at-most DISK=RATE]...
//!                            -- COMMAND [ARG]...

mod cgroup;
mod pressure;
mod turn;

use cgroup::Limits;
use pressure::Places;
use std::fmt::Write as _;
use std::fs::{self, File, TryLockError};
use std::io::{self, Write as _};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

/// One night job, as its arguments give it.
#[derive(Debug, PartialEq)]
struct Job {
    name: String,
    start_under: f64,
    stops: Vec<(String, f64)>,
    stop_machine: f64,
    rests: Duration,
    every: Duration,
    grace: Duration,
    limits: Limits,
    command: Vec<String>,
}

const USAGE: &str = "usage: night-shift turn NAME [--wait-at-most SECONDS] -- COMMAND [ARG]...\n       \
     night-shift run NAME [--start-under PCT] [--stop SLICE=PCT]... \
     [--stop-machine PCT] [--rests HOURS] [--every SECONDS] [--grace SECONDS] \
     [--memory-high SIZE] [--memory-max SIZE] [--swap-max SIZE] \
     [--reads-at-most DISK=RATE]... -- COMMAND [ARG]...";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("turn") {
        return match turn::parse(&args) {
            Ok(asked) => turn::take(&asked, &runtime_dir()),
            Err(problem) => {
                eprintln!("night-shift: {problem}");
                eprintln!("{USAGE}");
                ExitCode::from(2)
            }
        };
    }
    match parse(&args) {
        Ok(job) => run(&job),
        Err(problem) => {
            eprintln!("night-shift: {problem}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn parse(args: &[String]) -> Result<Job, String> {
    let (Some("run"), Some(name)) = (args.first().map(String::as_str), args.get(1)) else {
        return Err("it takes `run` or `turn` and a job's name".into());
    };
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("it takes `--` and the job's command")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut job = Job {
        name: name.clone(),
        start_under: 3.0,
        stops: Vec::new(),
        stop_machine: 25.0,
        rests: Duration::from_hours(12),
        every: Duration::from_secs(5),
        grace: Duration::from_secs(60),
        limits: Limits::default(),
        command,
    };
    let mut options = args[2..split].iter();
    while let Some(option) = options.next() {
        let value = options
            .next()
            .ok_or_else(|| format!("{option} takes a value"))?;
        let number = |text: &str| {
            text.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite() && *n >= 0.0)
                .ok_or_else(|| format!("{option} takes a number, not {text}"))
        };
        match option.as_str() {
            "--start-under" => job.start_under = number(value)?,
            "--stop-machine" => job.stop_machine = number(value)?,
            "--rests" => job.rests = Duration::from_secs_f64(number(value)? * 3600.0),
            "--every" => job.every = Duration::from_secs_f64(number(value)?),
            "--grace" => job.grace = Duration::from_secs_f64(number(value)?),
            "--stop" => {
                let (slice, bar) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--stop takes SLICE=PCT, not {value}"))?;
                job.stops.push((slice.to_owned(), number(bar)?));
            }
            "--memory-high" => job.limits.memory_high = Some(value.clone()),
            "--memory-max" => job.limits.memory_max = Some(value.clone()),
            "--swap-max" => job.limits.swap_max = Some(value.clone()),
            "--reads-at-most" => {
                let (disk, rate) = value
                    .split_once('=')
                    .ok_or_else(|| format!("--reads-at-most takes DISK=RATE, not {value}"))?;
                let bytes = cgroup::rate(rate).ok_or_else(|| {
                    format!("--reads-at-most takes a rate such as 50M, not {rate}")
                })?;
                job.limits.reads.push((PathBuf::from(disk), bytes));
            }
            _ => return Err(format!("{option} isn't an option")),
        }
    }
    Ok(job)
}

/// What a line of the job's log says, with its name.
fn say(job: &Job, what: &str) {
    println!("night-shift {}: {what}", job.name);
}

/// Where the lock is: one for the account, which a night job and a heavy
/// job by day take alike.
fn runtime_dir() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || dir("XDG_STATE_HOME", ".local/state").join("night-shift"),
        |dir| PathBuf::from(dir).join("night-shift"),
    )
}

fn dir(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var).map_or_else(
        || PathBuf::from(std::env::var_os("HOME").unwrap_or_default()).join(fallback),
        PathBuf::from,
    )
}

/// Where a job keeps what outlasts it, and where the lock is.
struct Folders {
    state: PathBuf,
    runtime: PathBuf,
}

fn run(job: &Job) -> ExitCode {
    let state = dir("XDG_STATE_HOME", ".local/state").join("night-shift");
    let places = Folders {
        state,
        runtime: runtime_dir(),
    };
    let (_lock, at) = match may_start(job, &places) {
        Ok(started) => started,
        Err(code) => return code,
    };
    let pressures = Places::of_this_machine();
    let watched: Vec<(String, PathBuf, f64)> = job
        .stops
        .iter()
        .map(|(slice, bar)| (slice.clone(), pressures.slice_file(slice), *bar))
        .collect();
    for (slice, file, _) in &watched {
        if pressure::read(file).is_none() {
            say(
                job,
                &format!(
                    "{slice} has no pressure to read at {}, so it can't stop the job",
                    file.display()
                ),
            );
        }
    }
    let samples = match File::create(places.state.join(format!("{}.samples", job.name))) {
        Ok(file) => file,
        Err(error) => {
            say(job, &format!("can't write its samples: {error}"));
            return ExitCode::FAILURE;
        }
    };
    let mut command = Command::new(&job.command[0]);
    command.args(&job.command[1..]).process_group(0);
    // Its own cgroup's pressure, the job's when it has one apart.
    let mut own = pressures.own_dir();
    // The job's cgroup's procs, held open until it has moved in.
    let mut procs_open = None;
    if job.limits.any() {
        let Some(unit) = own.clone() else {
            say(job, "can't find its own cgroup to give the job its limits");
            return ExitCode::FAILURE;
        };
        match cgroup::split(&unit, &job.limits) {
            Ok((cgroup, procs)) => {
                enter_on_start(&mut command, &procs);
                own = Some(cgroup);
                procs_open = Some(procs);
            }
            Err(error) => {
                say(
                    job,
                    &format!(
                        "can't give the job a cgroup of its own under {}, as its limits need: {error}; its unit needs Delegate=memory io",
                        unit.display()
                    ),
                );
                return ExitCode::FAILURE;
            }
        }
    }
    let own = own.map(|dir| dir.join("memory.pressure"));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            say(job, &format!("can't start {}: {error}", job.command[0]));
            return ExitCode::FAILURE;
        }
    };
    drop(procs_open);
    say(
        job,
        &format!(
            "started at memory full avg60 {at:.2}%: {}",
            job.command.join(" ")
        ),
    );
    match watch(
        job,
        &mut child,
        &pressures,
        own.as_deref(),
        &watched,
        samples,
    ) {
        Some(status) => finish(
            job,
            status,
            &places.state.join(format!("{}.done", job.name)),
        ),
        None => stop(job, &mut child),
    }
}

/// The lock and the machine's pressure a job starts at, or how the run
/// ends when it doesn't start: done already, another job running, or the
/// machine short of memory.
fn may_start(job: &Job, places: &Folders) -> Result<(File, f64), ExitCode> {
    for place in [&places.state, &places.runtime] {
        if let Err(error) = fs::create_dir_all(place) {
            say(job, &format!("can't make {}: {error}", place.display()));
            return Err(ExitCode::FAILURE);
        }
    }
    let done = places.state.join(format!("{}.done", job.name));
    if let Some(ago) = fs::metadata(&done)
        .and_then(|meta| meta.modified())
        .ok()
        .and_then(|at| at.elapsed().ok())
        .filter(|ago| *ago < job.rests)
    {
        say(
            job,
            &format!(
                "already done {} minutes ago, nothing to run",
                ago.as_secs() / 60
            ),
        );
        return Err(ExitCode::SUCCESS);
    }

    // Held until this process ends, however it ends.
    let lock = File::create(places.runtime.join("lock")).map_err(|error| {
        say(job, &format!("can't open the lock: {error}"));
        ExitCode::FAILURE
    })?;
    match lock.try_lock() {
        Ok(()) => turn::note(&places.runtime, &job.name),
        Err(TryLockError::WouldBlock) => {
            say(
                job,
                &format!(
                    "not started: another heavy job is running, {}, and only one runs at a time; a later try goes on",
                    turn::holder(&places.runtime)
                ),
            );
            return Err(ExitCode::SUCCESS);
        }
        Err(TryLockError::Error(error)) => {
            say(job, &format!("can't take the lock: {error}"));
            return Err(ExitCode::FAILURE);
        }
    }

    let now = pressure::read(&Places::of_this_machine().machine()).unwrap_or_default();
    if now.avg60 >= job.start_under {
        say(
            job,
            &format!(
                "not started: memory full avg60 is {:.2}%, not under {}%; a later try goes on",
                now.avg60, job.start_under
            ),
        );
        return Err(ExitCode::SUCCESS);
    }
    Ok((lock, now.avg60))
}

/// Samples the pressures until the job ends, giving its status, or until
/// one passes its bar, giving None once it has said which.
fn watch(
    job: &Job,
    child: &mut Child,
    pressures: &Places,
    own: Option<&Path>,
    watched: &[(String, PathBuf, f64)],
    mut samples: File,
) -> Option<ExitStatus> {
    let began = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Some(status),
            Ok(None) => {}
            Err(error) => {
                say(job, &format!("can't tell how the job is: {error}"));
                return None;
            }
        }
        let own_now = own.and_then(pressure::read).unwrap_or_default();
        let machine = pressure::read(&pressures.machine()).unwrap_or_default();
        let mut line = format!(
            "{}s machine {:.2}",
            began.elapsed().as_secs(),
            machine.avg10
        );
        let mut past = None;
        for (slice, file, bar) in watched {
            let reading = pressure::read(file).unwrap_or_default();
            let _ = write!(line, " {slice} {:.2}", reading.avg10);
            if reading.avg10 > *bar && past.is_none() {
                past = Some(format!(
                    "{slice}'s memory full avg10 is {:.2}%, past {bar}%",
                    reading.avg10
                ));
            }
        }
        if past.is_none() && machine.avg10 > job.stop_machine {
            past = Some(format!(
                "memory full avg10 is {:.2}%, past {}%",
                machine.avg10, job.stop_machine
            ));
        }
        let _ = writeln!(samples, "{line} own {:.2}", own_now.avg10);
        if let Some(why) = past {
            say(
                job,
                &format!("stopping: {why} (its own {:.2}%)", own_now.avg10),
            );
            return None;
        }
        thread::sleep(job.every);
    }
}

fn finish(job: &Job, status: ExitStatus, done: &Path) -> ExitCode {
    if status.success() {
        if let Err(error) = File::create(done) {
            say(job, &format!("finished, but can't mark it done: {error}"));
            return ExitCode::FAILURE;
        }
        say(job, "finished");
        return ExitCode::SUCCESS;
    }
    say(job, &format!("the job failed: {status}"));
    ExitCode::from(
        status
            .code()
            .and_then(|code| u8::try_from(code).ok())
            .unwrap_or(1),
    )
}

// std links libc, so these come from it.
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
    fn write(fd: i32, buf: *const std::ffi::c_void, count: usize) -> isize;
}

/// Has the job write itself into its cgroup's `procs` before its program
/// runs, so nothing it holds is counted outside its limits.
fn enter_on_start(command: &mut Command, procs: &File) {
    let fd = procs.as_raw_fd();
    // SAFETY: between fork and exec the closure only calls write(2) on a
    // descriptor the parent holds open until the spawn returns, and reads
    // errno, neither of which allocates or takes a lock.
    unsafe {
        command.pre_exec(move || {
            if write(fd, b"0".as_ptr().cast(), 1) < 0 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}

const SIGTERM: i32 = 15;
const SIGKILL: i32 = 9;

/// Sends `signal` to every process in the job's group.
fn signal_group(child: &Child, signal: i32) {
    if let Ok(group) = i32::try_from(child.id()) {
        // SAFETY: kill takes any pid and signal, and only reports errors.
        unsafe { kill(-group, signal) };
    }
}

/// TERMs the job's group, then kills what is left once the grace passes.
fn stop(job: &Job, child: &mut Child) -> ExitCode {
    signal_group(child, SIGTERM);
    let asked = Instant::now();
    while asked.elapsed() < job.grace {
        if let Ok(Some(_)) = child.try_wait() {
            signal_group(child, SIGKILL);
            say(job, "stopped; a later try goes on");
            return ExitCode::SUCCESS;
        }
        thread::sleep(Duration::from_millis(100));
    }
    say(job, "killing what is left of it");
    signal_group(child, SIGKILL);
    let _ = child.wait();
    say(job, "stopped; a later try goes on");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_owned).collect()
    }

    #[test]
    fn a_job_takes_its_bars_and_its_command() {
        let job = parse(&args(
            "run repack --start-under 2 --stop app.slice=1 --stop agents.slice=10 --stop-machine 30 --rests 6 -- git maintenance run",
        ))
        .unwrap();
        assert_eq!(job.name, "repack");
        assert!(
            (job.start_under - 2.0).abs() < f64::EPSILON,
            "it starts under {}, not 2",
            job.start_under
        );
        assert_eq!(
            job.stops,
            vec![
                ("app.slice".to_owned(), 1.0),
                ("agents.slice".to_owned(), 10.0)
            ]
        );
        assert!(
            (job.stop_machine - 30.0).abs() < f64::EPSILON,
            "it stops past {}, not 30",
            job.stop_machine
        );
        assert_eq!(job.rests, Duration::from_hours(6));
        assert_eq!(job.command, args("git maintenance run"));
    }

    #[test]
    fn a_job_takes_its_limits() {
        let job = parse(&args(
            "run repack --memory-high 12G --swap-max 2G --reads-at-most /dev/nvme0n1=50M -- git gc",
        ))
        .unwrap();
        assert_eq!(
            job.limits,
            Limits {
                memory_high: Some("12G".to_owned()),
                memory_max: None,
                swap_max: Some("2G".to_owned()),
                reads: vec![(PathBuf::from("/dev/nvme0n1"), 50_000_000)],
            }
        );
        assert!(
            parse(&args(
                "run repack --reads-at-most /dev/nvme0n1=quick -- true"
            ))
            .is_err(),
            "a rate of words was taken"
        );
    }

    #[test]
    fn a_job_with_no_command_is_refused() {
        assert!(
            parse(&args("run repack --")).is_err(),
            "an empty command was taken"
        );
        assert!(
            parse(&args("run repack")).is_err(),
            "a missing command was taken"
        );
    }

    #[test]
    fn a_bar_that_isnt_a_number_is_refused() {
        assert!(
            parse(&args("run repack --stop app.slice=lots -- true")).is_err(),
            "a bar of words was taken"
        );
        assert!(
            parse(&args("run repack --start-under -1 -- true")).is_err(),
            "a negative bar was taken"
        );
        assert!(
            parse(&args("run repack --stop app.slice -- true")).is_err(),
            "a slice with no bar was taken"
        );
    }
}

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
//! Usage: night-shift run NAME [--start-under PCT] [--stop SLICE=PCT]...
//!                            [--stop-machine PCT] [--rests HOURS]
//!                            [--every SECONDS] [--grace SECONDS]
//!                            -- COMMAND [ARG]...

mod pressure;

use pressure::Places;
use std::fmt::Write as _;
use std::fs::{self, File, TryLockError};
use std::io::Write as _;
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
    command: Vec<String>,
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match parse(&args) {
        Ok(job) => run(&job),
        Err(problem) => {
            eprintln!("night-shift: {problem}");
            eprintln!(
                "usage: night-shift run NAME [--start-under PCT] [--stop SLICE=PCT]... \
                 [--stop-machine PCT] [--rests HOURS] [--every SECONDS] [--grace SECONDS] \
                 -- COMMAND [ARG]..."
            );
            ExitCode::from(2)
        }
    }
}

fn parse(args: &[String]) -> Result<Job, String> {
    let (Some("run"), Some(name)) = (args.first().map(String::as_str), args.get(1)) else {
        return Err("it takes `run` and a job's name".into());
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
            _ => return Err(format!("{option} isn't an option")),
        }
    }
    Ok(job)
}

/// What a line of the job's log says, with its name.
fn say(job: &Job, what: &str) {
    println!("night-shift {}: {what}", job.name);
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
    let runtime = std::env::var_os("XDG_RUNTIME_DIR").map_or_else(
        || state.clone(),
        |dir| PathBuf::from(dir).join("night-shift"),
    );
    let places = Folders { state, runtime };
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
    let mut child = match Command::new(&job.command[0])
        .args(&job.command[1..])
        .process_group(0)
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            say(job, &format!("can't start {}: {error}", job.command[0]));
            return ExitCode::FAILURE;
        }
    };
    say(
        job,
        &format!(
            "started at memory full avg60 {at:.2}%: {}",
            job.command.join(" ")
        ),
    );
    match watch(job, &mut child, &pressures, &watched, samples) {
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
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            say(
                job,
                "not started: another night job is running, and only one runs at a time; a later try goes on",
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
    watched: &[(String, PathBuf, f64)],
    mut samples: File,
) -> Option<ExitStatus> {
    let own = pressures.own_file();
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
        let own_now = own.as_deref().and_then(pressure::read).unwrap_or_default();
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

// std links libc, so this comes from it.
unsafe extern "C" {
    fn kill(pid: i32, signal: i32) -> i32;
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

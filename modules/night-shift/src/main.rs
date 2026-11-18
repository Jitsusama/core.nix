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
//! Unless the night job waits too: with `--wait-at-most`, `run` waits for
//! the turn as `turn` does, looking each second, and only then reads the
//! machine's pressure, so it starts on what the machine is once its turn
//! comes. Heavy jobs by day take the turn again within a second of its
//! release, so a night job that tries once can lose every try of a busy
//! night. A run that waited as long as it was let ends 75, as `turn`
//! does, so its unit can say a later try goes on.
//!
//! With `--hours`, a start outside them runs nothing and succeeds, as one
//! on a busy machine does, since a switch can start a night job's timer,
//! and the timer its job, by day (src/hours.rs says when it did).
//!
//! Its help, its failures and its exits take the terminal kit's shape
//! (src/commands.rs); a failed job's run ends with the job's own code.

mod cgroup;
mod commands;
mod hours;
mod pressure;
mod turn;

use cgroup::Limits;
use hours::Hours;
use pressure::Places;
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, Write as _};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitCode, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure};

/// One night job, as its arguments give it.
#[derive(Debug, PartialEq)]
struct Job {
    name: String,
    hours: Option<Hours>,
    start_under: f64,
    stops: Vec<(String, f64)>,
    stop_machine: f64,
    rests: Duration,
    wait_at_most: Option<Duration>,
    every: Duration,
    grace: Duration,
    limits: Limits,
    command: Vec<String>,
}

fn main() -> ExitCode {
    terminal_kit::main(&commands::help(), env!("CARGO_PKG_VERSION"), |context| {
        let args = &context.args;
        if args.first().map(String::as_str) == Some("turn") {
            let asked = turn::parse(args).map_err(|problem| commands::wrongly(args, problem))?;
            return turn::take(&asked, &runtime_dir());
        }
        run(&parse(args).map_err(|problem| commands::wrongly(args, problem))?)
    })
}

fn parse(args: &[String]) -> Result<Job, String> {
    let (Some("run"), Some(name)) = (args.first().map(String::as_str), args.get(1)) else {
        return Err("night-shift takes run or turn, and the job's name".into());
    };
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("night-shift run takes the job's command after --")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut job = Job {
        name: name.clone(),
        hours: None,
        start_under: 3.0,
        stops: Vec::new(),
        stop_machine: 25.0,
        rests: Duration::from_hours(12),
        wait_at_most: None,
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
            "--hours" => job.hours = Some(Hours::read(value)?),
            "--start-under" => job.start_under = number(value)?,
            "--stop-machine" => job.stop_machine = number(value)?,
            "--rests" => job.rests = Duration::from_secs_f64(number(value)? * 3600.0),
            "--wait-at-most" => {
                job.wait_at_most = Some(Duration::from_secs_f64(number(value)?));
            }
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
            _ => return Err(format!("{option} isn't an option of run")),
        }
    }
    Ok(job)
}

/// Whether the job's hours, when it has them, leave it out now, saying so.
/// A clock it can't read leaves it out too, rather than risk the day.
fn outside_its_hours(job: &Job) -> bool {
    let Some(hours) = job.hours else {
        return false;
    };
    match hours::now() {
        Some(now) if hours.hold(now) => false,
        now => {
            let at = now.map_or_else(|| "an unread clock".to_owned(), hours::clock);
            say(
                job,
                &format!("{at} is outside its hours, {hours}, so it runs nothing"),
            );
            true
        }
    }
}

/// What a line of the job's log says, with its name.
fn say(job: &Job, what: &str) {
    println!("night-shift {}: {what}", job.name);
}

/// How the job's run fails, saying what failed with its name.
fn failed(job: &Job, what: &str) -> Failure {
    Failure::new(Exit::Failed, format!("night-shift {}: {what}", job.name))
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

fn run(job: &Job) -> Result<(), Failure> {
    let state = dir("XDG_STATE_HOME", ".local/state").join("night-shift");
    let places = Folders {
        state,
        runtime: runtime_dir(),
    };
    // Before anything is made, locked or written, so a start by day leaves
    // last night's samples and the turn as they were.
    if outside_its_hours(job) {
        return Ok(());
    }
    let Some((_lock, at)) = may_start(job, &places)? else {
        return Ok(());
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
    let samples = File::create(places.state.join(format!("{}.samples", job.name)))
        .map_err(|error| failed(job, "can't write its samples").because(error.to_string()))?;
    let mut command = Command::new(&job.command[0]);
    command.args(&job.command[1..]).process_group(0);
    // Its own cgroup's pressure, the job's when it has one apart.
    let mut own = pressures.own_dir();
    // The job's cgroup's procs, held open until it has moved in.
    let mut procs_open = None;
    if job.limits.any() {
        let Some(unit) = own.clone() else {
            return Err(failed(
                job,
                "can't find its own cgroup to give the job its limits",
            ));
        };
        match cgroup::split(&unit, &job.limits) {
            Ok((cgroup, procs)) => {
                enter_on_start(&mut command, &procs);
                own = Some(cgroup);
                procs_open = Some(procs);
            }
            Err(error) => {
                return Err(failed(
                    job,
                    &format!(
                        "can't give the job a cgroup of its own under {}, as its limits need",
                        unit.display()
                    ),
                )
                .because(format!("{error}; its unit needs Delegate=memory io")));
            }
        }
    }
    let own = own.map(|dir| dir.join("memory.pressure"));
    let mut child = command.spawn().map_err(|error| {
        failed(job, &format!("can't start {}", job.command[0])).because(error.to_string())
    })?;
    drop(procs_open);
    say(
        job,
        &format!(
            "started at memory full avg60 {at:.2}%: {}",
            job.command.join(" ")
        ),
    );
    if let Some(status) = watch(
        job,
        &mut child,
        &pressures,
        own.as_deref(),
        &watched,
        samples,
    ) {
        finish(
            job,
            status,
            &places.state.join(format!("{}.done", job.name)),
        )
    } else {
        stop(job, &mut child);
        Ok(())
    }
}

/// The lock and the machine's pressure a job starts at; none when it
/// doesn't start and leaves a later try its turn: done already, another
/// job running, or the machine short of memory.
fn may_start(job: &Job, places: &Folders) -> Result<Option<(File, f64)>, Failure> {
    for place in [&places.state, &places.runtime] {
        fs::create_dir_all(place).map_err(|error| {
            failed(job, &format!("can't make {}", place.display())).because(error.to_string())
        })?;
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
        return Ok(None);
    }

    // Held until this process ends, however it ends.
    let lock = File::create(places.runtime.join("lock"))
        .map_err(|error| failed(job, "can't open the lock").because(error.to_string()))?;
    let began = Instant::now();
    let mut told: Option<String> = None;
    let mut left = None;
    loop {
        // A free turn is left a while to a job Joel waits on, waiting for
        // it too.
        match turn::look(&lock, &places.runtime, false, &mut left) {
            Ok(turn::Look::Had) => break,
            Ok(found) => {
                let (why, waiting) = if let turn::Look::Left(next) = found {
                    // Left only a while, so even a job that waits for
                    // nobody waits that long.
                    if job.wait_at_most.is_none() {
                        if outside_its_hours(job) {
                            return Ok(None);
                        }
                        thread::sleep(turn::LOOK_EVERY);
                        continue;
                    }
                    let why = format!("the turn is left to {next}, which Joel waits on");
                    (why.clone(), format!("waiting, as {why}"))
                } else {
                    let who = turn::holder(&places.runtime);
                    (
                        format!("another heavy job is running, {who}, and only one runs at a time"),
                        format!("waiting for the turn, which {who} has"),
                    )
                };
                let Some(most) = job.wait_at_most else {
                    say(job, &format!("not started: {why}; a later try goes on"));
                    return Ok(None);
                };
                // Hours that close while it waits end the wait, rather than
                // one that runs on to say a later try goes on when none will.
                // It looks before saying it waits, so the line means this
                // look is over and the next is a whole LOOK_EVERY away.
                if outside_its_hours(job) {
                    return Ok(None);
                }
                if told.as_ref() != Some(&waiting) {
                    say(job, &waiting);
                    told = Some(waiting);
                }
                if began.elapsed() >= most {
                    return Err(Failure::new(
                        Exit::TryAgain,
                        format!(
                            "night-shift {}: not started, having waited as long as it was let",
                            job.name
                        ),
                    )
                    .because("another heavy job still has the turn; a later try goes on"));
                }
                thread::sleep(turn::LOOK_EVERY);
            }
            Err(error) => {
                return Err(failed(job, "can't take the lock").because(error.to_string()));
            }
        }
    }
    // The hours can close between the wait's last look and the turn, so
    // they're read again once it's had; the lock goes as this returns.
    if told.is_some() && outside_its_hours(job) {
        return Ok(None);
    }
    turn::note(&places.runtime, &job.name);
    if told.is_some() {
        say(
            job,
            &format!(
                "has the turn after {} s of waiting",
                began.elapsed().as_secs()
            ),
        );
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
        return Ok(None);
    }
    Ok(Some((lock, now.avg60)))
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

/// How a job that ended ends the run: marked done, or failed with its own
/// code, or 1 where a signal ended it.
fn finish(job: &Job, status: ExitStatus, done: &Path) -> Result<(), Failure> {
    if status.success() {
        File::create(done).map_err(|error| {
            failed(job, "finished, but can't mark it done").because(error.to_string())
        })?;
        say(job, "finished");
        return Ok(());
    }
    let code = status
        .code()
        .and_then(|code| u8::try_from(code).ok())
        .unwrap_or(1);
    Err(Failure::passed(
        code,
        format!("night-shift {}: the job failed: {status}", job.name),
    ))
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
fn stop(job: &Job, child: &mut Child) {
    signal_group(child, SIGTERM);
    let asked = Instant::now();
    while asked.elapsed() < job.grace {
        if let Ok(Some(_)) = child.try_wait() {
            signal_group(child, SIGKILL);
            say(job, "stopped; a later try goes on");
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }
    say(job, "killing what is left of it");
    signal_group(child, SIGKILL);
    let _ = child.wait();
    say(job, "stopped; a later try goes on");
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

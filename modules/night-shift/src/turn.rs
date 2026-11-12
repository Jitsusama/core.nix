//! `night-shift turn`: a heavy job run by day takes the turn a night job
//! takes, so it waits for one already running rather than starting beside
//! it.
//!
//! One heavy job at a time on the machine was each session's promise, and
//! a promise is kept only by those who heard it: on 11 November at 21:21
//! a test suite and a machine's evaluation ran at once, load reached 76,
//! and the processor's wait in Joel's apps reached 2.9%. So a heavy job
//! starts as `night-shift turn NAME -- COMMAND`. It takes night-shift's
//! lock, waiting while another job holds it and saying once who that is,
//! then becomes its command, which holds the lock until it ends however it
//! ends. A night job tried meanwhile finds it taken and leaves the turn to
//! a later try, as it would beside another night job.
//!
//! The command's output is its own: what night-shift says goes to stderr,
//! and its status is the command's. `--wait-at-most SECONDS` gives up
//! waiting with status 75, so a caller can try again later.

use std::fs::{self, File, TryLockError};
use std::os::fd::AsRawFd;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, ExitCode};
use std::thread;
use std::time::{Duration, Instant, SystemTime};

/// A run that waited as long as it was let, as sysexits.h names a failure
/// worth trying again.
pub const WAITED_TOO_LONG: u8 = 75;

/// How often a waiting job looks whether the turn is free: a heavy job
/// runs minutes, so a second's lateness taking it costs nothing.
const LOOK_EVERY: Duration = Duration::from_secs(1);

/// One heavy job asking for the turn, as its arguments give it.
#[derive(Debug, PartialEq)]
pub struct Turn {
    pub name: String,
    pub wait_at_most: Option<Duration>,
    pub command: Vec<String>,
}

/// Reads `turn NAME [--wait-at-most SECONDS] -- COMMAND [ARG]...`.
pub fn parse(args: &[String]) -> Result<Turn, String> {
    let Some(name) = args.get(1) else {
        return Err("`turn` takes the job's name".into());
    };
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("it takes `--` and the job's command")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut wait_at_most = None;
    let mut options = args[2..split].iter();
    while let Some(option) = options.next() {
        match option.as_str() {
            "--wait-at-most" => {
                let value = options.next().ok_or("--wait-at-most takes a value")?;
                let seconds = value
                    .parse::<f64>()
                    .ok()
                    .filter(|n| n.is_finite() && *n >= 0.0)
                    .ok_or_else(|| format!("--wait-at-most takes seconds, not {value}"))?;
                wait_at_most = Some(Duration::from_secs_f64(seconds));
            }
            _ => return Err(format!("{option} isn't an option of `turn`")),
        }
    }
    Ok(Turn {
        name: name.clone(),
        wait_at_most,
        command,
    })
}

/// Notes in `runtime` that `name`, this process, has the turn from now.
pub fn note(runtime: &Path, name: &str) {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let _ = fs::write(
        runtime.join("holder"),
        format!("{name}\t{}\t{since}\n", std::process::id()),
    );
}

/// Who has the turn, as the last to take it noted, while its process
/// still runs.
pub fn holder(runtime: &Path) -> String {
    let noted = fs::read_to_string(runtime.join("holder")).unwrap_or_default();
    let mut fields = noted.trim().split('\t');
    let (Some(name), Some(pid), Some(since)) = (fields.next(), fields.next(), fields.next()) else {
        return "a job that doesn't say".into();
    };
    if !Path::new("/proc").join(pid).exists() {
        return "a job that doesn't say".into();
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |now| now.as_secs());
    let minutes = now.saturating_sub(since.parse().unwrap_or(now)) / 60;
    format!("{name} (process {pid}, for {minutes} min)")
}

// std links libc, so this comes from it.
unsafe extern "C" {
    fn dup(fd: i32) -> i32;
}

/// Waits for the turn in `runtime`, then becomes the job's command, which
/// keeps it. Returns only when the job doesn't start.
pub fn take(turn: &Turn, runtime: &Path) -> ExitCode {
    let say = |what: &str| eprintln!("night-shift {}: {what}", turn.name);
    if let Err(error) = fs::create_dir_all(runtime) {
        say(&format!("can't make {}: {error}", runtime.display()));
        return ExitCode::FAILURE;
    }
    let lock = match File::create(runtime.join("lock")) {
        Ok(lock) => lock,
        Err(error) => {
            say(&format!("can't open the lock: {error}"));
            return ExitCode::FAILURE;
        }
    };
    let began = Instant::now();
    let mut told: Option<String> = None;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                let who = holder(runtime);
                if told.as_ref() != Some(&who) {
                    say(&format!("waiting for the turn, which {who} has"));
                    told = Some(who);
                }
                if turn
                    .wait_at_most
                    .is_some_and(|most| began.elapsed() >= most)
                {
                    say("not started: it waited as long as it was let; try again later");
                    return ExitCode::from(WAITED_TOO_LONG);
                }
                thread::sleep(LOOK_EVERY);
            }
            Err(TryLockError::Error(error)) => {
                say(&format!("can't take the lock: {error}"));
                return ExitCode::FAILURE;
            }
        }
    }
    note(runtime, &turn.name);
    if told.is_some() {
        say(&format!(
            "has the turn after {} s of waiting",
            began.elapsed().as_secs()
        ));
    }
    // A copy of the lock's descriptor without close-on-exec, so the
    // command holds the lock, which belongs to the open file both share,
    // until it and whatever it leaves running have ended.
    // SAFETY: dup takes any descriptor and only reports errors.
    if unsafe { dup(lock.as_raw_fd()) } < 0 {
        say(&format!(
            "can't hand the lock on: {}",
            std::io::Error::last_os_error()
        ));
        return ExitCode::FAILURE;
    }
    let error = Command::new(&turn.command[0])
        .args(&turn.command[1..])
        .exec();
    say(&format!("can't start {}: {error}", turn.command[0]));
    ExitCode::FAILURE
}

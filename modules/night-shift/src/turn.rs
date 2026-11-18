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
//! then runs its command and keeps the lock until the command ends. A
//! night job tried meanwhile finds it taken and leaves the turn to a later
//! try, as it would beside another night job.
//!
//! The lock never leaves night-shift. It used to pass to the command, and
//! so to everything the command started: on 17 November a cargo test
//! ended at 18:20 and two of its shells, one ignoring TERM, kept the turn
//! until they were killed by hand at 19:26, holding a switch Joel waited
//! on for 51 minutes. Now night-shift stays as the command's parent and
//! the parent of whatever it leaves, as a child subreaper, and once the
//! command ends it sends TERM to all of that, kills what is left after
//! `LEFTOVERS_GRACE`, and lets go. The same happens when night-shift is
//! sent HUP, INT or TERM, and when its caller dies, which the kernel tells
//! it with TERM, so a killed session takes its turn's work with it.
//!
//! `--ahead` marks a job Joel waits on, such as a switch's build: while
//! one waits, others leave a free turn to it, so it goes next whoever
//! asked first.
//!
//! The command's output is its own: what night-shift says goes to stderr,
//! and its status is the command's, or 128 and the signal's number when a
//! signal ended it. `--wait-at-most SECONDS` gives up waiting with status
//! 75, so a caller can try again later.

use std::ffi::c_void;
use std::fs::{self, File, TryLockError};
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use terminal_kit::{Exit, Failure};

/// How often a waiting job looks whether the turn is free: a heavy job
/// runs minutes, so a second's lateness taking it costs nothing.
pub const LOOK_EVERY: Duration = Duration::from_secs(1);

/// How long what a command leaves running gets to end on TERM before it
/// is killed: git and the like give back their locks in well under it.
pub const LEFTOVERS_GRACE: Duration = Duration::from_secs(5);

/// One heavy job asking for the turn, as its arguments give it.
#[derive(Debug, PartialEq)]
pub struct Turn {
    pub name: String,
    pub wait_at_most: Option<Duration>,
    pub ahead: bool,
    pub command: Vec<String>,
}

/// Reads `turn NAME [--wait-at-most SECONDS] [--ahead] -- COMMAND [ARG]...`.
pub fn parse(args: &[String]) -> Result<Turn, String> {
    let Some(name) = args.get(1) else {
        return Err("night-shift turn takes the job's name".into());
    };
    let split = args
        .iter()
        .position(|arg| arg == "--")
        .ok_or("night-shift turn takes the job's command after --")?;
    let command = args[split + 1..].to_vec();
    if command.is_empty() {
        return Err("the job's command is empty".into());
    }
    let mut wait_at_most = None;
    let mut ahead = false;
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
            "--ahead" => ahead = true,
            _ => return Err(format!("{option} isn't an option of turn")),
        }
    }
    Ok(Turn {
        name: name.clone(),
        wait_at_most,
        ahead,
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

/// Where each job waiting `--ahead` leaves a file named by its process.
fn aheads(runtime: &Path) -> PathBuf {
    runtime.join("ahead")
}

/// The fields of `/proc/PID/stat` after the program's name, which may
/// hold spaces and brackets of its own: the state first, then the parent.
fn stat(pid: &str) -> Option<Vec<String>> {
    let text = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let (_, after) = text.rsplit_once(") ")?;
    Some(after.split(' ').map(str::to_owned).collect())
}

/// When a process started, in the kernel's ticks since boot, which tells
/// it apart from a later one given the same number.
fn started(pid: &str) -> Option<String> {
    stat(pid)?.get(19).cloned()
}

/// The name of a job Joel waits on that waits for the turn, other than
/// this process, so it goes next. A file whose process has gone is
/// removed.
pub fn next(runtime: &Path) -> Option<String> {
    let own = std::process::id().to_string();
    let files = fs::read_dir(aheads(runtime)).ok()?;
    for file in files.flatten() {
        let pid = file.file_name().to_string_lossy().into_owned();
        if pid == own {
            continue;
        }
        let noted = fs::read_to_string(file.path()).unwrap_or_default();
        let Some((name, since)) = noted.trim().split_once('\t') else {
            continue;
        };
        if started(&pid).as_deref() == Some(since) {
            return Some(name.to_owned());
        }
        let _ = fs::remove_file(file.path());
    }
    None
}

/// Who a waiting job waits for: the holder, and a job Joel waits on
/// that goes before it.
pub fn whose(runtime: &Path) -> String {
    let has = holder(runtime);
    match next(runtime) {
        Some(next) => format!("{has}, then {next}, which Joel waits on"),
        None => has,
    }
}

/// A job waiting `--ahead`'s file, there from before its first look
/// until it has the turn or gives up.
struct Ahead(PathBuf);

impl Ahead {
    fn wait(runtime: &Path, name: &str) -> std::io::Result<Self> {
        let pid = std::process::id().to_string();
        let since =
            started(&pid).ok_or_else(|| std::io::Error::other("can't read its own start"))?;
        fs::create_dir_all(aheads(runtime))?;
        let file = aheads(runtime).join(&pid);
        fs::write(&file, format!("{name}\t{since}\n"))?;
        Ok(Ahead(file))
    }
}

impl Drop for Ahead {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

// std links libc, so these come from it.
unsafe extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn getppid() -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn sigemptyset(set: *mut SignalSet) -> i32;
    fn sigaddset(set: *mut SignalSet, signal: i32) -> i32;
    fn sigprocmask(how: i32, set: *const SignalSet, old: *mut SignalSet) -> i32;
    fn sigtimedwait(set: *const SignalSet, info: *mut c_void, timeout: *const Timespec) -> i32;
}

/// glibc's `sigset_t`, 1024 bits, and room for its `siginfo_t`, 128 bytes.
#[repr(C)]
struct SignalSet([u64; 16]);

#[repr(C)]
struct Timespec {
    seconds: i64,
    nanoseconds: i64,
}

const PR_SET_PDEATHSIG: i32 = 1;
const PR_SET_CHILD_SUBREAPER: i32 = 36;
const SIG_BLOCK: i32 = 0;
const SIG_SETMASK: i32 = 2;
const SIGHUP: i32 = 1;
const SIGINT: i32 = 2;
const SIGKILL: i32 = 9;
const SIGTERM: i32 = 15;
const SIGCHLD: i32 = 17;
const WNOHANG: i32 = 1;

/// Asks the kernel for TERM when this process's parent dies.
fn term_when_parent_dies() {
    // SAFETY: prctl with these arguments only sets the signal.
    unsafe { prctl(PR_SET_PDEATHSIG, u64::from(SIGTERM.unsigned_abs())) };
}

/// The signals that end a turn, and a child's end, held for
/// `sigtimedwait` rather than delivered.
fn hold_signals() -> SignalSet {
    let mut set = SignalSet([0; 16]);
    // SAFETY: each call writes only the set it is given, which lives here,
    // and sigprocmask only reports errors.
    unsafe {
        sigemptyset(&raw mut set);
        for signal in [SIGHUP, SIGINT, SIGTERM, SIGCHLD] {
            sigaddset(&raw mut set, signal);
        }
        sigprocmask(SIG_BLOCK, &raw const set, std::ptr::null_mut());
    }
    set
}

/// Lets every signal through again, as a command expects to start: a
/// held signal stays held across exec, and std doesn't let it go.
fn hold_none() {
    let mut none = SignalSet([0; 16]);
    // SAFETY: both calls are async-signal-safe and write only the set
    // they are given, which lives here.
    unsafe {
        sigemptyset(&raw mut none);
        sigprocmask(SIG_SETMASK, &raw const none, std::ptr::null_mut());
    }
}

/// Waits up to `most` for one of the held signals, giving its number.
fn signal_within(set: &SignalSet, most: Duration) -> Option<i32> {
    let timeout = Timespec {
        seconds: i64::try_from(most.as_secs()).unwrap_or(i64::MAX),
        nanoseconds: i64::from(most.subsec_nanos()),
    };
    let mut info = [0u64; 16];
    // SAFETY: the set, the info's 128 bytes and the timeout live here.
    let signal = unsafe { sigtimedwait(set, info.as_mut_ptr().cast(), &raw const timeout) };
    (signal > 0).then_some(signal)
}

/// Reaps every child that has ended, giving `top`'s status if it was one.
fn reap(top: i32) -> Option<ExitStatus> {
    let mut status = None;
    loop {
        let mut raw = 0;
        // SAFETY: waitpid writes only the status it is given.
        let pid = unsafe { waitpid(-1, &raw mut raw, WNOHANG) };
        if pid <= 0 {
            return status;
        }
        if pid == top {
            status = Some(ExitStatus::from_raw(raw));
        }
    }
}

/// Every process still running below this one, ended ones aside, read
/// from each process's parent in /proc.
fn descendants() -> Vec<i32> {
    let mut parents = Vec::new();
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let pid = entry.file_name().to_string_lossy().into_owned();
            let Ok(number) = pid.parse::<i32>() else {
                continue;
            };
            let Some(fields) = stat(&pid) else { continue };
            if fields.first().is_some_and(|state| state == "Z") {
                continue;
            }
            if let Some(parent) = fields.get(1).and_then(|parent| parent.parse::<i32>().ok()) {
                parents.push((number, parent));
            }
        }
    }
    let own = i32::try_from(std::process::id()).unwrap_or(i32::MAX);
    let mut below = vec![own];
    let mut at = 0;
    while at < below.len() {
        let of = below[at];
        below.extend(
            parents
                .iter()
                .filter(|(_, parent)| *parent == of)
                .map(|(pid, _)| *pid),
        );
        at += 1;
    }
    below.remove(0);
    below
}

fn signal_all(pids: &[i32], signal: i32) {
    for pid in pids {
        // SAFETY: kill takes any pid and signal, and only reports errors.
        unsafe { kill(*pid, signal) };
    }
}

/// Ends everything below this process: TERM, then KILL once the grace
/// passes, reaping as it goes. Gives how many there were, and `top`'s
/// status if it ended meanwhile.
fn end_what_is_left(top: i32) -> (usize, Option<ExitStatus>) {
    let mut status = reap(top);
    let left = descendants();
    if left.is_empty() {
        return (0, status);
    }
    signal_all(&left, SIGTERM);
    let began = Instant::now();
    loop {
        status = status.or(reap(top));
        let now = descendants();
        if now.is_empty() {
            break;
        }
        if began.elapsed() >= LEFTOVERS_GRACE {
            signal_all(&now, SIGKILL);
        }
        if began.elapsed() >= LEFTOVERS_GRACE * 2 {
            break;
        }
        // Polled: a process ending says so only to its parent, and these
        // may be any process's below.
        thread::sleep(Duration::from_millis(50));
    }
    (left.len(), status.or(reap(top)))
}

/// Waits for the turn in `runtime`, runs the job's command, and ends the
/// process with its status once it and whatever it left have ended.
/// Returns only when the job doesn't start, with why: waiting as long as
/// it was let is worth trying again, 75.
pub fn take(turn: &Turn, runtime: &Path) -> Result<(), Failure> {
    let say = |what: &str| eprintln!("night-shift {}: {what}", turn.name);
    let failed = |what: &str, error: &dyn std::fmt::Display| {
        Failure::new(Exit::Failed, format!("night-shift {}: {what}", turn.name))
            .because(error.to_string())
    };
    let caller = std::os::unix::process::parent_id();
    term_when_parent_dies();
    fs::create_dir_all(runtime)
        .map_err(|error| failed(&format!("can't make {}", runtime.display()), &error))?;
    // Opened close-on-exec, as std opens every file, so it stays here.
    let lock = File::create(runtime.join("lock"))
        .map_err(|error| failed("can't open the lock", &error))?;
    let ahead = if turn.ahead {
        Some(Ahead::wait(runtime, &turn.name).map_err(|error| failed("can't go ahead", &error))?)
    } else {
        None
    };
    let began = Instant::now();
    let mut told: Option<String> = None;
    loop {
        let tried = if ahead.is_none() && next(runtime).is_some() {
            Err(TryLockError::WouldBlock)
        } else {
            lock.try_lock()
        };
        match tried {
            Ok(()) => break,
            Err(TryLockError::WouldBlock) => {
                let who = whose(runtime);
                if told.as_ref() != Some(&who) {
                    say(&format!("waiting for the turn, which {who} has"));
                    told = Some(who);
                }
                if turn
                    .wait_at_most
                    .is_some_and(|most| began.elapsed() >= most)
                {
                    return Err(Failure::new(
                        Exit::TryAgain,
                        format!(
                            "night-shift {}: not started, having waited as long as it was let",
                            turn.name
                        ),
                    )
                    .because("another heavy job still has the turn; try again later"));
                }
                thread::sleep(LOOK_EVERY);
            }
            Err(TryLockError::Error(error)) => {
                return Err(failed("can't take the lock", &error));
            }
        }
    }
    drop(ahead);
    note(runtime, &turn.name);
    if told.is_some() {
        say(&format!(
            "has the turn after {} s of waiting",
            began.elapsed().as_secs()
        ));
    }
    let held = hold_signals();
    // SAFETY: getppid only reads.
    if u32::try_from(unsafe { getppid() }).ok() != Some(caller) {
        return Err(Failure::new(
            Exit::Failed,
            format!(
                "night-shift {}: not started, as its caller has gone",
                turn.name
            ),
        ));
    }
    // SAFETY: prctl with these arguments only marks this process.
    unsafe { prctl(PR_SET_CHILD_SUBREAPER, 1u64) };
    let mut command = Command::new(&turn.command[0]);
    command.args(&turn.command[1..]);
    // SAFETY: prctl and sigprocmask are async-signal-safe, as a closure
    // run between fork and exec must be.
    unsafe {
        command.pre_exec(|| {
            hold_none();
            term_when_parent_dies();
            Ok(())
        });
    }
    let child = command
        .spawn()
        .map_err(|error| failed(&format!("can't start {}", turn.command[0]), &error))?;
    let top = i32::try_from(child.id()).unwrap_or(i32::MAX);
    let mut ended_by = None;
    let mut status = None;
    while status.is_none() {
        match signal_within(&held, LOOK_EVERY) {
            Some(SIGCHLD) | None => status = reap(top),
            Some(signal) => {
                ended_by = Some(signal);
                break;
            }
        }
    }
    let (left, late) = end_what_is_left(top);
    let status = status.or(late);
    if left > 0 {
        say(&format!(
            "ended {left} process{} its command left running",
            if left == 1 { "" } else { "es" }
        ));
    }
    let code = match (ended_by, status) {
        (Some(signal), _) => 128 + signal,
        (None, Some(status)) => status
            .code()
            .or_else(|| status.signal().map(|signal| 128 + signal))
            .unwrap_or(1),
        (None, None) => 1,
    };
    std::process::exit(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_turn_reads_ahead_and_its_wait() {
        let args: Vec<String> = "turn switch --ahead --wait-at-most 5 -- nix build"
            .split(' ')
            .map(str::to_owned)
            .collect();
        assert_eq!(
            parse(&args).unwrap(),
            Turn {
                name: "switch".into(),
                wait_at_most: Some(Duration::from_secs(5)),
                ahead: true,
                command: vec!["nix".into(), "build".into()],
            }
        );
    }

    #[test]
    fn a_process_reads_its_own_start() {
        let own = std::process::id().to_string();
        assert!(started(&own).is_some_and(|ticks| ticks.parse::<u64>().is_ok()));
    }
}

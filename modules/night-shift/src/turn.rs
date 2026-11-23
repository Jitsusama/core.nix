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
//! on for 51 minutes. Now night-shift stays as the command's parent and,
//! as a child subreaper, the parent of whatever it leaves. Once the
//! command ends it sends TERM to what is left, kills what is still there
//! after `LEFTOVERS_GRACE`, and lets go.
//!
//! HUP and TERM sent to night-shift go on to the command, which gets the
//! grace to end before what is left is ended the same way, and
//! night-shift then ends by the signal, as the command it once became
//! would have. INT isn't passed on: a terminal's Ctrl-C reaches the
//! command itself, in night-shift's process group, and the command
//! decides. A turn's caller dying ends it as TERM does, the kernel saying
//! so, so a killed session takes its turn's work with it; work meant to
//! outlive its caller starts the turn under systemd, as
//! `systemd-run --user night-shift turn ...`. Killed outright,
//! night-shift can end nothing, but the lock goes with it and the command
//! is sent TERM.
//!
//! `--ahead` marks a job Joel waits on, such as a switch's build: while
//! one waits, others leave a free turn to it for up to
//! `AHEAD_IS_LEFT_AT_MOST`, so it goes next whoever asked first, and one
//! that is stopped or stuck holds nobody back for long. src/ahead.rs
//! keeps the marks that say who goes next.
//!
//! A night job holding the turn pauses for one too, since a night job can
//! run two hours: on 18 November a switch's evaluation waited 17 minutes
//! behind World's nightly roll and gave up. The night job is frozen where
//! it is, leaves the turn, and marks itself paused beside the jobs Joel
//! waits on (src/main.rs says how), so it takes the turn back once they
//! are done, ahead of any job that came to wait meanwhile.
//!
//! The command's output is its own: what night-shift says goes to stderr,
//! and its status is the command's. `--wait-at-most SECONDS` gives up
//! waiting with status 75, so a caller can try again later.
//!
//! A turn asked for by a turn's own command, or anything below it, runs
//! at once as part of that job, since the turn is already its own: as
//! nix-offload's cargo asks for one inside an agent's. Below means the
//! command's own processes: work it hands to a unit, or to a terminal
//! server started before it, asks as any other job does, and waits.

use crate::ahead::{self, Ahead, Waiter};
use std::collections::HashMap;
use std::ffi::c_void;
use std::fs::{self, File, TryLockError};
use std::io;
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::Path;
use std::process::{Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant, SystemTime};
use terminal_kit::{Exit, Failure};

/// How often a waiting job looks whether the turn is free: a heavy job
/// runs minutes, so a second's lateness taking it costs nothing.
pub const LOOK_EVERY: Duration = Duration::from_secs(1);

/// How often a job Joel waits on looks, so it takes a turn the moment
/// it's free.
pub const AHEAD_LOOKS_EVERY: Duration = Duration::from_millis(100);

/// How long a free turn is left to a job Joel waits on, in which it
/// looks 30 times: one that hasn't taken it by then is stopped or stuck.
pub const AHEAD_IS_LEFT_AT_MOST: Duration = Duration::from_secs(3);

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

/// Notes in `runtime` that `name`, this process, has the turn from now,
/// for those waiting to say who they wait for. It's written whole beside
/// the note and then named, so a reader never finds half of it, and a
/// tab or line break in the name, which would shift the note's fields, is
/// written as a space. Who holds the turn is the kernel's word alone
/// ([`held_above`]), so a note that can't be written only leaves those
/// waiting naming nobody, and is said.
pub fn note(runtime: &Path, name: &str) {
    let since = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let pid = std::process::id();
    let shown = name.replace(['\t', '\n', '\r'], " ");
    let writing = runtime.join(format!(".holder-{pid}"));
    let written = fs::write(&writing, format!("{shown}\t{pid}\t{since}\n"))
        .and_then(|()| fs::rename(&writing, runtime.join("holder")));
    if let Err(error) = written {
        let _ = fs::remove_file(&writing);
        say(
            name,
            &format!(
                "can't note that it has the turn ({error}), so those waiting for it can't say \
                 who has it"
            ),
        );
    }
}

/// Who has the turn, as the last to take it noted, while its process
/// still runs.
pub fn holder(runtime: &Path) -> String {
    noted(runtime).map_or_else(
        || "a job that doesn't say".into(),
        |(name, pid, minutes)| format!("{name} (process {pid}, for {minutes} min)"),
    )
}

/// The note's name, process and minutes held, while its process runs.
/// Only its line break is trimmed, so an empty name shifts no field.
fn noted(runtime: &Path) -> Option<(String, i32, u64)> {
    let noted = fs::read_to_string(runtime.join("holder")).ok()?;
    let mut fields = noted.trim_end_matches('\n').split('\t');
    let (Some(name), Some(pid), Some(since)) = (fields.next(), fields.next(), fields.next()) else {
        return None;
    };
    let pid = pid.parse::<i32>().ok().filter(|pid| *pid > 0)?;
    if !Path::new("/proc").join(pid.to_string()).exists() {
        return None;
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |now| now.as_secs());
    let minutes = now.saturating_sub(since.parse().unwrap_or(now)) / 60;
    Some((name.to_owned(), pid, minutes))
}

/// Who holds `lock`, as far as the kernel says.
enum Held {
    /// This process, or one of its parents, as the process named.
    Above(String),
    /// Another process, or nobody.
    Elsewhere,
    /// The kernel's word couldn't be read, or named nobody, as given.
    Unread(String),
}

/// Whether the turn in `runtime` is held by this process or one above it,
/// by the kernel's own word on who holds `lock`: a turn asked for inside a
/// turn's command has the turn already, and waiting for the lock its own
/// parent holds would wait for ever, as a cargo test an agent ran in its
/// turn did once nix-offload asked for a turn of its own. The note plays
/// no part but to name the holder, so one left standing, missing or
/// unwritten neither runs a turn beside another job nor makes one wait on
/// its own parent. A holder that let the lock go while it lives, as a
/// night job paused for a job Joel waits on does, holds nothing the
/// kernel names. "Above" is this process's parents alone: a turn asked
/// for by work a turn's command handed elsewhere, to a unit or a terminal
/// server started before it, waits like any other.
fn held_above(runtime: &Path, lock: &File) -> Held {
    let holders = match lock_holders(lock) {
        Ok(holders) if holders.is_empty() => {
            // Nobody, or a device the kernel names otherwise, which a
            // caller that found the lock taken tells apart.
            return Held::Unread("the kernel names no holder of its lock".into());
        }
        Ok(holders) => holders,
        Err(why) => return Held::Unread(why),
    };
    let Ok(mut at) = i32::try_from(std::process::id()) else {
        return Held::Elsewhere;
    };
    // pid 1 and the kernel's 0 hold no turn, and a chain longer than any
    // real one is a loop read mid-change.
    for _ in 0..4096 {
        if at <= 1 {
            break;
        }
        if holders.contains(&at) {
            let named = noted(runtime)
                .filter(|(_, pid, _)| *pid == at)
                .map_or_else(|| format!("process {at}"), |_| holder(runtime));
            return Held::Above(named);
        }
        let Some(parent) = ahead::stat(&at.to_string()).and_then(|stat| stat.get(1)?.parse().ok())
        else {
            break;
        };
        at = parent;
    }
    Held::Elsewhere
}

/// The processes holding a flock on `lock`, as the kernel's `/proc/locks`
/// says: lines `ID: FLOCK ADVISORY WRITE PID MAJ:MIN:INODE` for the lock's
/// file, a waiter's lines being marked `->`. The kernel names the file by
/// its filesystem's device, which stat's needn't be: on a btrfs subvolume
/// stat gives the subvolume's own (on 23 November, 0:533 for a file the
/// kernel named 00:22), so the device is the mount's, as mountinfo gives
/// it, or stat's where that can't be read.
fn lock_holders(lock: &File) -> Result<Vec<i32>, String> {
    use std::os::unix::fs::MetadataExt;
    let held = lock
        .metadata()
        .map_err(|error| format!("the lock can't be read: {error}"))?;
    let (major, minor) = mounted_device(lock).unwrap_or_else(|| {
        // Linux's own encoding of a device in 32 bits, 12 for the major
        // number and 20 for the minor, split around it.
        let dev = held.dev();
        ((dev >> 8) & 0xfff, (dev & 0xff) | ((dev >> 12) & 0xf_ff00))
    });
    let file = format!("{major:02x}:{minor:02x}:{}", held.ino());
    let locks = fs::read_to_string("/proc/locks")
        .map_err(|error| format!("/proc/locks can't be read: {error}"))?;
    Ok(locks
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            let theirs = words.get(1) == Some(&"FLOCK") && words.get(5) == Some(&file.as_str());
            words.get(4).filter(|_| theirs)?.parse().ok()
        })
        .collect())
}

/// The device of the filesystem `lock` is on, as `/proc/self/mountinfo`
/// gives its mount's: the one `/proc/locks` names it by.
fn mounted_device(lock: &File) -> Option<(u64, u64)> {
    use std::os::fd::AsRawFd;
    let info = fs::read_to_string(format!("/proc/self/fdinfo/{}", lock.as_raw_fd())).ok()?;
    let mount = info
        .lines()
        .find_map(|line| line.strip_prefix("mnt_id:"))?
        .trim();
    let mounts = fs::read_to_string("/proc/self/mountinfo").ok()?;
    let device = mounts.lines().find_map(|line| {
        let mut words = line.split(' ');
        (words.next() == Some(mount)).then(|| words.nth(1))?
    })?;
    let (major, minor) = device.split_once(':')?;
    Some((major.parse().ok()?, minor.parse().ok()?))
}

/// What one look at the turn found.
pub enum Look {
    /// This process has the turn.
    Had,
    /// Another job has it.
    Taken,
    /// It's free, and left to this job, which goes first.
    Left(Waiter),
}

/// Looks once at the turn, taking it when it's free, unless a job Joel
/// waits on waits for it too. `left` is when this job first left it so,
/// which bounds how long it does.
pub fn look(
    lock: &File,
    runtime: &Path,
    ahead: bool,
    left: &mut Option<Instant>,
) -> io::Result<Look> {
    match lock.try_lock() {
        Ok(()) => {}
        Err(TryLockError::WouldBlock) => {
            *left = None;
            return Ok(Look::Taken);
        }
        Err(TryLockError::Error(error)) => return Err(error),
    }
    if ahead {
        return Ok(Look::Had);
    }
    let Some(next) = ahead::next(runtime) else {
        return Ok(Look::Had);
    };
    if left.get_or_insert_with(Instant::now).elapsed() >= AHEAD_IS_LEFT_AT_MOST {
        return Ok(Look::Had);
    }
    lock.unlock()?;
    Ok(Look::Left(next))
}

// std links libc, so these come from it.
unsafe extern "C" {
    fn prctl(option: i32, ...) -> i32;
    fn kill(pid: i32, signal: i32) -> i32;
    fn raise(signal: i32) -> i32;
    fn signal(signal: i32, handler: usize) -> usize;
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
const SIGPIPE: i32 = 13;
const SIGTERM: i32 = 15;
const SIGCHLD: i32 = 17;
const SIG_DFL: usize = 0;
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
/// held signal stays held across exec, and std's spawn keeps the
/// parent's on purpose ("Inherit the signal mask from the parent rather
/// than resetting it", in 1.97's sys/process/unix/unix.rs), so the
/// command saw HUP, INT and TERM held until this did.
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

/// Reaps every child that has ended, keeping `top`'s status if it was one.
fn reap(top: i32, status: &mut Option<ExitStatus>) {
    loop {
        let mut raw = 0;
        // SAFETY: waitpid writes only the status it is given.
        let pid = unsafe { waitpid(-1, &raw mut raw, WNOHANG) };
        if pid <= 0 {
            return;
        }
        if pid == top {
            *status = Some(ExitStatus::from_raw(raw));
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
            let Some(fields) = ahead::stat(&pid) else {
                continue;
            };
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

fn send(pid: i32, signal: i32) {
    // SAFETY: kill takes any pid and signal, and only reports errors.
    unsafe { kill(pid, signal) };
}

/// What ending everything below this process found.
struct Ended {
    /// Processes other than the command, which it left running.
    left: usize,
    /// Those that hadn't ended even once killed.
    stuck: Vec<i32>,
}

/// Ends everything below this process, reaping as it goes. Those there
/// at the first look are sent TERM; each is killed once it has been seen
/// for the grace, so a process started meanwhile, such as a TERM trap's
/// cleanup, gets the grace too. Gives up after three graces, since a
/// process stuck in the kernel can't be ended and the turn mustn't wait
/// on it.
fn end_what_is_left(top: i32, status: &mut Option<ExitStatus>) -> Ended {
    let began = Instant::now();
    let mut seen: HashMap<i32, Instant> = HashMap::new();
    let mut stuck = Vec::new();
    let mut first = true;
    loop {
        reap(top, status);
        let now = descendants();
        if now.is_empty() {
            break;
        }
        for pid in &now {
            seen.entry(*pid).or_insert_with(|| {
                if first {
                    send(*pid, SIGTERM);
                }
                Instant::now()
            });
            if seen[pid].elapsed() >= LEFTOVERS_GRACE {
                send(*pid, SIGKILL);
            }
        }
        first = false;
        if began.elapsed() >= LEFTOVERS_GRACE * 3 {
            stuck = now;
            break;
        }
        // Polled: a process ending says so only to its parent, and these
        // may be any process's below.
        thread::sleep(Duration::from_millis(50));
    }
    reap(top, status);
    Ended {
        left: seen.keys().filter(|pid| **pid != top).count(),
        stuck,
    }
}

/// Ends this process by `signal`, as the command it stands for was, for
/// the signals that end a process without a core; for others, or should
/// it live on, it fails with 128 and the signal's number, as a shell
/// would say.
fn end_by(name: &str, ended: i32) -> Failure {
    if [SIGHUP, SIGINT, SIGKILL, SIGPIPE, SIGTERM].contains(&ended) {
        // SAFETY: signal sets the default action, std having ignored
        // SIGPIPE, and raise then sends it; both only report errors.
        unsafe {
            signal(ended, SIG_DFL);
            hold_none();
            raise(ended);
        }
    }
    Failure::passed(
        u8::try_from(128 + ended).unwrap_or(u8::MAX),
        format!("night-shift {name}: the command was ended by signal {ended}"),
    )
}

/// Says `what` for the turn named `name`, on stderr, where night-shift
/// says everything.
fn say(name: &str, what: &str) {
    eprintln!("night-shift {name}: {what}");
}

/// A failure of the turn named `name` to do `what`, for `error`.
fn failed(name: &str, what: &str, error: &dyn std::fmt::Display) -> Failure {
    Failure::new(Exit::Failed, format!("night-shift {name}: {what}")).because(error.to_string())
}

/// Waits for the turn in `runtime`, runs the job's command, and ends as
/// the command ended, once it and whatever it left have. Waiting as long
/// as it was let is worth trying again, 75.
pub fn take(turn: &Turn, runtime: &Path) -> Result<(), Failure> {
    let caller = std::os::unix::process::parent_id();
    term_when_parent_dies();
    // A caller gone before the kernel was asked is no longer this one's
    // parent, and its death sends nothing.
    if std::os::unix::process::parent_id() != caller {
        return Err(Failure::new(
            Exit::Failed,
            format!(
                "night-shift {}: not started, as its caller has gone",
                turn.name
            ),
        ));
    }
    fs::create_dir_all(runtime).map_err(|error| {
        failed(
            &turn.name,
            &format!("can't make {}", runtime.display()),
            &error,
        )
    })?;
    // Opened close-on-exec, as std opens every file, so it stays here,
    // and held until this process ends, however it ends.
    let lock = File::create(runtime.join("lock"))
        .map_err(|error| failed(&turn.name, "can't open the lock", &error))?;
    if let Held::Above(holder) = held_above(runtime, &lock) {
        return run_within(turn, &holder);
    }
    if let Some(holder) = wait_for(turn, runtime, &lock)? {
        return run_within(turn, &holder);
    }
    note(runtime, &turn.name);
    run(turn)
}

/// Runs the job's command in the turn `holder`, above this process, has.
fn run_within(turn: &Turn, holder: &str) -> Result<(), Failure> {
    say(
        &turn.name,
        &format!("runs in the turn {holder} already has, being part of its job"),
    );
    run(turn)
}

/// Waits until this job has the turn, saying once why each time that
/// changes, or names the holder above this process once it's seen there:
/// a parent that ended as the first walk read it leaves this process to
/// the holder, which reaps what its command leaves, so a later look finds
/// it.
fn wait_for(turn: &Turn, runtime: &Path, lock: &File) -> Result<Option<String>, Failure> {
    let ahead = if turn.ahead {
        Some(
            Ahead::wait(runtime, &turn.name)
                .map_err(|error| failed(&turn.name, "can't go ahead", &error))?,
        )
    } else {
        None
    };
    let every = if turn.ahead {
        AHEAD_LOOKS_EVERY
    } else {
        LOOK_EVERY
    };
    let began = Instant::now();
    let mut told: Option<String> = None;
    let mut left = None;
    loop {
        let said = match look(lock, runtime, turn.ahead, &mut left) {
            Ok(Look::Had) => break,
            Ok(Look::Taken) => match held_above(runtime, lock) {
                Held::Above(holder) => return Ok(Some(holder)),
                Held::Elsewhere => {
                    format!("waiting for the turn, which {} has", holder(runtime))
                }
                // Said, since a turn inside its holder's command then
                // waits on its own parent, and only this line shows it.
                Held::Unread(why) => format!(
                    "waiting for the turn, which {} has, though whether it's this job's own \
                     parent can't be told, as {why}",
                    holder(runtime)
                ),
            },
            Ok(Look::Left(next)) => format!("leaving the turn to {}", next.why()),
            Err(error) => return Err(failed(&turn.name, "can't take the lock", &error)),
        };
        if told.as_ref() != Some(&said) {
            say(&turn.name, &said);
            told = Some(said);
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
        thread::sleep(every);
    }
    drop(ahead);
    if told.is_some() {
        say(
            &turn.name,
            &format!(
                "has the turn after {} s of waiting",
                began.elapsed().as_secs()
            ),
        );
    }
    Ok(None)
}

/// Runs the job's command below this process, and ends as it ended once
/// it and whatever it left have.
fn run(turn: &Turn) -> Result<(), Failure> {
    let held = hold_signals();
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
    let child = command.spawn().map_err(|error| {
        failed(
            &turn.name,
            &format!("can't start {}", turn.command[0]),
            &error,
        )
    })?;
    let top = i32::try_from(child.id()).unwrap_or(i32::MAX);
    let (mut status, ended_by) = watch(top, &held);
    let ended = end_what_is_left(top, &mut status);
    if ended.left > 0 {
        say(
            &turn.name,
            &format!(
                "ended {} process{} its command left running",
                ended.left,
                if ended.left == 1 { "" } else { "es" }
            ),
        );
    }
    if !ended.stuck.is_empty() {
        let pids: Vec<String> = ended.stuck.iter().map(i32::to_string).collect();
        say(
            &turn.name,
            &format!(
                "lets go of the turn though {} didn't end even when killed",
                pids.join(", ")
            ),
        );
    }
    if let Some(signal) = ended_by {
        return Err(end_by(&turn.name, signal));
    }
    let Some(status) = status else {
        return Err(Failure::new(
            Exit::Failed,
            format!(
                "night-shift {}: the command's end couldn't be read",
                turn.name
            ),
        ));
    };
    match (status.code(), status.signal()) {
        (Some(0), _) => Ok(()),
        (Some(code), _) => Err(Failure::passed(
            u8::try_from(code).unwrap_or(1),
            format!("night-shift {}: the command failed: {status}", turn.name),
        )),
        (None, Some(signal)) => Err(end_by(&turn.name, signal)),
        (None, None) => Err(Failure::new(
            Exit::Failed,
            format!(
                "night-shift {}: the command ended as nothing says: {status}",
                turn.name
            ),
        )),
    }
}

/// Waits for the command `top` to end, giving its status, or for a
/// signal that ends the turn, which goes on to the command and gives it
/// the grace to end; INT is the command's to hear from its terminal.
fn watch(top: i32, held: &SignalSet) -> (Option<ExitStatus>, Option<i32>) {
    let mut status = None;
    while status.is_none() {
        match signal_within(held, LOOK_EVERY) {
            Some(SIGCHLD | SIGINT) | None => reap(top, &mut status),
            Some(signal) => {
                send(top, signal);
                let given = Instant::now();
                while status.is_none() && given.elapsed() < LEFTOVERS_GRACE {
                    signal_within(held, Duration::from_millis(100));
                    reap(top, &mut status);
                }
                return (status, Some(signal));
            }
        }
    }
    (status, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The kernel names a held lock's file by its filesystem's device,
    // which is read from the mount rather than from stat, so this process
    // is found holding it on whatever filesystem the temp folder is.
    #[test]
    fn a_lock_held_here_is_held_by_this_process_as_the_kernel_says() {
        let folder = std::env::temp_dir().join(format!("night-shift-held-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let lock = File::create(folder.join("lock")).unwrap();
        lock.lock().unwrap();
        let holders = lock_holders(&lock);
        let unheld = lock_holders(&File::create(folder.join("other")).unwrap());
        drop(lock);
        fs::remove_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        assert_eq!(
            holders,
            Ok(vec![own]),
            "the kernel's word on the lock was misread"
        );
        assert_eq!(unheld, Ok(vec![]), "a file nobody locked was read as held");
    }

    // A tab or line break in a name, as a night job's from its config may
    // hold, and an empty name, shift none of the note's fields.
    #[test]
    fn a_note_names_its_process_whatever_its_name_holds() {
        let folder = std::env::temp_dir().join(format!("night-shift-note-{}", std::process::id()));
        fs::create_dir_all(&folder).unwrap();
        let own = i32::try_from(std::process::id()).unwrap();
        let mut read = Vec::new();
        for name in ["x\t1", "x\n1", ""] {
            note(&folder, name);
            read.push(noted(&folder));
        }
        fs::remove_dir_all(&folder).unwrap();
        assert_eq!(
            read,
            [
                Some(("x 1".to_owned(), own, 0)),
                Some(("x 1".to_owned(), own, 0)),
                Some((String::new(), own, 0)),
            ],
            "a name shifted the note's fields"
        );
    }

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
}

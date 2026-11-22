//! What sessions-come-back asks of the machine it runs on: which boot and
//! login this is, whether a process runs, and what a command printed
//! within its time. It changes with the Mac's and Linux's programs and
//! with how a command is stopped, not with what a run keeps or brings
//! back.

use super::LOOK_WITHIN;
use std::fmt;
use std::fs;
use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};
use terminal_kit::{Exit, Failure};

/// Why a command gave no answer: it failed, saying why; it was still
/// running at its limit; it was still running at its limit, much of
/// which went on starting it; or it ended, but what it printed didn't
/// come through by its limit, nor `READ_AFTER_END` after it ended, as
/// when it leaves a process holding its output, or when the reader
/// itself is starved. Each was stopped, and the stalls are told apart
/// since only the first is the command's own slowness: a start held up
/// is the machine's, or this agent's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Unheard {
    Failed(String),
    Late(Duration),
    LateToStart { limit: Duration, starting: Duration },
    Unread(Duration),
}

impl fmt::Display for Unheard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(why) => f.write_str(why),
            Self::Late(limit) => write!(f, "no answer within {} s", limit.as_secs()),
            // A start can take longer than the whole limit, when the read
            // is stopped at its first look.
            Self::LateToStart { limit, starting } if starting >= limit => write!(
                f,
                "no answer, since starting it took {:.1} s, past its {} s limit",
                starting.as_secs_f64(),
                limit.as_secs()
            ),
            Self::LateToStart { limit, starting } => write!(
                f,
                "no answer within {} s, {:.1} s of which went on starting it",
                limit.as_secs(),
                starting.as_secs_f64()
            ),
            Self::Unread(limit) => write!(
                f,
                "it ended, but what it printed wasn't read within {} s, nor {} s after it ended",
                limit.as_secs(),
                READ_AFTER_END.as_secs()
            ),
        }
    }
}

/// Which boot and login a Mac's are: its boot session uuid and when the
/// account logged in at the console, from `who`. One with no console line
/// is said, rather than read as a new login. A clock step changes
/// neither, nor does a change of time zone, since `who` is asked in UTC,
/// and a login starting over changes the second, to the minute:
/// one that ends and starts again in the same minute goes unseen.
pub(super) fn era_of(uuid: &str, who: &str, user: &str) -> Result<String, String> {
    let uuid = uuid.trim();
    if uuid.is_empty() {
        return Err("the boot session uuid is empty".into());
    }
    let login = console_login(who, user)
        .ok_or_else(|| format!("who names no console login for {user:?}"))?;
    Ok(format!("{uuid} {login}"))
}

/// Which boot and login this is: a Mac's, or Linux's boot id. A Mac's
/// read slow to answer is told to `slow`.
pub(super) fn era(slow: &dyn Fn(&str)) -> Result<String, Failure> {
    if let Ok(id) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        return Ok(id.trim().into());
    }
    mac_era(&|command| within(command, LOOK_WITHIN), slow)
}

/// How long a Mac's boot read may take to answer before it's told, though
/// it answered: each answers in milliseconds by hand, so one that took
/// this long was held up, and when that happens says whether the
/// stalls line up with the Mac's busy spells.
pub(super) const SLOW_READ: Duration = Duration::from_secs(1);

/// A Mac's boot and login, from what `look` reads of each program. A read
/// that fails names its program, since a Mac's three each answer at once
/// by hand, and only the name says which one a busy Mac held up. One that
/// answered but took `SLOW_READ` or longer is told to `slow`, with how
/// long it took; one that failed after as long says so in its failure,
/// since a stall that ends in failure is told apart from a read refused
/// at once only by its time. A stall stopped at its limit says its time
/// already.
pub(super) fn mac_era(
    look: &dyn Fn(&mut Command) -> Result<String, Unheard>,
    slow: &dyn Fn(&str),
) -> Result<String, Failure> {
    let unread = |why: String| {
        Failure::new(Exit::Unavailable, "it couldn't read which boot this is").because(why)
    };
    let ran = |command: &mut Command| {
        let program = command.get_program().display().to_string();
        let asked = Instant::now();
        let answer = look(command).map_err(|e| {
            let took = asked.elapsed();
            unread(match e {
                Unheard::Failed(_) if took >= SLOW_READ => {
                    format!("{program}: {e}, after {:.1} s", took.as_secs_f64())
                }
                _ => format!("{program}: {e}"),
            })
        })?;
        let took = asked.elapsed();
        if took >= SLOW_READ {
            slow(&format!(
                "{program} answered the boot read, but took {:.1} s",
                took.as_secs_f64()
            ));
        }
        Ok(answer)
    };
    let uuid = ran(Command::new("/usr/sbin/sysctl").args(["-n", "kern.bootsessionuuid"]))?;
    let who = ran(Command::new("/usr/bin/who").env("TZ", "UTC0"))?;
    let user = ran(Command::new("/usr/bin/id").arg("-un"))?;
    era_of(&uuid, &who, user.trim()).map_err(unread)
}

/// When the account logged in at the console, from `who`'s line for it:
/// `someone      console      Nov 19 12:42`.
pub(super) fn console_login(who: &str, user: &str) -> Option<String> {
    who.lines().find_map(|line| {
        let mut words = line.split_whitespace();
        (words.next() == Some(user) && words.next() == Some("console"))
            .then(|| words.collect::<Vec<_>>().join(" "))
            .filter(|login| !login.is_empty())
    })
}

/// Whether a process runs: Linux's /proc, or a Mac's `kill -0`, which a
/// process of another account's refuses, and that isn't this one's pi.
/// A pid past what `kill` reads as one process, which it would read as a
/// group, runs nothing.
pub(super) fn runs(pid: u32) -> bool {
    if pid == 0 || i32::try_from(pid).is_err() {
        return false;
    }
    if Path::new("/proc/self").exists() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    within(
        Command::new("/bin/kill").args(["-0", &pid.to_string()]),
        LOOK_WITHIN,
    )
    .is_ok()
}

/// The program that stops a command's process group: a Mac's own, by its
/// path as every program here is, or on Linux, where this runs only under
/// test, the one on the PATH.
const KILL: &str = if cfg!(target_os = "macos") {
    "/bin/kill"
} else {
    "kill"
};

/// How long what a command printed has to come through once it ended,
/// whatever its limit has left, so one that ends just at its limit is
/// read rather than told as unread: only output held back, or a reader
/// starved, keeps it longer.
pub(super) const READ_AFTER_END: Duration = Duration::from_secs(2);

/// When what a command printed has to have been read by, once it ended.
pub(super) fn read_by(deadline: Instant, ended: Instant) -> Instant {
    deadline.max(ended + READ_AFTER_END)
}

/// Why a command still running at its `limit` was stopped: plain late,
/// unless starting it took `READ_AFTER_END` or more of the limit, which a
/// program that answers at once never does, when that's said too.
/// `starting` is only how long spawning it took, so a program slow to load
/// once it runs, as with a cold dynamic loader, still reads as plain late.
pub(super) fn late(limit: Duration, starting: Duration) -> Unheard {
    if starting >= READ_AFTER_END {
        Unheard::LateToStart { limit, starting }
    } else {
        Unheard::Late(limit)
    }
}

/// Reads a pipe whole on a thread of its own, and sends what it read.
fn drain(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<String> {
    let (send, heard) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut text);
        }
        let _ = send.send(text);
    });
    heard
}

/// What a command printed, or why it couldn't say: it failed, or gave no
/// answer in time, and was stopped with everything it started, so a node
/// that hangs, or leaves a process holding its output, holds up no run.
pub(super) fn within(command: &mut Command, limit: Duration) -> Result<String, Unheard> {
    let asked = Instant::now();
    let deadline = asked + limit;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| Unheard::Failed(e.to_string()))?;
    let starting = asked.elapsed();
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let stop = |child: &mut std::process::Child, why: Unheard| {
        let _ = Command::new(KILL)
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
        let _ = child.wait();
        why
    };
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Unheard::Failed(e.to_string()))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err(stop(&mut child, late(limit, starting)));
        }
        thread::sleep(Duration::from_millis(50));
    };
    let read_by = read_by(deadline, Instant::now());
    let left = || read_by.saturating_duration_since(Instant::now());
    let (Ok(out), Ok(err)) = (out.recv_timeout(left()), err.recv_timeout(left())) else {
        return Err(stop(&mut child, Unheard::Unread(limit)));
    };
    if status.success() {
        Ok(out)
    } else {
        let said: Vec<&str> = [err.trim(), out.trim()]
            .into_iter()
            .filter(|said| !said.is_empty())
            .collect();
        // One that said nothing is told by how it ended, an exit code or
        // the signal that stopped it, rather than by nothing at all.
        Err(Unheard::Failed(if said.is_empty() {
            status.to_string()
        } else {
            said.join("; ")
        }))
    }
}

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

/// Why a command gave no answer: it failed, saying why, or said nothing
/// in time and was stopped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Unheard {
    Failed(String),
    Late(Duration),
}

impl fmt::Display for Unheard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failed(why) => f.write_str(why),
            Self::Late(limit) => write!(f, "no answer within {} s", limit.as_secs()),
        }
    }
}

/// Which boot and login a Mac's are: its boot session uuid and when the
/// account logged in at the console, from `who`. One with no console line
/// is said, rather than read as a new login. A clock step changes
/// neither, and a login starting over changes the second, to the minute:
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

/// Which boot and login this is: a Mac's, or Linux's boot id.
pub(super) fn era() -> Result<String, Failure> {
    let unread = |why: String| {
        Failure::new(Exit::Unavailable, "it couldn't read which boot this is").because(why)
    };
    if let Ok(id) = fs::read_to_string("/proc/sys/kernel/random/boot_id") {
        return Ok(id.trim().into());
    }
    let ran =
        |command: &mut Command| within(command, LOOK_WITHIN).map_err(|e| unread(e.to_string()));
    let uuid = ran(Command::new("/usr/sbin/sysctl").args(["-n", "kern.bootsessionuuid"]))?;
    let who = ran(&mut Command::new("/usr/bin/who"))?;
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
pub(super) fn runs(pid: u32) -> bool {
    if Path::new("/proc/self").exists() {
        return Path::new(&format!("/proc/{pid}")).exists();
    }
    within(
        Command::new("/bin/kill").args(["-0", &pid.to_string()]),
        LOOK_WITHIN,
    )
    .is_ok()
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
    let deadline = Instant::now() + limit;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()
        .map_err(|e| Unheard::Failed(e.to_string()))?;
    let out = drain(child.stdout.take());
    let err = drain(child.stderr.take());
    let stop = |child: &mut std::process::Child| {
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = child.kill();
        let _ = child.wait();
        Unheard::Late(limit)
    };
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|e| Unheard::Failed(e.to_string()))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err(stop(&mut child));
        }
        thread::sleep(Duration::from_millis(50));
    };
    let left = || deadline.saturating_duration_since(Instant::now());
    let (Ok(out), Ok(err)) = (out.recv_timeout(left()), err.recv_timeout(left())) else {
        return Err(stop(&mut child));
    };
    if status.success() {
        Ok(out)
    } else {
        let said: Vec<&str> = [err.trim(), out.trim()]
            .into_iter()
            .filter(|said| !said.is_empty())
            .collect();
        Err(Unheard::Failed(said.join("; ")))
    }
}

//! Who asked, for an ask only the shell may make: Start New Work, which
//! begins new work as Joel (sprout.rs), and an answer, a row's act, a
//! line told or feedback, which the node takes as his (main.rs's
//! answered and `as_joel`). The agents daemon's socket takes any process
//! of Joel's account, an agent's command among them, and the node judges
//! the daemon as Joel, so a sprout an agent could ask here would start
//! work with more than the agent's own reach, an answer would answer its
//! own ask as him, and an act would steer a session as him. The kernel
//! names the process at the other end by pidfd (`SO_PEERPIDFD`), never a
//! pid an ask says, and its cgroup says which unit runs it: only the
//! shell's unit, under this daemon's own account, and only the unit's
//! main process, as its manager names it, running quickshell's own
//! program. Other programs run in that unit too (pi-ask while Super+S
//! talks, the shell's helpers, their children and orphans), and any of
//! them could run a program named quickshell, so the program alone is a
//! second line, as pineapple's peer.rs holds it (ISXEG8 on #5456).
//! The shell in turn starts new work and answers only from Joel's own
//! Return, never a walk from outside (Agents.qml's sprout and
//! answerKept, Now.qml's answerLook).
//!
//! What this holds to: a fenced command can't move itself out of its own
//! cgroup, but an unfenced process of Joel's could write itself into the
//! shell's.
//! The read is jobs-broker's (../jobs-broker/src/peer.rs), which needs
//! the pidfd for more than this does.
//!
//! The niri checks' daemon alone is built with the `walks` feature
//! (package.nix's `walks`), whose judge takes the shell by its program
//! alone: there the shell runs as a plain process in a build sandbox,
//! with no manager, no unit and, on a kernel before 6.5, no pidfd peer.
//! No installed daemon is built with it.
//!
//! Long because its cases are over a third of it: each holds one way an
//! asker that isn't the shell might pass, and they read the private
//! judge, its manager's reads and what it keeps between asks.

#![cfg_attr(
    feature = "walks",
    expect(
        dead_code,
        reason = "the installed judge's reads go unused in the walks' build"
    )
)]

use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

/// The names quickshell's program goes by: Nix's wrapper runs the real
/// one under this name, and an unwrapped build runs as itself.
const SHELL_PROGRAMS: [&str; 2] = [".quickshell-wrapped", "quickshell"];

/// The shell's unit, whose main process alone may ask.
const SHELL_UNIT: &str = "quickshell.service";

/// What a refusal says, whichever check refused.
pub const REFUSED: &str = "only the launcher starts new work here";

/// `SO_PEERPIDFD` (Linux 6.5), the same on every architecture the
/// machines run, which the libc crate names only in its newer layout.
const SO_PEERPIDFD: libc::c_int = 77;

/// Whether the process at the other end of `stream` is the shell's, or
/// why it can't be taken for it, in words the shell says.
///
/// # Errors
///
/// Why: it isn't the shell's, it ended before its cgroup could be known
/// to be its own, or the kernel names no peer by pidfd.
#[cfg(not(feature = "walks"))]
pub fn the_shell(stream: &UnixStream) -> Result<(), String> {
    let (pidfd, seen) = peer_of(stream).map_err(|why| untold(&why))?;
    // SAFETY: getuid has no failure and touches no memory.
    let uid = unsafe { libc::getuid() };
    judged(&TAKEN, pidfd, &seen, uid, || {
        remembered(&UNTOLD, Instant::now, shells_main_pid)
    })
}

/// The manager's last failure to name the shell's main process, and when
/// it ended. Each as-Joel ask on a connection is answered in turn, so a
/// slow manager's `MANAGER_PATIENCE` was paid again for each one the shell
/// sent together, and whatever it asked after them waited behind them
/// (round 000038 on #6796, F91).
static UNTOLD: Mutex<Option<Untold>> = Mutex::new(None);

/// How long a manager's failure answers for it: well under the launcher's
/// 4 s before it asks a refused act again (Daemon.qml's `askAgain`), so
/// the asks sent together are refused at once and each sent again gets a
/// fresh look.
const UNTOLD_FOR: Duration = Duration::from_secs(2);

/// A failure of the manager's, kept by when it ended and in its words.
struct Untold {
    at: Instant,
    kind: io::ErrorKind,
    why: String,
}

/// What `ask` says, or, within `UNTOLD_FOR` of its last failure as `kept`
/// holds it, that failure again, unasked. `clock` reads the time, so a
/// test holds it. A success forgets the failure.
///
/// # Errors
///
/// What `ask` gave, or its last failure, still kept.
fn remembered(
    kept: &Mutex<Option<Untold>>,
    clock: impl Fn() -> Instant,
    ask: impl FnOnce() -> io::Result<u32>,
) -> io::Result<u32> {
    let lock = || kept.lock().unwrap_or_else(PoisonError::into_inner);
    let still = lock()
        .as_ref()
        .filter(|last| clock().duration_since(last.at) < UNTOLD_FOR)
        .map(|last| io::Error::new(last.kind, last.why.clone()));
    if let Some(why) = still {
        return Err(why);
    }
    // Not held while the manager is asked, so no other connection waits on it.
    let asked = ask();
    *lock() = asked.as_ref().err().map(|why| Untold {
        at: clock(),
        kind: why.kind(),
        why: why.to_string(),
    });
    asked
}

/// Why who asked can't be told, in the words the shell says.
fn untold(why: &io::Error) -> String {
    format!("who asked can't be told: {why}")
}

/// Whether `seen`, the process `pidfd` holds, is the shell under account
/// `uid`: the one `kept` holds, while both run, with no manager asked;
/// else as `takes` judges it, asking `main_pid`, and then kept in its
/// place. Given its store and its manager, so a test holds both.
///
/// # Errors
///
/// Why it isn't taken for the shell, as `the_shell` says it.
fn judged(
    kept: &Mutex<Option<Taken>>,
    pidfd: OwnedFd,
    seen: &Seen,
    uid: u32,
    main_pid: impl FnOnce() -> io::Result<u32>,
) -> Result<(), String> {
    // The one taken is looked at before the asker's own pidfd, so an asker
    // with its pid is that same process: both run at once, and no two
    // running processes share a pid.
    if taken_already(taken(kept).as_ref(), seen.pid) && alive(&pidfd) {
        return Ok(());
    }
    if !takes(seen, uid, main_pid).map_err(|why| untold(&why))? {
        // Who it was is journalled, quoted as the kernel read it, so an
        // agent asking as Joel can be traced to its own process and unit.
        let unit = seen
            .cgroup
            .lines()
            .find_map(|line| line.strip_prefix("0::"));
        eprintln!(
            "agents: refused pid {} running {:?} in {:?}: it isn't the shell",
            seen.pid,
            seen.exe,
            unit.unwrap_or("no unified cgroup")
        );
        return Err(REFUSED.to_owned());
    }
    // Read last, so a pid taken again since any read above can't answer.
    if alive(&pidfd) {
        *taken(kept) = Some(Taken {
            pidfd,
            pid: seen.pid,
        });
        Ok(())
    } else {
        Err(untold(&gone()))
    }
}

/// The process last taken for the shell, by a pidfd held on it, so its
/// later asks are taken without asking its manager again. Asking it for
/// each made a slow manager hold up every act of Joel's, and answering
/// them each on a thread of its own let two land out of order, a share's
/// end before its start (round 000056 on #6796, F16 and F30). Only the
/// last is kept: a shell started since is taken in its place once it
/// asks, and one that ended is never taken again.
static TAKEN: Mutex<Option<Taken>> = Mutex::new(None);

/// A process taken for the shell: its pidfd, and the pid it had then.
struct Taken {
    pidfd: OwnedFd,
    pid: u32,
}

/// What `kept` holds, read through a panic that poisoned it, since a
/// `Taken` is whole whenever it's written.
fn taken(kept: &Mutex<Option<Taken>>) -> std::sync::MutexGuard<'_, Option<Taken>> {
    kept.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Whether `pid` is still the process `taken` stands for: it has that
/// pid and runs yet, so the pid can't since have gone to another.
fn taken_already(taken: Option<&Taken>, pid: u32) -> bool {
    taken.is_some_and(|one| one.pid == pid && alive(&one.pidfd))
}

/// Whether the process at the other end of `stream` runs quickshell's
/// program, as the walks' shell does and their own test process doesn't:
/// the kernel names it by the pid it gives the socket, read once, which
/// is all a sandbox on a kernel before 6.5 can tell.
///
/// # Errors
///
/// Why: it isn't the shell's program, or it can't be read.
#[cfg(feature = "walks")]
pub fn the_shell(stream: &UnixStream) -> Result<(), String> {
    let mut said = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = libc::socklen_t::try_from(size_of::<libc::ucred>())
        .map_err(|_| untold(&io::Error::from(io::ErrorKind::InvalidInput)))?;
    // SAFETY: SO_PEERCRED writes one ucred into the struct of the size
    // `length` gives, and the socket is open for the call.
    let read = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut said).cast(),
            &raw mut length,
        )
    };
    if read != 0 {
        return Err(untold(&io::Error::last_os_error()));
    }
    // 0 for a peer in no namespace of this daemon's.
    let pid = said.pid;
    if pid <= 0 {
        return Err(untold(&gone()));
    }
    let exe = std::fs::read_link(format!("/proc/{pid}/exe")).map_err(|why| untold(&why))?;
    if the_shells_program(&exe.to_string_lossy()) {
        Ok(())
    } else {
        Err(REFUSED.to_owned())
    }
}

/// What was read of the process at the other end of a socket.
#[derive(Debug)]
pub struct Seen {
    /// Its pid, as its pidfd names it.
    pub pid: u32,
    /// Its `/proc/<pid>/cgroup`.
    pub cgroup: String,
    /// Where its `/proc/<pid>/exe` points.
    pub exe: String,
}

/// Whether `seen` is the shell under account `uid`: in its unit, running
/// its program, and the unit's main process, which `main_pid` asks the
/// manager for only once the rest holds, so nothing else costs a call.
///
/// # Errors
///
/// What `main_pid` gave when the manager couldn't be asked.
pub fn takes(
    seen: &Seen,
    uid: u32,
    main_pid: impl FnOnce() -> io::Result<u32>,
) -> io::Result<bool> {
    if !(of_the_shell(&seen.cgroup, uid) && the_shells_program(&seen.exe)) {
        return Ok(false);
    }
    // 0 when the unit has no main process, which no peer's pid is.
    Ok(main_pid()? == seen.pid)
}

/// How long this account's manager has to name the shell's main process,
/// for every ask the shell alone may make: a start, the places, a start
/// told and an answer. An answer waits on it before the node's own 5 s
/// (acts.rs's `PATIENCE`), and the launcher lets Joel answer again once
/// 30 s pass unheard (TalkAnswers.js's `SENT_STANDS_MS`), so the two stay
/// well inside that. A start waits longer on the node (sprout.rs's
/// `PATIENCE`), but a manager taking more than this to say one property
/// is wedged, so a start asked then is refused as untold rather than
/// waiting with it.
const MANAGER_PATIENCE: Duration = Duration::from_secs(3);

/// How often a manager not yet done is looked at again.
const MANAGER_LOOKS_EVERY: Duration = Duration::from_millis(10);

/// The shell's unit's main process, as this account's manager says it
/// within `MANAGER_PATIENCE`.
fn shells_main_pid() -> io::Result<u32> {
    let mut asked = Command::new("systemctl");
    asked.args([
        "--user",
        "show",
        "--property=MainPID",
        "--value",
        SHELL_UNIT,
    ]);
    main_pid_of(asking(&mut asked)?, MANAGER_PATIENCE)
}

/// `command` started, its words piped to be read and nothing to read.
fn asking(command: &mut Command) -> io::Result<Child> {
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
}

/// The main process `asked` says, as `systemctl show --value` says it,
/// within `patience`, the command killed and reaped where it doesn't end
/// in time or can't be waited on. Its words are read once it ends, only
/// as far as they are written: a process it left behind holding its
/// pipes would otherwise hold the read past any patience.
fn main_pid_of(mut asked: Child, patience: Duration) -> io::Result<u32> {
    let deadline = Instant::now() + patience;
    let status = loop {
        let ended = asked.try_wait();
        let over = Instant::now() >= deadline;
        match ended {
            Ok(Some(status)) => break status,
            Ok(None) if !over => thread::sleep(MANAGER_LOOKS_EVERY),
            // Dropped, a child is neither killed nor reaped, so neither a
            // command out of time nor one that can't be waited on is left.
            Ok(None) => {
                let _ = asked.kill();
                let _ = asked.wait();
                return Err(io::Error::other(format!(
                    "systemd didn't say the shell's main process within {patience:?}"
                )));
            }
            Err(why) => {
                let _ = asked.kill();
                let _ = asked.wait();
                return Err(why);
            }
        }
    };
    // Its words are a line each, well inside a pipe's buffer, so it never
    // waits on them being read before it ends.
    let said = asked
        .stdout
        .take()
        .map(written)
        .transpose()?
        .unwrap_or_default();
    let why = asked
        .stderr
        .take()
        .map(written)
        .transpose()?
        .unwrap_or_default();
    if !status.success() {
        return Err(io::Error::other(format!(
            "systemd couldn't say the shell's main process: {}",
            why.trim()
        )));
    }
    said.trim()
        .parse()
        .map_err(|_| io::Error::other("systemd named no main process for the shell"))
}

/// What an ended command wrote to `pipe`, read without waiting: what's
/// in it now, up to its end or to what a process still holding it hasn't
/// written yet.
fn written(pipe: impl Into<OwnedFd>) -> io::Result<String> {
    let pipe: OwnedFd = pipe.into();
    let fd = pipe.as_raw_fd();
    // SAFETY: fcntl reading the status flags of a descriptor this process
    // owns.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl setting the status flags of the same descriptor.
    let set = unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };
    if set < 0 {
        return Err(io::Error::last_os_error());
    }
    let mut read = std::fs::File::from(pipe);
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 512];
    loop {
        match read.read(&mut chunk) {
            Ok(0) => break,
            Ok(got) => bytes.extend_from_slice(&chunk[..got]),
            Err(why) if why.kind() == io::ErrorKind::WouldBlock => break,
            Err(why) if why.kind() == io::ErrorKind::Interrupted => {}
            Err(why) => return Err(why),
        }
    }
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

/// Whether `exe`, as `/proc/<pid>/exe` points, is quickshell's own
/// program.
pub fn the_shells_program(exe: &str) -> bool {
    Path::new(exe)
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SHELL_PROGRAMS.contains(&name))
}

/// Whether `cgroup`, as `/proc/<pid>/cgroup` says it, is the shell's own
/// unit under account `uid`: its unified line, that account's manager's
/// session slice and the shell's unit in it, never a unit inside it, nor
/// one merely named like it in another slice, as `systemd-run --user
/// --unit quickshell` could start while the shell isn't running.
pub fn of_the_shell(cgroup: &str, uid: u32) -> bool {
    let Some(path) = cgroup.lines().find_map(|line| line.strip_prefix("0::")) else {
        return false;
    };
    let steps: Vec<&str> = path.split('/').collect();
    match steps.as_slice() {
        [
            "",
            "user.slice",
            account,
            manager,
            "session.slice",
            "quickshell.service",
        ] => *account == format!("user-{uid}.slice") && *manager == format!("user@{uid}.service"),
        _ => false,
    }
}

/// The pidfd of the process at the other end of `stream`, and its pid,
/// cgroup and program, read through that pid: `alive` on the pidfd after
/// they're judged says whether they were its own.
fn peer_of(stream: &UnixStream) -> io::Result<(OwnedFd, Seen)> {
    let mut fd: libc::c_int = -1;
    let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>())
        .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
    // SAFETY: SO_PEERPIDFD writes one descriptor into an int of the size
    // `length` gives, and the socket is open for the call.
    let read = unsafe {
        libc::getsockopt(
            stream.as_raw_fd(),
            libc::SOL_SOCKET,
            SO_PEERPIDFD,
            (&raw mut fd).cast(),
            &raw mut length,
        )
    };
    if read != 0 || fd < 0 {
        let error = io::Error::last_os_error();
        // A pid alone could be another process's by the time it's read,
        // so a kernel before 6.5 is refused rather than trusted.
        return Err(if error.raw_os_error() == Some(libc::ENOPROTOOPT) {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "this kernel names no peer by pidfd (Linux 6.5 and later do)",
            )
        } else {
            error
        });
    }
    // SAFETY: the kernel just handed this descriptor to this process, and
    // nothing else holds it.
    let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
    let info = std::fs::read_to_string(format!("/proc/self/fdinfo/{}", pidfd.as_raw_fd()))?;
    // `-1` once the process has ended, `0` for one in no namespace of
    // this daemon's.
    let pid = info
        .lines()
        .find_map(|line| line.strip_prefix("Pid:"))
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0)
        .ok_or_else(gone)?;
    let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
    let exe = std::fs::read_link(format!("/proc/{pid}/exe"))?
        .to_string_lossy()
        .into_owned();
    Ok((pidfd, Seen { pid, cgroup, exe }))
}

/// Whether the process a pidfd stands for still runs: its pidfd reads
/// as ready once it has ended.
fn alive(pidfd: &OwnedFd) -> bool {
    let mut asked = libc::pollfd {
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    // SAFETY: one pollfd, for a descriptor this process holds open.
    let ready = unsafe { libc::poll(&raw mut asked, 1, 0) };
    ready == 0
}

fn gone() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "the process asking ended")
}

/// The most of what an ask names (its act, a session's address, a want's
/// id) a refusal's journal line says, its quoted JSON cut short: each is
/// far shorter.
const NAMED_AT_MOST: usize = 120;

/// What `value`, an asker's own, says in a journal line: its quoted JSON
/// cut short, so no ask writes more than a line.
#[must_use]
pub fn named(value: &serde_json::Value) -> String {
    value.to_string().chars().take(NAMED_AT_MOST).collect()
}

/// Why an ask [`the_shell`] refused wasn't taken, as its journal line says
/// it: [`REFUSED`] is Joel's words for the launcher, so the journal says
/// what the check found instead.
#[must_use]
pub fn journalled(why: &str) -> &str {
    if why == REFUSED {
        "who asked isn't the shell"
    } else {
        why
    }
}

/// What an act done as Joel may name it's for, the first it names said
/// in its refusal's line: a steer's session, Forget's, feedback's
/// landing, a told line's quest or window, and a read's keys.
const FOR_WHAT: [&str; 6] = ["address", "session", "landing", "quest", "window", "keys"];

/// The journal line of an act done as Joel that [`the_shell`] refused:
/// the act and the first of [`FOR_WHAT`] it names, each [`named`], so
/// an agent acting as Joel can be traced to what it asked. An act
/// naming none, as a share's end, says none rather than a null (round
/// 000056 on #6796, F19 and F32).
#[must_use]
pub fn refused_line(why: &str, said: &serde_json::Value) -> String {
    let whose = FOR_WHAT
        .iter()
        .map(|field| &said[field])
        .find(|value| !value.is_null())
        .map_or_else(String::new, |value| format!(" for {}", named(value)));
    format!(
        "agents: a {}{whose} wasn't taken: {}",
        named(&said["ask"]),
        journalled(why)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_acts_line_names_what_it_was_for_only_where_it_names_one() {
        let said = serde_json::json!({"ask": "forget", "session": "s-1"});
        assert_eq!(
            refused_line(REFUSED, &said),
            "agents: a \"forget\" for \"s-1\" wasn't taken: who asked isn't the shell"
        );
        let said = serde_json::json!({"ask": "feedback", "landing": "landing:s-p:o#1"});
        assert_eq!(
            refused_line(REFUSED, &said),
            "agents: a \"feedback\" for \"landing:s-p:o#1\" wasn't taken: who asked isn't the shell"
        );
        let said = serde_json::json!({"ask": "sharing", "sharing": false});
        assert_eq!(
            refused_line(REFUSED, &said),
            "agents: a \"sharing\" wasn't taken: who asked isn't the shell"
        );
        let said = serde_json::json!({"ask": "talk", "address": "P9R1KI"});
        assert_eq!(
            refused_line("who asked can't be told: gone", &said),
            "agents: a \"talk\" for \"P9R1KI\" wasn't taken: who asked can't be told: gone"
        );
    }

    #[test]
    fn a_refused_acts_line_says_an_askers_own_words_short_and_on_one_line() {
        let long = format!("{}\nagents: a forged line", "x".repeat(300));
        let line = refused_line(
            REFUSED,
            &serde_json::json!({"ask": "talk", "address": long}),
        );
        assert!(!line.contains('\n'), "one line: {line}");
        assert!(
            line.len() < 200,
            "the address cut to {NAMED_AT_MOST}: {line}"
        );
    }

    const SESSION: &str = "0::/user.slice/user-1000.slice/user@1000.service/session.slice";

    #[test]
    fn only_the_shells_own_unit_is_the_shell() {
        assert!(
            of_the_shell(&format!("{SESSION}/quickshell.service\n"), 1000),
            "the shell's unit"
        );
        assert!(
            !of_the_shell(&format!("{SESSION}/quickshell.service\n"), 1001),
            "the shell's unit under another account"
        );
        for other in [
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/pi-host.service\n"
                .to_owned(),
            format!("{SESSION}/quickshell.service/fenced.scope\n"),
            format!("{SESSION}/not-quickshell.service\n"),
            format!("{SESSION}/quickshell.service.d\n"),
            "0::/user.slice/user-1000.slice/user@1000.service/app.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-1000.slice/user@1001.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/system.slice/session.slice/quickshell.service\n".to_owned(),
            "0::/x/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-someone.slice/user@someone.service/session.slice/quickshell.service\n"
                .to_owned(),
            "0::/user.slice/user-.slice/user@.service/session.slice/quickshell.service\n".to_owned(),
            "0::/\n".to_owned(),
            String::new(),
            "1:name=systemd:/quickshell.service\n".to_owned(),
        ] {
            assert!(!of_the_shell(&other, 1000), "{other:?} isn't the shell");
        }
    }

    #[test]
    fn only_quickshells_own_program_is_the_shell() {
        for shell in [
            "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/.quickshell-wrapped",
            "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/quickshell",
        ] {
            assert!(the_shells_program(shell), "{shell} is the shell's");
        }
        // What runs beside it in its unit: pi-ask's pi and bash, its helpers.
        for other in [
            "/nix/store/giny1jia-nodejs-22.23.5/bin/node",
            "/nix/store/hasnvc0j-bash-5.3p20/bin/bash",
            "/run/current-system/sw/bin/litra",
            "/nix/store/x/bin/quickshell-helper",
            "",
        ] {
            assert!(!the_shells_program(other), "{other:?} isn't the shell's");
        }
    }

    fn in_the_shells_unit(pid: u32, exe: &str) -> Seen {
        Seen {
            pid,
            cgroup: format!("{SESSION}/quickshell.service\n"),
            exe: exe.to_owned(),
        }
    }

    const WRAPPED: &str = "/nix/store/2mr2gyg6-quickshell-0.3.2/bin/.quickshell-wrapped";

    #[test]
    fn only_the_shells_main_process_is_the_shell() {
        let shell = in_the_shells_unit(4_106_561, WRAPPED);
        assert!(
            takes(&shell, 1000, || Ok(4_106_561)).expect("asked"),
            "the unit's main process"
        );
        // A helper in the unit, or its child or orphan, running a program
        // named quickshell: the store's own with its own QML, or a copy.
        assert!(
            !takes(&in_the_shells_unit(4_200_000, WRAPPED), 1000, || Ok(
                4_106_561
            ))
            .expect("asked"),
            "a process beside the main one isn't the shell"
        );
        assert!(
            !takes(&shell, 1000, || Ok(0)).expect("asked"),
            "no main process, as while the shell restarts, is no shell"
        );
        assert!(
            takes(&shell, 1000, || Err(io::Error::other("no bus"))).is_err(),
            "a manager that can't be asked refuses"
        );
    }

    #[test]
    fn the_manager_is_asked_only_once_the_rest_holds() {
        let asked = || -> io::Result<u32> { panic!("the manager was asked") };
        let node = in_the_shells_unit(4_106_561, "/nix/store/giny1jia-nodejs-22.23.5/bin/node");
        assert!(
            !takes(&node, 1000, asked).expect("not asked"),
            "pi-ask's pi"
        );
        let elsewhere = Seen {
            cgroup: format!("{SESSION}/not-quickshell.service\n"),
            ..in_the_shells_unit(4_106_561, WRAPPED)
        };
        assert!(
            !takes(&elsewhere, 1000, asked).expect("not asked"),
            "another unit"
        );
    }

    /// A manager stood in by `sh`, started, saying what `script` does.
    fn manager(script: &str) -> Child {
        let mut said = Command::new("sh");
        said.args(["-c", script]);
        asking(&mut said).expect("sh starts")
    }

    #[test]
    fn the_managers_main_process_is_read_from_what_it_says() {
        let patience = Duration::from_secs(5);
        assert_eq!(
            main_pid_of(manager("echo 4106561"), patience).expect("said"),
            4_106_561
        );
        // 0 is a unit with no main process, which no peer's pid is.
        assert_eq!(main_pid_of(manager("echo 0"), patience).expect("said"), 0);
        let unread = main_pid_of(manager("echo active"), patience).expect_err("no pid");
        assert!(
            unread.to_string().contains("named no main process"),
            "{unread}"
        );
    }

    #[test]
    fn a_manager_that_fails_says_why_in_its_own_words() {
        let failed = main_pid_of(
            manager("echo 'no such unit' >&2; exit 1"),
            Duration::from_secs(5),
        )
        .expect_err("it failed");
        assert!(failed.to_string().ends_with(": no such unit"), "{failed}");
    }

    #[test]
    fn a_manager_out_of_time_is_ended_within_its_patience() {
        let patience = Duration::from_secs(1);
        let began = Instant::now();
        // Its pid is known as it starts, so whether it was killed can be
        // read however late a loaded machine runs it.
        let wedged = manager("exec sleep 30");
        let pid = wedged.id().to_string();
        let late = main_pid_of(wedged, patience).expect_err("out of time");
        assert!(late.to_string().contains("within 1s"), "{late}");
        assert!(
            began.elapsed() < Duration::from_secs(5),
            "it waited {:?}",
            began.elapsed()
        );
        // Killed and reaped, it is no process at all, not even a zombie.
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "the wedged manager {pid} still runs"
        );
    }

    #[test]
    fn a_manager_ended_is_read_without_waiting_on_what_it_left_behind() {
        let began = Instant::now();
        // It ends at once, leaving a process holding its pipes open.
        let said = main_pid_of(manager("echo 7; sleep 5 &"), Duration::from_secs(1))
            .expect("said before it ended");
        assert_eq!(said, 7);
        assert!(
            began.elapsed() < Duration::from_secs(3),
            "its words were read for {:?}, as long as what it left held them",
            began.elapsed()
        );
    }

    /// A pidfd on the process `pid`, as the kernel opens one for it.
    fn pidfd_of(pid: u32) -> OwnedFd {
        let pid = libc::pid_t::try_from(pid).expect("a pid");
        // SAFETY: pidfd_open takes a pid and no flags, and gives a new
        // descriptor or -1.
        let fd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        assert!(fd >= 0, "pidfd_open: {}", io::Error::last_os_error());
        // SAFETY: the kernel just handed this descriptor to this process,
        // and nothing else holds it.
        unsafe { OwnedFd::from_raw_fd(libc::c_int::try_from(fd).expect("a descriptor")) }
    }

    #[test]
    fn a_process_taken_is_taken_again_only_while_it_runs() {
        let mut shell = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep starts");
        let pid = shell.id();
        let taken = Some(Taken {
            pidfd: pidfd_of(pid),
            pid,
        });
        assert!(taken_already(taken.as_ref(), pid), "the one taken, running");
        assert!(!taken_already(taken.as_ref(), pid + 1), "another pid");
        assert!(!taken_already(None, pid), "none taken yet");
        shell.kill().expect("it's killed");
        shell.wait().expect("it's reaped");
        assert!(
            !taken_already(taken.as_ref(), pid),
            "once it ends, its pid could go to anyone"
        );
    }

    #[test]
    fn the_shell_once_taken_is_taken_again_with_no_manager_asked_while_it_runs() {
        let mut shell = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep starts");
        let pid = shell.id();
        let kept = Mutex::new(None);
        let seen = in_the_shells_unit(pid, WRAPPED);
        judged(&kept, pidfd_of(pid), &seen, 1000, || Ok(pid)).expect("its manager names it");
        let unasked = || -> io::Result<u32> { panic!("the manager was asked") };
        judged(&kept, pidfd_of(pid), &seen, 1000, unasked).expect("taken as it was");
        // Its pid alone answers nothing for another process, wherever it runs.
        let mut other = Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sleep starts");
        let beside = Seen {
            cgroup: format!("{SESSION}/not-quickshell.service\n"),
            ..in_the_shells_unit(other.id(), WRAPPED)
        };
        assert_eq!(
            judged(&kept, pidfd_of(other.id()), &beside, 1000, unasked),
            Err(REFUSED.to_owned()),
            "another process isn't the one taken"
        );
        shell.kill().expect("it's killed");
        shell.wait().expect("it's reaped");
        // Ended, it is never taken again, so the next asker is judged
        // whole, its manager asked.
        let asked = std::cell::Cell::new(false);
        let named = || {
            asked.set(true);
            Ok(pid)
        };
        let next = in_the_shells_unit(other.id(), WRAPPED);
        assert_eq!(
            judged(&kept, pidfd_of(other.id()), &next, 1000, named),
            Err(REFUSED.to_owned()),
            "its manager names another main process"
        );
        assert!(
            asked.get(),
            "the manager was asked once the taken one ended"
        );
        other.kill().expect("it's killed");
        other.wait().expect("it's reaped");
    }

    #[test]
    fn a_managers_failure_answers_for_it_only_a_moment() {
        let began = Instant::now();
        let now = std::cell::Cell::new(began);
        let clock = || now.get();
        let kept = Mutex::new(None);
        let asked = std::cell::Cell::new(0);
        let failing = || {
            asked.set(asked.get() + 1);
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "no answer within 3s",
            ))
        };
        let first = remembered(&kept, clock, failing).expect_err("it failed");
        now.set(began + Duration::from_millis(1500));
        let again = remembered(&kept, clock, failing).expect_err("still failed");
        assert_eq!(asked.get(), 1, "the manager isn't asked again so soon");
        assert_eq!(
            (again.kind(), again.to_string()),
            (first.kind(), first.to_string()),
            "its failure is said again in its own words"
        );
        now.set(began + UNTOLD_FOR);
        let named = remembered(&kept, clock, || {
            asked.set(asked.get() + 1);
            Ok(7)
        });
        assert_eq!(named.expect("asked afresh"), 7);
        assert_eq!(asked.get(), 2, "once the moment passes it's asked again");
        assert!(
            kept.lock().expect("kept").is_none(),
            "a success forgets the failure"
        );
    }

    #[test]
    fn a_process_not_the_shells_is_refused() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let refused = the_shell(&ours).expect_err("this test isn't the shell");
        // The lent builders run Linux 6.1, where who asked can't be told;
        // there that refusal is what's held to.
        assert!(
            refused == REFUSED || refused.contains("names no peer by pidfd"),
            "it says why: {refused}"
        );
    }
}

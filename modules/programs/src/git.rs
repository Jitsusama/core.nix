//! Git as Code runs it on a checkout (`pull.rs`, `switch.rs`, `code.rs`), with
//! Joel's config, since a fetch needs his keys and what his ssh and
//! credential helpers say, as his own git would. It runs in a session of
//! its own with no terminal, and with no askpass, hooks or fsmonitor, so it
//! asks at no terminal, and a program a hook names never runs beside it.
//! It's told the checkout's own folder outright, so it never looks past it
//! for a repository further up. One run past its wait is stopped with all
//! it started, and what git said when it failed goes to the daemon's
//! journal.

use std::io::{Read, Write};
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long git is given to let go of its locks once asked to stop,
/// before it's stopped outright.
const LETTING_GO: Duration = Duration::from_secs(5);

/// The longest reading a checkout may take before it's stopped, for Pull
/// and Switch Branch alike: its branch and the one it follows, its
/// branches, its changes and how far apart they are.
pub const READ: Duration = Duration::from_secs(30);

/// How a run of git failed: how it exited, none when it was stopped,
/// never started or printed what isn't text, and what it said, in C's
/// words whatever Joel's language, so a refusal can be told by them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    pub exit: Option<i32>,
    pub said: String,
}

/// How many files git tracks in the checkout at `folder` that differ from
/// its last commit, staged or not, or none when git couldn't say within
/// `wait`. Files it doesn't track aren't counted. It takes none of the
/// locks a status may, so it never stands in the way of Joel's own git,
/// and looks for no renames, which in a partial clone would fetch what a
/// file held from its remote; a file renamed counts as two.
#[must_use]
pub fn uncommitted(git: &str, folder: &Path, wait: Duration) -> Option<u32> {
    let status = [
        "--no-optional-locks",
        "status",
        "--porcelain",
        "--untracked-files=no",
        "--no-renames",
    ];
    let said = tried(git, folder, &status, Some(wait)).ok()?;
    Some(u32::try_from(said.lines().count()).unwrap_or(u32::MAX))
}

/// As [`tried`], answering only how git exited when it failed.
///
/// # Errors
///
/// How git exited when it failed, or none when it was stopped, never
/// started or printed what isn't text.
pub fn ran(
    git: &str,
    folder: &Path,
    args: &[&str],
    wait: Option<Duration>,
) -> Result<String, Option<i32>> {
    tried(git, folder, args, wait).map_err(|failure| failure.exit)
}

/// Runs git on the checkout at `folder`, named outright, with nothing to
/// answer it, answering what it printed, or else how it exited, none if
/// it was stopped or never started. It runs in a session of its own, with
/// no terminal for ssh to ask at and no askpass, hooks or fsmonitor. Past
/// `wait`, where one is given, it's stopped with all it started.
///
/// # Errors
///
/// How git failed, and what it said.
pub fn tried(
    git: &str,
    folder: &Path,
    args: &[&str],
    wait: Option<Duration>,
) -> Result<String, Failure> {
    told(git, folder, args, None, wait)
}

/// As [`ran`], with `input` on git's standard input, as `rev-list --stdin`
/// reads what would pass the command line's limit.
///
/// # Errors
///
/// How git exited when it failed, or none when it was stopped, never
/// started or printed what isn't text.
pub fn fed(
    git: &str,
    folder: &Path,
    args: &[&str],
    input: String,
    wait: Option<Duration>,
) -> Result<String, Option<i32>> {
    told(git, folder, args, Some(input), wait).map_err(|failure| failure.exit)
}

/// As [`tried`], with `input`, where there's one, on git's standard input.
fn told(
    git: &str,
    folder: &Path,
    args: &[&str],
    input: Option<String>,
    wait: Option<Duration>,
) -> Result<String, Failure> {
    let failed = |exit| Failure {
        exit,
        said: String::new(),
    };
    let mut command = Command::new(git);
    command
        .arg("-C")
        .arg(folder)
        .args(["-c", "core.askPass=", "-c", "core.fsmonitor=false"])
        .args(["-c", "core.hooksPath=/dev/null"])
        .args(args)
        .env("GIT_DIR", folder.join(".git"))
        .env("GIT_WORK_TREE", folder)
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("SSH_ASKPASS_REQUIRE", "never")
        // Its messages in C's words, which only they are read for. LC_ALL
        // would outrank them, so what it said becomes LANG's, and the rest
        // of Joel's settings stay as he set them.
        .env("LC_MESSAGES", "C")
        .env_remove("LC_ALL")
        .env_remove("LANGUAGE")
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(all) = std::env::var_os("LC_ALL").filter(|all| !all.is_empty()) {
        command.env("LANG", all);
    }
    // Nothing the daemon was started with points git at another repository.
    for elsewhere in [
        "GIT_COMMON_DIR",
        "GIT_INDEX_FILE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
    ] {
        command.env_remove(elsewhere);
    }
    // SAFETY: setsid is async-signal-safe and touches no memory of ours.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = command.spawn().map_err(|_| failed(None))?;
    // Written on a thread of its own, closed once written, so git never
    // waits on what's still to come while its own pipes fill.
    if let (Some(input), Some(mut pipe)) = (input, child.stdin.take()) {
        std::thread::spawn(move || {
            let _ = pipe.write_all(input.as_bytes());
        });
    }
    // Read as it prints, so a long status never fills its pipe and waits.
    let (printed, said) = (drained(child.stdout.take()), drained(child.stderr.take()));
    if let Some(wait) = wait
        && !ended_within(&child, wait)
    {
        stop(&mut child);
        return Err(failed(None));
    }
    let exit = child.wait().map_err(|_| failed(None))?;
    // A helper it left running, as ssh's shared connection is, may hold
    // its pipes open, so what was printed is waited on only so long.
    let printed = printed.recv_timeout(LETTING_GO).unwrap_or_default();
    if !exit.success() {
        let said = said.recv_timeout(LETTING_GO).unwrap_or_default();
        let said = String::from_utf8_lossy(&said).trim().to_owned();
        eprintln!(
            "git {} in {} failed: {said}",
            // Its subcommand, past any option given git itself.
            args.iter().find(|arg| !arg.starts_with('-')).unwrap_or(&""),
            folder.display(),
        );
        return Err(Failure {
            exit: exit.code(),
            said,
        });
    }
    String::from_utf8(printed).map_err(|_| failed(exit.code()))
}

/// What a pipe holds, read to its end on a thread of its own.
fn drained(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<Vec<u8>> {
    let (sent, read) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut all = Vec::new();
            let _ = pipe.read_to_end(&mut all);
            let _ = sent.send(all);
        });
    }
    read
}

/// Whether `child` ended within `wait`, left unreaped, so its id names its
/// group until it's waited on and no other group can take it meanwhile.
fn ended_within(child: &Child, wait: Duration) -> bool {
    let pid: libc::id_t = child.id();
    let started = Instant::now();
    loop {
        // SAFETY: waitid writes only the siginfo it's given; WNOWAIT
        // leaves the child to be reaped by `wait`.
        let mut info: libc::siginfo_t = unsafe { std::mem::zeroed() };
        let asked = unsafe {
            libc::waitid(
                libc::P_PID,
                pid,
                &raw mut info,
                libc::WEXITED | libc::WNOHANG | libc::WNOWAIT,
            )
        };
        // SAFETY: si_pid is set by waitid, and zero while it runs on.
        if asked != 0 || unsafe { info.si_pid() } != 0 {
            return true;
        }
        if started.elapsed() >= wait {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Stops `child` and everything it started, its session's whole group: a
/// fetch's ssh and remote helper as well as git. Asked first, so git
/// takes away its lock files, and stopped outright past [`LETTING_GO`],
/// all before git itself is reaped, while its id still names its group.
fn stop(child: &mut Child) {
    let Ok(group) = libc::pid_t::try_from(child.id()) else {
        let _ = child.kill();
        let _ = child.wait();
        return;
    };
    // SAFETY: signals the group the child leads, which setsid made.
    unsafe { libc::kill(-group, libc::SIGTERM) };
    ended_within(child, LETTING_GO);
    // What it started may outlive it, until stopped outright.
    // SAFETY: as above.
    unsafe { libc::kill(-group, libc::SIGKILL) };
    let _ = child.wait();
}

/// What the acts' tests share to run a stub git in place of git.
#[cfg(test)]
pub mod stub {
    use std::path::{Path, PathBuf};

    /// The first sh on the path, which a stub git is run by, as the build
    /// sandbox has no /bin/sh.
    pub fn shell() -> PathBuf {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(|folder| Path::new(folder).join("sh"))
            .find(|sh| sh.is_file())
            .expect("no sh on the path")
    }
}

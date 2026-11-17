//! Git as Code's acts run it on a checkout (`pull.rs`, `switch.rs`), with
//! Joel's config, since a fetch needs his keys and what his ssh and
//! credential helpers say, as his own git would. It runs in a session of
//! its own with no terminal, and with no askpass, hooks or fsmonitor, so it
//! asks at no terminal, and a program a hook names never runs beside it.
//! It's told the checkout's own folder outright, so it never looks past it
//! for a repository further up. One run past its wait is stopped with all
//! it started, and what git said when it failed goes to the daemon's
//! journal.

use std::io::Read;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How long git is given to let go of its locks once asked to stop,
/// before it's stopped outright.
const LETTING_GO: Duration = Duration::from_secs(5);

/// Runs git on the checkout at `folder`, named outright, with nothing to
/// answer it, answering what it printed, or else how it exited, none if
/// it was stopped or never started. It runs in a session of its own, with
/// no terminal for ssh to ask at and no askpass, hooks or fsmonitor. Past
/// `wait`, where one is given, it's stopped with all it started.
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
        .env_remove("GIT_ASKPASS")
        .env_remove("SSH_ASKPASS")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
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
    let mut child = command.spawn().map_err(|_| None)?;
    // Read as it prints, so a long status never fills its pipe and waits.
    let (printed, said) = (drained(child.stdout.take()), drained(child.stderr.take()));
    if let Some(wait) = wait
        && !ended_within(&child, wait)
    {
        stop(&mut child);
        return Err(None);
    }
    let exit = child.wait().map_err(|_| None)?;
    // A helper it left running, as ssh's shared connection is, may hold
    // its pipes open, so what was printed is waited on only so long.
    let printed = printed.recv_timeout(LETTING_GO).unwrap_or_default();
    if !exit.success() {
        let said = said.recv_timeout(LETTING_GO).unwrap_or_default();
        let said = String::from_utf8_lossy(&said);
        eprintln!(
            "git {} in {} failed: {}",
            args.first().unwrap_or(&""),
            folder.display(),
            said.trim()
        );
        return Err(exit.code());
    }
    String::from_utf8(printed).map_err(|_| exit.code())
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

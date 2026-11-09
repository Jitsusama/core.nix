//! A command run for at most a while: in a process group of its own, so
//! that when its time is up everything it started is stopped with it,
//! asked first and then made to. A check that ignores being asked, or a
//! child left holding its output, can't hold up every other to-do, and
//! nothing it started outlives it, though it ends in time.

use std::io::{self, Read};
use std::mem::MaybeUninit;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use std::os::unix::process::CommandExt;

/// How long a command asked to stop has before it's made to.
pub const GRACE: Duration = Duration::from_secs(5);
/// How long its output is waited for once it's gone, since a process
/// that left its group can keep the pipe open for ever.
const OUTPUT_AFTER: Duration = Duration::from_secs(1);

/// How a command ended.
#[derive(Debug)]
pub struct Ended {
    /// Its status, or none where it was stopped for running too long.
    pub status: Option<ExitStatus>,
    /// Its output, then its errors.
    pub output: Vec<u8>,
    pub took: Duration,
}

/// Sends `signal` to every process of the group `leader` leads.
fn tell_group(leader: u32, signal: libc::c_int) {
    if let Ok(leader) = libc::pid_t::try_from(leader) {
        // SAFETY: kill takes no pointers; a group already gone is ESRCH,
        // which is nothing to do.
        unsafe {
            libc::kill(-leader, signal);
        }
    }
}

/// Waits for `leader` to exit, leaving it unreaped, so its pid, and its
/// group's, stay its own until it's reaped.
fn exited(leader: u32) -> io::Result<()> {
    loop {
        let mut info = MaybeUninit::<libc::siginfo_t>::zeroed();
        // SAFETY: info is a siginfo_t for waitid to fill, which it
        // outlives.
        let waited = unsafe {
            libc::waitid(
                libc::P_PID,
                leader,
                info.as_mut_ptr(),
                libc::WEXITED | libc::WNOWAIT,
            )
        };
        if waited == 0 {
            return Ok(());
        }
        let error = io::Error::last_os_error();
        if error.kind() != io::ErrorKind::Interrupted {
            return Err(error);
        }
    }
}

/// Reads all of `from` on a thread of its own, into `into`.
fn gather(from: Option<impl Read + Send + 'static>, into: mpsc::Sender<Vec<u8>>) {
    if let Some(mut from) = from {
        thread::spawn(move || {
            let mut read = Vec::new();
            let _ = from.read_to_end(&mut read);
            let _ = into.send(read);
        });
    }
}

/// Runs `command` with no input, for at most `bound` and then `grace`
/// more once asked to stop.
pub fn run(mut command: Command, bound: Duration, grace: Duration) -> io::Result<Ended> {
    let started = Instant::now();
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .process_group(0)
        .spawn()?;
    let leader = child.id();
    let (out_tx, out_rx) = mpsc::channel();
    let (err_tx, err_rx) = mpsc::channel();
    gather(child.stdout.take(), out_tx);
    gather(child.stderr.take(), err_tx);
    let (ended_tx, ended_rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = ended_tx.send(exited(leader));
    });
    let waited = if let Ok(waited) = ended_rx.recv_timeout(bound) {
        waited.map(|()| true)
    } else {
        tell_group(leader, libc::SIGTERM);
        if ended_rx.recv_timeout(grace).is_err() {
            tell_group(leader, libc::SIGKILL);
        }
        ended_rx
            .recv()
            .map_err(io::Error::other)
            .and_then(|waited| waited.map(|()| false))
    };
    // Whatever it started may still be going, ended in time or not; none
    // of it outlives the check. Its leader isn't reaped yet, so the group
    // is still its own.
    tell_group(leader, libc::SIGKILL);
    let status = child.wait()?;
    let status = waited?.then_some(status);
    let mut output = out_rx.recv_timeout(OUTPUT_AFTER).unwrap_or_default();
    output.extend(err_rx.recv_timeout(OUTPUT_AFTER).unwrap_or_default());
    Ok(Ended {
        status,
        output,
        took: started.elapsed(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str) -> Command {
        let mut command = Command::new("sh");
        command.args(["-c", script]);
        command
    }

    #[test]
    fn a_command_in_time_says_its_status_and_output() {
        let ended = run(sh("echo out; echo err >&2; exit 3"), GRACE, GRACE).unwrap();
        assert_eq!(ended.status.and_then(|s| s.code()), Some(3));
        assert_eq!(ended.output, b"out\nerr\n");
    }

    #[test]
    fn a_command_in_time_leaves_nothing_it_started_running() {
        // The sleep stands for a child left going in the background, which
        // would hold the output open for its 30 s if it were left.
        let ended = run(sh("sleep 30 & echo started"), GRACE, GRACE).unwrap();
        assert_eq!(ended.status.and_then(|s| s.code()), Some(0));
        assert_eq!(ended.output, b"started\n");
        assert!(ended.took < OUTPUT_AFTER, "took {:?}", ended.took);
    }

    #[test]
    fn a_command_that_wont_stop_is_made_to_with_all_it_started() {
        // The sleeps stand for a check that hangs: one ignores being asked
        // to stop, and one in the background holds the output open.
        let script = "trap '' TERM; sleep 30 & echo started; sleep 30";
        let bound = Duration::from_millis(300);
        let grace = Duration::from_millis(300);
        let ended = run(sh(script), bound, grace).unwrap();
        assert!(ended.status.is_none(), "a command stopped has no status");
        assert!(ended.took < Duration::from_secs(5), "took {:?}", ended.took);
        assert_eq!(ended.output, b"started\n");
    }
}

//! Who pressed: the process at the other end of a connection to
//! `pineapple serve`, as the kernel names it (`SO_PEERPIDFD`), never as
//! an ask says, and whether it may act on a stop. Only Joel resumes, so
//! only his shell may: the launcher, running as the `quickshell` user
//! service. Its cgroup alone isn't enough, since other programs run in
//! it too (pi-ask while Super+S talks, the shell's helpers), so the
//! program must be quickshell's own as well. The held shell during a
//! switch stays in the same cgroup, so a press then still counts.
//!
//! Lifted from the jobs broker's peer (../jobs-broker/src/peer.rs), with
//! the program read beside the cgroup.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// `SO_PEERPIDFD` (Linux 6.5), the same on every architecture the
/// machines run, which the libc crate names only in its newer layout.
const SO_PEERPIDFD: libc::c_int = 77;

/// The cgroup Joel's shell runs in, as the end of its path.
const SHELL_UNIT: &str = "/session.slice/quickshell.service";

/// The names quickshell's program goes by: Nix's wrapper runs the real
/// one under this name, and an unwrapped build runs as itself.
const SHELL_PROGRAMS: [&str; 2] = [".quickshell-wrapped", "quickshell"];

/// The process that pressed, as it stood when it was read.
#[derive(Debug)]
pub struct Peer {
    /// Its pid in this process's namespace.
    pub pid: u32,
    /// Its `/proc/<pid>/cgroup`, read while its pidfd showed it alive.
    pub cgroup: String,
    /// Its program, as `/proc/<pid>/exe` points.
    pub exe: String,
}

/// Whether a process in `cgroup` running `exe` may act on a stop: only
/// Joel's shell, by its unit and its own program together.
#[must_use]
pub fn may_act(cgroup: &str, exe: &str) -> bool {
    let in_shell = cgroup
        .lines()
        .any(|line| line.starts_with("0::") && line.trim_end().ends_with(SHELL_UNIT));
    let program = Path::new(exe)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    in_shell && SHELL_PROGRAMS.contains(&program.as_str())
}

impl Peer {
    /// The peer of `stream`, refused when it ended before what was read of
    /// it could be known to be its own, or when the kernel can't say.
    ///
    /// # Errors
    ///
    /// Why the peer couldn't be named.
    pub fn of(stream: &UnixStream) -> io::Result<Self> {
        let mut fd: libc::c_int = -1;
        let mut length = libc::socklen_t::try_from(size_of::<libc::c_int>())
            .map_err(|_| io::Error::from(io::ErrorKind::InvalidInput))?;
        // SAFETY: SO_PEERPIDFD writes one descriptor into an int of the
        // size `length` gives, and the socket is open for the call.
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
            // A pid alone could be another process's by the time it's
            // read, so a kernel that names no peer by pidfd is refused.
            return Err(if error.raw_os_error() == Some(libc::ENOPROTOOPT) {
                io::Error::new(
                    io::ErrorKind::Unsupported,
                    "this kernel names no peer by pidfd (Linux 6.5 and later do)",
                )
            } else {
                error
            });
        }
        // SAFETY: the kernel just handed this descriptor to this process,
        // and nothing else holds it.
        let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
        let pid = pid_of(&pidfd)?;
        let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
        let exe = std::fs::read_link(format!("/proc/{pid}/exe"))?
            .to_string_lossy()
            .into_owned();
        // Alive after the reads, so the pid was still this process's and
        // what was read is its own, not a later one's given the same pid.
        if !alive(&pidfd) {
            return Err(gone());
        }
        Ok(Peer { pid, cgroup, exe })
    }
}

/// Whether a pidfd's process still runs: it reads as ready once it ends.
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

/// The pid a pidfd stands for, from its fdinfo, which says `-1` once the
/// process has ended and `0` for one in no namespace of this process's.
fn pid_of(pidfd: &OwnedFd) -> io::Result<u32> {
    let info = std::fs::read_to_string(format!("/proc/self/fdinfo/{}", pidfd.as_raw_fd()))?;
    info.lines()
        .find_map(|line| line.strip_prefix("Pid:"))
        .and_then(|pid| pid.trim().parse::<u32>().ok())
        .filter(|pid| *pid > 0)
        .ok_or_else(gone)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHELL: &str =
        "0::/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service\n";
    const WRAPPED: &str = "/nix/store/0000-quickshell-0.3.2/bin/.quickshell-wrapped";

    #[test]
    fn only_the_shells_own_program_in_its_unit_may_act() {
        assert!(may_act(SHELL, WRAPPED), "Joel's shell may");
        assert!(
            may_act(SHELL, "/usr/bin/quickshell"),
            "so may an unwrapped one"
        );
        assert!(
            !may_act(SHELL, "/nix/store/0000-pi-ask/bin/pi-ask"),
            "not another program in the shell's cgroup"
        );
        assert!(
            !may_act(SHELL, "/nix/store/0000-bash/bin/bash"),
            "not a shell the shell started"
        );
        let agent = "0::/user.slice/user-1000.slice/user@1000.service/jobs.slice/x.scope\n";
        assert!(
            !may_act(agent, WRAPPED),
            "not quickshell's program run anywhere else"
        );
        let deeper = "0::/user.slice/user-1000.slice/user@1000.service/session.slice/quickshell.service/x.scope\n";
        assert!(!may_act(deeper, WRAPPED), "not a scope beneath it");
    }

    #[test]
    fn a_connection_names_the_process_at_its_other_end() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let peer = match Peer::of(&ours) {
            // The lent builders run Linux 6.1, which has no SO_PEERPIDFD;
            // there the refusal itself is what's held to.
            Err(error) if error.kind() == io::ErrorKind::Unsupported => {
                assert!(
                    error.to_string().contains("names no peer by pidfd"),
                    "a kernel without SO_PEERPIDFD is named as the cause: {error}"
                );
                return;
            }
            read => read.expect("this process is its own peer"),
        };
        assert_eq!(peer.pid, std::process::id(), "the peer is this process");
        assert!(
            !may_act(&peer.cgroup, &peer.exe),
            "and a test isn't Joel's shell: {} {}",
            peer.cgroup,
            peer.exe
        );
    }
}

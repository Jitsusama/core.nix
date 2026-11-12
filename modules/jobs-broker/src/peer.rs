//! The process on the other end of a connection, as the kernel names it:
//! a pidfd for it (`SO_PEERPIDFD`), never a pid an ask says, so a process
//! can ask only for itself, and a pid reused once it's gone can never be
//! moved in its place.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;

/// `SO_PEERPIDFD` (Linux 6.5), the same on every architecture the
/// machines run, which the libc crate names only in its newer layout.
const SO_PEERPIDFD: libc::c_int = 77;

/// The process that asked, as it stood when it was read.
#[derive(Debug)]
pub struct Peer {
    pub pidfd: OwnedFd,
    /// Its pid in the broker's own namespace, whatever namespace the fence
    /// gave it.
    pub pid: u32,
    /// Its `/proc/<pid>/cgroup`, read while the pidfd showed it alive.
    pub cgroup: String,
}

impl Peer {
    /// The peer of `stream`, its pid and its cgroup, refused when it ended
    /// before the cgroup read could be known to be its own.
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
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the kernel just handed this descriptor to this process,
        // and nothing else holds it.
        let pidfd = unsafe { OwnedFd::from_raw_fd(fd) };
        let pid = pid_of(&pidfd)?;
        let cgroup = std::fs::read_to_string(format!("/proc/{pid}/cgroup"))?;
        let peer = Peer { pidfd, pid, cgroup };
        // Alive after the read, so the pid read was still this process's
        // and the cgroup is its own, not a later one's given the same pid.
        if !peer.alive() {
            return Err(gone());
        }
        Ok(peer)
    }

    /// Whether it's still running: a pidfd reads as ready once its process
    /// has ended.
    pub fn alive(&self) -> bool {
        let mut asked = libc::pollfd {
            fd: self.pidfd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        // SAFETY: one pollfd, for a descriptor this peer holds open.
        let ready = unsafe { libc::poll(&raw mut asked, 1, 0) };
        ready == 0
    }
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

    #[test]
    fn a_connection_names_the_process_at_its_other_end() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let peer = Peer::of(&ours).expect("this process is its own peer");
        assert_eq!(peer.pid, std::process::id());
        assert!(peer.alive(), "and it's running");
        assert!(
            peer.cgroup.starts_with("0::"),
            "its cgroup as the kernel says it: {}",
            peer.cgroup
        );
    }
}

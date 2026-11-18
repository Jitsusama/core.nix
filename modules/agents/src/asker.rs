//! Who asked, for an ask only the shell may make: Start New Work, which
//! begins new work as Joel (sprout.rs). The agents daemon's socket takes
//! any process of Joel's account, an agent's command among them, and the
//! node judges the daemon as Joel, so a sprout an agent could ask here
//! would start work with more than the agent's own reach. The kernel
//! names the process at the other end by pidfd (`SO_PEERPIDFD`), never a
//! pid an ask says, and its cgroup says which unit runs it: only the
//! shell's unit, under this daemon's own account, and only quickshell's
//! own program in it, since other programs run there too (pi-ask while
//! Super+S talks, the shell's helpers), as pineapple's peer.rs holds.
//! The shell in turn starts new work only from Joel's own Return, never
//! a walk from outside (Agents.qml's sprout).
//!
//! What this holds to: a fenced command can't move itself out of its own
//! cgroup, but an unfenced process of Joel's could write itself into the
//! shell's, as any process of his may steer his sessions here already.
//! The read is jobs-broker's (../jobs-broker/src/peer.rs), which needs
//! the pidfd for more than this does.

use std::io;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::net::UnixStream;
use std::path::Path;

/// The names quickshell's program goes by: Nix's wrapper runs the real
/// one under this name, and an unwrapped build runs as itself.
const SHELL_PROGRAMS: [&str; 2] = [".quickshell-wrapped", "quickshell"];

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
pub fn the_shell(stream: &UnixStream) -> Result<(), String> {
    let (cgroup, exe) = peer_of(stream).map_err(|why| format!("who asked can't be told: {why}"))?;
    // SAFETY: getuid has no failure and touches no memory.
    let uid = unsafe { libc::getuid() };
    if of_the_shell(&cgroup, uid) && the_shells_program(&exe) {
        Ok(())
    } else {
        Err("only the launcher starts new work here".to_owned())
    }
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

/// The cgroup and program of the process at the other end of `stream`,
/// read while its pidfd showed it alive, so a pid taken again since
/// can't answer.
fn peer_of(stream: &UnixStream) -> io::Result<(String, String)> {
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
    if alive(&pidfd) {
        Ok((cgroup, exe))
    } else {
        Err(gone())
    }
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

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn a_process_not_the_shells_is_refused() {
        let (ours, _theirs) = UnixStream::pair().expect("a socket pair");
        let refused = the_shell(&ours).expect_err("this test isn't the shell");
        // The lent builders run Linux 6.1, where who asked can't be told;
        // there that refusal is what's held to.
        assert!(
            refused == "only the launcher starts new work here"
                || refused.contains("names no peer by pidfd"),
            "it says why: {refused}"
        );
    }
}

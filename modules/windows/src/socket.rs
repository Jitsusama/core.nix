//! niri's socket, found when the daemon starts before niri does.
//!
//! A boot's first ask of the windows socket can start the daemon before
//! niri has named its socket to the user manager, so its own environment
//! holds none. Rather than fail, which logs an error each boot and blames
//! the daemon for an order nothing chose, it reads what the user manager
//! hands the units it starts now, every two seconds, until niri is named.

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// niri's socket, as named in an environment listing of `NAME=value`
/// lines, the shape `systemctl --user show-environment` says it in.
pub fn socket_in(environment: &str) -> Option<PathBuf> {
    environment
        .lines()
        .find_map(|line| line.strip_prefix("NIRI_SOCKET="))
        .filter(|socket| !socket.is_empty())
        .map(PathBuf::from)
}

/// niri's socket, as the user manager hands it to the units it starts
/// now; none while niri hasn't named one, or the manager can't be asked.
pub fn managed() -> Option<PathBuf> {
    let output = Command::new("systemctl")
        .args(["--user", "show-environment"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    socket_in(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn niris_socket_is_read_from_the_managers_environment() {
        let environment = "HOME=/home/j\nNIRI_SOCKET=/run/user/1000/niri.sock\nPATH=/bin\n";

        assert_eq!(
            socket_in(environment),
            Some(PathBuf::from("/run/user/1000/niri.sock"))
        );
    }

    #[test]
    fn an_environment_niri_hasnt_named_holds_no_socket() {
        let environment = "HOME=/home/j\nNIRI_SOCKETS_SEEN=2\nPATH=/bin\n";

        assert_eq!(socket_in(environment), None);
    }

    #[test]
    fn an_empty_socket_is_none() {
        assert_eq!(socket_in("NIRI_SOCKET=\n"), None);
    }
}

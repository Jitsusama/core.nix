//! The pi session a client's process says it runs in, for the journal
//! alone.
//!
//! Every fenced command runs as the person, so the session it names in its
//! environment is only a claim. The session a call is counted against is
//! the one whose socket it came in on, which the fence binds for that
//! session alone; the claim is said beside it as `claimed-session`, and
//! nothing is refused on it.

use std::fs;
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;

/// What a line says when the client's session can't be read.
pub const UNKNOWN: &str = "unknown";

/// The longest session id said, longer than any pi mints.
const LONGEST: usize = 64;

/// The session `client`'s process claims in its environment, or
/// [`UNKNOWN`]: a process gone, an environment it can't read, or an id
/// with a character a journal line's field shouldn't carry.
pub fn claimed_session(client: &UnixStream) -> String {
    pid(client)
        .and_then(|pid| fs::read(format!("/proc/{pid}/environ")).ok())
        .and_then(|environ| session_in(&environ))
        .unwrap_or_else(|| UNKNOWN.to_owned())
}

/// The process on the far end of `client`, as the kernel says.
fn pid(client: &UnixStream) -> Option<i32> {
    let mut credentials = libc::ucred {
        pid: 0,
        uid: 0,
        gid: 0,
    };
    let mut length = libc::socklen_t::try_from(std::mem::size_of::<libc::ucred>()).ok()?;
    // SAFETY: the socket is open for as long as `client` is borrowed, and
    // the kernel writes at most `length` bytes into `credentials`.
    let read = unsafe {
        libc::getsockopt(
            client.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&raw mut credentials).cast(),
            &raw mut length,
        )
    };
    (read == 0 && credentials.pid > 0).then_some(credentials.pid)
}

/// `PI_SESSION_ID` in an environment as `/proc/PID/environ` holds one,
/// when it's plain.
fn session_in(environ: &[u8]) -> Option<String> {
    let value = environ
        .split(|byte| *byte == 0)
        .find_map(|pair| pair.strip_prefix(b"PI_SESSION_ID="))?;
    let text = std::str::from_utf8(value).ok()?;
    plain_session(text).then(|| text.to_owned())
}

/// Whether `id` is a session id as pi mints them: letters, digits, `-`
/// and `_`, and not too long, so it can name a folder and a field alike.
pub fn plain_session(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= LONGEST
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_session_is_read_wherever_it_stands() {
        for environ in [
            &b"PI_SESSION_ID=01a1d112-2f75_x\0HOME=/home/someone\0"[..],
            b"HOME=/home/someone\0PI_SESSION_ID=01a1d112-2f75_x",
        ] {
            assert_eq!(session_in(environ).as_deref(), Some("01a1d112-2f75_x"));
        }
    }

    #[test]
    fn a_session_that_could_forge_a_field_or_a_folder_is_not() {
        for environ in [
            &b"PI_SESSION_ID=a b\0"[..],
            b"PI_SESSION_ID=a\nrefused why=x\0",
            b"PI_SESSION_ID=../x\0",
            b"PI_SESSION_ID=\0",
            b"NOT_PI_SESSION_ID=a\0",
        ] {
            assert_eq!(session_in(environ), None, "{environ:?}");
        }
    }

    #[test]
    fn a_peer_in_this_process_claims_this_processs_session() {
        let (ours, _theirs) = UnixStream::pair().expect("a pair");
        let own = std::env::var("PI_SESSION_ID")
            .ok()
            .filter(|id| plain_session(id))
            .unwrap_or_else(|| UNKNOWN.to_owned());
        assert_eq!(claimed_session(&ours), own);
    }
}

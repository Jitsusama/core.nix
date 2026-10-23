//! Waiting on a node's opens and closes, and on being told to stop, with
//! the few C library calls that take: inotify, signalfd and poll.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::Path;
use std::time::Duration;

const IN_CLOEXEC: c_int = 0o2_000_000;
const IN_CLOSE_WRITE: u32 = 0x008;
const IN_CLOSE_NOWRITE: u32 = 0x010;
const IN_OPEN: u32 = 0x020;
const IN_DELETE_SELF: u32 = 0x400;
const IN_IGNORED: u32 = 0x8000;
const POLLIN: i16 = 0x001;
const SIG_BLOCK: c_int = 0;
const SIGINT: c_int = 2;
const SIGTERM: c_int = 15;
const SFD_CLOEXEC: c_int = 0o2_000_000;

#[repr(C)]
struct PollFd {
    fd: c_int,
    events: i16,
    revents: i16,
}

/// glibc's `sigset_t`, 1024 bits.
#[repr(C)]
struct SigSet([u64; 16]);

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
    fn poll(fds: *mut PollFd, count: u64, timeout: c_int) -> c_int;
    fn sigemptyset(set: *mut SigSet) -> c_int;
    fn sigaddset(set: *mut SigSet, signal: c_int) -> c_int;
    fn sigprocmask(how: c_int, set: *const SigSet, old: *mut SigSet) -> c_int;
    fn signalfd(fd: c_int, set: *const SigSet, flags: c_int) -> c_int;
}

/// What a wait ended on.
pub enum Woken {
    Timeout,
    /// Something opened or closed the node.
    Changed,
    /// The node went.
    NodeGone,
    /// SIGTERM or SIGINT came.
    Stopped,
}

pub struct Events {
    stop: File,
    /// The watch on the node in hand, a queue of its own, so a node let go
    /// of can't be heard as the next one going.
    node: Option<File>,
}

fn owned(fd: c_int) -> io::Result<File> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor was just made and is ours alone; the file
    // closes it.
    Ok(unsafe { File::from_raw_fd(fd) })
}

/// Whether a run of inotify events says the node has gone.
pub fn gone(events: &[u8]) -> bool {
    let mut offset = 0;
    while offset + 16 <= events.len() {
        let field = |at: usize| {
            u32::from_ne_bytes([
                events[offset + at],
                events[offset + at + 1],
                events[offset + at + 2],
                events[offset + at + 3],
            ])
        };
        if field(4) & (IN_DELETE_SELF | IN_IGNORED) != 0 {
            return true;
        }
        offset += 16 + field(12) as usize;
    }
    false
}

impl Events {
    /// Blocks SIGTERM and SIGINT, so they wait to be read rather than end
    /// the program before it tidies.
    pub fn new() -> io::Result<Self> {
        let mut set = SigSet([0; 16]);
        // SAFETY: plain calls on a set of our own.
        let fd = unsafe {
            sigemptyset(&raw mut set);
            sigaddset(&raw mut set, SIGTERM);
            sigaddset(&raw mut set, SIGINT);
            if sigprocmask(SIG_BLOCK, &raw const set, std::ptr::null_mut()) != 0 {
                return Err(io::Error::last_os_error());
            }
            signalfd(-1, &raw const set, SFD_CLOEXEC)
        };
        Ok(Self {
            stop: owned(fd)?,
            node: None,
        })
    }

    /// Listens for the node being opened, closed or removed.
    pub fn watch(&mut self, node: &Path) -> io::Result<()> {
        self.node = None;
        // SAFETY: a plain call.
        let queue = owned(unsafe { inotify_init1(IN_CLOEXEC) })?;
        let path = CString::new(node.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        let mask = IN_OPEN | IN_CLOSE_WRITE | IN_CLOSE_NOWRITE | IN_DELETE_SELF;
        // SAFETY: the path is a valid C string for the call.
        if unsafe { inotify_add_watch(queue.as_raw_fd(), path.as_ptr(), mask) } < 0 {
            return Err(io::Error::last_os_error());
        }
        self.node = Some(queue);
        Ok(())
    }

    fn poll(&self, timeout: Option<Duration>, node: bool) -> (bool, bool) {
        let mut waiting = vec![PollFd {
            fd: self.stop.as_raw_fd(),
            events: POLLIN,
            revents: 0,
        }];
        if let (true, Some(queue)) = (node, &self.node) {
            waiting.push(PollFd {
                fd: queue.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            });
        }
        let millis = timeout.map_or(-1, |timeout| {
            c_int::try_from(timeout.as_millis()).unwrap_or(c_int::MAX)
        });
        // SAFETY: valid pollfds for the length given.
        let ready = unsafe { poll(waiting.as_mut_ptr(), waiting.len() as u64, millis) };
        if ready <= 0 {
            return (false, false);
        }
        let stopped = waiting[0].revents & POLLIN != 0;
        let changed = waiting.get(1).is_some_and(|queue| queue.revents != 0);
        (stopped, changed)
    }

    /// Waits for the node to change, for the time given if any, or for
    /// the word to stop.
    pub fn wait(&mut self, timeout: Option<Duration>) -> Woken {
        let (stopped, changed) = self.poll(timeout, true);
        if stopped {
            return Woken::Stopped;
        }
        if !changed {
            return Woken::Timeout;
        }
        // A program opens a camera several times as it starts; one look
        // after they settle answers for all of them.
        std::thread::sleep(Duration::from_millis(100));
        // The node's events carry no names, so a page holds plenty.
        let mut events = [0u8; 4096];
        let Some(queue) = self.node.as_mut() else {
            return Woken::NodeGone;
        };
        match queue.read(&mut events) {
            Ok(length) if gone(&events[..length]) => Woken::NodeGone,
            Ok(_) => Woken::Changed,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => Woken::Changed,
            Err(_) => Woken::NodeGone,
        }
    }

    /// Sleeps for the time given unless told to stop first.
    pub fn sleep(&mut self, time: Duration) -> Woken {
        match self.poll(Some(time), false) {
            (true, _) => Woken::Stopped,
            _ => Woken::Timeout,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(mask: u32, name: &[u8]) -> Vec<u8> {
        let mut event = Vec::new();
        event.extend_from_slice(&1i32.to_ne_bytes());
        event.extend_from_slice(&mask.to_ne_bytes());
        event.extend_from_slice(&0u32.to_ne_bytes());
        event.extend_from_slice(&u32::try_from(name.len()).unwrap().to_ne_bytes());
        event.extend_from_slice(name);
        event
    }

    #[test]
    fn the_node_going_is_heard_among_other_events() {
        let mut events = event(IN_OPEN, b"");
        events.extend(event(IN_CLOSE_NOWRITE, b"video0\0\0"));
        assert!(!gone(&events));
        events.extend(event(IN_DELETE_SELF, b""));
        assert!(gone(&events));
        assert!(gone(&event(IN_IGNORED, b"")));
    }
}

//! inotify: watching the folders facts come from, and reading what its
//! events name.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io;
use std::os::fd::FromRawFd;
use std::path::PathBuf;

const IN_NONBLOCK: c_int = 0o4000;
const IN_CLOEXEC: c_int = 0o2_000_000;
pub(crate) const IN_CLOSE_WRITE: u32 = 0x008;
pub(crate) const IN_CLOSE_NOWRITE: u32 = 0x010;
pub(crate) const IN_OPEN: u32 = 0x020;
pub(crate) const IN_MOVED_TO: u32 = 0x080;
pub(crate) const IN_CREATE: u32 = 0x100;
pub(crate) const IN_DELETE: u32 = 0x200;
pub(crate) const IN_Q_OVERFLOW: u32 = 0x4000;

unsafe extern "C" {
    fn inotify_init1(flags: c_int) -> c_int;
    fn inotify_add_watch(fd: c_int, path: *const c_char, mask: u32) -> c_int;
}

/// inotify, watching each of these folders for what the mask names. A
/// folder that isn't there yet, as /dev/snd is on a machine with no sound
/// card, is heard of by its uevent instead.
pub(crate) fn watches(watched: &[(PathBuf, u32)]) -> io::Result<File> {
    // SAFETY: a plain call.
    let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: the descriptor is ours alone, and the file closes it.
    let file = unsafe { File::from_raw_fd(fd) };
    for (path, mask) in watched {
        let mask = *mask;
        let path = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        unsafe { inotify_add_watch(fd, path.as_ptr(), mask) };
    }
    Ok(file)
}

/// Whether inotify's events name a file `wanted` takes, or lost some that
/// might have: a queue that overflowed, or an event cut short.
pub(crate) fn names(events: &[u8], wanted: impl Fn(&[u8]) -> bool) -> bool {
    let mut rest = events;
    while let Some((head, tail)) = rest.split_at_checked(16) {
        let word =
            |at: usize| u32::from_ne_bytes([head[at], head[at + 1], head[at + 2], head[at + 3]]);
        let (mask, length) = (word(4), word(12) as usize);
        let Some((name, after)) = tail.split_at_checked(length) else {
            return true;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or(name);
        if mask & IN_Q_OVERFLOW != 0 || wanted(name) {
            return true;
        }
        rest = after;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::*;

    #[test]
    fn events_name_what_is_wanted_or_what_might_have_been_lost() {
        let wanted = |name: &[u8]| name == b"wanted";
        let mut two = event(IN_CLOSE_WRITE, "other");
        two.extend(event(IN_MOVED_TO, "wanted"));
        let cut = event(IN_MOVED_TO, "wanted");
        assert_eq!(
            (
                names(&two, wanted),
                names(&event(IN_CLOSE_WRITE, "other"), wanted),
                names(&event(IN_Q_OVERFLOW, ""), wanted),
                names(&cut[..cut.len() - 1], |_| false),
                names(&[], |_| true),
            ),
            (true, false, true, true, false),
            "an event past the first wasn't read, one not wanted was taken, or a queue \
             overflowed or an event cut short wasn't taken for what might have been"
        );
    }
}

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
/// folder that isn't there, as /dev/snd is on a machine with no sound card,
/// is said in the journal and left unwatched, since a device that comes
/// later is heard of by its uevent. Any other refusal is an error, so the
/// unit restarts rather than going deaf to a folder that is there.
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
        let named = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        if unsafe { inotify_add_watch(fd, named.as_ptr(), mask) } < 0 {
            let error = io::Error::last_os_error();
            if error.kind() != io::ErrorKind::NotFound {
                return Err(io::Error::new(
                    error.kind(),
                    format!("can't watch {}: {error}", path.display()),
                ));
            }
            eprintln!(
                "<4>{} isn't there, so what changes in it goes unheard till a uevent",
                path.display()
            );
        }
    }
    Ok(file)
}

/// Whether inotify's events name a file `wanted` takes, or lost some that
/// might have: a queue that overflowed, or an event cut short.
pub(crate) fn names(events: &[u8], wanted: impl Fn(&[u8]) -> bool) -> bool {
    let mut rest = events;
    while !rest.is_empty() {
        let Some((head, tail)) = rest.split_at_checked(16) else {
            return true;
        };
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
    fn events_name_what_is_wanted() {
        let wanted = |name: &[u8]| name == b"wanted";
        let mut two = event(IN_CLOSE_WRITE, "other");
        two.extend(event(IN_MOVED_TO, "wanted"));
        assert!(names(&two, wanted), "an event past the first wasn't read");
        assert!(
            !names(&event(IN_CLOSE_WRITE, "other"), wanted),
            "an event naming a file not wanted was taken"
        );
        assert!(!names(&[], |_| true), "no events were taken for one");
    }

    #[test]
    fn each_event_s_name_is_passed_over_not_read_as_the_next_event() {
        // Read as a header, this name's last word is a length far past
        // the events, which would count as an event cut short.
        let mut two = event(IN_CLOSE_WRITE, "aaaaaaaaaaaabbbb");
        two.extend(event(IN_CLOSE_WRITE, "other"));
        assert!(
            !names(&two, |_| false),
            "a name was read as the event after it"
        );
    }

    #[test]
    fn events_that_might_have_been_lost_are_taken_for_what_was_wanted() {
        assert!(
            names(&event(IN_Q_OVERFLOW, ""), |_| false),
            "a queue that overflowed wasn't taken"
        );
        let cut = event(IN_MOVED_TO, "wanted");
        assert!(
            names(&cut[..cut.len() - 1], |_| false),
            "an event whose name was cut short wasn't taken"
        );
        let mut header_cut = event(IN_CLOSE_WRITE, "other");
        header_cut.extend(&event(IN_MOVED_TO, "wanted")[..15]);
        assert!(
            names(&header_cut, |_| false),
            "an event whose header was cut short wasn't taken"
        );
    }
}

//! inotify: watching the folders facts come from, and reading what its
//! events name.

use std::ffi::{CString, c_char, c_int};
use std::fs::File;
use std::io;
use std::os::fd::{AsRawFd, FromRawFd};
use std::path::{Path, PathBuf};

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
    fn inotify_rm_watch(fd: c_int, wd: c_int) -> c_int;
}

/// Why a folder can't be watched yet, when it may be later.
#[derive(Debug, PartialEq)]
enum Later {
    /// It isn't there, or what it's in isn't a folder, as /dev/snd isn't on
    /// a machine with no sound card till one is plugged in.
    Missing,
    /// The kernel has no room for another watch, of this person's or at all.
    Full,
}

/// Whether a refused watch may be made later, and why. Any other refusal
/// is a fault of the program's, such as a mask the kernel won't take.
fn later(error: &io::Error) -> Option<Later> {
    match error.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::NotADirectory => Some(Later::Missing),
        io::ErrorKind::StorageFull | io::ErrorKind::OutOfMemory => Some(Later::Full),
        _ => None,
    }
}

/// A folder not yet watched, and the watch on the folder it would be made
/// in, while it's missing, which hears it made.
struct Waiting {
    path: PathBuf,
    mask: u32,
    parent: Option<c_int>,
}

/// inotify over a set of folders, and those of them it can't watch yet.
///
/// A folder that's missing is watched for from the nearest folder above it
/// that's there, so its being made wakes whoever polls the file, as does a
/// device's uevent; then `watch_waiting` watches it. Till then anything made
/// in that folder above may read its kind again, at no cost but a reading.
/// A folder refused for want of room is tried again each time too, so the
/// rest stay heard rather than the unit restarting into the same refusal.
pub(crate) struct Watches {
    pub(crate) file: File,
    waiting: Vec<Waiting>,
}

impl Watches {
    /// Watches each of these folders for what its mask names, saying in the
    /// journal each it can't yet. Any other refusal is an error.
    pub(crate) fn new(watched: &[(PathBuf, u32)]) -> io::Result<Watches> {
        // SAFETY: a plain call.
        let fd = unsafe { inotify_init1(IN_NONBLOCK | IN_CLOEXEC) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: the descriptor is ours alone, and the file closes it.
        let file = unsafe { File::from_raw_fd(fd) };
        let mut inotify = Watches {
            file,
            waiting: Vec::new(),
        };
        for (path, mask) in watched {
            let Err(error) = inotify.add(path, *mask) else {
                continue;
            };
            let Some(why) = later(&error) else {
                return Err(io::Error::new(
                    error.kind(),
                    format!("can't watch {}: {error}", path.display()),
                ));
            };
            let parent = if why == Later::Missing {
                inotify.watch_for(path)?
            } else {
                None
            };
            eprintln!(
                "<4>can't watch {} yet ({error}), so it's watched once it can be",
                path.display()
            );
            inotify.waiting.push(Waiting {
                path: path.clone(),
                mask: *mask,
                parent,
            });
        }
        Ok(inotify)
    }

    /// Tries again each folder not yet watched, saying whether one now is,
    /// so what it holds is read again. A folder made watched lets go of the
    /// watch over the folder above it, unless another still waits on it.
    pub(crate) fn watch_waiting(&mut self) -> bool {
        let mut watched = Vec::new();
        for (at, waiting) in self.waiting.iter().enumerate() {
            if self.add(&waiting.path, waiting.mask).is_ok() {
                eprintln!("<6>{} is watched now", waiting.path.display());
                watched.push(at);
            }
        }
        for at in watched.iter().rev() {
            let gone = self.waiting.remove(*at);
            if let Some(wd) = gone.parent
                && !self.waiting.iter().any(|each| each.parent == Some(wd))
            {
                // SAFETY: a plain call; a watch already gone is refused.
                unsafe { inotify_rm_watch(self.file.as_raw_fd(), wd) };
            }
        }
        !watched.is_empty()
    }

    /// Watches a folder, giving the watch's number.
    fn add(&self, path: &Path, mask: u32) -> io::Result<c_int> {
        let named = CString::new(path.as_os_str().as_encoded_bytes()).map_err(io::Error::other)?;
        // SAFETY: the path is a valid C string for the call.
        let wd = unsafe { inotify_add_watch(self.file.as_raw_fd(), named.as_ptr(), mask) };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(wd)
    }

    /// Watches the nearest folder above a missing one that's there for what
    /// is made or moved in, giving the watch, or none when nothing above it
    /// can be watched.
    fn watch_for(&self, missing: &Path) -> io::Result<Option<c_int>> {
        for above in missing.ancestors().skip(1) {
            match self.add(above, IN_CREATE | IN_MOVED_TO) {
                Ok(wd) => return Ok(Some(wd)),
                Err(error) if later(&error).is_some() => {}
                Err(error) => return Err(error),
            }
        }
        Ok(None)
    }
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

    #[cfg(target_os = "linux")]
    use std::{fs, io::Read};

    /// Everything an inotify file has to say now.
    #[cfg(target_os = "linux")]
    fn said(file: &mut File) -> Vec<u8> {
        let mut all = Vec::new();
        let mut buffer = [0; 4096];
        while let Ok(read) = file.read(&mut buffer) {
            if read == 0 {
                break;
            }
            all.extend_from_slice(&buffer[..read]);
        }
        all
    }

    #[test]
    fn only_a_missing_folder_or_a_full_kernel_waits() {
        let cases = [
            (2, Some(Later::Missing), "ENOENT"),
            (20, Some(Later::Missing), "ENOTDIR"),
            (28, Some(Later::Full), "ENOSPC"),
            (12, Some(Later::Full), "ENOMEM"),
            (22, None, "EINVAL"),
            (13, None, "EACCES"),
        ];
        for (code, expected, name) in cases {
            assert_eq!(
                later(&io::Error::from_raw_os_error(code)),
                expected,
                "{name}"
            );
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_missing_folder_waits_and_any_other_refusal_fails() {
        let sys = Sys::new("watch-refused");
        // The kernel refuses a mask of nothing as invalid.
        assert!(
            Watches::new(&[(sys.0.clone(), 0)]).is_err(),
            "a refusal that waiting can't mend was let wait"
        );
        let watches = Watches::new(&[(sys.0.join("later"), IN_CREATE)])
            .expect("a missing folder was refused rather than let wait");
        assert_eq!(
            watches.waiting.len(),
            1,
            "a missing folder wasn't kept to watch"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_folder_made_after_its_watch_is_watched_once_made() {
        let sys = Sys::new("watch-later");
        let later = sys.0.join("later");
        let mut watches = Watches::new(&[(later.clone(), IN_CLOSE_WRITE)]).unwrap();
        let before = watches.watch_waiting();
        fs::create_dir(&later).unwrap();
        let made = said(&mut watches.file);
        let after = watches.watch_waiting();
        fs::write(later.join("file"), "x").unwrap();
        let written = said(&mut watches.file);
        // The folder above is let go of once nothing waits on it.
        fs::create_dir(sys.0.join("other")).unwrap();
        let unheard = said(&mut watches.file);
        assert_eq!(
            (
                before,
                names(&made, |name| name == b"later"),
                after,
                names(&written, |name| name == b"file"),
                unheard.is_empty(),
            ),
            (false, true, true, true, true),
            "a folder made later wasn't heard made, watched, then heard from, with the folder \
             above let go of"
        );
    }

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

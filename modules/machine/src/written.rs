//! A watch on folders for the files written in them, through the kernel's
//! inotify, so a file another program rewrites is heard as it's written
//! rather than at a read a second later: a file written in place and
//! closed, one renamed into place, as a file written whole is, or one
//! removed. A folder that isn't there yet is looked for again at each
//! wait, so one made later is watched from then, and so is one taken
//! away and made again, as systemd does a service's runtime folder.

use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::PathBuf;
use std::time::Duration;

/// What's heard of a file in a folder watched.
const HEARD: u32 = libc::IN_CLOSE_WRITE | libc::IN_MOVED_TO | libc::IN_DELETE;
/// Each event's fixed part: its watch, mask, cookie and its name's length,
/// four bytes each, before the name.
const HEADER: usize = 16;

pub struct Written {
    fd: OwnedFd,
    missing: Vec<PathBuf>,
    /// Each folder watched, by its watch.
    watched: Vec<(libc::c_int, PathBuf)>,
}

impl Written {
    /// A watch on these folders, or none where the kernel gives no watch.
    pub fn on(folders: impl IntoIterator<Item = PathBuf>) -> Option<Self> {
        // SAFETY: inotify_init1 takes only flags and returns a descriptor
        // of its own, or -1.
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if fd < 0 {
            return None;
        }
        // SAFETY: the descriptor was just made, and nothing else holds it.
        let fd = unsafe { OwnedFd::from_raw_fd(fd) };
        let mut written = Self {
            fd,
            missing: folders.into_iter().collect(),
            watched: Vec::new(),
        };
        written.look_again();
        Some(written)
    }

    /// Whether a folder it was given isn't there yet, and so is still
    /// looked for at each wait.
    pub fn looking(&self) -> bool {
        !self.missing.is_empty()
    }

    /// Watches each folder not watched yet that's there now, and says
    /// whether any was found.
    fn look_again(&mut self) -> bool {
        let fd = self.fd.as_raw_fd();
        let before = self.missing.len();
        let watched = &mut self.watched;
        self.missing.retain(|folder| {
            let Ok(path) = CString::new(folder.as_os_str().as_bytes()) else {
                return false;
            };
            // SAFETY: the path is a valid C string, alive across the call,
            // and the descriptor is this watch's own.
            let watch = unsafe { libc::inotify_add_watch(fd, path.as_ptr(), HEARD) };
            if watch >= 0 {
                watched.push((watch, folder.clone()));
            }
            watch < 0
        });
        self.missing.len() < before
    }

    /// The names of the files written within `wait`, all at once, with an
    /// empty name where a folder only now there began to be watched, since
    /// what's in it was never heard, or one watched was taken away; none
    /// when nothing was.
    pub fn wait(&mut self, wait: Duration) -> Vec<String> {
        let found = !self.missing.is_empty() && self.look_again();
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ms = libc::c_int::try_from(wait.as_millis()).unwrap_or(libc::c_int::MAX);
        // SAFETY: poll reads and writes only the one pollfd it's given.
        let ready = unsafe { libc::poll(&raw mut poll, 1, if found { 0 } else { ms }) };
        let mut names = if ready > 0 { self.drain() } else { Vec::new() };
        if found {
            names.push(String::new());
        }
        names
    }

    /// Reads every event waiting, each name once, a folder taken away
    /// looked for again from the next wait.
    fn drain(&mut self) -> Vec<String> {
        let mut names = Vec::new();
        let mut buffer = [0u8; 4096];
        loop {
            // SAFETY: read writes at most the buffer's length into it, and
            // the descriptor is this watch's own.
            let read = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            let Ok(read) = usize::try_from(read) else {
                break;
            };
            if read == 0 {
                break;
            }
            for (watch, mask, name) in events(&buffer[..read]) {
                if mask & libc::IN_IGNORED != 0 {
                    self.lost(watch);
                    names.push(String::new());
                } else if !name.is_empty() {
                    names.push(name);
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    /// Lets go of a watch the kernel ended, as it does when its folder is
    /// taken away, so the folder's looked for again.
    fn lost(&mut self, watch: libc::c_int) {
        if let Some(at) = self.watched.iter().position(|(each, _)| *each == watch) {
            let (_, folder) = self.watched.remove(at);
            self.missing.push(folder);
        }
    }
}

/// Each event in a read of them, as the kernel lays them out: its watch,
/// its mask and its file's name, an event of the folder itself having
/// none; each its fixed part, then its name, padded with nuls.
fn events(mut read: &[u8]) -> Vec<(libc::c_int, u32, String)> {
    let mut events = Vec::new();
    while read.len() >= HEADER {
        let word = |at: usize| -> [u8; 4] { read[at..at + 4].try_into().unwrap_or_default() };
        let watch = libc::c_int::from_ne_bytes(word(0));
        let mask = u32::from_ne_bytes(word(4));
        let length = usize::try_from(u32::from_ne_bytes(word(12))).unwrap_or(usize::MAX);
        let Some(name) = read.get(HEADER..HEADER.saturating_add(length)) else {
            break;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
        events.push((watch, mask, String::from_utf8_lossy(name).into_owned()));
        read = &read[HEADER + length..];
    }
    events
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::Instant;

    use super::*;

    #[test]
    fn a_file_written_whole_or_in_place_is_heard_at_once() {
        let folder = std::env::temp_dir().join(format!("machine-written-{}", std::process::id()));
        let _ = fs::remove_dir_all(&folder);
        fs::create_dir_all(&folder).unwrap();
        let later = folder.join("later");
        let mut written = Written::on([folder.clone(), later.clone()]).unwrap();
        assert_eq!(written.wait(Duration::ZERO), Vec::<String>::new());
        // Written whole, beside it then renamed into place.
        fs::write(folder.join("now.writing"), "level\t0.4\n").unwrap();
        fs::rename(folder.join("now.writing"), folder.join("now")).unwrap();
        let started = Instant::now();
        let heard = written.wait(Duration::from_secs(5));
        assert!(heard.contains(&"now".to_owned()), "{heard:?}");
        assert!(
            started.elapsed() < Duration::from_secs(1),
            "heard as it was written"
        );
        // Written in place.
        fs::write(folder.join("warmth"), "{}\n").unwrap();
        assert!(
            written
                .wait(Duration::from_secs(5))
                .contains(&"warmth".to_owned()),
            "a file written in place is heard"
        );
        // Written over in place and closed, as the keyboard-lights watcher
        // says caps lock, never emptied between.
        {
            use std::os::unix::fs::FileExt;
            let file = fs::OpenOptions::new()
                .write(true)
                .open(folder.join("warmth"))
                .unwrap();
            file.write_at(b"1\n", 0).unwrap();
        }
        assert!(
            written
                .wait(Duration::from_secs(5))
                .contains(&"warmth".to_owned()),
            "a file written over in place is heard as it's closed"
        );
        // A folder made later is watched from then, and said found.
        fs::create_dir_all(&later).unwrap();
        assert_eq!(written.wait(Duration::ZERO), [String::new()]);
        fs::write(later.join("learned"), "28 0.9\n").unwrap();
        assert!(
            written
                .wait(Duration::from_secs(5))
                .contains(&"learned".to_owned()),
            "a file in a folder made later is heard"
        );
        // A folder taken away is said, and watched again once it's back.
        fs::remove_dir_all(&later).unwrap();
        assert!(
            written
                .wait(Duration::from_secs(5))
                .contains(&String::new()),
            "a folder taken away is said"
        );
        fs::create_dir_all(&later).unwrap();
        assert_eq!(written.wait(Duration::ZERO), [String::new()]);
        fs::write(later.join("learned"), "30 0.9\n").unwrap();
        assert!(
            written
                .wait(Duration::from_secs(5))
                .contains(&"learned".to_owned()),
            "a file in a folder made again is heard"
        );
        fs::remove_dir_all(&folder).unwrap();
    }
}

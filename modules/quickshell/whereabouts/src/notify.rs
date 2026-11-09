//! inotify, which tells this daemon when a folder's files change: one
//! descriptor, a watch a folder, and each change's watch and file name.
//! Every folder is watched for what's made, written, moved or removed in
//! it, since the clock's zone is a link replaced by moving a new one over
//! it, and the pin and the clock's move are files replaced the same way.

use std::ffi::CString;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

/// What's made, written, moved in, out or over, or removed.
const CHANGES: u32 = libc::IN_CREATE
    | libc::IN_CLOSE_WRITE
    | libc::IN_MOVED_TO
    | libc::IN_MOVED_FROM
    | libc::IN_DELETE;

/// One inotify descriptor.
#[derive(Debug)]
pub struct Notify {
    fd: OwnedFd,
}

impl Notify {
    pub fn new() -> std::io::Result<Self> {
        // SAFETY: inotify_init1 takes only flags and returns a new
        // descriptor, or -1.
        let fd = unsafe { libc::inotify_init1(libc::IN_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: the descriptor is new and ours alone.
        Ok(Self {
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
        })
    }

    /// Watches a folder, answering its watch's number, or none where it
    /// can't be watched, as a folder that isn't there.
    pub fn watch(&self, folder: &Path) -> Option<i32> {
        let path = CString::new(folder.as_os_str().as_bytes()).ok()?;
        // SAFETY: the path is a valid C string that outlives the call.
        let watch = unsafe { libc::inotify_add_watch(self.fd.as_raw_fd(), path.as_ptr(), CHANGES) };
        (watch >= 0).then_some(watch)
    }

    /// Waits for changes, and answers each one's watch and file name.
    pub fn changes(&self) -> std::io::Result<Vec<(i32, String)>> {
        let mut buffer = [0u8; 16 * 1024];
        // SAFETY: read writes at most the buffer's length into it.
        let read = unsafe {
            libc::read(
                self.fd.as_raw_fd(),
                buffer.as_mut_ptr().cast(),
                buffer.len(),
            )
        };
        let Ok(read) = usize::try_from(read) else {
            return Err(std::io::Error::last_os_error());
        };
        Ok(events(&buffer[..read]))
    }
}

/// The changes inotify wrote, each a `struct inotify_event` and its name.
pub fn events(bytes: &[u8]) -> Vec<(i32, String)> {
    const HEAD: usize = std::mem::size_of::<libc::inotify_event>();
    let mut said = Vec::new();
    let mut at = 0;
    while at + HEAD <= bytes.len() {
        let word = |offset: usize| {
            let mut four = [0u8; 4];
            four.copy_from_slice(&bytes[at + offset..at + offset + 4]);
            four
        };
        let watch = i32::from_ne_bytes(word(0));
        let length = u32::from_ne_bytes(word(12)) as usize;
        let name_at = at + HEAD;
        let Some(name) = bytes.get(name_at..name_at + length) else {
            break;
        };
        let name = name.split(|byte| *byte == 0).next().unwrap_or_default();
        said.push((watch, String::from_utf8_lossy(name).into_owned()));
        at = name_at + length;
    }
    said
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_moved_into_a_watched_folder_is_said() {
        let folder =
            std::env::temp_dir().join(format!("whereabouts-notify-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let notify = Notify::new().unwrap();
        let watch = notify.watch(&folder).unwrap();
        std::fs::write(folder.join("pin.json.new"), "{}").unwrap();
        std::fs::rename(folder.join("pin.json.new"), folder.join("pin.json")).unwrap();
        let mut changes = notify.changes().unwrap();
        while !changes.contains(&(watch, "pin.json".to_owned())) {
            changes = notify.changes().unwrap();
        }
        std::fs::remove_dir_all(&folder).unwrap();
    }

    #[test]
    fn a_folder_that_isnt_there_has_no_watch() {
        let notify = Notify::new().unwrap();
        assert_eq!(notify.watch(Path::new("/nowhere/at/all")), None);
    }
}

//! Hears pictures come into the screenshots' folder and go from it, from
//! the kernel's inotify, so nothing is looked at while nothing changes. A
//! picture counts once it's written whole and closed, or moved in whole.

use std::ffi::{CString, OsStr, c_char};
use std::fs::File;
use std::io::Read;
use std::os::fd::FromRawFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

/// What changed in the folder.
#[derive(Debug, PartialEq, Eq)]
pub enum Change {
    /// A picture written, or moved in.
    Came(PathBuf),
    /// A picture removed, or moved out.
    Went(PathBuf),
}

// inotify's flags and events, Linux's own.
const IN_CLOEXEC: i32 = 0o2_000_000;
const IN_CLOSE_WRITE: u32 = 0x8;
const IN_MOVED_FROM: u32 = 0x40;
const IN_MOVED_TO: u32 = 0x80;
const IN_DELETE: u32 = 0x200;

/// An event's fixed part: its watch, mask, cookie and the name's length.
const HEADER: usize = 16;

unsafe extern "C" {
    fn inotify_init1(flags: i32) -> i32;
    fn inotify_add_watch(fd: i32, path: *const c_char, mask: u32) -> i32;
}

/// Hears `folder` on a thread of its own, sending each change until
/// nothing takes them.
pub fn watch(folder: &Path, changes: Sender<Change>) -> Result<(), String> {
    let name = CString::new(folder.as_os_str().as_bytes())
        .map_err(|_| format!("{} has a nul in it", folder.display()))?;
    // SAFETY: plain calls with no pointers kept; the descriptor is owned
    // by the File made from it, which closes it once.
    let events = unsafe {
        let fd = inotify_init1(IN_CLOEXEC);
        if fd < 0 {
            return Err(format!("inotify: {}", std::io::Error::last_os_error()));
        }
        let events = File::from_raw_fd(fd);
        let mask = IN_CLOSE_WRITE | IN_MOVED_TO | IN_MOVED_FROM | IN_DELETE;
        if inotify_add_watch(fd, name.as_ptr(), mask) < 0 {
            let why = std::io::Error::last_os_error();
            return Err(format!("watching {}: {why}", folder.display()));
        }
        events
    };
    let folder = folder.to_owned();
    std::thread::spawn(move || hear(events, &folder, &changes));
    Ok(())
}

fn hear(mut events: File, folder: &Path, changes: &Sender<Change>) {
    let mut bytes = vec![0; 64 * 1024];
    loop {
        let read = match events.read(&mut bytes) {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        for change in changes_in(&bytes[..read], folder) {
            if changes.send(change).is_err() {
                return;
            }
        }
    }
}

/// The changes in a read of inotify's events, each the event's fixed part
/// followed by its name, padded with nuls.
fn changes_in(bytes: &[u8], folder: &Path) -> Vec<Change> {
    let mut changes = Vec::new();
    let mut at = 0;
    while at + HEADER <= bytes.len() {
        let word = |from: usize| {
            u32::from_ne_bytes([
                bytes[from],
                bytes[from + 1],
                bytes[from + 2],
                bytes[from + 3],
            ])
        };
        let mask = word(at + 4);
        let length = word(at + 12) as usize;
        let end = (at + HEADER + length).min(bytes.len());
        let name = &bytes[at + HEADER..end];
        let name = &name[..name
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(name.len())];
        at = end;
        if name.is_empty() {
            continue;
        }
        let path = folder.join(OsStr::from_bytes(name));
        if mask & (IN_CLOSE_WRITE | IN_MOVED_TO) != 0 {
            changes.push(Change::Came(path));
        } else if mask & (IN_MOVED_FROM | IN_DELETE) != 0 {
            changes.push(Change::Went(path));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(mask: u32, name: &str) -> Vec<u8> {
        let mut padded = name.as_bytes().to_vec();
        padded.resize(name.len().next_multiple_of(4) + 4, 0);
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1_i32.to_ne_bytes());
        bytes.extend_from_slice(&mask.to_ne_bytes());
        bytes.extend_from_slice(&0_u32.to_ne_bytes());
        bytes.extend_from_slice(&u32::try_from(padded.len()).unwrap().to_ne_bytes());
        bytes.extend_from_slice(&padded);
        bytes
    }

    #[test]
    fn a_picture_written_or_moved_in_came_and_one_removed_went() {
        let folder = Path::new("/shots");
        let bytes = [
            event(IN_CLOSE_WRITE, "a.png"),
            event(IN_MOVED_FROM, "b.png"),
            event(IN_MOVED_TO, "c.png"),
            event(IN_DELETE, "d.png"),
        ]
        .concat();
        assert_eq!(
            changes_in(&bytes, folder),
            [
                Change::Came(folder.join("a.png")),
                Change::Went(folder.join("b.png")),
                Change::Came(folder.join("c.png")),
                Change::Went(folder.join("d.png")),
            ]
        );
    }

    #[test]
    fn a_real_folder_is_heard() {
        let folder = tempfile::tempdir().unwrap();
        let (changes, heard) = std::sync::mpsc::channel();
        watch(folder.path(), changes).unwrap();
        std::fs::write(folder.path().join("shot.png"), "x").unwrap();
        std::fs::remove_file(folder.path().join("shot.png")).unwrap();
        let wait = std::time::Duration::from_secs(5);
        assert_eq!(
            heard.recv_timeout(wait).unwrap(),
            Change::Came(folder.path().join("shot.png"))
        );
        assert_eq!(
            heard.recv_timeout(wait).unwrap(),
            Change::Went(folder.path().join("shot.png"))
        );
    }
}

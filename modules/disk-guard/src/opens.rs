//! Watching who opens a build's artifacts, with fanotify. Under noatime
//! a read leaves no trace on disk, so a build read by every test run looks
//! as old as one nobody needs; an inode mark on each `deps` hears every
//! open of what's in it instead, by any process of any account.
//!
//! An account gets fanotify without privilege when events name files by
//! handle (`FAN_REPORT_DFID_NAME`) and marks are on inodes, which is all
//! this asks. Each event names the directory by its file handle and the
//! file by its name, and the opener's pid is reported only when it's the
//! guard itself, whose own opens date nothing.

use std::collections::HashMap;
use std::ffi::{CString, OsStr};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, PoisonError};
use std::time::Duration;

/// What a watch saw.
#[derive(Clone, Debug, PartialEq)]
pub enum Seen {
    Opened(PathBuf),
    /// Events were lost, and anything watched may have been opened.
    Lost,
}

/// A fanotify group hearing opens in the directories it watches.
pub struct Watch {
    fd: OwnedFd,
    /// Each watched directory, by its filesystem's id and file handle, the
    /// way events name it.
    dirs: Mutex<HashMap<Vec<u8>, PathBuf>>,
    me: i32,
}

/// The longest file handle the kernel gives (`MAX_HANDLE_SZ`).
const MAX_HANDLE: usize = 128;

impl Watch {
    pub fn new() -> io::Result<Watch> {
        let flags = libc::FAN_CLASS_NOTIF
            | libc::FAN_CLOEXEC
            | libc::FAN_NONBLOCK
            | libc::FAN_REPORT_DFID_NAME;
        let opened = (libc::O_RDONLY | libc::O_LARGEFILE | libc::O_CLOEXEC) as libc::c_uint;
        // SAFETY: fanotify_init takes two flag words and returns a new fd.
        let fd = unsafe { libc::fanotify_init(flags, opened) };
        if fd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Watch {
            // SAFETY: fd is a new descriptor nothing else owns.
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            dirs: Mutex::new(HashMap::new()),
            // The kernel reports pids as signed, and gives none past i32::MAX.
            me: std::process::id().cast_signed(),
        })
    }

    /// Hears every open of what's in `dir` from now on, until the
    /// directory goes. Watching one again changes nothing.
    pub fn watch(&self, dir: &Path) -> io::Result<()> {
        let path = CString::new(dir.as_os_str().as_bytes())?;
        let key = key(&path)?;
        let mask = libc::FAN_OPEN | libc::FAN_OPEN_EXEC | libc::FAN_EVENT_ON_CHILD;
        let flags = libc::FAN_MARK_ADD | libc::FAN_MARK_ONLYDIR | libc::FAN_MARK_DONT_FOLLOW;
        // SAFETY: the fd is a fanotify group and path a NUL-ended string.
        let marked = unsafe {
            libc::fanotify_mark(
                self.fd.as_raw_fd(),
                flags,
                mask,
                libc::AT_FDCWD,
                path.as_ptr(),
            )
        };
        if marked != 0 {
            return Err(io::Error::last_os_error());
        }
        self.dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key, dir.to_owned());
        Ok(())
    }

    /// What was seen, waiting up to `within` for something.
    pub fn wait(&self, within: Duration) -> io::Result<Vec<Seen>> {
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = i32::try_from(within.as_millis()).unwrap_or(i32::MAX);
        // SAFETY: one pollfd, counted as one.
        if unsafe { libc::poll(&raw mut poll, 1, millis) } < 0 {
            let error = io::Error::last_os_error();
            return if error.kind() == io::ErrorKind::Interrupted {
                Ok(Vec::new())
            } else {
                Err(error)
            };
        }
        let mut seen = Vec::new();
        // Aligned for the event records read out of it.
        let mut buffer = vec![0u64; 8192];
        loop {
            let bytes = buffer.len() * size_of::<u64>();
            // SAFETY: the buffer holds `bytes` writable bytes.
            let read =
                unsafe { libc::read(self.fd.as_raw_fd(), buffer.as_mut_ptr().cast(), bytes) };
            // Negative, the read failed.
            let Ok(read) = usize::try_from(read) else {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Ok(seen);
                }
                return Err(error);
            };
            if read == 0 {
                return Ok(seen);
            }
            // SAFETY: the kernel wrote `read` bytes into the buffer.
            let events = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), read) };
            self.parse(events, &mut seen);
        }
    }

    fn parse(&self, mut events: &[u8], seen: &mut Vec<Seen>) {
        let header = size_of::<libc::fanotify_event_metadata>();
        while events.len() >= header {
            // SAFETY: at least a header's bytes remain; read_unaligned needs
            // no alignment.
            let event: libc::fanotify_event_metadata =
                unsafe { std::ptr::read_unaligned(events.as_ptr().cast()) };
            let length = event.event_len as usize;
            if event.vers != libc::FANOTIFY_METADATA_VERSION
                || length < header
                || length > events.len()
            {
                return;
            }
            if event.mask & libc::FAN_Q_OVERFLOW != 0 {
                seen.push(Seen::Lost);
            } else if event.pid != self.me
                && let Some(path) = self.opened(&events[event.metadata_len as usize..length])
            {
                seen.push(Seen::Opened(path));
            }
            events = &events[length..];
        }
    }

    /// The file an event's directory and name record names, if the
    /// directory is one this watch watches.
    fn opened(&self, mut records: &[u8]) -> Option<PathBuf> {
        while records.len() >= 4 {
            let kind = records[0];
            let length = u16::from_ne_bytes([records[2], records[3]]) as usize;
            if length < 4 || length > records.len() {
                return None;
            }
            let record = &records[..length];
            if kind == libc::FAN_EVENT_INFO_TYPE_DFID_NAME {
                // The header, the filesystem's id, then a file_handle: its
                // length, its type and its bytes, and the name after them.
                let handle_bytes =
                    u32::from_ne_bytes(record.get(12..16)?.try_into().ok()?) as usize;
                let handle_end = 20 + handle_bytes;
                let mut key = record.get(4..12)?.to_vec();
                key.extend_from_slice(record.get(16..handle_end)?);
                let name = record.get(handle_end..)?;
                let name = &name[..name.iter().position(|&byte| byte == 0)?];
                let dirs = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
                return dirs.get(&key).map(|dir| dir.join(OsStr::from_bytes(name)));
            }
            records = &records[length..];
        }
        None
    }
}

/// A directory as fanotify names it: its filesystem's id, then its file
/// handle's type and bytes.
fn key(path: &CString) -> io::Result<Vec<u8>> {
    // SAFETY: statfs fills the struct it's given.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::statfs(path.as_ptr(), &raw mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // A file_handle followed by room for the longest handle, kept aligned.
    let mut room = vec![0u32; (size_of::<libc::file_handle>() + MAX_HANDLE).div_ceil(4)];
    let handle = room.as_mut_ptr().cast::<libc::file_handle>();
    let mut mount = 0;
    let longest = u32::try_from(MAX_HANDLE).map_err(io::Error::other)?;
    // SAFETY: handle points at a file_handle with MAX_HANDLE bytes after it.
    unsafe { (*handle).handle_bytes = longest };
    if unsafe { libc::name_to_handle_at(libc::AT_FDCWD, path.as_ptr(), handle, &raw mut mount, 0) }
        != 0
    {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fsid_t is two ints, read as their bytes.
    let fsid: [u8; 8] = unsafe { std::mem::transmute(stat.f_fsid) };
    let mut key = fsid.to_vec();
    // SAFETY: the kernel filled in the handle's length and type.
    let (bytes, kind) = unsafe { ((*handle).handle_bytes as usize, (*handle).handle_type) };
    key.extend_from_slice(&kind.to_ne_bytes());
    let handle_bytes: Vec<u8> = room.iter().flat_map(|word| word.to_ne_bytes()).collect();
    let header = size_of::<libc::file_handle>();
    key.extend_from_slice(&handle_bytes[header..header + bytes]);
    Ok(key)
}

//! A folder's watch, its listing, and the reading of one file.
//!
//! The folders followed are local state, where inotify hears every
//! writer. A writer either renames a whole file in (`IN_MOVED_TO`) or
//! writes one in place (`IN_CLOSE_WRITE`); a file goes by `IN_DELETE` or
//! `IN_MOVED_FROM`. The watch also says when it missed changes (a full
//! queue) or lost its folder (moved, removed or unmounted), and either
//! sends the reader back to a listing.

use std::collections::BTreeMap;
use std::ffi::{CString, OsStr};
use std::fs;
use std::io::{self, Read};
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::Path;
use std::time::Duration;

/// The largest file read, in bytes. A presence is about a kilobyte; one
/// past this is taken as not there rather than held in memory for every
/// client.
pub const LARGEST_FILE: u64 = 1 << 20;

/// The most files a folder's listing takes. The presence folder holds
/// about 160; past this the rest are left out rather than read.
pub const MOST_FILES: usize = 4096;

/// What the watch heard.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Heard {
    /// The file of this name may have changed, come or gone.
    Named(String),
    /// Changes were missed: list the folder again.
    Missed,
    /// The folder was moved, removed or unmounted: watch it again.
    Lost,
}

/// An inotify watch on one folder.
pub struct Watch {
    fd: OwnedFd,
}

/// What a watch listens for: a file renamed in or out, written, or
/// removed, and the folder itself going.
const MASK: u32 = libc::IN_CLOSE_WRITE
    | libc::IN_MOVED_TO
    | libc::IN_MOVED_FROM
    | libc::IN_DELETE
    | libc::IN_DELETE_SELF
    | libc::IN_MOVE_SELF
    | libc::IN_ONLYDIR;

/// The most a read of the watch takes at once: a few thousand events.
const READ_AT_ONCE: usize = 64 * 1024;

/// The fixed part of an inotify event: wd, mask, cookie and len.
const EVENT_HEADER: usize = 16;

impl Watch {
    /// Watch the folder at `dir`.
    pub fn new(dir: &Path) -> io::Result<Self> {
        // SAFETY: inotify_init1 takes only flags and returns a new
        // descriptor or -1.
        let raw = unsafe { libc::inotify_init1(libc::IN_CLOEXEC | libc::IN_NONBLOCK) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: raw is a descriptor inotify_init1 just made, owned here
        // alone.
        let fd = unsafe { OwnedFd::from_raw_fd(raw) };
        let path = CString::new(dir.as_os_str().as_bytes())
            .map_err(|_| io::Error::other("the folder's path holds a NUL"))?;
        // SAFETY: fd is an inotify descriptor and path a NUL-ended string
        // that outlives the call.
        let wd = unsafe { libc::inotify_add_watch(fd.as_raw_fd(), path.as_ptr(), MASK) };
        if wd < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Watch { fd })
    }

    /// What the watch hears within `within`: nothing when it hears
    /// nothing in that time.
    pub fn hear(&self, within: Duration) -> io::Result<Vec<Heard>> {
        let mut poll = libc::pollfd {
            fd: self.fd.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let millis = i32::try_from(within.as_millis()).unwrap_or(i32::MAX);
        // SAFETY: poll reads and writes the one pollfd given, which
        // outlives the call.
        let ready = unsafe { libc::poll(&raw mut poll, 1, millis) };
        if ready < 0 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                return Ok(Vec::new());
            }
            return Err(error);
        }
        if ready == 0 {
            return Ok(Vec::new());
        }
        let mut heard = Vec::new();
        let mut buffer = vec![0u8; READ_AT_ONCE];
        loop {
            // SAFETY: read writes at most the buffer's length into the
            // buffer, which outlives the call.
            let read = unsafe {
                libc::read(
                    self.fd.as_raw_fd(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                )
            };
            if read > 0 {
                heard.extend(events(&buffer[..read.unsigned_abs()]));
                continue;
            }
            if read == 0 {
                break;
            }
            let error = io::Error::last_os_error();
            match error.kind() {
                io::ErrorKind::WouldBlock => break,
                io::ErrorKind::Interrupted => {}
                _ => return Err(error),
            }
        }
        Ok(heard)
    }
}

/// The events in what one read of an inotify descriptor gave.
fn events(bytes: &[u8]) -> Vec<Heard> {
    let mut heard = Vec::new();
    let mut at = 0;
    while at + EVENT_HEADER <= bytes.len() {
        let word = |offset: usize| {
            u32::from_ne_bytes([
                bytes[at + offset],
                bytes[at + offset + 1],
                bytes[at + offset + 2],
                bytes[at + offset + 3],
            ])
        };
        let mask = word(4);
        let len = word(12) as usize;
        let end = (at + EVENT_HEADER + len).min(bytes.len());
        let name = &bytes[at + EVENT_HEADER..end];
        let name = &name[..name.iter().position(|&b| b == 0).unwrap_or(name.len())];
        if mask & libc::IN_Q_OVERFLOW != 0 {
            heard.push(Heard::Missed);
        } else if mask
            & (libc::IN_DELETE_SELF | libc::IN_MOVE_SELF | libc::IN_IGNORED | libc::IN_UNMOUNT)
            != 0
        {
            heard.push(Heard::Lost);
        } else if !name.is_empty() {
            heard.push(Heard::Named(
                OsStr::from_bytes(name).to_string_lossy().into_owned(),
            ));
        }
        at = end;
    }
    heard
}

/// Whether a file of this name is one a client asked for: its suffix, and
/// not a writer's hidden half-written file.
pub fn wanted(name: &str, suffix: &str) -> bool {
    name.ends_with(suffix) && !name.starts_with('.')
}

/// Every wanted file in the folder, with what each holds.
pub fn list(dir: &Path, suffix: &str) -> io::Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    for entry in fs::read_dir(dir)? {
        let Ok(entry) = entry else { continue };
        let Some(name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        if !wanted(&name, suffix) {
            continue;
        }
        if files.len() >= MOST_FILES {
            break;
        }
        if let Some(text) = read(dir, &name)? {
            files.insert(name, text);
        }
    }
    Ok(files)
}

/// What the named file holds, or nothing when it isn't there, isn't a
/// file, is too large or isn't text.
pub fn read(dir: &Path, name: &str) -> io::Result<Option<String>> {
    let path = dir.join(name);
    let file = match fs::File::open(&path) {
        Ok(file) => file,
        Err(error) if gone(&error) => return Ok(None),
        Err(error) => return Err(error),
    };
    let meta = file.metadata()?;
    if !meta.is_file() || meta.len() > LARGEST_FILE {
        return Ok(None);
    }
    let mut bytes = Vec::with_capacity(usize::try_from(meta.len()).unwrap_or(0));
    file.take(LARGEST_FILE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > LARGEST_FILE {
        return Ok(None);
    }
    Ok(String::from_utf8(bytes).ok())
}

/// Whether a failed open means the file simply isn't there to read.
fn gone(error: &io::Error) -> bool {
    matches!(
        error.raw_os_error(),
        Some(libc::ENOENT | libc::ENOTDIR | libc::ELOOP | libc::EISDIR)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn folder(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("pi-mesh-reader-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn lists_only_the_files_asked_for() {
        let dir = folder("lists");
        fs::write(dir.join("a.json"), "{}").unwrap();
        fs::write(dir.join(".a.json"), "half").unwrap();
        fs::write(dir.join("a.json.tmp"), "half").unwrap();
        fs::write(dir.join("b.txt"), "no").unwrap();
        fs::create_dir(dir.join("c.json")).unwrap();
        fs::write(dir.join("big.json"), vec![b'x'; usize::try_from(LARGEST_FILE).unwrap() + 1]).unwrap();
        fs::write(dir.join("bytes.json"), [0xff, 0xfe]).unwrap();
        let listed = list(&dir, ".json").unwrap();
        assert_eq!(listed.keys().collect::<Vec<_>>(), vec!["a.json"]);
        assert_eq!(listed["a.json"], "{}");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_nothing_of_a_file_not_there() {
        let dir = folder("missing");
        assert_eq!(read(&dir, "a.json").unwrap(), None);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hears_a_file_written_renamed_in_and_removed() {
        let dir = folder("hears");
        let watch = Watch::new(&dir).unwrap();
        fs::write(dir.join("a.json"), "1").unwrap();
        fs::write(dir.join(".b.tmp"), "2").unwrap();
        fs::rename(dir.join(".b.tmp"), dir.join("b.json")).unwrap();
        fs::remove_file(dir.join("a.json")).unwrap();
        let mut heard = Vec::new();
        for _ in 0..10 {
            heard.extend(watch.hear(Duration::from_millis(100)).unwrap());
        }
        for name in ["a.json", "b.json"] {
            assert!(
                heard.contains(&Heard::Named(name.into())),
                "{name} was heard: {heard:?}"
            );
        }
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hears_its_folder_go() {
        let dir = folder("lost");
        let watch = Watch::new(&dir).unwrap();
        fs::rename(&dir, dir.with_extension("moved")).unwrap();
        let heard = watch.hear(Duration::from_secs(1)).unwrap();
        assert!(
            heard.contains(&Heard::Lost),
            "the move was heard: {heard:?}"
        );
        fs::remove_dir_all(dir.with_extension("moved")).unwrap();
    }

    #[test]
    fn hears_nothing_when_nothing_changes() {
        let dir = folder("quiet");
        let watch = Watch::new(&dir).unwrap();
        assert_eq!(watch.hear(Duration::from_millis(50)).unwrap(), Vec::new());
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn reads_overflow_and_named_events_from_their_bytes() {
        let mut bytes = Vec::new();
        let mut event = |mask: u32, name: &[u8]| {
            let padded = name.len().div_ceil(16) * 16;
            bytes.extend(1i32.to_ne_bytes());
            bytes.extend(mask.to_ne_bytes());
            bytes.extend(0u32.to_ne_bytes());
            bytes.extend(u32::try_from(padded).unwrap().to_ne_bytes());
            bytes.extend(name);
            bytes.extend(vec![0u8; padded - name.len()]);
        };
        event(libc::IN_CLOSE_WRITE, b"a.json");
        event(libc::IN_Q_OVERFLOW, b"");
        event(libc::IN_IGNORED, b"");
        assert_eq!(
            events(&bytes),
            vec![Heard::Named("a.json".into()), Heard::Missed, Heard::Lost]
        );
    }
}

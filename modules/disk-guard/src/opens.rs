//! Watching who opens a build's artifacts, with fanotify. Under noatime
//! a read leaves no trace on disk, so a build read by every test run looks
//! as old as one nobody needs; an inode mark on each profile and its `deps`
//! hears every open of what's in it instead, by any process of any
//! account.
//!
//! An account gets fanotify without privilege when events name files by
//! handle (`FAN_REPORT_DFID_NAME`) and marks are on inodes, which is all
//! this asks. Each event names the directory by its file handle and the
//! file by its name, and the opener's pid is reported only when it's the
//! guard itself, whose own opens date nothing.
//!
//! A group that names files by handle marks folders of one filesystem id
//! alone, and the kernel refuses a mark under another with EXDEV. Each
//! Btrfs subvolume has an id of its own, and every work tree is one, so a
//! watch keeps a group for each id it marks under, and gives one back once
//! none of the folders it watched under that id is there any more.

use std::collections::{HashMap, HashSet};
use std::ffi::{CString, OsStr};
use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use std::time::Duration;

/// What a watch saw.
#[derive(Clone, Debug, PartialEq)]
pub enum Seen {
    Opened(PathBuf),
    /// Events were lost, and anything watched may have been opened.
    Lost,
}

/// Fanotify groups hearing opens in the directories they watch.
pub struct Watch {
    /// A group for each filesystem id marked under, by that id, and one
    /// made before any mark, which the first id takes. A group given back
    /// closes once [`Watch::wait`] no longer holds it, which polls them
    /// unlocked.
    groups: Mutex<Vec<Arc<Group>>>,
    /// Each watched directory, by its filesystem's id and file handle, the
    /// way events name it.
    dirs: Mutex<HashMap<Vec<u8>, PathBuf>>,
    me: i32,
}

/// One fanotify group, and the filesystem id its marks are under, once it
/// has one.
struct Group {
    fsid: OnceLock<[u8; 8]>,
    fd: OwnedFd,
}

impl Group {
    fn new(fsid: Option<[u8; 8]>) -> io::Result<Group> {
        Ok(Group {
            fsid: fsid.map_or_else(OnceLock::new, OnceLock::from),
            fd: group()?,
        })
    }
}

/// The longest file handle the kernel gives (`MAX_HANDLE_SZ`).
const MAX_HANDLE: usize = 128;

/// The most groups a watch makes: half of the 128 an account may hold by
/// default, so its other programs keep room for theirs.
pub const MOST_GROUPS: usize = 64;

/// A new fanotify group that names files by handle and never blocks.
fn group() -> io::Result<OwnedFd> {
    let flags =
        libc::FAN_CLASS_NOTIF | libc::FAN_CLOEXEC | libc::FAN_NONBLOCK | libc::FAN_REPORT_DFID_NAME;
    let opened = (libc::O_RDONLY | libc::O_LARGEFILE | libc::O_CLOEXEC) as libc::c_uint;
    // SAFETY: fanotify_init takes two flag words and returns a new fd.
    let fd = unsafe { libc::fanotify_init(flags, opened) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd is a new descriptor nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Marks `folder` in `group`, to hear every open of what's in it.
fn mark(group: &OwnedFd, folder: &OwnedFd) -> io::Result<()> {
    let mask = libc::FAN_OPEN | libc::FAN_OPEN_EXEC | libc::FAN_EVENT_ON_CHILD;
    let flags = libc::FAN_MARK_ADD | libc::FAN_MARK_ONLYDIR | libc::FAN_MARK_DONT_FOLLOW;
    // SAFETY: the fd is a fanotify group, and "." resolved from the
    // descriptor marks the directory it holds. A null path would ask the
    // kernel to take the descriptor as a file, which it refuses for one
    // opened only as a path.
    let marked = unsafe {
        libc::fanotify_mark(
            group.as_raw_fd(),
            flags,
            mask,
            folder.as_raw_fd(),
            c".".as_ptr(),
        )
    };
    if marked != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

impl Watch {
    /// A watch with one group made, so a kernel that refuses this account
    /// fanotify is known at once.
    pub fn new() -> io::Result<Watch> {
        Ok(Watch {
            groups: Mutex::new(vec![Arc::new(Group::new(None)?)]),
            dirs: Mutex::new(HashMap::new()),
            // The kernel reports pids as signed, and gives none past i32::MAX.
            me: std::process::id().cast_signed(),
        })
    }

    /// Hears every open of what's in `dir` from now on, until the
    /// directory goes. Watching one again changes nothing. Answers the
    /// [`handle`] of the folder the mark landed on: both are read through
    /// one descriptor of it, so a folder swapped in at the path between
    /// them can't pass as the one marked. A folder under a filesystem id
    /// no group marks yet gets a group of its own, up to [`MOST_GROUPS`],
    /// once the groups of ids whose folders have all gone are given back.
    pub fn watch(&self, dir: &Path) -> io::Result<Vec<u8>> {
        let folder = folder(dir)?;
        let key = key(&folder)?;
        let fsid: [u8; 8] = key[..8].try_into().map_err(io::Error::other)?;
        let mut groups = self.groups.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(found) = groups.iter().find(|group| group.fsid.get() == Some(&fsid)) {
            mark(&found.fd, &folder)?;
        } else {
            self.give_back(&mut groups, |dir, key| {
                handle(dir).is_ok_and(|now| now == key)
            });
            if let Some(unused) = groups.iter().find(|group| group.fsid.get().is_none()) {
                mark(&unused.fd, &folder)?;
                // Under the groups' lock, so no other id took it meanwhile.
                let _ = unused.fsid.set(fsid);
            } else if groups.len() >= MOST_GROUPS {
                return Err(io::Error::other(format!(
                    "its subvolume would need a fanotify group past the {MOST_GROUPS} a watch keeps"
                )));
            } else {
                let made = Group::new(Some(fsid))?;
                mark(&made.fd, &folder)?;
                groups.push(Arc::new(made));
            }
        }
        drop(groups);
        self.dirs
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(key.clone(), dir.to_owned());
        Ok(key)
    }

    /// Forgets each watched folder that `still` says is no longer the one
    /// marked, by its path and key, and gives back every group under an id
    /// none of the folders left lies under. Closing a group drops its
    /// marks, which hold their inodes. A watch that kept every group it
    /// made would have work trees removed hours before still take a group
    /// each, and run out of its 64 within a daemon's life.
    fn give_back(&self, groups: &mut Vec<Arc<Group>>, still: impl Fn(&Path, &[u8]) -> bool) {
        let mut dirs = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
        dirs.retain(|key, dir| still(dir, key));
        let live: HashSet<&[u8]> = dirs.keys().filter_map(|key| key.get(..8)).collect();
        groups.retain(|group| group.fsid.get().is_none_or(|fsid| live.contains(&fsid[..])));
    }

    /// How many fanotify groups the watch holds.
    pub fn groups(&self) -> usize {
        self.groups
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }

    /// What was seen in any group, waiting up to `within` for something.
    /// A group made meanwhile is heard from the next wait on.
    pub fn wait(&self, within: Duration) -> io::Result<Vec<Seen>> {
        // Held to the end of the wait, so a group given back meanwhile
        // closes only once nothing here polls or reads it.
        let held: Vec<Arc<Group>> = self
            .groups
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone();
        let mut polls: Vec<libc::pollfd> = held
            .iter()
            .map(|group| libc::pollfd {
                fd: group.fd.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            })
            .collect();
        let millis = i32::try_from(within.as_millis()).unwrap_or(i32::MAX);
        let count = libc::nfds_t::try_from(polls.len()).map_err(io::Error::other)?;
        // SAFETY: the pollfds are counted as they are, and each descriptor
        // stays open while `held` keeps its group.
        if unsafe { libc::poll(polls.as_mut_ptr(), count, millis) } < 0 {
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
        for poll in polls.iter().filter(|poll| poll.revents != 0) {
            self.read_out(poll.fd, &mut buffer, &mut seen)?;
        }
        drop(held);
        Ok(seen)
    }

    /// Reads everything group `fd` holds into `seen`, through `buffer`.
    fn read_out(&self, fd: i32, buffer: &mut [u64], seen: &mut Vec<Seen>) -> io::Result<()> {
        loop {
            let bytes = size_of_val(buffer);
            // SAFETY: the buffer holds `bytes` writable bytes.
            let read = unsafe { libc::read(fd, buffer.as_mut_ptr().cast(), bytes) };
            // Negative, the read failed.
            let Ok(read) = usize::try_from(read) else {
                let error = io::Error::last_os_error();
                if error.kind() == io::ErrorKind::WouldBlock {
                    return Ok(());
                }
                return Err(error);
            };
            if read == 0 {
                return Ok(());
            }
            // SAFETY: the kernel wrote `read` bytes into the buffer.
            let events = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), read) };
            self.parse(events, seen);
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
                || (event.metadata_len as usize) < header
                || event.metadata_len as usize > length
            {
                // The rest of the read can't be told apart, and could hold
                // opens of anything marked.
                seen.push(Seen::Lost);
                return;
            }
            if event.mask & libc::FAN_Q_OVERFLOW != 0 {
                seen.push(Seen::Lost);
            } else if event.pid != self.me {
                match self.opened(&events[event.metadata_len as usize..length]) {
                    Ok(Some(path)) => seen.push(Seen::Opened(path)),
                    Ok(None) => {}
                    Err(Unreadable) => seen.push(Seen::Lost),
                }
            }
            events = &events[length..];
        }
    }

    /// The file an event's directory and name record names: nothing when
    /// its directory isn't one this watch watches, and [`Unreadable`] for
    /// a record that can't be read, which could have named anything.
    fn opened(&self, mut records: &[u8]) -> Result<Option<PathBuf>, Unreadable> {
        while records.len() >= 4 {
            let kind = records[0];
            let length = u16::from_ne_bytes([records[2], records[3]]) as usize;
            if length < 4 || length > records.len() {
                return Err(Unreadable);
            }
            let record = &records[..length];
            if kind == libc::FAN_EVENT_INFO_TYPE_DFID_NAME {
                let (key, name) = dfid_name(record).ok_or(Unreadable)?;
                let dirs = self.dirs.lock().unwrap_or_else(PoisonError::into_inner);
                return Ok(dirs.get(&key).map(|dir| dir.join(OsStr::from_bytes(name))));
            }
            records = &records[length..];
        }
        Ok(None)
    }
}

/// An event's record that can't be read.
struct Unreadable;

/// A directory and name record's key, as [`key`] makes it, and its name.
fn dfid_name(record: &[u8]) -> Option<(Vec<u8>, &[u8])> {
    // The header, the filesystem's id, then a file_handle: its length, its
    // type and its bytes, and the name after them.
    let handle_bytes = u32::from_ne_bytes(record.get(12..16)?.try_into().ok()?) as usize;
    let handle_end = 20 + handle_bytes;
    let mut key = record.get(4..12)?.to_vec();
    key.extend_from_slice(record.get(16..handle_end)?);
    let name = record.get(handle_end..)?;
    Some((key, &name[..name.iter().position(|&byte| byte == 0)?]))
}

/// Which folder `dir` is now, by its filesystem's id and file handle: the
/// way a mark names it. The handle carries the inode's generation, so a
/// folder made anew where one was removed differs even when its inode
/// number is reused.
pub fn handle(dir: &Path) -> io::Result<Vec<u8>> {
    key(&folder(dir)?)
}

/// The directory at `dir` itself, never through a symlink, held by a
/// descriptor that reads nothing, so everything asked of it is asked of
/// that one folder.
fn folder(dir: &Path) -> io::Result<OwnedFd> {
    let path = CString::new(dir.as_os_str().as_bytes())?;
    let flags = libc::O_PATH | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC;
    // SAFETY: path is a NUL-ended string, and open returns a new fd.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd is a new descriptor nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// A directory as fanotify names it: its filesystem's id, then its file
/// handle's type and bytes.
fn key(folder: &OwnedFd) -> io::Result<Vec<u8>> {
    // SAFETY: fstatfs fills the struct it's given.
    let mut stat: libc::statfs = unsafe { std::mem::zeroed() };
    if unsafe { libc::fstatfs(folder.as_raw_fd(), &raw mut stat) } != 0 {
        return Err(io::Error::last_os_error());
    }
    // A file_handle followed by room for the longest handle, kept aligned.
    let mut room = vec![0u32; (size_of::<libc::file_handle>() + MAX_HANDLE).div_ceil(4)];
    let handle = room.as_mut_ptr().cast::<libc::file_handle>();
    let mut mount = 0;
    let longest = u32::try_from(MAX_HANDLE).map_err(io::Error::other)?;
    // SAFETY: handle points at a file_handle with MAX_HANDLE bytes after it.
    unsafe { (*handle).handle_bytes = longest };
    // SAFETY: an empty path with AT_EMPTY_PATH names the descriptor's own
    // folder.
    let named = unsafe {
        libc::name_to_handle_at(
            folder.as_raw_fd(),
            c"".as_ptr(),
            handle,
            &raw mut mount,
            libc::AT_EMPTY_PATH,
        )
    };
    if named != 0 {
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

// The kernel never hands a watch a malformed event to test with, so these
// cases build the bytes themselves and read them as a watch would.
#[cfg(test)]
mod tests {
    use super::{Seen, Watch};
    use std::collections::HashMap;
    use std::os::fd::OwnedFd;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex, OnceLock, PoisonError};

    const HANDLE: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];

    /// A group under `fsid` that holds `/dev/null` rather than fanotify.
    fn stand_in(fsid: Option<[u8; 8]>) -> super::Group {
        super::Group {
            fsid: fsid.map_or_else(OnceLock::new, OnceLock::from),
            fd: OwnedFd::from(std::fs::File::open("/dev/null").unwrap()),
        }
    }

    #[test]
    fn a_group_goes_back_once_every_folder_under_its_id_has_gone() {
        let watch = watch();
        let kept = 7_u64.to_ne_bytes();
        let went = 9_u64.to_ne_bytes();
        let mut key = went.to_vec();
        key.extend_from_slice(&HANDLE);
        watch
            .dirs
            .lock()
            .unwrap()
            .insert(key, PathBuf::from("/gone"));
        let mut groups = vec![
            Arc::new(stand_in(None)),
            Arc::new(stand_in(Some(kept))),
            Arc::new(stand_in(Some(went))),
        ];
        watch.give_back(&mut groups, |dir, _| dir != Path::new("/gone"));
        let ids: Vec<_> = groups
            .iter()
            .map(|group| group.fsid.get().copied())
            .collect();
        assert_eq!(
            ids,
            [None, Some(kept)],
            "a group was kept with every folder under its id gone, or given back with one left"
        );
        let dirs = watch.dirs.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            dirs.values().collect::<Vec<_>>(),
            [&PathBuf::from("/w")],
            "a folder gone since it was marked was still watched"
        );
    }

    /// A watch over `/w`, by [`HANDLE`], that reads what it's given and
    /// holds no fanotify group.
    fn watch() -> Watch {
        let mut key = 7_u64.to_ne_bytes().to_vec();
        key.extend_from_slice(&1_i32.to_ne_bytes());
        key.extend_from_slice(&HANDLE);
        Watch {
            groups: Mutex::new(vec![Arc::new(stand_in(None))]),
            dirs: Mutex::new(HashMap::from([(key, PathBuf::from("/w"))])),
            me: 0,
        }
    }

    /// A directory and name record for [`HANDLE`], of type 1 on
    /// filesystem 7, naming `lib.rlib`, as `alter` leaves it before its
    /// length is filled in.
    fn record(alter: impl FnOnce(&mut Vec<u8>)) -> Vec<u8> {
        let mut record = vec![libc::FAN_EVENT_INFO_TYPE_DFID_NAME, 0, 0, 0];
        record.extend_from_slice(&7_u64.to_ne_bytes());
        record.extend_from_slice(&u32::try_from(HANDLE.len()).unwrap().to_ne_bytes());
        record.extend_from_slice(&1_i32.to_ne_bytes());
        record.extend_from_slice(&HANDLE);
        record.extend_from_slice(b"lib.rlib\0");
        alter(&mut record);
        let length = u16::try_from(record.len()).unwrap();
        record[2..4].copy_from_slice(&length.to_ne_bytes());
        record
    }

    /// What a watch reads of an open by another process carrying
    /// `records`, in an event of version `vers`.
    fn read(vers: u8, records: &[u8]) -> Vec<Seen> {
        parsed(&read_bytes(vers, records))
    }

    /// What a watch reads of `events`.
    fn parsed(events: &[u8]) -> Vec<Seen> {
        let mut seen = Vec::new();
        watch().parse(events, &mut seen);
        seen
    }

    /// The bytes of an open by another process carrying `records`, in an
    /// event of version `vers`.
    fn read_bytes(vers: u8, records: &[u8]) -> Vec<u8> {
        let header = size_of::<libc::fanotify_event_metadata>();
        let metadata = libc::fanotify_event_metadata {
            event_len: u32::try_from(header + records.len()).unwrap(),
            vers,
            reserved: 0,
            metadata_len: u16::try_from(header).unwrap(),
            mask: libc::FAN_OPEN,
            fd: -1,
            pid: 42,
        };
        // SAFETY: the struct is plain data with no padding, read as its
        // bytes.
        let bytes = unsafe { std::slice::from_raw_parts((&raw const metadata).cast(), header) };
        let mut events = bytes.to_vec();
        events.extend_from_slice(records);
        events
    }

    #[test]
    fn an_event_that_cant_be_read_counts_as_lost() {
        let version = libc::FANOTIFY_METADATA_VERSION;
        assert_eq!(
            read(version, &record(|_| {})),
            [Seen::Opened(PathBuf::from("/w/lib.rlib"))],
            "a well-formed record wasn't read as the open it names"
        );
        assert_eq!(
            read(version + 1, &record(|_| {})),
            [Seen::Lost],
            "an event of another version was read as if it were this one"
        );
        let header = u16::try_from(size_of::<libc::fanotify_event_metadata>()).unwrap();
        let mut events = read_bytes(version, &record(|_| {}));
        events[6..8].copy_from_slice(&(header + 200).to_ne_bytes());
        assert_eq!(
            parsed(&events),
            [Seen::Lost],
            "an event whose header runs past it was passed over"
        );
        let mut short = record(|_| {});
        short[2..4].copy_from_slice(&2_u16.to_ne_bytes());
        assert_eq!(
            read(version, &short),
            [Seen::Lost],
            "a record shorter than its own header was passed over"
        );
        let unended = record(|record| {
            record.pop();
        });
        assert_eq!(
            read(version, &unended),
            [Seen::Lost],
            "a record whose name never ends was passed over"
        );
        let overlong = record(|record| record[12..16].copy_from_slice(&200_u32.to_ne_bytes()));
        assert_eq!(
            read(version, &overlong),
            [Seen::Lost],
            "a record whose handle runs past it was passed over"
        );
    }
}

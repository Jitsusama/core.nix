//! How much of the disk files take, as Btrfs keeps them. A compressed
//! file's size, and the blocks `stat` gives it, are what it holds once
//! read, not what it takes: Go's build cache on a zstd disk takes a third
//! of its size or less. Only the extent items in the filesystem's tree say
//! what each run of a file takes on the disk, and reading them asks for
//! root, so the machine's guard reads them for an account's.
//!
//! Each run counts once however many files or parts of files name it, so a
//! file copied by reference doesn't count twice. A run shared with a file
//! outside the paths asked about still counts, so what clearing frees can
//! be less where a tree shares its files with the one it was cloned from.

use std::collections::HashSet;
use std::path::Path;

/// The type of a file extent item in a Btrfs tree.
const EXTENT_DATA: u32 = 108;
/// A search result's header: transid, objectid and offset, each a u64,
/// then the item's type and length, each a u32.
const HEADER: usize = 32;
/// A file extent item's fields before its inline data or its disk run.
const ITEM_HEAD: usize = 21;
/// An item's kind, at this byte of it: inline, a run or a preallocated run.
const KIND_AT: usize = 20;
const INLINE: u8 = 0;
/// How many entries one footprint may walk, so a request for a huge tree
/// can't hold the machine's guard for long.
pub const WALK_BUDGET: usize = 2_000_000;

/// What one file extent item takes on the disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// Data kept in the tree itself, this many bytes of it.
    Inline(u64),
    /// A run of the disk, `bytes` long from `at`, which every item naming
    /// it shares.
    Run { at: u64, bytes: u64 },
    /// A hole, which takes nothing.
    Hole,
}

/// The extent items in a search's result buffer, which holds `items` of
/// them, each with the file offset it starts at. A buffer cut short ends
/// the list where it's cut.
pub fn extents(buffer: &[u8], items: u32) -> Vec<(u64, Extent)> {
    let u64_at = |at: usize| -> Option<u64> {
        buffer
            .get(at..at + 8)
            .map(|bytes| u64::from_ne_bytes(bytes.try_into().unwrap_or_default()))
    };
    let u32_at = |at: usize| -> Option<u32> {
        buffer
            .get(at..at + 4)
            .map(|bytes| u32::from_ne_bytes(bytes.try_into().unwrap_or_default()))
    };
    let mut found = Vec::new();
    let mut at = 0;
    for _ in 0..items {
        let (Some(offset), Some(kind), Some(length)) =
            (u64_at(at + 16), u32_at(at + 24), u32_at(at + 28))
        else {
            break;
        };
        let length = length as usize;
        let item = at + HEADER;
        if item + length > buffer.len() {
            break;
        }
        if kind == EXTENT_DATA && length > KIND_AT {
            let extent = if buffer[item + KIND_AT] == INLINE {
                Extent::Inline(length.saturating_sub(ITEM_HEAD) as u64)
            } else {
                match (u64_at(item + ITEM_HEAD), u64_at(item + ITEM_HEAD + 8)) {
                    (Some(0), _) => Extent::Hole,
                    (Some(start), Some(bytes)) => Extent::Run { at: start, bytes },
                    _ => break,
                }
            };
            found.push((offset, extent));
        }
        at = item + length;
    }
    found
}

/// What the extents found so far take on the disk, counting each run once.
#[derive(Debug, Default)]
pub struct Tally {
    runs: HashSet<u64>,
    pub disk: u64,
}

impl Tally {
    pub fn add(&mut self, extent: Extent) {
        match extent {
            Extent::Inline(bytes) => self.disk += bytes,
            Extent::Run { at, bytes } => {
                if self.runs.insert(at) {
                    self.disk += bytes;
                }
            }
            Extent::Hole => {}
        }
    }
}

/// The bytes on the disk what lies under `path` takes, which has to be
/// a path with no symlink on it, as `canonicalize` gives: never through a
/// symlink or into another filesystem or subvolume. None for a path that
/// isn't on Btrfs, is gone, holds more than the walk's budget, or whose
/// extents can't be read, as for anyone but root.
pub fn footprint(path: &Path) -> Option<u64> {
    search::footprint(path).ok()
}

/// What the files under `path` hold once read, which anyone who can read
/// them can learn, walked as a footprint is.
pub fn files(path: &Path) -> u64 {
    fn add(path: &Path, device: Option<u64>, seen: &mut usize) -> u64 {
        use std::os::unix::fs::MetadataExt;
        *seen += 1;
        let Ok(meta) = std::fs::symlink_metadata(path) else {
            return 0;
        };
        if *seen > WALK_BUDGET || device.is_some_and(|device| device != meta.dev()) {
            return 0;
        }
        if meta.is_dir() {
            std::fs::read_dir(path).map_or(0, |entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|entry| add(&entry.path(), Some(meta.dev()), seen))
                    .sum()
            })
        } else if meta.is_file() {
            meta.len()
        } else {
            0
        }
    }
    add(path, None, &mut 0)
}

#[cfg(target_os = "linux")]
mod search {
    use super::{EXTENT_DATA, Extent, Tally, WALK_BUDGET, extents};
    use crate::anchor::Dir;
    use rustix::fs::{AtFlags, FileType};
    use std::io;
    use std::os::fd::{AsRawFd, OwnedFd};
    use std::path::Path;

    /// BTRFS_IOC_TREE_SEARCH_V2: _IOWR(0x94, 17, the 112-byte header).
    const TREE_SEARCH_V2: libc::c_ulong = 0xC070_9411;
    /// The search key's fields, then the buffer's size, before the buffer.
    const KEY: usize = 104;
    const ARGS: usize = KEY + 8;
    const BUFFER: usize = 64 << 10;
    /// Where the search key keeps how many items to give, and then gave.
    const ITEMS_AT: usize = 64;

    pub fn footprint(path: &Path) -> io::Result<u64> {
        if !rustix::fs::statfs(path).is_ok_and(|stats| crate::linux::is_btrfs(&stats)) {
            return Err(io::Error::from(io::ErrorKind::Unsupported));
        }
        let mut tally = Tally::default();
        let mut add = |dir: &OwnedFd, inode: u64| -> io::Result<()> {
            for extent in file(dir, inode)? {
                tally.add(extent);
            }
            Ok(())
        };
        match Dir::open_unlinked(path) {
            Ok(dir) => {
                let mut budget = WALK_BUDGET;
                dir.each_file(&mut budget, &mut add)?;
            }
            Err(error) if error.raw_os_error() == Some(libc::ENOTDIR) => {
                // One file, searched through the folder it lies in.
                let (Some(parent), Some(name)) = (path.parent(), path.file_name()) else {
                    return Err(error);
                };
                let parent = Dir::open_unlinked(parent)?;
                let fd = parent.fd();
                let stat = rustix::fs::statat(fd, name, AtFlags::SYMLINK_NOFOLLOW)?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::RegularFile {
                    return Err(error);
                }
                add(fd, stat.st_ino)?;
            }
            Err(error) => return Err(error),
        }
        Ok(tally.disk)
    }

    /// Every extent item of the file with inode `inode` in the subvolume
    /// `dir` lies in, the folder it was found in.
    fn file(dir: &OwnedFd, inode: u64) -> io::Result<Vec<Extent>> {
        let mut found = Vec::new();
        let mut from = 0u64;
        loop {
            let mut args = vec![0u8; ARGS + BUFFER];
            let mut put = |at: usize, value: u64| {
                args[at..at + 8].copy_from_slice(&value.to_ne_bytes());
            };
            // Tree 0 is the subvolume the folder lies in.
            put(8, inode);
            put(16, inode);
            put(24, from);
            put(32, u64::MAX);
            put(48, u64::MAX);
            put(KEY, BUFFER as u64);
            args[56..60].copy_from_slice(&EXTENT_DATA.to_ne_bytes());
            args[60..64].copy_from_slice(&EXTENT_DATA.to_ne_bytes());
            args[ITEMS_AT..ITEMS_AT + 4].copy_from_slice(&u32::MAX.to_ne_bytes());
            // SAFETY: args is the header and buffer the ioctl reads and
            // writes, sized as its buffer-size field says.
            let done = unsafe { libc::ioctl(dir.as_raw_fd(), TREE_SEARCH_V2, args.as_mut_ptr()) };
            if done < 0 {
                return Err(io::Error::last_os_error());
            }
            let items =
                u32::from_ne_bytes(args[ITEMS_AT..ITEMS_AT + 4].try_into().unwrap_or_default());
            let listed = extents(&args[ARGS..], items);
            let Some(&(last, _)) = listed.last() else {
                return Ok(found);
            };
            found.extend(listed.into_iter().map(|(_, extent)| extent));
            match last.checked_add(1) {
                Some(next) => from = next,
                None => return Ok(found),
            }
        }
    }
}

#[cfg(not(target_os = "linux"))]
mod search {
    use std::io;
    use std::path::Path;

    /// Only Btrfs keeps extents the guard can read, and only on Linux.
    pub fn footprint(_path: &Path) -> io::Result<u64> {
        Err(io::Error::from(io::ErrorKind::Unsupported))
    }
}

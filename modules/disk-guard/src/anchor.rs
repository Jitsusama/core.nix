//! Directories held open, so nothing beneath a root is reached any other way.
//!
//! A root comes from the config. Everything beneath it is opened relative to
//! an open directory, refusing a symlink, a `..` or another filesystem
//! anywhere on the way, so nothing swapped in between a check and a delete
//! can send the guard somewhere else. A Btrfs subvolume reads as another
//! device, so it's never crossed either.

use rustix::fs::{AtFlags, FileType, Mode, OFlags, ResolveFlags};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::Path;

pub struct Dir {
    fd: OwnedFd,
    dev: u64,
}

const DIRECTORY: OFlags = OFlags::DIRECTORY
    .union(OFlags::NOFOLLOW)
    .union(OFlags::CLOEXEC)
    .union(OFlags::RDONLY);

impl Dir {
    /// Opens a root the config names. Its own path is trusted, as Nix wrote
    /// it, but it has to be a directory rather than a symlink to one.
    pub fn open(path: &Path) -> io::Result<Dir> {
        let fd = rustix::fs::open(path, DIRECTORY, Mode::empty())?;
        Dir::from_fd(fd)
    }

    fn from_fd(fd: OwnedFd) -> io::Result<Dir> {
        let stat = rustix::fs::fstat(&fd)?;
        Ok(Dir {
            fd,
            dev: stat.st_dev,
        })
    }

    /// Opens a directory beneath this one.
    pub fn child(&self, relative: &Path) -> io::Result<Dir> {
        if relative.is_absolute() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not beneath"));
        }
        let resolve = ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV;
        let fd = rustix::fs::openat2(&self.fd, relative, DIRECTORY, Mode::empty(), resolve)?;
        let child = Dir::from_fd(fd)?;
        if child.dev != self.dev {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "another filesystem",
            ));
        }
        Ok(child)
    }

    /// Opens one entry of this directory, if it's a directory on the same
    /// filesystem.
    fn entry(&self, name: &OsStr) -> io::Result<Option<Dir>> {
        let fd = rustix::fs::openat(&self.fd, name, DIRECTORY, Mode::empty())?;
        let child = Dir::from_fd(fd)?;
        Ok((child.dev == self.dev).then_some(child))
    }

    /// The names in this directory, without `.` and `..`.
    fn names(&self) -> io::Result<Vec<Vec<u8>>> {
        let mut names = Vec::new();
        for entry in rustix::fs::Dir::read_from(&self.fd)? {
            let name = entry?.file_name().to_bytes().to_vec();
            if name != b"." && name != b".." {
                names.push(name);
            }
        }
        Ok(names)
    }

    /// The bytes allocated to everything beneath this directory, on its own
    /// filesystem, with each file counted once however many links it has.
    /// What vanishes or can't be read during the walk, as a running build
    /// replaces its files, is left out rather than failing the count.
    pub fn size(&self) -> io::Result<u64> {
        let mut seen = HashSet::new();
        self.size_into(&mut seen)
    }

    fn size_into(&self, seen: &mut HashSet<u64>) -> io::Result<u64> {
        let mut total = 0;
        for name in self.names()? {
            let name = OsStr::from_bytes(&name);
            let Ok(stat) = rustix::fs::statat(&self.fd, name, AtFlags::SYMLINK_NOFOLLOW) else {
                continue;
            };
            if stat.st_dev != self.dev {
                continue;
            }
            let kind = FileType::from_raw_mode(stat.st_mode);
            if stat.st_nlink > 1 && kind != FileType::Directory && !seen.insert(stat.st_ino) {
                continue;
            }
            total += stat.st_blocks as u64 * 512;
            if kind == FileType::Directory
                && let Ok(Some(child)) = self.entry(name)
            {
                total += child.size_into(seen).unwrap_or(0);
            }
        }
        Ok(total)
    }
}

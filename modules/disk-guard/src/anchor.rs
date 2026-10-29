//! Directories held open, so nothing beneath a root is reached any other way.
//!
//! A root comes from the config. Everything beneath it is opened relative to
//! an open directory, refusing a symlink, a `..` or another filesystem
//! anywhere on the way, so nothing swapped in between a check and a delete
//! can send the guard somewhere else. A Btrfs subvolume reads as another
//! device too. One reached through the same mount and writable, as the work
//! broker clones each tree, is opened on the way down to what's reclaimed,
//! since the walk that found it went in; emptying never enters one, so one
//! inside what's emptied is left whole.

#[cfg(target_os = "linux")]
use rustix::fs::ResolveFlags;
use rustix::fs::{AtFlags, Dev, FileType, Mode, OFlags, RawMode, RenameFlags};
use rustix::io::Errno;
use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::io;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Component, Path};

pub struct Dir {
    fd: OwnedFd,
    dev: Dev,
}

/// What a detach moved aside: a trash directory inside the directory it
/// came from, so the move is a rename on one filesystem and nothing that
/// opens the directory's old entries by name finds anything half gone.
pub struct Detached<'a> {
    dir: &'a Dir,
    name: OsString,
    /// Directories whose owner was given write permission so they could
    /// move, with the modes they had, for a restore to give back.
    modes: Vec<(OsString, RawMode)>,
}

/// The name every trash starts with, so one left by a crash is found and
/// emptied, and never itself detached into another.
const TRASH: &[u8] = b".disk-guard-trash-";

/// Deeper than any build tree goes, and short of running out of open files.
const DEEPEST: usize = 256;

/// The smallest file whose extents are asked after when sizing: what's
/// shared between builds is mostly libraries and binaries of megabytes,
/// and mapping every small file would cost a call each.
const SHARED_FROM: u64 = 1 << 20;

/// What a size has counted already: files with several links, by device
/// and inode, and shared extents, by where they lie on the disk.
#[derive(Default)]
pub struct Counted {
    inodes: HashSet<(Dev, u64)>,
    extents: HashSet<u64>,
}

impl Counted {
    /// How many of a file's bytes lie in shared extents counted already,
    /// counting those that aren't yet.
    fn again(&mut self, file: &OwnedFd) -> u64 {
        crate::pool::shared_extents(file)
            .into_iter()
            .filter(|&(physical, _)| !self.extents.insert(physical))
            .map(|(_, length)| length)
            .sum()
    }
}

impl Detached<'_> {
    pub fn name(&self) -> &OsStr {
        &self.name
    }

    /// Puts every entry back where it was, refusing to overwrite anything
    /// that has appeared there since. What can't go back stays in the trash,
    /// and the first failure is returned.
    pub fn restore(self) -> io::Result<()> {
        let trash = self.dir.writable(&self.name)?;
        let mut failed = None;
        let mut back = Vec::new();
        for name in trash.names()? {
            let name = OsStr::from_bytes(&name);
            match rename(&trash, name, self.dir, name) {
                Ok(()) => back.push(name.to_owned()),
                Err(error) => {
                    failed.get_or_insert(error);
                }
            }
        }
        // Only what came back gets its mode back: whatever else holds the
        // name now is someone else's.
        for (name, mode) in self.modes.iter().filter(|(name, _)| back.contains(name)) {
            if let Err(error) = self.dir.set_mode(name, *mode) {
                failed.get_or_insert(error);
            }
        }
        match failed {
            Some(error) => Err(error),
            None => Ok(rustix::fs::unlinkat(
                &self.dir.fd,
                &self.name,
                AtFlags::REMOVEDIR,
            )?),
        }
    }

    /// Removes the trash and everything in it. A symlink is removed, never
    /// followed, and a read-only directory is made writable first, since
    /// the guard only ever empties a trash its own account owns.
    pub fn empty(self) -> io::Result<()> {
        self.empty_paced(&mut || {})
    }

    /// Empties the trash as `empty` does, calling `unlinked` after each
    /// entry goes, so a pacer can rest between batches.
    pub fn empty_paced(self, unlinked: &mut dyn FnMut()) -> io::Result<()> {
        remove(self.dir, &self.name, 0, unlinked)
    }
}

fn rename(from: &Dir, name: &OsStr, to: &Dir, new: &OsStr) -> io::Result<()> {
    Ok(rustix::fs::renameat_with(
        &from.fd,
        name,
        &to.fd,
        new,
        RenameFlags::NOREPLACE,
    )?)
}

/// Removes one entry of `parent`, all the way down, never leaving its
/// filesystem. Whatever vanishes on the way was already gone. Each unlink
/// is told to `unlinked` once it's made.
fn remove(parent: &Dir, name: &OsStr, depth: usize, unlinked: &mut dyn FnMut()) -> io::Result<()> {
    let gone = |result: Result<(), Errno>, unlinked: &mut dyn FnMut()| {
        unlinked();
        match result {
            Err(Errno::NOENT) => Ok(()),
            other => other.map_err(io::Error::from),
        }
    };
    let stat = match rustix::fs::statat(&parent.fd, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => return Ok(()),
        other => other?,
    };
    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
        return gone(
            rustix::fs::unlinkat(&parent.fd, name, AtFlags::empty()),
            unlinked,
        );
    }
    if stat.st_dev != parent.dev {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "another filesystem",
        ));
    }
    if depth >= DEEPEST {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "too deep"));
    }
    let child = match parent.writable(name) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        other => other?,
    };
    let mut failed = None;
    for entry in child.names()? {
        if let Err(error) = remove(&child, OsStr::from_bytes(&entry), depth + 1, unlinked) {
            failed.get_or_insert(error);
        }
    }
    drop(child);
    match failed {
        Some(error) => Err(error),
        None => gone(
            rustix::fs::unlinkat(&parent.fd, name, AtFlags::REMOVEDIR),
            unlinked,
        ),
    }
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

    /// Opens a directory an account named, by a path with no symlink on it,
    /// as `canonicalize` gives one: a link swapped in since is refused
    /// rather than followed.
    #[cfg(target_os = "linux")]
    pub fn open_unlinked(path: &Path) -> io::Result<Dir> {
        let resolve = ResolveFlags::NO_SYMLINKS;
        let fd = rustix::fs::openat2(rustix::fs::CWD, path, DIRECTORY, Mode::empty(), resolve)?;
        Dir::from_fd(fd)
    }

    /// The open directory itself, for a call on what lies in it.
    pub fn fd(&self) -> &OwnedFd {
        &self.fd
    }

    /// Calls `each` with this directory, or the one beneath it a file lies
    /// in, and that file's inode, for every regular file beneath it on its
    /// own filesystem, never through a symlink. It gives up once it has
    /// looked at `budget` entries, and on the first error `each` returns.
    pub fn each_file(
        &self,
        budget: &mut usize,
        each: &mut dyn FnMut(&OwnedFd, u64) -> io::Result<()>,
    ) -> io::Result<()> {
        for name in self.names()? {
            *budget = budget
                .checked_sub(1)
                .ok_or_else(|| io::Error::other("too much to walk"))?;
            let name = OsStr::from_bytes(&name);
            let Ok(stat) = rustix::fs::statat(&self.fd, name, AtFlags::SYMLINK_NOFOLLOW) else {
                continue;
            };
            if stat.st_dev != self.dev {
                continue;
            }
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::RegularFile => each(&self.fd, stat.st_ino)?,
                FileType::Directory => {
                    if let Ok(Some(child)) = self.entry(name) {
                        child.each_file(budget, each)?;
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn from_fd(fd: OwnedFd) -> io::Result<Dir> {
        let stat = rustix::fs::fstat(&fd)?;
        Ok(Dir {
            fd,
            dev: stat.st_dev,
        })
    }

    /// Opens a directory beneath this one, or this one again for the empty
    /// path, as the parent of an entry lying directly in it is.
    #[cfg(not(target_os = "linux"))]
    pub fn child(&self, relative: &Path) -> io::Result<Dir> {
        self.child_stepwise(relative)
    }

    /// Opens a directory beneath this one, or this one again for the empty
    /// path, as the parent of an entry lying directly in it is.
    #[cfg(target_os = "linux")]
    pub fn child(&self, relative: &Path) -> io::Result<Dir> {
        if relative.is_absolute() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not beneath"));
        }
        let relative = if relative.as_os_str().is_empty() {
            Path::new(".")
        } else {
            relative
        };
        let resolve = ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS | ResolveFlags::NO_XDEV;
        let fd = rustix::fs::openat2(&self.fd, relative, DIRECTORY, Mode::empty(), resolve)?;
        let child = Dir::from_fd(fd)?;
        // `NO_XDEV` refuses a mount, but not a subvolume on this one.
        if self.foreign(&child) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "another filesystem",
            ));
        }
        Ok(child)
    }

    /// Whether a directory opened beneath this one is on another filesystem:
    /// another device, unless it's a subvolume `pool::beneath` lets through.
    fn foreign(&self, child: &Dir) -> bool {
        child.dev != self.dev && !crate::pool::beneath(&self.fd, &child.fd)
    }

    /// Opens a directory beneath this one a component at a time, each one
    /// without following a symlink and checked to be on the filesystem of
    /// the one before, refusing any `..`. It's how `child` resolves where there's no
    /// `openat2`, and public so Linux's tests hold it to the same rules.
    #[doc(hidden)]
    pub fn child_stepwise(&self, relative: &Path) -> io::Result<Dir> {
        let mut dir = Dir::from_fd(rustix::fs::openat(&self.fd, ".", DIRECTORY, Mode::empty())?)?;
        for component in relative.components() {
            let name = match component {
                Component::CurDir => continue,
                Component::Normal(name) => name,
                _ => {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "not beneath"));
                }
            };
            let next = Dir::from_fd(rustix::fs::openat(&dir.fd, name, DIRECTORY, Mode::empty())?)?;
            if dir.foreign(&next) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "another filesystem",
                ));
            }
            dir = next;
        }
        Ok(dir)
    }

    /// Opens one regular file of this directory, to read or to read and
    /// write, never through a symlink and never waiting on a pipe someone
    /// left in its place.
    pub(crate) fn file(&self, name: &OsStr, write: bool) -> io::Result<OwnedFd> {
        let access = if write { OFlags::RDWR } else { OFlags::RDONLY };
        let flags = access | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK;
        let fd = rustix::fs::openat(&self.fd, name, flags, Mode::empty())?;
        let kind = FileType::from_raw_mode(rustix::fs::fstat(&fd)?.st_mode);
        if kind != FileType::RegularFile {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a regular file",
            ));
        }
        Ok(fd)
    }

    /// Whether an entry of this directory is a directory, without following
    /// it, and `None` when it's gone.
    pub(crate) fn kind(&self, name: &OsStr) -> Option<FileType> {
        let stat = rustix::fs::statat(&self.fd, name, AtFlags::SYMLINK_NOFOLLOW).ok()?;
        Some(FileType::from_raw_mode(stat.st_mode))
    }

    /// Opens one entry of this directory, if it's a directory on the same
    /// filesystem.
    pub(crate) fn entry(&self, name: &OsStr) -> io::Result<Option<Dir>> {
        let fd = rustix::fs::openat(&self.fd, name, DIRECTORY, Mode::empty())?;
        let child = Dir::from_fd(fd)?;
        Ok((child.dev == self.dev).then_some(child))
    }

    /// Opens a directory entry to reclaim it whole: on this filesystem, or
    /// a subvolume of its own that `pool::beneath` lets through, as the
    /// harness makes each quest's scratch. Emptying it still never enters
    /// another subvolume within, such as a tree cloned there.
    pub(crate) fn whole_entry(&self, name: &OsStr) -> io::Result<Option<Dir>> {
        let fd = rustix::fs::openat(&self.fd, name, DIRECTORY, Mode::empty())?;
        let child = Dir::from_fd(fd)?;
        Ok((!self.foreign(&child)).then_some(child))
    }

    /// Removes one entry all the way down, as emptying a trash does.
    pub(crate) fn remove_entry(&self, name: &OsStr) -> io::Result<()> {
        remove(self, name, 0, &mut || {})
    }

    /// Removes one entry all the way down, calling `unlinked` after each
    /// entry goes.
    pub(crate) fn remove_entry_paced(
        &self,
        name: &OsStr,
        unlinked: &mut dyn FnMut(),
    ) -> io::Result<()> {
        remove(self, name, 0, unlinked)
    }

    /// Removes a directory only if nothing is in it: true if it went.
    pub(crate) fn remove_if_empty(&self, name: &OsStr) -> io::Result<bool> {
        match rustix::fs::unlinkat(&self.fd, name, AtFlags::REMOVEDIR) {
            Ok(()) => Ok(true),
            Err(Errno::NOTEMPTY | Errno::EXIST) => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    /// The names in this directory, without `.` and `..`.
    pub(crate) fn names(&self) -> io::Result<Vec<Vec<u8>>> {
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
    /// filesystem, with each file counted once however many links it has,
    /// and each extent shared among its files once.
    /// What vanishes or can't be read during the walk, as a running build
    /// replaces its files, is left out rather than failing the count.
    pub fn size(&self) -> io::Result<u64> {
        self.size_counted(&mut Counted::default())
    }

    /// The bytes allocated beneath this directory, as `size` says, less any
    /// already in `counted`, which takes in what's counted here. Measuring
    /// several directories through one counts what they share once, for
    /// the first measured, as deleting all of them frees it once.
    pub fn size_counted(&self, counted: &mut Counted) -> io::Result<u64> {
        self.size_into(counted)
    }

    /// Moves every entry but those kept into a fresh trash inside this
    /// directory. If any entry won't move, those already moved go back and
    /// the failure is returned.
    pub fn detach(&self, keep: &[&str]) -> io::Result<Detached<'_>> {
        let names: Vec<Vec<u8>> = self
            .names()?
            .into_iter()
            .filter(|name| !name.starts_with(TRASH))
            .filter(|name| !keep.iter().any(|kept| kept.as_bytes() == name.as_slice()))
            .collect();
        let trash = self.make_trash()?;
        let mut detached = Detached {
            dir: self,
            name: trash,
            modes: Vec::new(),
        };
        let into = self.writable(&detached.name)?;
        for (moved, name) in names.iter().enumerate() {
            let name = OsStr::from_bytes(name);
            if let Err(error) = self.move_into(name, &into, &mut detached.modes) {
                // What failed to move is still in place; the rest goes back
                // if it can, and only what's back gets its mode back.
                let mut back = vec![name.to_owned()];
                for earlier in &names[..moved] {
                    let earlier = OsStr::from_bytes(earlier);
                    if rename(&into, earlier, self, earlier).is_ok() {
                        back.push(earlier.to_owned());
                    }
                }
                for (name, mode) in detached
                    .modes
                    .iter()
                    .filter(|(name, _)| back.contains(name))
                {
                    let _ = self.set_mode(name, *mode);
                }
                drop(into);
                let _ = rustix::fs::unlinkat(&self.fd, &detached.name, AtFlags::REMOVEDIR);
                return Err(error);
            }
        }
        Ok(detached)
    }

    /// A new trash directory, named so it can't be one that exists.
    fn make_trash(&self) -> io::Result<OsString> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos());
        for attempt in 0..16 {
            let mut name = TRASH.to_vec();
            name.extend(format!("{}-{}", std::process::id(), nanos + attempt).as_bytes());
            let name = OsString::from_vec(name);
            match rustix::fs::mkdirat(&self.fd, &name, Mode::RWXU) {
                Ok(()) => return Ok(name),
                // A name another took is passed over for the next.
                Err(Errno::EXIST) => {}
                Err(error) => return Err(error.into()),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "no free trash name",
        ))
    }

    /// The trash directories in this directory, left by a pass that never
    /// finished.
    pub fn leftovers(&self) -> io::Result<Vec<Detached<'_>>> {
        let mut found = Vec::new();
        for name in self.names()? {
            if !name.starts_with(TRASH) {
                continue;
            }
            let name = OsString::from_vec(name);
            let stat = rustix::fs::statat(&self.fd, &name, AtFlags::SYMLINK_NOFOLLOW)?;
            if FileType::from_raw_mode(stat.st_mode) == FileType::Directory {
                found.push(Detached {
                    dir: self,
                    name,
                    modes: Vec::new(),
                });
            }
        }
        Ok(found)
    }

    /// Opens a directory entry to change what's in it, making it readable
    /// and writable by its owner first if it isn't.
    fn writable(&self, name: &OsStr) -> io::Result<Dir> {
        let fd = match rustix::fs::openat(&self.fd, name, DIRECTORY, Mode::empty()) {
            Err(Errno::ACCESS) => {
                self.chmod_entry(name)?;
                rustix::fs::openat(&self.fd, name, DIRECTORY, Mode::empty())?
            }
            other => other?,
        };
        let dir = Dir::from_fd(fd)?;
        if dir.dev != self.dev {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "another filesystem",
            ));
        }
        let mode = rustix::fs::fstat(&dir.fd)?.st_mode;
        if mode & 0o700 != 0o700 {
            rustix::fs::fchmod(&dir.fd, Mode::from_raw_mode(mode | 0o700))?;
        }
        Ok(dir)
    }

    /// Makes a directory its owner can't even open usable.
    fn chmod_entry(&self, name: &OsStr) -> io::Result<()> {
        self.set_mode(name, Mode::RWXU.bits())
    }

    /// Moves one entry into a trash. A directory moving to a new parent
    /// needs its owner's write permission, since its `..` changes, so one
    /// refused for lack of it is given that and moved, and its mode noted.
    fn move_into(
        &self,
        name: &OsStr,
        into: &Dir,
        modes: &mut Vec<(OsString, RawMode)>,
    ) -> io::Result<()> {
        match rename(self, name, into, name) {
            Err(error) if error.kind() == io::ErrorKind::PermissionDenied => {
                let stat = rustix::fs::statat(&self.fd, name, AtFlags::SYMLINK_NOFOLLOW)?;
                if FileType::from_raw_mode(stat.st_mode) != FileType::Directory {
                    return Err(error);
                }
                let mode = stat.st_mode & 0o7777;
                self.set_mode(name, mode | 0o700)?;
                modes.push((name.to_owned(), mode));
                rename(self, name, into, name)
            }
            other => other,
        }
    }

    /// Sets a directory entry's mode, never through a symlink.
    #[cfg(not(target_os = "linux"))]
    fn set_mode(&self, name: &OsStr, mode: RawMode) -> io::Result<()> {
        let mode = Mode::from_raw_mode(mode);
        Ok(rustix::fs::chmodat(
            &self.fd,
            name,
            mode,
            AtFlags::SYMLINK_NOFOLLOW,
        )?)
    }

    /// Sets a directory entry's mode, never through a symlink: Linux has no
    /// fchmodat that refuses symlinks on every kernel, so the entry is
    /// opened as a path, which opens nothing, and changed through that.
    #[cfg(target_os = "linux")]
    fn set_mode(&self, name: &OsStr, mode: RawMode) -> io::Result<()> {
        let flags = OFlags::PATH | DIRECTORY.difference(OFlags::RDONLY);
        let path = rustix::fs::openat(&self.fd, name, flags, Mode::empty())?;
        let link = format!("/proc/self/fd/{}", std::os::fd::AsRawFd::as_raw_fd(&path));
        Ok(rustix::fs::chmod(link.as_str(), Mode::from_raw_mode(mode))?)
    }

    fn size_into(&self, counted: &mut Counted) -> io::Result<u64> {
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
            let inode = (stat.st_dev, stat.st_ino);
            if stat.st_nlink > 1 && kind != FileType::Directory && !counted.inodes.insert(inode) {
                continue;
            }
            // The kernel never counts a negative number of blocks.
            let blocks = u64::try_from(stat.st_blocks).unwrap_or(0) * 512;
            let shared = if kind == FileType::RegularFile && blocks >= SHARED_FROM {
                self.file(name, false)
                    .map_or(0, |file| counted.again(&file))
            } else {
                0
            };
            total += blocks.saturating_sub(shared);
            if kind == FileType::Directory
                && let Ok(Some(child)) = self.entry(name)
            {
                total += child.size_into(counted).unwrap_or(0);
            }
        }
        Ok(total)
    }
}
